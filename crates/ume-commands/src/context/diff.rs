//! DIFF marks (Sakura's DIFF差分表示): setting and clearing the marks of the lines, and the jumps
//! to the next and the previous marked line. The marks are kept in the document
//! ([`ume_core::diff`]), which also computes the difference.
//!
//! Sakura: `Command_Diff_Next`, `Command_Diff_Prev` and `Command_Diff_Reset`
//! (`sakura_core/cmd/CViewCommander_Diff.cpp`), `CDiffLineMgr` (`sakura_core/docplus/CDiffManager.cpp`).

use ume_core::buffer::Pos;
use ume_core::diff::Hunk;
use ume_core::search::Direction;

use super::CommandContext;
use crate::command::Result;

/// How a jump to a DIFF mark ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffJump {
    /// The caret is on the marked line now. `wrapped`: the search went on from the other end of
    /// the file (`STR_ERR_SRNEXT1`, `STR_ERR_SRPREV1` in the status bar).
    Found { wrapped: bool },
    /// There is no marked line in that direction.
    NotFound,
}

impl CommandContext<'_> {
    /// `F_DIFF_RESET`: removes every DIFF mark (`Command_Diff_Reset`).
    pub fn clear_diff_marks(&mut self) {
        self.model.diff_marks_mut().clear();
        // the views of the document draw the marks too (the flag that repaints the bookmarks)
        self.changes.bookmarks_changed = true;
    }

    /// Marks the lines for the hunks of a comparison, after clearing the marks that were there
    /// (`ViewDiffInfo`: "今あるDIFF差分を消去する"). `own_is_old`: the hunks go from this
    /// document to the other text. Returns whether any line got a mark (`IsDiffUse`).
    pub fn set_diff_marks(&mut self, hunks: &[Hunk], own_is_old: bool) -> bool {
        let line_count = self.doc().line_count();
        self.model
            .diff_marks_mut()
            .replace_with_hunks(hunks, own_is_old, line_count);
        self.changes.bookmarks_changed = true;
        self.model.diff_marks().in_use()
    }

    /// `F_DIFF_NEXT` and `F_DIFF_PREV`: moves the caret to the start of the next (or previous)
    /// marked line, as a plain move (it extends the selection while the selection is locked).
    /// With `wrap` (Sakura's option 先頭（末尾）から再検索, `m_bSearchAll`) a search that finds
    /// nothing goes on from the other end of the file.
    pub fn jump_to_diff(&mut self, direction: Direction, wrap: bool) -> Result<DiffJump> {
        let line = self.caret().line;
        let marks = self.model.diff_marks();
        let (found, wrapped) = match direction {
            Direction::Forward => match marks.next_after(line) {
                Some(l) => (Some(l), false),
                None if wrap => (marks.first(), true),
                None => (None, false),
            },
            Direction::Backward => match marks.prev_before(line) {
                Some(l) => (Some(l), false),
                None if wrap => (marks.last(), true),
                None => (None, false),
            },
        };
        let Some(target) = found else {
            return Ok(DiffJump::NotFound);
        };
        let extend = self.extends(false);
        self.move_caret(Pos::new(target, 0), extend)?;
        Ok(DiffJump::Found { wrapped })
    }
}
