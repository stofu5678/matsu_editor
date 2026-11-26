//! Caret moves and selection changes, shared by the movement commands and their selecting
//! variants (`F_LEFT` and `F_LEFT_SEL` call the same method with `extend` false and true).

use matsu_core::buffer::Pos;
use matsu_core::cursor::{char_left, char_right};

use super::CommandContext;
use crate::command::Result;

/// Whether `c` counts as indentation: a space, a tab, or an ideographic space.
///
/// Sakura: `WCODE::IsIndentChar` in `sakura_core/charset/charcode.h`. The ideographic space is
/// included because the type setting `m_bAutoIndent_ZENSPACE` is on by default
/// (`sakura_core/types/CType.cpp`, `InitTypeConfigDefault`); it is a setting in Sakura, which
/// this crate does not read yet.
fn is_indent_char(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{3000}')
}

/// `Command_GOLINETOP` flag 1: go to the start whatever the caret position.
pub const LINE_TOP_ONLY: i64 = 1;
/// `Command_GOLINETOP` and `Command_GOLINEEND` flag 4: extend the selection.
pub const LINE_TOP_SELECT: i64 = 4;
/// `Command_GOLINETOP` and `Command_GOLINEEND` flag 8: work on the line, not the row.
pub const LINE_TOP_LINE_UNIT: i64 = 8;

