//! Indent and unindent, shared by `F_INDENT_TAB`, `F_UNINDENT_TAB`, `F_INDENT_SPACE` and
//! `F_UNINDENT_SPACE`.
//!
//! Sakura: `CViewCommander::Command_INDENT` and `Command_UNINDENT` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`. A box selection is indented by
//! `indent_box`; unindent refuses it, as Sakura does.
//!
//! The tab key is a soft tab when the type says so (`m_bInsSpace`, [`CommandContext::is_ins_space`]):
//! `F_INDENT_TAB` (and `F_UNINDENT_TAB` without a selection, which calls it) then puts spaces up to
//! the next tab stop in place of the tab. `F_INDENT_SPACE` is never soft.
//!
//! The width of the tab is the one of the layout (`m_nTabSpace`). `F_UNINDENT_TAB` removes up to
//! that many spaces; Sakura uses `m_nTabSpace` there too.

use matsu_core::buffer::Pos;

use super::CommandContext;
use crate::command::Result;

/// How many bytes an edit put into (positive) or took out of (negative) the start of a line.
type Shifts = Vec<(usize, isize)>;

impl CommandContext<'_> {
    /// Drops the selection and keeps the caret.
    pub(crate) fn clear_selection(&mut self) {
        self.cursor.clear_selection();
    }

    /// The rows that indent and unindent look at, and the first position after them (`None`: to
    /// the end of the document). From the row that has the start of the selection to the row
    /// that has the end of it; the row of the end is left out if the selection ends at the start
    /// of it. (Sakura: `sSelectOld` in `Command_INDENT`, a row is a layout line.)
    fn rows_of_selection(&self, from: Pos, to: Pos) -> Result<(Vec<Pos>, Option<Pos>)> {
        let (layout, doc) = (self.layout(), self.doc());
        let limit = if layout.row_start(doc, to)? == to {
            Some(to)
        } else {
            layout.next_row_start(doc, to)?
        };
        let mut rows = Vec::new();
        let mut row = layout.row_start(doc, from)?;
        while limit.is_none_or(|limit| row < limit) {
            rows.push(row);
            match layout.next_row_start(doc, row)? {
                Some(next) => row = next,
                None => break,
            }
        }
        Ok((rows, limit))
    }

    /// Selects from the start of the first of `rows` to `limit`, with the caret at the end, after
    /// edits that moved the starts of some lines (`shifts`) and, with mixed line endings, can have
    /// joined a CR and an LF into one line ending, so that there are fewer lines than before
    /// (`lines_before`). The lines after `limit` were not touched, so `limit` moves by the change
    /// of the number of lines.
    fn select_indented_rows(
        &mut self,
        rows: &[Pos],
        limit: Option<Pos>,
        shifts: &Shifts,
        lines_before: usize,
    ) -> Result<()> {
        let end = match limit {
            Some(limit) => {
                let shift = shifts
                    .iter()
                    .find(|&&(line, _)| line == limit.line && limit.col > 0)
                    .map_or(0, |&(_, shift)| shift);
                let line = (limit.line + self.doc().line_count()).saturating_sub(lines_before);
                Pos::new(line, limit.col.saturating_add_signed(shift))
            }
            None => self.doc().end_pos()?,
        };
        self.select_range(rows[0], end)
    }

    /// Whether the tab is typed as spaces: `unit` is a tab and the type says so (`bSoftTab` in
    /// `Command_INDENT`, which only `INDENT_TAB` can make true).
    pub(crate) fn is_soft_tab(&self, unit: char) -> bool {
        unit == '\t' && self.ins_space
    }

    /// The spaces that a soft tab typed at display column `col` is: up to the next tab stop
    /// (`SSoftTabData::Len` in `Command_INDENT`; the width of a space is 1 column, and a
    /// full-width char counts as 2 in `col`).
    pub(crate) fn soft_tab_at(&self, col: usize) -> String {
        let width = self.layout().tab_width().max(1);
        " ".repeat(width - col % width)
    }

    /// Types `unit` at the caret, or in place of a selection inside one row. A soft tab is the
    /// spaces up to the next tab stop, counted from the start of the selection. In overwrite mode
    /// with nothing selected, it takes the place of the char at the caret, as any typed char does
    /// (Sakura: `Command_WCHAR` and, for the soft tab, `DelCharForOverwrite`).
    fn type_indent_unit(&mut self, unit: char) -> Result<()> {
        if !self.is_soft_tab(unit) {
            self.type_text(unit.encode_utf8(&mut [0; 4]))?;
            return Ok(());
        }
        self.refuse_edit_in_box()?;
        let selection = self.selection();
        let (from, mut to) = selection.unwrap_or((self.caret(), self.caret()));
        // `DelCharForOverwrite(pData, nDataLen)` with the tab char: with `m_bOverWriteFixMode` the
        // width the replaced char had beyond a tab char's (one column) stays as spaces after the
        // caret.
        let mut pad = 0;
        if selection.is_none() && self.is_overwrite() {
            let plan = self.overwrite_replacement(from, "\t")?;
            to = plan.end;
            pad = plan.pad;
        }
        let spaces = self.soft_tab_at(self.layout().display_col(self.doc(), from)?);
        let edit = self.replace(from, to, &format!("{spaces}{}", " ".repeat(pad)))?;
        self.cursor.move_to(
            matsu_core::buffer::Pos::new(edit.end.line, edit.end.col - pad),
            false,
        );
        Ok(())
    }

    /// `F_INDENT_TAB` (`unit` is a tab) and `F_INDENT_SPACE` (a space).
    ///
    /// * No selection, or a selection inside one row: types `unit`, replacing the selection
    ///   (Sakura calls `Command_WCHAR`; a soft tab is typed as spaces, see `type_indent_unit`).
    /// * Several rows: puts `unit` before the text of each row that starts a line and is not
    ///   empty (a soft tab is as many spaces as the tab width), then selects from the start of the first row to the start of the row after the
    ///   last, with the caret at the end.
    pub(crate) fn indent_selection(&mut self, unit: char) -> Result<()> {
        if self.cursor.is_box() {
            return self.indent_box(unit);
        }
        let Some((from, to)) = self.selection() else {
            return self.type_indent_unit(unit);
        };
        if self.layout().row_start(self.doc(), from)? == self.layout().row_start(self.doc(), to)? {
            return self.type_indent_unit(unit);
        }
        let unit_text = if self.is_soft_tab(unit) {
            self.soft_tab_at(0)
        } else {
            unit.to_string()
        };
        let (rows, limit) = self.rows_of_selection(from, to)?;
        let (mut shifts, lines_before) = (Shifts::new(), self.doc().line_count());
        // from the bottom, so that the edits do not move the rows that are still to do
        for row in rows.iter().rev().filter(|row| row.col == 0) {
            if self.doc().line_len(row.line)?.is_some_and(|len| len > 0) {
                let at = Pos::new(row.line, 0);
                self.replace(at, at, &unit_text)?;
                shifts.push((row.line, unit_text.len() as isize));
            }
        }
        self.select_indented_rows(&rows, limit, &shifts, lines_before)
    }

    /// `F_UNINDENT_TAB` (`unit` is a tab) and `F_UNINDENT_SPACE` (a space).
    ///
    /// * No selection: types `unit`, like indent, also as a soft tab (Sakura calls
    ///   `Command_INDENT`; it also shows a message, which is not done here).
    /// * A selection: takes one `unit` off the start of each row that starts a line. For the tab,
    ///   a row that starts with spaces loses up to the tab width of them. Then selects the rows
    ///   as indent does.
    pub(crate) fn unindent_selection(&mut self, unit: char) -> Result<()> {
        self.refuse_box_selection()?;
        let Some((from, to)) = self.selection() else {
            return self.type_indent_unit(unit);
        };
        let tab_width = self.layout().tab_width().max(1);
        let (rows, limit) = self.rows_of_selection(from, to)?;
        let (mut shifts, lines_before) = (Shifts::new(), self.doc().line_count());
        for row in rows.iter().rev().filter(|row| row.col == 0) {
            let Some(line) = self.doc().line(row.line)? else {
                continue;
            };
            let count = if line.text.starts_with(unit) {
                1
            } else if unit == '\t' {
                line.text
                    .chars()
                    .take(tab_width)
                    .take_while(|&c| c == ' ')
                    .count()
            } else {
                0
            };
            if count > 0 {
                // the removed chars are a tab or spaces: one byte each
                self.replace(Pos::new(row.line, 0), Pos::new(row.line, count), "")?;
                shifts.push((row.line, -(count as isize)));
            }
        }
        self.select_indented_rows(&rows, limit, &shifts, lines_before)
    }
}
