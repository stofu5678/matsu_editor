//! Sakura's free cursor mode (フリーカーソル, `m_bIsFreeCursorMode`): the caret may stand to the
//! right of the end of a line.
//!
//! The caret position stays the end of the text of the row, and the cursor keeps the display
//! column the caret stands at ([`Cursor::free_col`](matsu_core::cursor::Cursor::free_col)), like
//! the corner of a box does. Sakura reads the mode in three places:
//!
//! * a vertical move keeps the goal column on a row that is shorter
//!   (`CCaret::Cursor_UPDOWN`, `sakura_core/view/CCaret.cpp`);
//! * a click to the right of a row puts the caret there, up to the longest line
//!   (`CCaret::MoveCursorProperly`);
//! * `Command_RIGHT` goes on one column at a time through the empty space, and to the next line
//!   only at the right edge (`sakura_core/cmd/CViewCommander_Cursor.cpp`); `Command_LEFT` comes
//!   back one column at a time.
//!
//! What is typed there is put after spaces that fill the gap
//! (`CEditView::InsertData_CEditView`, `sakura_core/view/CEditView_Command_New.cpp`: "行終端より右に
//! 挿入しようとした"), and `Command_DELETE` pads the same way before it deletes the line ending
//! (`CViewCommander_Edit.cpp`, the 2008.08.03 nasukoji change). `Command_DELETE_BACK` moves the
//! caret left through the gap and deletes nothing.

use super::CommandContext;
use super::boxsel::MAX_BOX_COL;
use crate::command::Result;
use crate::edit_options;

