//! Editing a box selection (Sakura's 矩形選択): copy, delete, typing, indent, conversions and
//! pasting a box.
//!
//! A box is the rows between two display columns (see [`boxsel`](super::boxsel)). The edits here
//! work on it row by row; each is one edit of the document, so a command is one undo step. The
//! edits drop the box mode first (the low-level [`replace`](CommandContext::replace) refuses to
//! edit while a box is selected, so that a command that does not know boxes cannot take the
//! stream between the corners for its selection) and put a box back when Sakura does.
//!
//! Sakura:
//!
//! * `CEditView::GetSelectedData` (copy) and `CEditView::DeleteData` (delete), the box branches,
//!   in `sakura_core/view/CEditView.cpp` and `CEditView_Command_New.cpp`;
//! * `CViewCommander::Command_INDENT` in `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`, which
//!   typing a char (`Command_WCHAR`, `Command_IME_CHAR`) and inserting text (`Command_INSTEXT`)
//!   call when a box is selected, and which indents with a tab or a space;
//! * `CViewCommander::Command_PASTEBOX` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`, with
//!   `CEditView::InsertData_CEditView` in `CEditView_Command_New.cpp` (padding with spaces);
//! * `CEditView::ConvSelectedArea` in `sakura_core/view/CEditView.cpp` (conversions).
//!
//! Differences from Sakura:
//!
//! * The rows are walked from the bottom up, so that an edit does not move the rows still to do;
//!   the result is the same.
//! * The text of a row never has its line ending. Sakura removes one char at the end of the
//!   copied row when it is a line ending delimiter, which for CRLF would leave the CR.
//! * Overwrite mode is not looked at: Sakura deletes the box first only with the option
//!   `m_bOverWriteBoxDelete`, which is off by default (`sakura_core/env/CShareData.cpp`).

use ume_core::buffer::{Edit, Pos};
use ume_core::cursor::char_left;

use super::{CommandContext, MAX_BOX_COL};
use crate::command::Result;

/// The corners of a box selection, as the rectangle that Sakura's `TwoPointToRect` makes.
pub(super) struct BoxRect {
    /// The first display column of the box.
    pub left: usize,
    /// The column after the last one of the box.
    pub right: usize,
    /// The first position of each row of the box, top to bottom.
    pub rows: Vec<Pos>,
}

/// What an edit needs to know about a row.
struct RowInfo {
    /// The row is the empty line after a final line ending: there is no text in it.
    eof: bool,
    /// The end of the text of the row (before the line ending, or at the wrap point).
    end: Pos,
    /// The display column of `end`.
    width: usize,
}

/// One insertion of the text into a row of a box.
struct Insertion {
    /// Where the text goes, after `pad` spaces.
    at: Pos,
    pad: usize,
    /// The display column where the text starts.
    col: usize,
}

/// Whether Sakura's `Command_INDENT` skips a char when it checks the left side of the box
/// (`IsIndentCharSpaceTab`).
fn is_space_or_tab(c: char) -> bool {
    c == ' ' || c == '\t'
}

