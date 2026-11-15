//! Box selection (Sakura's 矩形選択): a selection between two display columns and two rows.
//!
//! Sakura keeps a box as the same range as a normal selection (`CViewSelect::m_sSelect`, in layout
//! coordinates) and a flag (`IsBoxSelecting`); the range is read as the corners of a rectangle.
//! A corner can be to the right of the end of its row, where there is no text: while a box is
//! selected the caret may go there (`CCaret::Cursor_UPDOWN`, `Command_RIGHT`). The cursor keeps
//! the horizontal position ([`X`]) of each corner for that ([`Cursor::move_box_to`]); the position
//! is then the end of the row.
//!
//! The moves of the box commands are the ordinary moves ([`CommandContext::move_left`] and the
//! others) run on a cursor that is in box mode; this file has the box versions of the moves
//! whose rules differ, and [`CommandContext::begin_box`].
//!
//! Sakura: `CViewCommander::Command_BEGIN_BOXSELECT` (`cmd/CViewCommander_Select.cpp`),
//! `Sub_BoxSelectLock` (`cmd/CViewCommander.cpp`), `Command_LEFT`, `Command_RIGHT`
//! (`cmd/CViewCommander_Cursor.cpp`) and `CCaret::Cursor_UPDOWN` (`view/CCaret.cpp`).

use matsu_core::buffer::Pos;
use matsu_core::cursor::char_left;

