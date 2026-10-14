//! Caret moves and selection changes, shared by the movement commands and their selecting
//! variants (`F_LEFT` and `F_LEFT_SEL` call the same method with `extend` false and true).

use ume_core::buffer::Pos;
use ume_core::cursor::{char_left, char_right};

use super::CommandContext;
use crate::command::Result;

/// Whether `c` counts as indentation: a space, a tab, or an ideographic space.
///
/// Sakura: `WCODE::IsIndentChar` in `sakura_core/charset/charcode.h`. The ideographic space is
/// included because the type setting `m_bAutoIndent_ZENSPACE` is on by default
/// (`sakura_core/types/CType.cpp`, `InitTypeConfigDefault`); it is a setting in Sakura, which
/// this crate does not read yet.
pub(crate) fn is_indent_char(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{3000}')
}

impl CommandContext<'_> {
    /// Moves the caret to `pos` (clamped). With `extend` the selection follows the caret.
    pub fn move_caret(&mut self, pos: Pos, extend: bool) -> Result<()> {
        let pos = self.doc().clamp_pos(pos)?;
        self.cursor.move_to(pos, extend);
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
        let end = self.doc().end_pos()?;
        self.select_range(Pos::new(0, 0), end)
    }

    /// One char left. Without `extend`, a selection is dropped and the caret goes to the start
    /// of it instead of moving (Sakura: `Command_LEFT` calls `Command_CANCEL_MODE(1)` and that
    /// uses up the move).
    pub fn move_left(&mut self, extend: bool) -> Result<()> {
        if !extend && let Some((start, _)) = self.selection() {
            return self.move_caret(start, false);
        }
        if let Some(pos) = char_left(self.doc(), self.caret())? {
            self.move_caret(pos, extend)?;
        }
        Ok(())
    }

    /// One char right. Without `extend`, a selection is dropped and the caret goes to the end of
    /// it (Sakura: `Command_RIGHT`, `Command_CANCEL_MODE(2)`).
    pub fn move_right(&mut self, extend: bool) -> Result<()> {
        if !extend && let Some((_, end)) = self.selection() {
            return self.move_caret(end, false);
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
    pub fn move_vertical(&mut self, rows: isize, extend: bool) -> Result<()> {
        let goal = match self.cursor.goal_col() {
            Some(goal) => goal,
            None => self.caret_display_col()?,
        };
        let target = self
            .layout
            .move_rows(self.model.doc(), self.caret(), rows, goal)?;
        self.cursor.move_to(target, extend);
        self.cursor.set_goal_col(Some(goal));
        Ok(())
    }

    /// The start of the row, or the first char that is not indentation if the caret is not
    /// there already (and the start of the row if it is). A row that is only indentation goes
    /// to the start. Sakura: `Command_GOLINETOP` without the "line top only" flag.
    pub fn move_line_top(&mut self, extend: bool) -> Result<()> {
        let caret = self.caret();
        let row_start = self.layout.row_start(self.doc(), caret)?;
        let row_end = self.layout.row_end(self.doc(), caret)?;
        let text = self.doc().text_between(row_start, row_end)?;
        let first = text
            .char_indices()
            .find(|&(_, c)| !is_indent_char(c))
            .map_or(row_start, |(i, _)| {
                Pos::new(row_start.line, row_start.col + i)
            });
        let target = if caret == first { row_start } else { first };
        self.move_caret(target, extend)
    }

    /// The end of the row, before the line ending (Sakura: `Command_GOLINEEND`).
    pub fn move_line_end(&mut self, extend: bool) -> Result<()> {
        let target = self.layout.row_end(self.doc(), self.caret())?;
        self.move_caret(target, extend)
    }

    /// The start of the document (Sakura: `Command_GOFILETOP`).
    pub fn move_file_top(&mut self, extend: bool) -> Result<()> {
        self.move_caret(Pos::new(0, 0), extend)
    }

    /// The end of the document, which is on the empty line after a final line ending
    /// (Sakura: `Command_GOFILEEND`).
    pub fn move_file_end(&mut self, extend: bool) -> Result<()> {
        let end = self.doc().end_pos()?;
        self.move_caret(end, extend)
    }
}
