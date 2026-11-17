//! Grep 置換 of the text of a window (`:HWND:`, 「この編集中のテキストから検索する」): the engine plans
//! the edits on a snapshot, and this makes them in the document, in one undo step as the
//! 「すべて置換」 of the replace dialog does ([`EditorView::replace_all`]).
//!
//! Sakura replaces nothing here and only prints the count (`CGrepAgent::DoGrep` reaches
//! `DoGrepFile`, not `DoGrepReplaceFile`, for a window). The count says 置換しました, so MATSU
//! makes the replacement: type A of ADR-0018 (see [`super::engine`], `replace_window`).

use gpui::Context;
use matsu_commands::command::CommandError;
use matsu_core::search::AppliedEdit;

use crate::file_ops::windows::editors;
use crate::search::ops::{apply_edits, shift_start};
use crate::view::EditorView;

/// What the result window says when the document changed while the search ran: the edits were
/// planned for the text that was read, so none of them is made.
pub(crate) const CHANGED: &str =
    "編集中のテキストが検索の間に変わったので、置換しませんでした。もう一度実行してください。";

impl EditorView {
    /// Makes the edits of a Grep 置換 in the document that was searched (`replace_target`: its
    /// id, and the version the search read). The window that owns it may be this one.
    pub(super) fn replace_in_window(&mut self, edits: &[AppliedEdit], cx: &mut Context<Self>) {
        let Some((id, version)) = self.grep.replace_target.take() else {
            return;
        };
        let own = cx.entity_id();
        let made = if self.document.entity_id().as_u64() == id {
            // The result was written into this document while the search ran: it is not what was
            // read.
            Some(false)
        } else {
            editors(cx).into_iter().find_map(|(_, view)| {
                if view.entity_id() == own {
                    return None;
                }
                let found = view.read(cx).document().entity_id().as_u64() == id;
                found.then(|| {
                    view.update(cx, |target, cx| target.replace_planned(version, edits, cx))
                })
            })
        };
        // Told when the search ends: the result text written after this would clear the notice.
        self.grep.replace_refused = made != Some(true);
    }

    /// Makes `edits` as one undo step, if the document is still at `version`. Returns whether it
    /// did.
    fn replace_planned(
        &mut self,
        version: u64,
        edits: &[AppliedEdit],
        cx: &mut Context<Self>,
    ) -> bool {
        if self.document.read(cx).version() != version {
            return false;
        }
        self.run_with(cx, |c| -> Result<(), CommandError> {
            c.begin_block();
            // The caret goes with the text, as it does for 「すべて置換」: one before an edit stays where it
            // is, one after moves with the text. The selection is dropped.
            // `move_caret` keeps it inside the document, which a shorter line or a removed line
            // break would otherwise leave.
            let caret = c.caret();
            // A box selection refuses edits: the selection goes first (the caret is put back below).
            let made = c.move_caret(caret, false).and_then(|()| {
                let ends = apply_edits(c, edits)?;
                c.move_caret(shift_start(caret, edits, &ends), false)
            });
            c.end_block();
            made
        })
        .is_ok()
    }
}