impl CommandContext<'_> {
    /// Whether the free cursor mode is on (`m_bIsFreeCursorMode`).
    pub fn is_free_cursor(&self) -> bool {
        edit_options::get().free_cursor
    }

    /// Whether a caret at `pos` may stand to the right of the text: at the end of the text of the
    /// last row of a line. Not on the empty row after a final line ending (only its first
    /// column is valid, `CCaret::GetAdjustCursorPos`), and not at the end of a wrapped row,
    /// which is the start of the next one.
    fn can_stand_free(&self, pos: matsu_core::buffer::Pos) -> Result<bool> {
        Ok(pos.line < self.doc().line_count()
            && self.layout.row_end(self.doc(), pos)? == pos
            && !self.row_is_wrapped(pos)?)
    }

    /// The display column the caret stands at when it is to the right of the end of its line, if
    /// the mode is on and it is.
    pub fn caret_free_col(&self) -> Result<Option<usize>> {
        let Some(col) = self.cursor.free_col().filter(|_| self.is_free_cursor()) else {
            return Ok(None);
        };
        let caret = self.caret();
        Ok((self.can_stand_free(caret)? && col > self.caret_display_col()?).then_some(col))
    }

    /// The display column of the caret, counting the space to the right of the text.
    pub fn caret_col_with_free(&self) -> Result<usize> {
        let at = self.caret_display_col()?;
        Ok(self.caret_free_col()?.map_or(at, |c| c.max(at)))
    }

    /// Lets the caret, which is at a position already, stand at display column `col` if that is
    /// to the right of the text of its row and the mode is on. The column goes up to the longest
    /// line ([`MAX_BOX_COL`]).
    pub(super) fn stand_free(&mut self, col: usize) -> Result<()> {
        if self.is_free_cursor()
            && self.can_stand_free(self.caret())?
            && col > self.caret_display_col()?
        {
            self.cursor.set_free_col(Some(col.min(MAX_BOX_COL)));
        }
        Ok(())
    }

    /// Moves the caret to `pos`, and to display column `col` if that is to the right of the text
    /// there and the mode is on (a click to the right of a row, `CCaret::MoveCursorProperly`).
    pub fn move_caret_free(
        &mut self,
        pos: matsu_core::buffer::Pos,
        col: usize,
        extend: bool,
    ) -> Result<()> {
        self.move_caret(pos, extend)?;
        if !(extend && self.cursor.is_box()) {
            self.stand_free(col)?;
        }
        Ok(())
    }

    /// `Command_RIGHT` in the free cursor mode, at the end of the text of a row: one column to the
    /// right through the empty space. At the right edge (the longest line) the caret goes to the
    /// start of the next line, and stops there on the last line (`on_x_max`,
    /// `MOVE_NEXTLINE_IMMEDIATELY` and `STOP`). Returns `false` when the move is not a free one
    /// (the caret is not at the end of a row, or it goes to the next line).
    pub(super) fn free_right(&mut self, extend: bool) -> Result<bool> {
        if !self.is_free_cursor() || !self.can_stand_free(self.caret())? {
            return Ok(false);
        }
        let next = self.caret_col_with_free()? + 1;
        let has_next_line = matsu_core::cursor::char_right(self.doc(), self.caret())?.is_some();
        if next >= MAX_BOX_COL && has_next_line {
            return Ok(false);
        }
        if next > MAX_BOX_COL {
            return Ok(true);
        }
        self.move_caret_free(self.caret(), next, extend)?;
        Ok(true)
    }

    /// `Command_LEFT` while the caret stands to the right of the text: one column back, and onto
    /// the text at its end. Returns `false` when the caret is not there.
    pub(super) fn free_left(&mut self, extend: bool) -> Result<bool> {
        let Some(col) = self.caret_free_col()? else {
            return Ok(false);
        };
        self.move_caret_free(self.caret(), col - 1, extend)?;
        Ok(true)
    }

    /// Fills the gap between the end of the text and the caret with spaces, so that the caret is
    /// on the text again, before something is put there (`InsertData_CEditView`: one space per
    /// column). Does nothing when the caret is not in the gap. A selection is not looked at: the
    /// callers pad only without one.
    pub fn pad_free_caret(&mut self) -> Result<()> {
        let Some(col) = self.caret_free_col()? else {
            return Ok(());
        };
        let caret = self.caret();
        let gap = col - self.caret_display_col()?;
        let edit = self.replace(caret, caret, &" ".repeat(gap))?;
        self.cursor.move_to(edit.end, false);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Pos;

    use crate::command::Arg;
    use crate::edit_options::{EditOptions, with_options};
    use crate::testing::Editor;

    fn free() -> EditOptions {
        EditOptions {
            free_cursor: true,
            ..EditOptions::DEFAULT
        }
    }

    fn col(e: &Editor) -> Option<usize> {
        e.cursor().free_col()
    }

    #[test]
    fn down_keeps_the_column_on_a_shorter_line() {
        with_options(free(), || {
            let mut e = Editor::new("abcdef|\nxy\nlonger line");
            e.run("cursor_down");
            // the caret stands at column 6 of "xy", right of its end
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(1, 2), Some(6)));
            e.run("cursor_down");
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(2, 6), None));
            e.run("cursor_up").run("cursor_up");
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(0, 6), None));
        });
    }

    #[test]
    fn without_the_mode_the_caret_stops_at_the_end_of_the_line() {
        let mut e = Editor::new("abcdef|\nxy\nlonger line");
        e.run("cursor_down");
        assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(1, 2), None));
    }

    #[test]
    fn the_column_counts_wide_chars_and_tabs() {
        with_options(free(), || {
            // "日本" is 4 columns; a tab to column 4 and "a" is 5
            let mut e = Editor::new("\tab|\n日本");
            e.run("cursor_down");
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(1, 6), Some(6)));
            let mut e = Editor::new("日本語|\nab");
            e.run("cursor_down");
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(1, 2), Some(6)));
        });
    }

    #[test]
    fn the_empty_row_after_the_last_line_ending_has_only_its_first_column() {
        with_options(free(), || {
            let mut e = Editor::new("abc|\n");
            e.run("cursor_down");
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(1, 0), None));
            // the last line without a line ending does allow it
            let mut e = Editor::new("abcdef|\nxy");
            e.run("cursor_down");
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(1, 2), Some(6)));
        });
    }

    #[test]
    fn right_goes_through_the_gap_and_left_comes_back() {
        with_options(free(), || {
            let mut e = Editor::new("ab|\ncd");
            e.run("cursor_right");
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(0, 2), Some(3)));
            e.run("cursor_right");
            assert_eq!(col(&e), Some(4));
            e.run("cursor_left");
            assert_eq!(col(&e), Some(3));
            e.run("cursor_left");
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(0, 2), None));
            e.run("cursor_left");
            assert_eq!(e.cursor().caret(), Pos::new(0, 1));
        });
    }

    #[test]
    fn right_at_the_longest_line_goes_to_the_next_line() {
        with_options(free(), || {
            let mut e = Editor::new("ab|\ncd");
            for _ in 0..super::MAX_BOX_COL - 3 {
                e.run("cursor_right");
            }
            assert_eq!(col(&e), Some(super::MAX_BOX_COL - 1));
            e.run("cursor_right");
            assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(1, 0), None));
            // on the last line without a line ending it stops at the edge
            let mut e = Editor::new("ab|");
            for _ in 0..super::MAX_BOX_COL + 5 {
                e.run("cursor_right");
            }
            assert_eq!(col(&e), Some(super::MAX_BOX_COL));
        });
    }

    #[test]
    fn right_without_the_mode_goes_to_the_next_line() {
        let mut e = Editor::new("ab|\ncd");
        e.run("cursor_right");
        assert_eq!((e.cursor().caret(), col(&e)), (Pos::new(1, 0), None));
    }

    #[test]
    fn typing_in_the_gap_fills_it_with_spaces() {
        with_options(free(), || {
            let mut e = Editor::new("ab|\ncd");
            e.run("cursor_right")
                .run("cursor_right")
                .run("cursor_right");
            e.run_with("input_char", &[Arg::Char('x')]);
            assert_eq!(e.marked(), "ab   x|\ncd");
            // one undo step for the spaces and the char
            e.run("undo");
            assert_eq!(e.marked().replace(['|', '['], ""), "ab\ncd");
        });
    }

    #[test]
    fn typing_in_the_gap_keeps_a_crlf_and_a_japanese_line() {
        with_options(free(), || {
            let mut e = Editor::new("日本|\r\nab");
            e.run("cursor_right").run("cursor_right");
            e.run_with("input_char", &[Arg::Char('あ')]);
            assert_eq!(e.marked(), "日本  あ|\r\nab");
        });
    }

    #[test]
    fn typing_in_the_gap_in_overwrite_mode_inserts() {
        with_options(free(), || {
            let mut e = Editor::new("ab|\ncd");
            e.set_overwrite(true);
            e.run("cursor_right").run("cursor_right");
            e.run_with("input_char", &[Arg::Char('x')]);
            assert_eq!(e.marked(), "ab  x|\ncd");
        });
    }

    #[test]
    fn a_tab_typed_in_the_gap_goes_after_the_spaces() {
        with_options(free(), || {
            let mut e = Editor::new("ab|");
            e.run("cursor_right").run("cursor_right");
            e.run("indent_tab");
            assert_eq!(e.marked().replace('|', ""), "ab  \t");
        });
    }

    #[test]
    fn a_soft_tab_in_the_gap_counts_from_the_caret_column() {
        with_options(free(), || {
            let mut e = Editor::new("ab|").with_tab_width(4).with_ins_space(true);
            for _ in 0..4 {
                e.run("cursor_right");
            }
            assert_eq!(e.cursor().free_col(), Some(6));
            e.run("indent_tab");
            // the gap is filled to column 6, then the tab goes on to the stop at column 8
            assert_eq!(e.marked(), "ab      |");
        });
    }

    #[test]
    fn a_box_paste_starts_at_the_column_of_the_free_caret() {
        use crate::clipboard::ClipKind;
        with_options(free(), || {
            let mut e = Editor::new("ab|\ncdefgh").with_clipboard("12\n34", ClipKind::Normal);
            e.run("cursor_right").run("cursor_right");
            e.run("pastebox");
            // the first row is padded to column 4, the second gets the text at column 4 (the last row of a paste gets a line ending, as in `pastebox`)
            assert_eq!(e.marked().replace(['|', '['], ""), "ab  12\ncdef34gh\n");
        });
    }

    #[test]
    fn a_new_line_in_the_gap_leaves_the_spaces_on_the_old_line() {
        with_options(free(), || {
            let mut e = Editor::new("ab|\ncd");
            e.run("cursor_right").run("cursor_right");
            e.run_with("input_char", &[Arg::Char('\n')]);
            assert_eq!(e.marked(), "ab  \n|\ncd");
        });
    }

    #[test]
    fn any_other_move_forgets_the_gap() {
        with_options(free(), || {
            let mut e = Editor::new("abcdef|\nxy");
            e.run("cursor_down");
            assert_eq!(col(&e), Some(6));
            e.run("go_line_top");
            assert_eq!(col(&e), None);
            e.run("cursor_down");
            assert_eq!(col(&e), None);
        });
    }

    #[test]
    fn a_selection_that_starts_in_the_gap_selects_the_text_after_it() {
        with_options(free(), || {
            let mut e = Editor::new("abcdef|\nxy\nz");
            e.run("cursor_down");
            e.run("cursor_down_sel");
            // the selection runs from the end of "xy" to the start of the third line's end
            assert_eq!(e.cursor().selection().map(|s| s.0), Some(Pos::new(1, 2)));
        });
    }
}