impl CommandContext<'_> {
    /// Whether a move extends the selection: it was asked to, or the selection is locked
    /// (Sakura passes `m_bSelectingLock` as `bSelect` for the plain moves, `HandleCommand`).
    ///
    /// Also the one place that every caret-motion command passes through before it decides
    /// whether the caret can move, so it is where a box that was just begun stops being selected
    /// text: every cursor command in Sakura ends in `ChangeSelectAreaByCurrentCursor` (when it
    /// extends), whose `...TEST` clears an empty range whether or not the caret moved, or in
    /// `DisableSelectArea` (when it does not) (`view/CViewSelect.cpp`, `cmd/CViewCommander_Cursor.cpp`,
    /// `view/CCaret.cpp`). A move that cannot move (the end of the document, the top of the file)
    /// is no different.
    pub fn extends(&mut self, extend: bool) -> bool {
        let extend = extend || self.cursor.selecting_lock();
        if extend {
            self.cursor.box_moved();
        } else if self.cursor.box_just_begun() {
            self.cursor.clear_selection();
        }
        extend
    }

    /// `Command_LEFT` and `Command_RIGHT` without a selecting move, with a box just begun: it is
    /// selected text, so `Command_CANCEL_MODE` ends it and uses up the move (`continue` in
    /// `cmd/CViewCommander_Cursor.cpp`: "選択のキャンセルで ... 移動を 1消費"). The caret stays.
    /// Call before [`extends`](Self::extends). Returns whether it did.
    fn cancel_begun_box_for_move(&mut self, extend: bool) -> Result<bool> {
        if extend || self.cursor.selecting_lock() {
            return Ok(false);
        }
        self.end_begun_box()
    }

    /// Moves the caret to `pos` (clamped). With `extend` the selection follows the caret; in a
    /// box selection the box follows it, with the caret at the display column of `pos`.
    pub fn move_caret(&mut self, pos: Pos, extend: bool) -> Result<()> {
        let pos = self.doc().clamp_pos(pos)?;
        if extend && self.cursor.is_box() {
            let x = self.layout.display_x(self.doc(), pos)?;
            self.set_box_caret(pos, x);
        } else {
            self.cursor.move_to(pos, extend);
        }
        Ok(())
    }

    /// Selects from `anchor` to `caret`.
    pub fn select_range(&mut self, anchor: Pos, caret: Pos) -> Result<()> {
        let (anchor, caret) = (self.doc().clamp_pos(anchor)?, self.doc().clamp_pos(caret)?);
        self.cursor.select(anchor, caret);
        Ok(())
    }

    /// Selects the whole document. The caret goes to the end.
    pub fn select_all(&mut self) -> Result<()> {
        // Command_SELECTALL (sakura_core/cmd/CViewCommander_Select.cpp) records the place first.
        self.add_to_jump_history();
        let end = self.doc().end_pos()?;
        self.select_range(Pos::new(0, 0), end)
    }

    /// One char left. Without `extend`, a selection is dropped and the caret goes to the start
    /// of it instead of moving (Sakura: `Command_LEFT` calls `Command_CANCEL_MODE(1)` and that
    /// uses up the move).
    ///
    /// In a box selection the caret stays on its row and can go through the empty space to the
    /// right of the text (Sakura: `Command_LEFT`, which does not wrap to the previous row while a
    /// box is selected).
    pub fn move_left(&mut self, extend: bool) -> Result<()> {
        if self.cancel_begun_box_for_move(extend)? {
            return Ok(());
        }
        let extend = self.extends(extend);
        if extend && self.cursor.is_box() {
            return self.box_left();
        }
        if !extend && self.cursor.is_box() && self.cursor.has_selection() {
            return self.collapse_box(false);
        }
        if !extend && let Some((start, _)) = self.selection() {
            return self.move_caret(start, false);
        }
        // In the free cursor mode the caret comes back through the gap one column at a time.
        if self.free_left(extend)? {
            return Ok(());
        }
        if let Some(pos) = char_left(self.doc(), self.caret())? {
            self.move_caret(pos, extend)?;
        }
        Ok(())
    }

    /// One char right. Without `extend`, a selection is dropped and the caret goes to the end of
    /// it (Sakura: `Command_RIGHT`, `Command_CANCEL_MODE(2)`).
    ///
    /// In a box selection the caret goes on through the empty space to the right of the text, up
    /// to [`max_box_x`](CommandContext::max_box_x) (Sakura: `Command_RIGHT`).
    pub fn move_right(&mut self, extend: bool) -> Result<()> {
        if self.cancel_begun_box_for_move(extend)? {
            return Ok(());
        }
        let extend = self.extends(extend);
        if extend && self.cursor.is_box() {
            return self.box_right();
        }
        if !extend && self.cursor.is_box() && self.cursor.has_selection() {
            return self.collapse_box(true);
        }
        if !extend && let Some((_, end)) = self.selection() {
            return self.move_caret(end, false);
        }
        // In the free cursor mode the caret goes on through the gap, one column at a time.
        if self.free_right(extend)? {
            return Ok(());
        }
        if let Some(pos) = char_right(self.doc(), self.caret())? {
            self.move_caret(pos, extend)?;
        }
        Ok(())
    }

    /// `rows` rows down (negative: up), to the goal column. The goal column is the display
    /// column the caret had before the first vertical move; other moves and edits forget it
    /// (Sakura: `CCaret::Cursor_UPDOWN` and `m_nCaretPosX_Prev`). A selection is dropped, not
    /// used as the place to start from.
    ///
    /// In a box selection the goal column can be past the end of the text, and the caret then
    /// stays in the empty space (Sakura: `CCaret::Cursor_UPDOWN` with `IsBoxSelecting`).
    pub fn move_vertical(&mut self, rows: isize, extend: bool) -> Result<()> {
        let extend = self.extends(extend);
        if extend && self.cursor.is_box() {
            return self.box_vertical(rows);
        }
        let goal = match self.goal_x() {
            Some(goal) => goal,
            None => self.caret_x_with_free()?,
        };
        // From the gap of a wrapped row the move starts on that row (`caret_row_pos`).
        let from = self.caret_row_pos()?;
        let target = self.layout.move_rows(self.model.doc(), from, rows, goal)?;
        self.cursor.move_to(target, extend);
        // The free cursor mode keeps the goal column on a row that is shorter
        // (`m_bIsFreeCursorMode` in `CCaret::Cursor_UPDOWN`).
        self.stand_free(goal)?;
        self.set_goal_x(Some(goal));
        Ok(())
    }

    /// `Command_GOLINETOP` with the extended flags that a macro passes
    /// (`sakura_core/cmd/CViewCommander_Cursor.cpp`): `LINE_TOP_ONLY` goes to the start
    /// whatever the caret position (no stop at the first char that is not indentation),
    /// `LINE_TOP_SELECT` extends the selection, `LINE_TOP_LINE_UNIT` works on the line instead
    /// of the row. Other bits are ignored, as Sakura does.
    ///
    /// Without the line unit flag, a row that is only indentation goes to the start of the row.
    /// With it, a wrapped row that is only indentation does not stop the search: the first char
    /// that is not indentation in the rows of the line counts, and the start of the line is the
    /// answer when there is none.
    pub fn move_line_top_flags(&mut self, extend: bool, flags: i64) -> Result<()> {
        let extend = self.extends(extend || flags & LINE_TOP_SELECT != 0);
        let caret = self.caret();
        let line_unit = flags & LINE_TOP_LINE_UNIT != 0;
        let top = if line_unit {
            Pos::new(caret.line, 0)
        } else {
            self.layout.row_start(self.doc(), self.caret_row_pos()?)?
        };
        if flags & LINE_TOP_ONLY != 0 {
            return self.move_caret(top, extend);
        }
        let (mut row, mut first) = (top, top);
        loop {
            let end = self.layout.row_end(self.doc(), row)?;
            let text = self.doc().text_between(row, end)?;
            if let Some((i, _)) = text.char_indices().find(|&(_, c)| !is_indent_char(c)) {
                first = Pos::new(row.line, row.col + i);
                break;
            }
            if !line_unit {
                break;
            }
            match self.layout.next_row_start(self.doc(), row)? {
                Some(next) if next.line == top.line => row = next,
                _ => break,
            }
        }
        // A box caret in the empty space is not at the first char, even when the row ends there.
        let at_first = caret == first && !self.caret_is_in_empty_space()?;
        self.move_caret(if at_first { top } else { first }, extend)
    }

    /// `Command_GOLINEEND` with the extended flags that a macro passes
    /// (`sakura_core/cmd/CViewCommander_Cursor.cpp`): flag 4 extends the selection, flag 8 goes
    /// to the end of the last row of the line instead of the end of the row.
    pub fn move_line_end_flags(&mut self, extend: bool, flags: i64) -> Result<()> {
        let extend = self.extends(extend || flags & LINE_TOP_SELECT != 0);
        let mut row = self.caret_row_pos()?;
        if flags & LINE_TOP_LINE_UNIT != 0 {
            while let Some(next) = self.layout.next_row_start(self.doc(), row)? {
                if next.line != row.line {
                    break;
                }
                row = next;
            }
        } else if extend && self.cursor.is_box() {
            return self.box_move_to_row_end();
        }
        let target = self.layout.row_end(self.doc(), row)?;
        self.move_caret(target, extend)?;
        // The end of a wrapped row is the start of the next row for a position; Sakura keeps the
        // caret on the row (`nPosXY.x = CalcLayoutWidth`, `nPosXY.y` is the row).
        self.mark_wrapped_row_end()
    }

    /// The start of the document (Sakura: `Command_GOFILETOP`, which first adds the place to the
    /// jump history).
    ///
    /// In a box selection the caret keeps its column and goes to the first row (Sakura:
    /// `Command_GOFILETOP`).
    pub fn move_file_top(&mut self, extend: bool) -> Result<()> {
        self.add_to_jump_history();
        let extend = self.extends(extend);
        if extend && self.cursor.is_box() {
            let x = self.box_caret_x()?;
            return self.box_move_to_x(Pos::new(0, 0), x);
        }
        self.move_caret(Pos::new(0, 0), extend)
    }

    /// The end of the document, which is on the empty line after a final line ending
    /// (Sakura: `Command_GOFILEEND`).
    ///
    /// In a box selection the caret keeps its column and goes to the last row with text, not to
    /// the empty row after a final line ending (Sakura: `Command_GOFILEEND`).
    pub fn move_file_end(&mut self, extend: bool) -> Result<()> {
        self.add_to_jump_history();
        let extend = self.extends(extend);
        if extend && self.cursor.is_box() {
            self.box_vertical(isize::MAX)?;
            if !self.cursor.has_selection() {
                let x = self.box_caret_x()?;
                self.set_goal_x(Some(x));
            }
            return Ok(());
        }
        let end = self.doc().end_pos()?;
        self.move_caret(end, extend)
    }
}
