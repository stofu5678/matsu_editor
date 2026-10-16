//! Box selection (Sakura's 矩形選択): a selection between two display columns and two rows.
//!
//! Sakura keeps a box as the same range as a normal selection (`CViewSelect::m_sSelect`, in layout
//! coordinates) and a flag (`IsBoxSelecting`); the range is read as the corners of a rectangle.
//! A corner can be to the right of the end of its row, where there is no text: while a box is
//! selected the caret may go there (`CCaret::Cursor_UPDOWN`, `Command_RIGHT`). The cursor keeps
//! the display column of each corner for that ([`Cursor::move_box_to`]); the position is then the
//! end of the row.
//!
//! The moves of the box commands are the ordinary moves ([`CommandContext::move_left`] and the
//! others) run on a cursor that is in box mode; this file has the box versions of the moves
//! whose rules differ, and [`CommandContext::begin_box`].
//!
//! Sakura: `CViewCommander::Command_BEGIN_BOXSELECT` (`cmd/CViewCommander_Select.cpp`),
//! `Sub_BoxSelectLock` (`cmd/CViewCommander.cpp`), `Command_LEFT`, `Command_RIGHT`
//! (`cmd/CViewCommander_Cursor.cpp`) and `CCaret::Cursor_UPDOWN` (`view/CCaret.cpp`).

use ume_core::buffer::Pos;
use ume_core::cursor::char_left;

use super::CommandContext;
use crate::command::Result;

/// The rightmost display column of a box caret: Sakura's `GetMaxLineLayout()`, which is the
/// longest line (`m_nMaxLineKetas`, 10240 columns by default, `MAXLINEKETAS`). Without wrapping
/// at the width of the window that is the limit; this crate has no setting for it yet.
pub const MAX_BOX_COL: usize = 10240;

