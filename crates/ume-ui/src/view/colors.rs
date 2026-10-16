//! The syntax coloring of the view: the color state at the start of each line, kept up to date as
//! the document is edited and made in the background for a large document.
//!
//! `ume-coloring` has the strategies and [`ColorStates`]; this is the glue to the view, in the
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
//!   above them (see [`ume_coloring::ColorStates::state_at`]), so the colors of the lines far
//!   down may change once as the states arrive. Sakura makes all states before the first paint.

use std::time::Duration;

use gpui::{Context, Window};
use ume_coloring::LineEdit;
use ume_commands::context::Changes;

use super::EditorView;

/// How long one frame, or one edit, may spend on the states of the lines.
const COLORS_BUDGET: Duration = Duration::from_millis(4);

impl EditorView {
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
        if self.color_states.is_complete(shared.doc(), &rules) {
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