impl CommandContext<'_> {
    // ---- the rectangle ----

    /// The rectangle of the box selection, or `None` if no box covers anything.
    pub(super) fn box_rect(&self) -> Result<Option<BoxRect>> {
        let Some(((a, a_col), (c, c_col))) = self.cursor.box_corners() else {
            return Ok(None);
        };
        let first = self.layout.row_start(self.doc(), a.min(c))?;
        let last = self.layout.row_start(self.doc(), a.max(c))?;
        let mut rows = vec![first];
        while let Some(&row) = rows.last()
            && row < last
            && let Some(next) = self.layout.next_row_start(self.doc(), row)?
        {
            rows.push(next);
        }
        Ok(Some(BoxRect {
            left: a_col.min(c_col),
            right: a_col.max(c_col),
            rows,
        }))
    }

    fn box_row_info(&self, row: Pos) -> Result<RowInfo> {
        if row.line >= self.doc().line_count() {
            return Ok(RowInfo {
                eof: true,
                end: row,
                width: 0,
            });
        }
        let end = self.layout.row_end(self.doc(), row)?;
        let width = if self.row_is_wrapped(row)? {
            // The end of a wrapped row is the start of the next row for a position, so its
            // column may be that of the next row: take the first column that reaches the end.
            let mut width = match char_left(self.doc(), end)? {
                Some(last) if last >= row => self.layout.display_col(self.doc(), last)?,
                _ => 0,
            };
            while self.layout.pos_at_col(self.doc(), row, width)? < end {
                width += 1;
            }
            width
        } else {
            self.layout.display_col(self.doc(), end)?
        };
        Ok(RowInfo {
            eof: false,
            end,
            width,
        })
    }

    /// The text of `row` from the char that covers column `left` to the char that covers column
    /// `right`, as positions, without the line ending (Sakura: `LineColumnToIndex`).
    fn box_row_range(&self, row: Pos, left: usize, right: usize) -> Result<(Pos, Pos)> {
        if self.box_row_info(row)?.eof {
            return Ok((row, row));
        }
        let from = self.layout.pos_at_col(self.doc(), row, left)?;
        let to = self.layout.pos_at_col(self.doc(), row, right)?;
        Ok((from, to.max(from)))
    }

    /// The position of the box corner at display column `col` of `row`: the char that covers the
    /// column, the end of the text if the column is past it, and on a wrapped row the last char.
    fn box_corner_pos(&self, row: Pos, col: usize) -> Result<Pos> {
        let info = self.box_row_info(row)?;
        if info.eof {
            return Ok(row);
        }
        let pos = self.layout.pos_at_col(self.doc(), row, col)?;
        if pos == info.end
            && self.row_is_wrapped(row)?
            && let Some(last) = char_left(self.doc(), info.end)?
            && last >= row
        {
            return Ok(last);
        }
        Ok(pos)
    }

    /// Makes a box selection with the given corners (position and display column), keeping the
    /// lock of the plain moves as it was.
    fn set_box(&mut self, anchor: (Pos, usize), caret: (Pos, usize), lock: bool) {
        self.cursor.clear_selection();
        self.cursor.move_to(anchor.0, false);
        self.cursor.begin_box(anchor.1, lock);
        self.cursor.move_box_to(caret.0, caret.1);
    }

    // ---- copy and delete ----

    /// The text of the box for the clipboard: the text of each row of it, each followed by CRLF
    /// whatever the line ending of the document is, and an empty line for a row without text
    /// (Sakura: `GetSelectedData`). `None` if no box covers anything.
    pub(crate) fn box_text(&self) -> Result<Option<String>> {
        let Some(rect) = self.box_rect()? else {
            return Ok(None);
        };
        let mut out = String::new();
        for &row in &rect.rows {
            let (from, to) = self.box_row_range(row, rect.left, rect.right)?;
            out.push_str(&self.doc().text_between(from, to)?);
            out.push_str("\r\n");
        }
        Ok(Some(out))
    }

    /// Deletes the text of the box, row by row, and puts the caret at its top left corner. A box
    /// that has no text at its left column just ends (Sakura: `DeleteData`, `IsEmptyArea`). The
    /// selection is gone afterwards.
    pub(crate) fn delete_box(&mut self) -> Result<()> {
        self.refuse_in_view_mode()?;
        let Some(rect) = self.box_rect()? else {
            return Ok(());
        };
        let mut ranges = Vec::with_capacity(rect.rows.len());
        for &row in &rect.rows {
            ranges.push(self.box_row_range(row, rect.left, rect.right)?);
        }
        // The caret goes to the left column of the top row; past the text it stays at the end.
        let caret = ranges[0].0;
        self.cursor.clear_selection();
        for &(from, to) in ranges.iter().rev() {
            if from < to {
                self.replace(from, to, "")?;
            }
        }
        self.cursor.move_to(caret, false);
        Ok(())
    }

    // ---- inserting ----

    /// Inserts `text` at the box caret when the caret is to the right of the text of its row, the
    /// way Sakura pads the row with spaces (`InsertData_CEditView`). Ends the box.
    pub(super) fn insert_in_empty_space(&mut self, text: &str) -> Result<Edit> {
        self.refuse_in_view_mode()?;
        let caret = self.caret();
        let pad = self
            .box_caret_col()?
            .saturating_sub(self.caret_display_col()?);
        self.cursor.clear_selection();
        let padded = format!("{}{text}", " ".repeat(pad));
        let edit = self.replace(caret, caret, &padded)?;
        self.cursor.move_to(edit.end, false);
        Ok(edit)
    }

    /// `Command_INSTEXT` and `Command_WCHAR` with a box selected: the first line of `text` (up to
    /// the first line ending) goes before the box in every row. An empty first line does
    /// nothing. The box moves right by the width of the text.
    pub(super) fn insert_into_box(&mut self, text: &str) -> Result<Edit> {
        let first = text.split(['\r', '\n']).next().unwrap_or_default();
        let caret = self.caret();
        if first.is_empty() {
            return Ok(Edit {
                start: caret,
                end: caret,
            });
        }
        self.insert_into_box_rows(first, None)?;
        Ok(Edit {
            start: caret,
            end: self.caret(),
        })
    }

    /// Plans where `Command_INDENT` puts `text` in the rows of the box. `indent` is the char of
    /// an indent (a tab or a space), which also skips the rows that have nothing to indent and
    /// the rows whose left side `align` says to leave alone. Returns the insertions, top to
    /// bottom, and whether a row was left alone to keep full-width chars aligned.
    fn plan_box_insertions(
        &self,
        rect: &BoxRect,
        indent: Option<char>,
        align: bool,
    ) -> Result<(Vec<Insertion>, bool)> {
        let (left, right) = (rect.left, rect.right);
        let mut plan = Vec::new();
        let mut aligned = false;
        for &row in &rect.rows {
            let info = self.box_row_info(row)?;
            let (from, to) = self.box_row_range(row, left, right)?;
            // `from` and `to` are the chars at the two columns; past the text both are the end.
            let (at, pad, col) = if left >= info.width {
                (info.end, left - info.width, left)
            } else {
                (from, 0, self.layout.display_col(self.doc(), from)?)
            };
            if indent.is_some() {
                // Nothing to indent in an empty row, or to the right of the text.
                if info.eof || info.end == row || left > info.width {
                    continue;
                }
                let first_char = self.doc().text_between(from, info.end)?.chars().next();
                let on_indent_char = first_char.is_some_and(is_space_or_tab);
                // The box lies inside one indent char: leave it (but not for a box of no width).
                if from == to && on_indent_char && left < right {
                    continue;
                }
                // The left side of a full-width char fits the box exactly, or the char is an
                // indent char: no insertion, so the indents line up.
                if align && (col == left || on_indent_char) {
                    aligned = true;
                    continue;
                }
            }
            plan.push(Insertion { at, pad, col });
        }
        Ok((plan, aligned))
    }

    /// The core of `Command_INDENT` for a box: puts `text` (typed text, or the indent) into each
    /// row, then moves the box right by the width of the text, and leaves it selected with the
    /// caret at its top left corner. `indent` is `Some` for the tab and the space indents.
    fn insert_into_box_rows(&mut self, text: &str, indent: Option<char>) -> Result<()> {
        self.refuse_in_view_mode()?;
        let Some(rect) = self.box_rect()? else {
            return Ok(());
        };
        let lock = self.cursor.selecting_lock();
        // The left side of a full-width char is aligned when the box starts on a tab stop (tab)
        // or is one column wide (space). If that skips every row, the indent is done again
        // without it. (`alignFullWidthChar` in `Command_INDENT`.)
        let align = match indent {
            Some('\t') => rect.left % self.layout.tab_width().max(1) == 0,
            Some(' ') => rect.right - rect.left == 1,
            _ => false,
        };
        let (mut plan, mut aligned) = self.plan_box_insertions(&rect, indent, align)?;
        if plan.is_empty() && align {
            (plan, aligned) = self.plan_box_insertions(&rect, indent, false)?;
        }
        self.cursor.clear_selection();
        // From the bottom up: the rows above do not move.
        // `shift` is how far the text pushed the chars right, the least over the rows (0 if a
        // row was left alone for the alignment); the box moves by it.
        let mut shift = if aligned { Some(0) } else { None };
        for ins in plan.iter().rev() {
            let padded = format!("{}{text}", " ".repeat(ins.pad));
            let edit = self.replace(ins.at, ins.at, &padded)?;
            let end_col = self.layout.display_col(self.doc(), edit.end)?;
            let width = end_col.saturating_sub(ins.col);
            shift = Some(shift.map_or(width, |s: usize| s.min(width)));
        }
        let shift = shift.unwrap_or(0);
        let (left, right) = (
            (rect.left + shift).min(MAX_BOX_COL),
            (rect.right + shift).min(MAX_BOX_COL),
        );
        let (top, bottom) = (rect.rows[0], rect.rows[rect.rows.len() - 1]);
        let caret = (self.box_corner_pos(top, left)?, left);
        let anchor = (self.box_corner_pos(bottom, right)?, right);
        self.set_box(anchor, caret, lock);
        Ok(())
    }

    /// `Command_INDENT` for a box: `F_INDENT_TAB` and `F_INDENT_SPACE`. A box of no width first
    /// just grows to the right edge (nothing is inserted; a second press indents). Otherwise
    /// `unit` goes before the box in each row that has text there.
    pub(super) fn indent_box(&mut self, unit: char) -> Result<()> {
        let (anchor, caret) = (self.cursor.box_anchor_col(), self.cursor.box_caret_col());
        if anchor == caret {
            let row = self.layout.row_start(self.doc(), self.caret())?;
            return self.box_move_to_col(row, MAX_BOX_COL);
        }
        let mut utf8 = [0; 4];
        self.insert_into_box_rows(unit.encode_utf8(&mut utf8), Some(unit))
    }

    /// `Command_PASTEBOX`: pastes `text` as a box with its top left corner at the caret. The
    /// selection, a box or not, is deleted first. Each line of `text` goes into the next row at
    /// the caret's display column; a row shorter than that is padded with spaces, and a row that
    /// does not exist is added (a row that has no line ending gets one, as Sakura does for the
    /// last line). The caret ends where it started.
    pub(crate) fn paste_box(&mut self, text: &str) -> Result<()> {
        self.refuse_in_view_mode()?;
        if self.cursor.is_box() {
            if self.cursor.has_selection() {
                self.delete_box()?;
            }
        } else {
            self.delete_selection()?;
        }
        let col = if self.cursor.is_box() {
            self.box_caret_col()?
        } else {
            self.caret_display_col()?
        };
        let first_row = self.layout.row_start(self.doc(), self.caret())?;
        self.cursor.clear_selection();
        let new_line = ume_core::model::eol_text(self.new_line());

        // The lines of the text, ending at CR, LF or CRLF; text after the last ending is a line,
        // and an empty text has none (the loop of `Command_PASTEBOX`).
        let mut lines: Vec<&str> = Vec::new();
        let mut rest = text;
        while !rest.is_empty() {
            let end = rest.find(['\r', '\n']).unwrap_or(rest.len());
            lines.push(&rest[..end]);
            let ending = if rest[end..].starts_with("\r\n") {
                2
            } else {
                1
            };
            rest = rest.get(end + ending..).unwrap_or_default();
        }

        let mut row = first_row;
        for (i, piece) in lines.iter().enumerate() {
            if i > 0 {
                // the next row exists: the previous one got a line ending
                row = self
                    .layout
                    .next_row_start(self.doc(), row)?
                    .unwrap_or(self.doc().end_pos()?);
            }
            if !piece.is_empty() {
                self.insert_at_col(row, col, piece)?;
            }
            // Sakura: a row that does not end with a line ending gets one (the last line of the
            // document, the empty row at the end, and also a wrapped row).
            let info = self.box_row_info(row)?;
            let has_eol = !info.eof
                && !self.row_is_wrapped(row)?
                && self
                    .doc()
                    .line(row.line)?
                    .is_some_and(|line| line.eol.is_some());
            if !has_eol {
                let end = if info.eof { row } else { info.end };
                self.replace(end, end, new_line)?;
            }
        }
        let caret = self.box_corner_pos(first_row, col)?;
        self.cursor.move_to(caret, false);
        Ok(())
    }

    /// Inserts `text` into `row` at display column `col`, after the padding that a column past
    /// the end of the text needs (Sakura: `InsertData_CEditView`).
    fn insert_at_col(&mut self, row: Pos, col: usize, text: &str) -> Result<Edit> {
        let info = self.box_row_info(row)?;
        let (at, pad) = if col >= info.width {
            (info.end, col - info.width)
        } else {
            (self.layout.pos_at_col(self.doc(), row, col)?, 0)
        };
        self.replace(at, at, &format!("{}{text}", " ".repeat(pad)))
    }

    // ---- conversions ----

    /// Replaces the text of each row of the box by `convert(text, tab width, start column)`
    /// (Sakura: the box branch of `ConvSelectedArea`). Rows without text are left alone. The
    /// box is dropped and the caret goes to its top left corner.
    pub(super) fn convert_box(
        &mut self,
        mut convert: impl FnMut(&str, usize, usize) -> String,
    ) -> Result<()> {
        self.refuse_in_view_mode()?;
        let Some(rect) = self.box_rect()? else {
            return Ok(());
        };
        let tab_width = self.layout.tab_width();
        let mut ranges = Vec::with_capacity(rect.rows.len());
        for &row in &rect.rows {
            ranges.push(self.box_row_range(row, rect.left, rect.right)?);
        }
        let caret = ranges[0].0;
        self.cursor.clear_selection();
        for &(from, to) in ranges.iter().rev() {
            if from >= to {
                continue;
            }
            let old = self.doc().text_between(from, to)?;
            let start_col = self.layout.display_col(self.doc(), from)?;
            let new = convert(&old, tab_width, start_col);
            if new != old {
                self.replace(from, to, &new)?;
            }
        }
        self.cursor.move_to(caret, false);
        Ok(())
    }
}