impl CommandContext<'_> {
    /// `F_BEGIN_BOX`: starts a box selection at the caret. A selection that was there is dropped.
    /// Nothing is selected until the caret moves. The box is locked, so that the plain moves
    /// extend it (Sakura: `Command_BEGIN_BOXSELECT(true)`; the text must be in a fixed-pitch
    /// font, which the view here always is).
    pub fn begin_box(&mut self) -> Result<()> {
        self.start_box(true)
    }

    /// What a box move does first: starts a box at the caret unless one is going on (Sakura:
    /// `Sub_BoxSelectLock`, with the "lock on box moves" option, which is on by default,
    /// `m_bBoxSelectLock` in `sakura_core/env/CShareData.cpp`; the macro arguments that change it
    /// are not supported). A normal selection is dropped, as Sakura does since 2014.
    pub fn ensure_box(&mut self) -> Result<()> {
        if self.cursor.is_box() {
            Ok(())
        } else {
            self.start_box(true)
        }
    }

    fn start_box(&mut self, lock: bool) -> Result<()> {
        let col = self.box_caret_col()?;
        self.cursor.begin_box(col, lock);
        Ok(())
    }

    /// The display column of the caret: for a box selection the column of its corner, which can
    /// be past the end of the text; otherwise that of the caret position.
    pub fn box_caret_col(&self) -> Result<usize> {
        let at = self.caret_display_col()?;
        Ok(self.cursor.box_caret_col().map_or(at, |c| c.max(at)))
    }

    /// Whether the caret of a box selection is to the right of the end of the text of its row.
    pub(super) fn caret_is_in_empty_space(&self) -> Result<bool> {
        Ok(self.cursor.is_box() && self.box_caret_col()? > self.caret_display_col()?)
    }

    /// Moves the box caret to display column `col` of the row of `row_pos`: onto the char that
    /// covers the column, or into the empty space after the text.
    pub(super) fn box_move_to_col(&mut self, row_pos: Pos, col: usize) -> Result<()> {
        let pos = self.layout.pos_at_col(self.doc(), row_pos, col)?;
        let at = self.layout.display_col(self.doc(), pos)?;
        let in_space = pos == self.layout.row_end(self.doc(), pos)? && at < col;
        self.cursor
            .move_box_to(pos, if in_space { col } else { at });
        Ok(())
    }

    /// One column left; through the empty space, then one char at a time over the text. The caret
    /// does not go to the previous row.
    pub(super) fn box_left(&mut self) -> Result<()> {
        let (caret, col) = (self.caret(), self.box_caret_col()?);
        let end = self.layout.row_end(self.doc(), caret)?;
        if col > self.layout.display_col(self.doc(), end)? {
            self.cursor.move_box_to(end, col - 1);
            return Ok(());
        }
        // inside the text: the previous char of the row, if there is one
        if caret > self.layout.row_start(self.doc(), caret)?
            && let Some(prev) = char_left(self.doc(), caret)?
        {
            let col = self.layout.display_col(self.doc(), prev)?;
            self.cursor.move_box_to(prev, col);
        }
        Ok(())
    }

    /// One char right over the text, then one column at a time through the empty space up to
    /// [`MAX_BOX_COL`]. The caret does not go to the next row.
    pub(super) fn box_right(&mut self) -> Result<()> {
        let (caret, col) = (self.caret(), self.box_caret_col()?);
        let end = self.layout.row_end(self.doc(), caret)?;
        if col >= self.layout.display_col(self.doc(), end)? {
            if col < MAX_BOX_COL {
                self.cursor.move_box_to(end, col + 1);
            }
            return Ok(());
        }
        if let Some(next) = ume_core::cursor::char_right(self.doc(), caret)? {
            let col = self.layout.display_col(self.doc(), next)?;
            self.cursor.move_box_to(next, col);
        }
        Ok(())
    }

    /// `rows` rows down (negative: up) to the goal column, which may be past the end of the
    /// text of the row there. Moving down never goes onto the empty row after a final line
    /// ending when the caret is not in the first column, so that the column does not change
    /// (Sakura: `CCaret::Cursor_UPDOWN`).
    pub(super) fn box_vertical(&mut self, rows: isize) -> Result<()> {
        let caret_col = self.box_caret_col()?;
        let goal = self.cursor.goal_col().unwrap_or(caret_col);
        let caret = self.caret();
        let mut target = self.layout.move_rows(self.doc(), caret, rows, goal)?;
        let eof_row = self.doc().line_count();
        if rows > 0 && caret_col != 0 && target.line >= eof_row {
            target = if caret.line >= eof_row {
                self.layout.move_rows(self.doc(), caret, 0, goal)?
            } else {
                self.layout.move_rows(self.doc(), target, -1, goal)?
            };
        }
        self.box_move_to_col(target, goal)?;
        self.cursor.set_goal_col(Some(goal));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ume_core::buffer::Pos;

    use crate::command::Arg;
    use crate::testing::Editor;

    #[test]
    fn edits_beep_and_change_nothing_while_a_box_is_selected() {
        let mut e = Editor::new("ab|cd\nefgh");
        e.run("begin_box").run("cursor_down");
        for id in ["delete", "delete_back", "word_delete", "tab_to_space"] {
            e.run(id);
            assert_eq!(e.model().doc().line_count(), 2, "{id}");
            assert_eq!(e.box_state(), Some(((0, 2), (1, 2))), "{id}");
        }
        e.run_with("input_char", &[Arg::Char('x')]);
        assert!(e.changes().beep);
        assert!(!e.changes().text_changed());
        assert_eq!(e.box_state(), Some(((0, 2), (1, 2))));
        e.run("cut").run("copy");
        assert!(e.clipboard().is_none());
        e.run("undo");
        e.assert_box((0, 2), (1, 2));
    }

    #[test]
    fn editing_in_the_empty_space_of_a_box_beeps() {
        // Sakura would pad the row with spaces to the column of the caret
        let mut e = Editor::new("ab|");
        e.run("cursor_right_box");
        e.run_with("input_char", &[Arg::Char('x')]);
        assert!(e.changes().beep);
        // the text is "ab"; the anchor and the caret are both at its end
        assert_eq!(e.marked(), "ab[|");
        e.assert_box((0, 2), (0, 3));
    }

    #[test]
    fn a_move_inside_the_empty_space_is_reported_as_a_move() {
        let mut e = Editor::new("ab|");
        e.run("cursor_right_box");
        assert!(e.changes().caret_moved);
        assert!(e.changes().selection_changed);
        e.run("cursor_right_box");
        // the position is the same, the column is not
        assert!(e.changes().caret_moved);
        assert!(!e.changes().selection_changed);
    }

    #[test]
    fn a_normal_selection_move_in_a_box_goes_on_with_the_box() {
        // Shift+Down in a box is the same as the box move
        let mut e = Editor::new("ab|cd\nefgh");
        e.run("begin_box").run("cursor_down_sel");
        e.assert_box((0, 2), (1, 2));
        assert!(e.cursor().is_box());
    }

    #[test]
    fn a_normal_selection_becomes_a_box_at_the_caret() {
        let mut e = Editor::new("[abcd|\nefgh");
        e.run("cursor_down_box");
        e.assert_box((0, 4), (1, 4));
        assert_eq!(e.cursor().anchor(), Some(Pos::new(0, 4)));
    }
}
