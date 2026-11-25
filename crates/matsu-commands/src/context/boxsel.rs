//! Box selection (Sakura's 矩形選択): a selection between two display columns and two rows.
//!
//! Sakura keeps a box as the same range as a normal selection (`CViewSelect::m_sSelect`, in layout
//! coordinates) and a flag (`IsBoxSelecting`); the range is read as the corners of a rectangle.
//! A corner can be to the right of the end of its row, where there is no text: while a box is
//! selected the caret may go there (`CCaret::Cursor_UPDOWN`, `Command_RIGHT`). The cursor keeps
//! the horizontal position ([`X`]) of each corner for that ([`Cursor::move_box_to`]); the position
//! is then the end of the row.
//!
//! The end of a wrapped row is the start of the next row for a position, and a corner can stand
//! at the end of a wrapped row and right of it too (Sakura's corners are layout points; the
//! caret of a box goes there by `Command_RIGHT`, `Cursor_UPDOWN`, and a click). Such a corner has
//! the position of the start of the next row and the mark of the free cursor
//! ([`Cursor::free_col`](matsu_core::cursor::Cursor::free_col) for the caret,
//! [`anchor_free_col`](matsu_core::cursor::Cursor::anchor_free_col) for the anchor), which says
//! that it is on the row before ([`Corner`]); the mark is checked where it is read
//! ([`CommandContext::box_corners`]). Unlike a free caret, it needs no free cursor mode.
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
use crate::command::{Arg, Result};
use crate::edit_options;
use crate::layout::X;

/// A corner of a box selection: its position and `X`, and the row it stands on when that is not the
/// row of its position (see the header of this file).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Corner {
    pub pos: Pos,
    pub x: X,
    /// The last char of the wrapped row whose gap the corner stands in; `pos` is the start of the
    /// next row then.
    pub gap_last: Option<Pos>,
}

impl Corner {
    /// A position on the row of the corner, to ask the layout for the row.
    pub fn row_pos(&self) -> Pos {
        self.gap_last.unwrap_or(self.pos)
    }

    pub fn in_gap(&self) -> bool {
        self.gap_last.is_some()
    }
}

/// The corner that [`CommandContext::wrapped_end_corner`] decides: in the gap after the wrapped row
/// (its position is the start of the next row), or on a char.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RowCorner {
    Gap { x: X },
    Char { pos: Pos, x: X },
}

/// The lock option of a box move (the argument of `Sub_BoxSelectLock`, `CViewCommander.cpp`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxLock {
    /// 0 or no argument: the setting 「矩形選択移動で選択をロックする」 (`m_bBoxSelectLock`,
    /// [`edit_options::EditOptions::box_select_lock`]; on by default).
    Setting,
    /// 1: the box is locked.
    Locked,
    /// 2, and any other value: the box is not locked (`bSelLock` starts as false).
    Unlocked,
}

impl BoxLock {
    /// The option from the number of a macro argument. Do this where the argument is read, and
    /// pass the result on: the number shares its place with the row count and the flags.
    pub fn from_arg(n: i64) -> Self {
        match n {
            0 => Self::Setting,
            1 => Self::Locked,
            _ => Self::Unlocked,
        }
    }

    /// The option from the argument at `index` (a missing or non-numeric one is 0).
    pub fn from_args(args: &[Arg], index: usize) -> Self {
        Self::from_arg(Arg::int_at(args, index))
    }

    /// Whether the box is locked, reading the setting for [`Setting`](Self::Setting).
    pub fn is_locked(self) -> bool {
        match self {
            Self::Setting => edit_options::get().box_select_lock,
            Self::Locked => true,
            Self::Unlocked => false,
        }
    }
}

