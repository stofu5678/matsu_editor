//! Grep 置換 of the text of a window (`:HWND:`, 「この編集中のテキストから検索する」): the engine plans
//! the edits on a snapshot, and this makes them in the document, in one undo step as the
//! 「すべて置換」 of the replace dialog does ([`EditorView::replace_all`]).
//!
//! Sakura replaces nothing here and only prints the count (`CGrepAgent::DoGrep` reaches
//! `DoGrepFile`, not `DoGrepReplaceFile`, for a window). The count says 置換しました, so MATSU
//! makes the replacement: type A of ADR-0018 (see [`super::engine`], `replace_window`).

use gpui::Context;
use matsu_commands::command::CommandError;
use matsu_core::buffer::{Document, Pos};
use matsu_core::search::AppliedEdit;

use super::engine::TextShift;
use crate::file_ops::windows::editors;
use crate::search::ops::apply_edits;
use crate::view::EditorView;

/// What the result window says when the document changed while the search ran: the edits were
/// planned for the text that was read, so none of them is made.
pub(crate) const CHANGED: &str =
    "編集中のテキストが検索の間に変わったので、置換しませんでした。もう一度実行してください。";

impl EditorView {
    /// Makes the edits of a Grep 置換 in the document that was searched (`replace_target`: its
    /// id, and the version the search read). The window that owns it may be this one.
    pub(super) fn replace_in_window(
        &mut self,
        edits: &[AppliedEdit],
        shifts: &[TextShift],
        cx: &mut Context<Self>,
    ) {
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
                    view.update(cx, |target, cx| {
                        target.replace_planned(version, edits, shifts, cx)
                    })
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
        shifts: &[TextShift],
        cx: &mut Context<Self>,
    ) -> bool {
        if self.document.read(cx).version() != version {
            return false;
        }
        self.run_with(cx, |c| -> Result<(), CommandError> {
            c.begin_block();
            // The caret goes with the text, as it does for 「すべて置換」: one before a match stays where
            // it is, one after moves with the text. The selection is dropped. The place is worked
            // out on the text of the lines (the bytes of the text and the breaks), and then found in
            // the document that the edits made, so it is never between a CR and an LF, and never
            // outside the document.
            let caret = c.caret();
            let before = text_offset(c.doc(), caret);
            // A box selection refuses edits: the selection goes first (the caret is put back below).
            let made = c.move_caret(caret, false).and_then(|()| {
                apply_edits(c, edits)?;
                let at = match before {
                    Some(offset) => pos_at(c.doc(), moved_offset(offset, shifts)),
                    None => caret,
                };
                c.move_caret(at, false)
            });
            c.end_block();
            made
        })
        .is_ok()
    }
}

/// Where `offset` (in the text of the lines of a document and their breaks) is after the matches
/// that `shifts` (in order) changed: before a match it stays, after it it moves by what the match
/// added, inside it it goes to where the new text of the match begins.
fn moved_offset(offset: usize, shifts: &[TextShift]) -> usize {
    let mut delta: i64 = 0;
    for shift in shifts {
        if offset <= shift.at {
            break;
        }
        if offset < shift.at + shift.old_len {
            return shift.at.saturating_add_signed(delta as isize);
        }
        delta += shift.new_len as i64 - shift.old_len as i64;
    }
    offset.saturating_add_signed(delta as isize)
}

/// The offset of `pos` in the text of the lines of `doc` and their breaks (a line is its text and
/// its break), or `None` if a line cannot be read.
fn text_offset(doc: &Document, pos: Pos) -> Option<usize> {
    let mut offset = 0;
    for index in 0..pos.line {
        let line = doc.line(index).ok()??;
        offset += line.text.len()
            + line
                .eol
                .map_or(0, |eol| matsu_core::model::eol_text(eol).len());
    }
    Some(offset + pos.col)
}

/// The position of `offset` in the text of the lines of `doc` and their breaks: in the line that
/// has it, at most at the end of the text of the line (an offset in a break is the end of the
/// line), or the end of the document.
fn pos_at(doc: &Document, offset: usize) -> Pos {
    let mut start = 0;
    for index in 0..doc.line_count() {
        let Ok(Some(line)) = doc.line(index) else {
            break;
        };
        let eol_len = line
            .eol
            .map_or(0, |eol| matsu_core::model::eol_text(eol).len());
        let end = start + line.text.len() + eol_len;
        if offset < end {
            return Pos::new(index, (offset - start).min(line.text.len()));
        }
        start = end;
    }
    doc.end_pos().unwrap_or_else(|_| Pos::new(0, 0))
}