use super::CommandContext;
use crate::command::Result;
use crate::layout::X;

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
        let x = self.box_caret_x()?;
        self.begin_box_at(x, lock);
        Ok(())
    }

    /// The `X` of the caret: for a box selection that of its corner, which can be past the end of
    /// the text; otherwise that of the caret position.
    pub fn box_caret_x(&self) -> Result<X> {
        let at = self.caret_x_with_free()?;
        Ok(self.stored_box_caret_x().map_or(at, |c| c.max(at)))
    }

    /// Whether the row of `row_pos` ends because the line wraps there. The end of such a row is
    /// the start of the next one for a position, so a box caret never rests at the end of a
    /// wrapped row, and there is no empty space to its right.
    pub(super) fn row_is_wrapped(&self, row_pos: Pos) -> Result<bool> {
        let end = self.layout.row_end(self.doc(), row_pos)?;
        Ok(self.layout.next_row_start(self.doc(), row_pos)? == Some(end))
    }

    /// Whether the caret of a box selection is to the right of the end of the text of its row.
    pub(super) fn caret_is_in_empty_space(&self) -> Result<bool> {
        let caret = self.caret();
        Ok(self.cursor.is_box()
            && !self.row_is_wrapped(caret)?
            && self.box_caret_x()? > self.caret_display_x()?)
    }

    /// Moves the box caret to `x` on the row of `row_pos`: onto the char that covers it, or into
    /// the empty space after the text. On a wrapped row, the last char of the row is as far as
    /// the caret goes.
    pub(super) fn box_move_to_x(&mut self, row_pos: Pos, x: X) -> Result<()> {
        let mut pos = self.layout.pos_at_x(self.doc(), row_pos, x)?;
        let end = self.layout.row_end(self.doc(), row_pos)?;
        let wrapped = self.row_is_wrapped(row_pos)?;
        if pos == end && wrapped {
            if let Some(last) = char_left(self.doc(), end)?
                && last >= self.layout.row_start(self.doc(), row_pos)?
            {
                pos = last;
            }
            let at = self.layout.display_x(self.doc(), pos)?;
            self.set_box_caret(pos, at);
            return Ok(());
        }
        let at = self.layout.display_x(self.doc(), pos)?;
        let in_space = pos == end && at < x;
        self.set_box_caret(pos, if in_space { x } else { at });
        Ok(())
    }

    /// Moves the box caret to the end of the text of its row (on a wrapped row, its last char).
    pub(super) fn box_move_to_row_end(&mut self) -> Result<()> {
        let caret = self.caret();
        let (start, end) = (
            self.layout.row_start(self.doc(), caret)?,
            self.layout.row_end(self.doc(), caret)?,
        );
        let mut pos = end;
        if self.row_is_wrapped(caret)?
            && let Some(last) = char_left(self.doc(), end)?
            && last >= start
        {
            pos = last;
        }
        let x = self.layout.display_x(self.doc(), pos)?;
        self.set_box_caret(pos, x);
        Ok(())
    }

    /// One column left; through the empty space, then one char at a time over the text. The caret
    /// does not go to the previous row.
    ///
    /// In the empty space the move is one column (`dx_default`, [`column_x`](crate::layout::Layout::column_x))
    /// and stops at the end of the text (`x = max(x - dx_default, end)`, `Command_LEFT`).
    pub(super) fn box_left(&mut self) -> Result<()> {
        let (caret, x) = (self.caret(), self.box_caret_x()?);
        if self.caret_is_in_empty_space()? {
            let end = self.layout.row_end(self.doc(), caret)?;
            let text_end = self.layout.display_x(self.doc(), end)?;
            let back = x.saturating_sub(self.layout.column_x()).max(text_end);
            self.set_box_caret(end, back);
            return Ok(());
        }
        // inside the text: the previous char of the row, if there is one
        if caret > self.layout.row_start(self.doc(), caret)?
            && let Some(prev) = char_left(self.doc(), caret)?
        {
            let x = self.layout.display_x(self.doc(), prev)?;
            self.set_box_caret(prev, x);
        }
        Ok(())
    }

    /// One char right over the text, then one column at a time through the empty space up to
    /// [`max_box_x`](Self::max_box_x) (`to_x = min(x + dx_default, x_max)`, `Command_RIGHT`). The
    /// caret does not go to the next row, and not onto the end of a wrapped row.
    pub(super) fn box_right(&mut self) -> Result<()> {
        let (caret, x) = (self.caret(), self.box_caret_x()?);
        let end = self.layout.row_end(self.doc(), caret)?;
        let wrapped = self.row_is_wrapped(caret)?;
        if !wrapped && x >= self.layout.display_x(self.doc(), end)? {
            let max = self.max_box_x();
            if x < max {
                let next = (x + self.layout.column_x()).min(max);
                self.set_box_caret(end, next);
            }
            return Ok(());
        }
        if let Some(next) = matsu_core::cursor::char_right(self.doc(), caret)?
            && !(wrapped && next >= end)
        {
            let x = self.layout.display_x(self.doc(), next)?;
            self.set_box_caret(next, x);
        }
        Ok(())
    }

    /// `rows` rows down (negative: up) to the goal column, which may be past the end of the
    /// text of the row there. Moving down never goes onto the empty row after a final line
    /// ending when the caret is not in the first column, so that the column does not change
    /// (Sakura: `CCaret::Cursor_UPDOWN`).
    pub(super) fn box_vertical(&mut self, rows: isize) -> Result<()> {
        let caret_x = self.box_caret_x()?;
        let goal = self.goal_x().unwrap_or(caret_x);
        let caret = self.caret();
        // The start of the row, not the position at the goal column: the end of a wrapped row
        // belongs to the next one.
        let mut row = self.layout.move_rows(self.doc(), caret, rows, X::ZERO)?;
        let eof_row = self.doc().line_count();
        if rows > 0 && caret_x != X::ZERO && row.line >= eof_row {
            row = if caret.line >= eof_row {
                self.layout.move_rows(self.doc(), caret, 0, X::ZERO)?
            } else {
                self.layout.move_rows(self.doc(), row, -1, X::ZERO)?
            };
        }
        self.box_move_to_x(row, goal)?;
        self.set_goal_x(Some(goal));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Pos;

    use crate::clipboard::ClipKind;
    use crate::command::Arg;
    use crate::testing::Editor;

    #[test]
    fn commands_that_sakura_does_not_do_for_boxes_beep_and_change_nothing() {
        // Sakura: `Command_WordDeleteToStart`, `Command_WordDeleteToEnd` and `Command_UNINDENT`
        // beep at once with a box selected.
        let mut e = Editor::new("ab|cd\nefgh").with_clipboard("x", ClipKind::Normal);
        e.run("begin_box").run("cursor_down");
        let text = e.marked();
        for id in [
            "word_delete_to_start",
            "word_delete_to_end",
            "unindent_tab",
            "unindent_space",
        ] {
            let result = e.try_run(id, &[]);
            assert!(result.is_ok(), "{id}: {result:?}");
            assert!(e.changes().beep, "{id}");
            assert!(!e.changes().text_changed(), "{id}");
            assert_eq!(e.marked(), text, "{id}");
            assert_eq!(e.box_state(), Some(((0, 2), (1, 2))), "{id}");
        }
    }

    #[test]
    fn sorting_a_box_sorts_by_its_columns_not_by_the_stream_between_the_corners() {
        // a box of width 0 sorts by the text from its column on
        let mut e = Editor::new("z|b\nya\nxc\n");
        e.run("begin_box").run("cursor_down").run("cursor_down");
        e.run("sort_asc");
        assert_eq!(e.marked().replace(['|', '['], ""), "ya\nzb\nxc\n");
        e.assert_box((0, 1), (2, 1));
    }

    #[test]
    fn typing_in_the_empty_space_of_a_box_inserts_at_the_end_of_the_text() {
        let mut e = Editor::new("ab|");
        e.run("cursor_right_box");
        e.run_with("input_char", &[Arg::Char('x')]);
        // the box was one column wide, past the text; it moves right by the typed char
        assert_eq!(e.marked(), "abx[|");
        e.assert_box((0, 4), (0, 3));
        assert!(!e.changes().beep);
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
