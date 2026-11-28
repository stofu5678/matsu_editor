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
use matsu_coloring::{
    ColorRules, ColorRun, ColorState, ColorStates, LineEdit, line_end_state, line_runs,
};
use matsu_commands::context::Changes;
use matsu_commands::input_options::{LineStart, LineStartColors};
use matsu_layout::{LineLayout, RowInfo};

use super::EditorView;

/// How long one frame, or one edit, may spend on the states of the lines.
const COLORS_BUDGET: Duration = Duration::from_millis(4);

/// A line this long (in bytes) has its colors kept between frames: scanning it from its start
/// takes long enough to be felt at every blink of the caret.
const LONG_LINE: usize = 2048;

/// The colors of the last long line that was painted, with what they were made from.
#[derive(Default)]
pub(crate) struct LineColorCache(RefCell<Option<CachedLine>>);

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
}

/// The states of the view as the C/C++ smart indent reads them (`CLayout::GetColorTypePrev`): the string or
/// the block comment 1 that a line starts in.
pub(super) struct StartStates<'a> {
    pub states: &'a ColorStates,
    pub rules: &'a ColorRules,
}

impl LineStartColors for StartStates<'_> {
    fn at_line_start(&self, doc: &matsu_core::buffer::Document, line: usize) -> LineStart {
        match self.states.state_at(doc, self.rules, line).0 {
            ColorState::Block(0) => LineStart::BlockComment,
            ColorState::Quote { double: true, .. } => LineStart::DoubleQuote,
            ColorState::Quote { double: false, .. } => LineStart::SingleQuote,
            _ => LineStart::Normal,
        }
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
    /// The length in bytes of the line as the file has it, for a document that is read by ranges
    /// ([`Document::line_raw_len`]); `None` for any other. A line of
    /// [`matsu_coloring::SKIP_LINE_BYTES`] or more has no colors there (ADR-0020, 9.10), by the
    /// same measure as the states use, so that the colors do not depend on what is on the screen.
    pub raw_len: Option<usize>,
}

/// The `raw_len` of a request for `line` of `doc`.
pub(super) fn raw_len_of(doc: &matsu_core::buffer::Document, line: usize) -> Option<usize> {
    if doc.is_ranged() {
        doc.line_raw_len(line).ok().flatten()
    } else {
        None
    }
}

/// Whether the colors of `line` are made row by row, from the text of the row that is shown: a
/// line of 1 MiB or more of a document that is read whole ([`Document::is_long_line`]). Its rows
/// are cut at fixed places, and only the rows on the screen are read, so the line is not scanned
/// from its start (#614). A document read by ranges has no colors in such a line instead
/// ([`LineColorRequest::raw_len`]).
pub(super) fn is_colored_by_row(doc: &matsu_core::buffer::Document, line: usize) -> bool {
    !doc.is_ranged() && doc.is_long_line(line)
}

/// Where the scan of the rows of a long line ended last: the state it left, to start the next row
/// of the same line in. The first row on the screen starts in the state the line starts in, so a
/// comment or a string that began before the screen is not seen.
pub(super) struct RowCarry {
    line: usize,
    /// The column where the last scanned row ended.
    col: usize,
    state: ColorState,
}

/// The runs of the colors of one row of a long line (see [`is_colored_by_row`]), with the columns
/// of the line. `line_start` makes the state the line starts in, which the first row asks for.
pub(super) fn colored_row(
    rules: &ColorRules,
    row: &RowInfo,
    carry: &mut Option<RowCarry>,
    line_start: impl FnOnce() -> ColorState,
) -> Arc<Vec<ColorRun>> {
    let state = match carry.take() {
        Some(c) if c.line == row.line && c.col == row.start.col => c.state,
        _ => line_start(),
    };
    let text = row.text();
    let mut runs = line_runs(rules, text, row.eol, &state, text.len());
    for run in &mut runs {
        run.start += row.start.col;
        run.end += row.start.col;
    }
    *carry = Some(RowCarry {
        line: row.line,
        col: row.end.col,
        state: line_end_state(rules, text, row.eol, &state),
    });
    Arc::new(runs)
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
            raw_len,
        } = request;
        let _measure = matsu_core::measure::span("ui.colored_line");
        // No colors for a line of a very long line (and so no scan of it).
        if raw_len.is_some_and(|n| n >= matsu_coloring::SKIP_LINE_BYTES) {
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
                hit.next =
                    next_known.or_else(|| Some(line_end_state(rules, &ll.text, ll.eol, state)));
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
        let _measure = matsu_core::measure::span("ui.colors_after_edit");
        let rules = self.prefs.settings.coloring.clone();
        let doc = self.document.read(cx).doc();
        if self.color_version + 1 != version {
            // Something else changed the document too: start again.
            self.color_states.reset();
        } else if changes.replaced_all() {
            self.color_states.reset();
        } else if let [edit] = changes.edits() {
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
        } else if let Some(first) = changes.edits().iter().map(|e| e.start.line).min() {
            // Several edits in one command: their lines are not of one document.
            self.color_states.invalidate_after(first);
        }
        self.color_version = version;
    }
}
