//! The syntax coloring of the view: the color state at the start of each line, kept up to date as
//! the document is edited and made in the background for a large document.
//!
//! `matsu-coloring` has the strategies and [`ColorStates`]; this is the glue to the view, in the
//! places where the layout of the view is kept up to date too:
//!
//! * a command that edited the text tells the states ([`EditorView::colors_after_edit`], like
//!   `RowLayout::edited` tells the layout); they are made again from the first changed line
//!   until a line starts in the state it had;
//! * a text that changed in another way (another view of the document, the file loaded again)
//!   starts the states again ([`EditorView::advance_colors`] notices by the version of the
//!   document), and so do other rules (`apply_settings`, for a change of the document type);
//! * the states of a large document are made a few milliseconds per frame, like its layout, and
//!   the lines before the point they have reached are painted from a scan of a window of lines
//!   above them (see [`matsu_coloring::ColorStates::state_at`]), so the colors of the lines far
//!   down may change once as the states arrive. Sakura makes all states before the first paint.

use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;

use gpui::{Context, Window};
use matsu_coloring::{ColorRules, ColorRun, ColorState, LineEdit, line_end_state, line_runs};
use matsu_commands::context::Changes;
use matsu_layout::LineLayout;

use super::EditorView;

/// How long one frame, or one edit, may spend on the states of the lines.
const COLORS_BUDGET: Duration = Duration::from_millis(4);

/// A line this long (in bytes) has its colors kept between frames: scanning it from its start
/// takes long enough to be felt at every blink of the caret.
const LONG_LINE: usize = 2048;

/// A line of this many bytes is not colored in a file that is read by ranges (ADR-0020, 9.10):
/// the lines made of a very long line are about 1 MiB each, and scanning one at every frame would
/// make the window crawl. The states skip them too.
const RANGED_NO_COLOR_BYTES: usize = matsu_coloring::SKIP_LINE_BYTES;

/// The colors of the last long line that was painted, with what they were made from, and whether
/// the document is read by ranges.
#[derive(Default)]
pub(crate) struct LineColorCache(RefCell<Option<CachedLine>>, std::cell::Cell<bool>);

struct CachedLine {
    version: u64,
    line: usize,
    state: ColorState,
    upto: usize,
    runs: Arc<Vec<ColorRun>>,
    /// The state the next line starts in, once it was asked for.
    next: Option<ColorState>,
}

impl LineColorCache {
    pub(crate) fn clear(&self) {
        self.0.borrow_mut().take();
    }

    /// The document is read by ranges (or is not): see [`RANGED_NO_COLOR_BYTES`].
    pub(crate) fn set_ranged(&self, ranged: bool) {
        self.1.set(ranged);
    }
}

/// What [`EditorView::colored_line`] is asked for.
pub(super) struct LineColorRequest<'a> {
    pub rules: &'a ColorRules,
    /// The version of the document.
    pub version: u64,
    pub line: usize,
    pub layout: &'a LineLayout,
    /// The state the line starts in.
    pub state: &'a ColorState,
    /// The end of the last row of the line on the screen, in bytes.
    pub upto: usize,
    /// The state of the next line is needed (a next line is on the screen).
    pub want_next: bool,
    /// The state of the next line, when the states have it already: it saves scanning the line a
    /// second time.
    pub next_known: Option<ColorState>,
}

/// The colors of a line for the painting.
pub(super) struct ColoredLine {
    /// The runs from the start of the line to the end of the last row on the screen.
    pub runs: Arc<Vec<ColorRun>>,
    /// The state the next line starts in, if it was asked for.
    pub next: Option<ColorState>,
}

impl EditorView {
    /// The runs of the colors of a line from its start up to the end of its last row on the
    /// screen, and the state the next line starts in if that is wanted. The line is scanned from
    /// its start, as Sakura paints a row; the result for a long line is kept for the next frame,
    /// so that a blinking caret does not scan it again.
    pub(super) fn colored_line(&self, request: LineColorRequest<'_>) -> ColoredLine {
        let LineColorRequest {
            rules,
            version,
            line,
            layout: ll,
            state,
            upto,
            want_next,
            next_known,
        } = request;
        // No colors for a line of a very long line (and so no scan of it).
        if self.line_color_cache.1.get() && ll.text.len() >= RANGED_NO_COLOR_BYTES {
            return ColoredLine {
                runs: Arc::new(Vec::new()),
                next: want_next.then(|| next_known.unwrap_or_else(|| state.clone())),
            };
        }
        let long = ll.text.len() >= LONG_LINE;
        if long
            && let Some(hit) = self.line_color_cache.0.borrow_mut().as_mut().filter(|c| {
                c.version == version && c.line == line && c.state == *state && c.upto == upto
            })
        {
            if want_next && hit.next.is_none() {
                hit.next = next_known
                    .clone()
                    .or_else(|| Some(line_end_state(rules, &ll.text, ll.eol, state)));
            }
            return ColoredLine {
                runs: hit.runs.clone(),
                next: hit.next.clone(),
            };
        }
        let runs = Arc::new(line_runs(rules, &ll.text, ll.eol, state, upto));
        let next = want_next
            .then(|| next_known.unwrap_or_else(|| line_end_state(rules, &ll.text, ll.eol, state)));
        if long {
            *self.line_color_cache.0.borrow_mut() = Some(CachedLine {
                version,
                line,
                state: state.clone(),
                upto,
                runs: runs.clone(),
                next: next.clone(),
            });
        }
        ColoredLine { runs, next }
    }

    /// Makes sure the states match the document and the rules, and makes more of them for a
    /// large document. Called before each frame; asks for another frame while there is more to do.
    pub(crate) fn advance_colors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rules = self.prefs.settings.coloring.clone();
        let shared = self.document.read(cx);
        let version = shared.version();
        if self.color_version != version {
            self.color_states.reset();
            self.color_version = version;
        }
        // A file read by ranges has no states made from the top: the colors of the lines on the
        // screen come from a window of lines above them (`ColorStates::state_at`), and a comment
        // that began further up is missed (ADR-0020, chapter 6).
        if shared.is_ranged() || self.color_states.is_complete(shared.doc(), &rules) {
            return;
        }
        let done = self
            .color_states
            .advance(shared.doc(), &rules, COLORS_BUDGET);
        if !done {
            window.request_animation_frame();
        }
    }

    /// The text changed by the commands of `changes` (the document is at `version` now): brings the
    /// states along, so that the next frame does not start them again.
    pub(crate) fn colors_after_edit(
        &mut self,
        changes: &Changes,
        version: u64,
        cx: &mut Context<Self>,
    ) {
        let rules = self.prefs.settings.coloring.clone();
        let doc = self.document.read(cx).doc();
        if self.color_version + 1 != version {
            // Something else changed the document too: start again.
            self.color_states.reset();
        } else if changes.replaced_all {
            self.color_states.reset();
        } else if let [edit] = changes.edits.as_slice() {
            self.color_states.edited(
                doc,
                &rules,
                LineEdit {
                    first: edit.start.line,
                    old_last: edit.old_end.line,
                    new_last: edit.new_end.line,
                },
                COLORS_BUDGET,
            );
        } else if let Some(first) = changes.edits.iter().map(|e| e.start.line).min() {
            // Several edits in one command: their lines are not of one document.
            self.color_states.invalidate_after(first);
        }
        self.color_version = version;
    }
}
