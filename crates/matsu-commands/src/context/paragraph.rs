//! Moving by paragraphs, and the jump history, for the cursor commands.

use matsu_core::buffer::Pos;

use super::CommandContext;
use crate::command::Result;

/// Sakura's option `m_bStopsBothEndsWhenSearchParagraph` ("stop at both ends of a paragraph");
/// off by default (`sakura_core/env/CShareData.cpp`). See [`crate::edit_options`].
fn stops_both_ends() -> bool {
    crate::edit_options::get().stops_both_ends_paragraph
}

impl CommandContext<'_> {
    /// Whether line `line` is an "empty line" in Sakura's sense: only spaces and tabs before the
    /// line ending (`CDocLine::IsEmptyLine`, `sakura_core/doc/logic/CDocLine.cpp`). `None` when
    /// there is no such line: the empty text after the last line ending is not a line.
    fn is_empty_line(&self, line: usize) -> Result<Option<bool>> {
        Ok(self
            .doc()
            .line(line)?
            .map(|l| l.text.chars().all(|c| c == ' ' || c == '\t')))
    }

    /// How many rows down from the row of `from` the row of `to` is (negative: up), stopping at
    /// the last row.
    fn rows_between(&self, from: Pos, to: Pos) -> Result<isize> {
        let (first, second, sign) = if to < from {
            (to, from, -1)
        } else {
            (from, to, 1)
        };
        let target = self.layout.row_start(self.doc(), second)?;
        let mut rows = 0;
        let mut at = first;
        while self.layout.row_start(self.doc(), at)? != target {
            match self.layout.next_row_start(self.doc(), at)? {
                Some(next) => at = next,
                None => break,
            }
            rows += 1;
        }
        Ok(sign * rows)
    }

    /// Moves the caret `lines` lines of the document (not rows) down or up, keeping the goal
    /// column. Sakura turns the distance into rows with `LogicToLayout` and calls
    /// `Cursor_UPDOWN` with it; the column of the caret picks the row of a wrapped line.
    fn move_vertical_by_lines(&mut self, lines: isize, extend: bool) -> Result<()> {
        let caret = self.caret();
        let line = caret.line.saturating_add_signed(lines);
        let target = self.doc().clamp_pos(Pos::new(line, caret.col))?;
        let rows = self.rows_between(caret, target)?;
        self.move_vertical(rows, extend)
    }

    /// Sakura's `Command_GONEXTPARAGRAPH` (`sakura_core/cmd/CViewCommander_Cursor.cpp`): from a
    /// line with text, down to the first empty line and then on to the first line with text
    /// after it; from an empty line, down to the first line with text. With "stop at both ends"
    /// it stops on the empty line. Does nothing on the line after the last one.
    pub fn move_next_paragraph(&mut self, extend: bool) -> Result<()> {
        let line = self.caret().line;
        let Some(first_is_empty) = self.is_empty_line(line)? else {
            return Ok(());
        };
        let mut n = 1;
        while self.is_empty_line(line + n)? == Some(first_is_empty) {
            n += 1;
        }
        if !first_is_empty && !stops_both_ends() {
            while self.is_empty_line(line + n)? == Some(true) {
                n += 1;
            }
        }
        self.move_vertical_by_lines(n as isize, extend)
    }

    /// Sakura's `Command_GOPREVPARAGRAPH`: the mirror of
    /// [`move_next_paragraph`](Self::move_next_paragraph). Sakura looks at the line just above
    /// the caret: with text there, the caret goes to the first line of that block of text; with
    /// an empty line there, it skips the empty lines and goes to the first line of the block of
    /// text above them. Does nothing when the target would be above the first line.
    /// A line above that does not exist (the caret is on the line after the last one) counts as
    /// empty.
    pub fn move_prev_paragraph(&mut self, extend: bool) -> Result<()> {
        let line = self.caret().line as isize;
        let at = |n: isize| -> Result<Option<bool>> {
            match usize::try_from(line + n) {
                Ok(l) => self.is_empty_line(l),
                Err(_) => Ok(None),
            }
        };
        let mut n = -1;
        let first_is_empty = at(n)?.unwrap_or(true);
        n -= 1;
        while at(n)? == Some(first_is_empty) {
            n -= 1;
        }
        if first_is_empty && !stops_both_ends() {
            while at(n)? == Some(false) {
                n -= 1;
            }
        }
        n += 1;
        if line + n < 0 {
            return Ok(());
        }
        self.move_vertical_by_lines(n, extend)
    }

    /// Adds the caret position to the jump history (Sakura: `CEditView::AddCurrentLineToHistory`).
    pub fn add_to_jump_history(&mut self) {
        let caret = self.caret();
        self.cursor.jump_history_mut().add(caret);
    }

    /// `Command_JUMPHIST_PREV`: goes back one place in the jump history. When there is no place
    /// after the current one, the caret position is added first so that "next" can come back.
    pub fn jump_history_prev(&mut self, extend: bool) -> Result<()> {
        if !self.cursor.jump_history().has_next() {
            self.add_to_jump_history();
            self.cursor.jump_history_mut().go_prev();
        }
        if self.cursor.jump_history().has_prev() {
            self.cursor.jump_history_mut().go_prev();
            return self.go_to_jump_history_current(extend);
        }
        Ok(())
    }

    /// `Command_JUMPHIST_NEXT`: goes forward one place in the jump history.
    pub fn jump_history_next(&mut self, extend: bool) -> Result<()> {
        if self.cursor.jump_history().has_next() {
            self.cursor.jump_history_mut().go_next();
            return self.go_to_jump_history_current(extend);
        }
        Ok(())
    }

    fn go_to_jump_history_current(&mut self, extend: bool) -> Result<()> {
        // Sakura moves with `MoveCursorSelecting(pt, m_bSelectingLock)`.
        let extend = self.extends(extend);
        match self.cursor.jump_history().current() {
            // The history does not follow edits, so the place may be gone: `move_caret` clamps.
            Some(pos) => self.move_caret(pos, extend),
            None => Ok(()),
        }
    }
}
