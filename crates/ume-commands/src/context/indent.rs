//! Indent and unindent, shared by `F_INDENT_TAB`, `F_UNINDENT_TAB`, `F_INDENT_SPACE` and
//! `F_UNINDENT_SPACE`.
//!
//! Sakura: `CViewCommander::Command_INDENT` and `Command_UNINDENT` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`. Rectangular selection is not done.
//!
//! Not done yet, with Sakura's defaults used (`InitTypeConfigDefault` in
//! `sakura_core/types/CType.cpp`): the tab width is 4 (`m_nTabSpace`; the `Layout` trait does not
//! tell it), and the tab is a real tab (`m_bInsSpace` is off, so no soft tab).

use ume_core::buffer::Pos;

use super::CommandContext;
use crate::command::Result;

/// Sakura's default tab width, which `F_UNINDENT_TAB` uses as the most spaces it removes.
const TAB_WIDTH: usize = 4;

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

    /// `F_INDENT_TAB` (`unit` is a tab) and `F_INDENT_SPACE` (a space).
    ///
    /// * No selection, or a selection inside one row: types `unit`, replacing the selection
    ///   (Sakura calls `Command_WCHAR`).
    /// * Several rows: puts `unit` before the text of each row that starts a line and is not
    ///   empty, then selects from the start of the first row to the start of the row after the
    ///   last, with the caret at the end.
    pub(crate) fn indent_selection(&mut self, unit: char) -> Result<()> {
        self.refuse_box_selection()?;
        let unit_text = unit.encode_utf8(&mut [0; 4]).to_owned();
        let Some((from, to)) = self.selection() else {
            self.insert_text(&unit_text)?;
            return Ok(());
        };
        if self.layout().row_start(self.doc(), from)? == self.layout().row_start(self.doc(), to)? {
            self.insert_text(&unit_text)?;
            return Ok(());
        }
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
    /// * No selection: types `unit`, like indent (Sakura calls `Command_INDENT`; it also shows
    ///   a message, which is not done here).
    /// * A selection: takes one `unit` off the start of each row that starts a line. For the tab,
    ///   a row that starts with spaces loses up to the tab width of them. Then selects the rows
    ///   as indent does.
    pub(crate) fn unindent_selection(&mut self, unit: char) -> Result<()> {
        self.refuse_box_selection()?;
        let Some((from, to)) = self.selection() else {
            self.insert_text(unit.encode_utf8(&mut [0; 4]))?;
            return Ok(());
        };
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
                    .take(TAB_WIDTH)
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