impl CommandContext<'_> {
    /// `F_BEGIN_BOX`: starts a box selection at the caret. A selection that was there is dropped.
    /// Nothing is selected until the caret moves. The box is locked, so that the plain moves
    /// extend it (Sakura: `Command_BEGIN_BOXSELECT(true)`; the text must be in a fixed-pitch
    /// font, which the view here always is).
    pub fn begin_box(&mut self) -> Result<()> {
        self.start_box(true)
    }

    /// What a box move does first: starts a box at the caret unless one is going on (Sakura:
    /// `Sub_BoxSelectLock`). `lock` says whether the new box is locked. A box that is going on
    /// keeps its lock and ignores it. A normal selection is dropped, as Sakura does since 2014.
    pub fn ensure_box(&mut self, lock: BoxLock) -> Result<()> {
        if self.cursor.is_box() {
            Ok(())
        } else {
            self.start_box(lock.is_locked())
        }
    }

    fn start_box(&mut self, lock: bool) -> Result<()> {
        // A caret in the gap at the end of a wrapped row begins the box there, on that row
        // (Sakura's caret is a layout point, and `BeginSelectArea` copies it).
        let in_gap = self.caret_wrapped_gap()?.is_some();
        // (a box that is going on keeps the column of its corner, also in the empty space)
        let x = self.box_caret_x()?;
        self.begin_box_at(x, lock);
        if in_gap {
            self.set_free_x(Some(x));
        }
        Ok(())
    }

    /// The `X` of the caret: for a box selection that of its corner, which can be past the end of
    /// the text; otherwise that of the caret position.
    pub fn box_caret_x(&self) -> Result<X> {
        let at = self.caret_x_with_free()?;
        Ok(self.stored_box_caret_x().map_or(at, |c| c.max(at)))
    }

    /// The two corners of the box selection, anchor first, with the rows they stand on: a corner
    /// that carries a good mark stands in the gap of the wrapped row before its position. `None`
    /// when no box covers anything.
    pub(super) fn box_corners(&self) -> Result<Option<(Corner, Corner)>> {
        let Some(((a, a_x), (c, c_x))) = self.box_corner_xs() else {
            return Ok(None);
        };
        let gap_row = |gap: Option<(X, crate::layout::WrappedGap)>| gap.map(|(_, gap)| gap.last);
        Ok(Some((
            Corner {
                pos: a,
                x: a_x,
                gap_last: gap_row(self.anchor_wrapped_gap()?),
            },
            Corner {
                pos: c,
                x: c_x,
                gap_last: gap_row(self.caret_wrapped_gap()?),
            },
        )))
    }

    /// The column of the caret of a box selection that stands in the gap after a wrapped row, to
    /// be given back with [`keep_gap_mark`](Self::keep_gap_mark) after the box has ended. `None`
    /// for any other caret.
    pub(super) fn box_gap_mark(&self) -> Result<Option<X>> {
        if !self.cursor.is_box() {
            return Ok(None);
        }
        Ok(self.caret_wrapped_gap()?.map(|(x, _)| x))
    }

    /// Puts the caret back in the gap that [`box_gap_mark`](Self::box_gap_mark) saw, after a move
    /// that ended the box dropped the mark. Without the free cursor mode the column is brought
    /// back to the end of the text.
    pub(super) fn keep_gap_mark(&mut self, mark: Option<X>) -> Result<()> {
        match mark {
            Some(x) => self.set_gap_free_x(x),
            None => Ok(()),
        }
    }

    /// Ends the box selection (and any other selection) where the caret is: the caret does not
    /// move, and one in the gap after a wrapped row stays on that row. The one place that every
    /// command that drops a box in place goes through (`F_CANCEL_MODE`, a copy that disables the
    /// selection, an empty box that the mouse ends, the commands that drop the selection and work
    /// on the caret's row). A command that moves the caret anyway does not need it.
    pub fn end_box_in_place(&mut self) -> Result<()> {
        self.end_box_with(|cx| {
            cx.cursor.clear_selection();
            Ok(())
        })
    }

    /// [`end_box_in_place`](Self::end_box_in_place) with the step that ends the box left to the
    /// caller (`cancel_mode` and the mouse end the box with `move_caret`, which reports a move):
    /// the mark is taken before the step and put back after it, in this one order.
    pub(super) fn end_box_with(
        &mut self,
        step: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<()> {
        let mark = self.box_gap_mark()?;
        step(self)?;
        self.keep_gap_mark(mark)
    }

    /// A position on the row of each corner of the box selection, anchor first (for a corner in
    /// the gap after a wrapped row, the last char of that row; otherwise the corner's position),
    /// for what counts the rows of the corners. `None` when no box covers anything.
    pub fn box_corner_rows(&self) -> Result<Option<(Pos, Pos)>> {
        Ok(self.box_corners()?.map(|(a, c)| (a.row_pos(), c.row_pos())))
    }

    /// Whether the row of `row_pos` ends because the line wraps there. The end of such a row is
    /// the start of the next one for a position, so a box caret never rests at the end of a
    /// wrapped row, and there is no empty space to its right.
    pub(super) fn row_is_wrapped(&self, row_pos: Pos) -> Result<bool> {
        Ok(crate::layout::row_is_wrapped(
            &*self.layout,
            self.doc(),
            row_pos,
        )?)
    }

    /// The `X` where the text of the row of `row_pos` ends ([`crate::layout::row_text_end_x`]).
    pub(super) fn row_text_end_x(&self, row_pos: Pos) -> Result<X> {
        Ok(crate::layout::row_text_end_x(
            &*self.layout,
            self.doc(),
            row_pos,
        )?)
    }

    /// Whether the caret of a box selection is to the right of the end of the text of its row.
    pub(super) fn caret_is_in_empty_space(&self) -> Result<bool> {
        // The row of the caret: at the end of a wrapped row (or right of it) it is that row.
        let row = self.caret_row_pos()?;
        Ok(self.cursor.is_box()
            && !self.row_is_wrapped(row)?
            && self.box_caret_x()? > self.caret_display_x()?)
    }

    /// Where a corner goes when the pointer, or the column, is at `x` on the wrapped row of
    /// `row_pos` and rounds to the end of the row. The one place that decides it, for a drag
    /// ([`box_move_to_x`](Self::box_move_to_x)) and a press (`mouse_begin_box_at`): `x` at the end
    /// of the text or right of it is the gap after the row, up to the width of a row; `x` inside
    /// the last char (a wide char or a tab) is the left edge of that char.
    pub(super) fn wrapped_end_corner(&self, row_pos: Pos, x: X) -> Result<RowCorner> {
        let end = self.layout.row_end(self.doc(), row_pos)?;
        let text_end = self.row_text_end_x(row_pos)?;
        if x >= text_end {
            return Ok(RowCorner::Gap {
                x: x.min(self.max_box_x().max(text_end)),
            });
        }
        let mut pos = end;
        if let Some(last) = char_left(self.doc(), end)?
            && last >= self.layout.row_start(self.doc(), row_pos)?
        {
            pos = last;
        }
        let at = self.layout.display_x(self.doc(), pos)?;
        Ok(RowCorner::Char { pos, x: at })
    }

    /// Moves the box caret to `x` on the row of `row_pos`: onto the char that covers it, or into
    /// the empty space after the text. On a wrapped row, `x` at the end of the text or right of it
    /// (up to the width of a row) is the gap after the row, where the corner is on the row and its
    /// position is the start of the next one.
    pub(super) fn box_move_to_x(&mut self, row_pos: Pos, x: X) -> Result<()> {
        let pos = self.layout.pos_at_x(self.doc(), row_pos, x)?;
        let end = self.layout.row_end(self.doc(), row_pos)?;
        let wrapped = self.row_is_wrapped(row_pos)?;
        if wrapped && pos == end {
            match self.wrapped_end_corner(row_pos, x)? {
                RowCorner::Gap { x } => self.set_box_gap_caret(end, x),
                RowCorner::Char { pos, x } => self.set_box_caret(pos, x),
            }
            return Ok(());
        }
        let at = self.layout.display_x(self.doc(), pos)?;
        let in_space = pos == end && at < x;
        self.set_box_caret(pos, if in_space { x } else { at });
        Ok(())
    }

    /// Moves the box caret to the end of the text of its row (on a wrapped row, the gap after it).
    pub(super) fn box_move_to_row_end(&mut self) -> Result<()> {
        let row = self.caret_row_pos()?;
        let end = self.layout.row_end(self.doc(), row)?;
        if self.row_is_wrapped(row)? {
            let text_end = self.row_text_end_x(row)?;
            self.set_box_gap_caret(end, text_end);
            return Ok(());
        }
        let x = self.layout.display_x(self.doc(), end)?;
        self.set_box_caret(end, x);
        Ok(())
    }

    /// One column left; through the empty space, then one char at a time over the text. The caret
    /// does not go to the previous row.
    ///
    /// In the empty space the move is one column (`dx_default`, [`column_x`](crate::layout::Layout::column_x))
    /// and stops at the end of the text (`x = max(x - dx_default, end)`, `Command_LEFT`). In the
    /// gap after a wrapped row it is the same, and from the end of the text the caret goes onto the
    /// last char of the row.
    pub(super) fn box_left(&mut self) -> Result<()> {
        if let Some((x, gap)) = self.caret_wrapped_gap()? {
            if x > gap.text_end {
                let back = x.saturating_sub(self.layout.column_x()).max(gap.text_end);
                self.set_box_gap_caret(self.caret(), back);
            } else {
                let at = self.layout.display_x(self.doc(), gap.last)?;
                self.set_box_caret(gap.last, at);
            }
            return Ok(());
        }
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
    /// caret does not go to the next row. From the last char of a wrapped row it goes into the
    /// gap after the row, and on through it.
    pub(super) fn box_right(&mut self) -> Result<()> {
        let max = self.max_box_x();
        if let Some((x, gap)) = self.caret_wrapped_gap()? {
            if x < max {
                let next = (x + self.layout.column_x()).min(max);
                self.set_box_gap_caret(self.caret(), next.max(gap.text_end));
            }
            return Ok(());
        }
        let (caret, x) = (self.caret(), self.box_caret_x()?);
        let end = self.layout.row_end(self.doc(), caret)?;
        let wrapped = self.row_is_wrapped(caret)?;
        if !wrapped && x >= self.layout.display_x(self.doc(), end)? {
            if x < max {
                let next = (x + self.layout.column_x()).min(max);
                self.set_box_caret(end, next);
            }
            return Ok(());
        }
        if let Some(next) = matsu_core::cursor::char_right(self.doc(), caret)? {
            if wrapped && next >= end {
                // past the last char of the row: the end of its text, in the gap
                let text_end = self.row_text_end_x(caret)?;
                self.set_box_gap_caret(end, text_end);
            } else {
                let x = self.layout.display_x(self.doc(), next)?;
                self.set_box_caret(next, x);
            }
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
        // the row of the caret, which in the gap after a wrapped row is not that of its position
        let caret = self.caret_row_pos()?;
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
