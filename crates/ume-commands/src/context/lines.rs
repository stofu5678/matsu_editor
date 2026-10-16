//! Whole-line edits shared by the sort, merge and trim commands.
//!
//! Sakura: `CViewCommander::Command_SORT`, `Command_MERGE` and `Command_TRIM` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`, `CEditView::ConvSelectedArea` in
//! `sakura_core/view/CEditView.cpp` and `CConvert_Trim::DoConvert` in
//! `sakura_core/convert/CConvert_Trim.cpp`.

use ume_core::buffer::Pos;
use ume_core::model::eol_text;

use super::CommandContext;
use crate::command::Result;

/// What counts as white space for `F_LTRIM` and `F_RTRIM`: a space, a tab, and the ideographic
/// space. Sakura: `WCODE::IsBlank` in `sakura_core/charset/charcode.h`.
fn is_blank(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{3000}')
}

impl CommandContext<'_> {
    /// The lines that `F_SORT_*` and `F_MERGE` work on, as `first..end`: from the line of the
    /// start of the selection to the line of its end, and the line of the end too if the
    /// selection ends inside it (after column 0) and that line has a line ending. `None` if
    /// nothing is selected or there is no line in the range (a selection inside the last line,
    /// when it has no line ending).
    ///
    /// A last line without a line ending is never in the range, so it is never sorted or
    /// merged (Sakura: "the last line of the file is not a target", in the comments of
    /// `Command_SORT` and `Command_MERGE`).
    pub(crate) fn selected_whole_lines(&mut self) -> Result<Option<(usize, usize)>> {
        self.refuse_box_selection()?;
        let Some((from, to)) = self.selection() else {
            return Ok(None);
        };
        let mut end = to.line;
        if to.col > 0 && self.doc().line(to.line)?.is_some_and(|l| l.eol.is_some()) {
            end += 1;
        }
        Ok((end > from.line).then_some((from.line, end)))
    }

    /// Lines `first..end` with their line endings, one string each.
    fn lines_with_eol(&self, first: usize, end: usize) -> Result<Vec<String>> {
        self.doc()
            .lines(first..end)
            .map(|line| {
                let line = line?;
                Ok(line.text + line.eol.map_or("", eol_text))
            })
            .collect()
    }

    /// Replaces lines `first..end` with `new_lines` (which carry their line endings), unless they
    /// are the same, and selects the new lines from the start of the first to the start of the
    /// line after the last. The caret goes to the start of the selection if `caret_at_start`,
    /// otherwise to its end. (Sakura selects `(0, first)` to `(0, end)` after the command.)
    fn replace_whole_lines(
        &mut self,
        first: usize,
        end: usize,
        new_lines: &[String],
        caret_at_start: bool,
    ) -> Result<()> {
        let (start, old_end) = (Pos::new(first, 0), Pos::new(end, 0));
        let new_text = new_lines.concat();
        // Sakura replaces the lines even when they are the same; no edit is made here then, so
        // that the document does not count as modified.
        let (start, new_end) = if self.doc().text_between(start, old_end)? == new_text {
            (start, old_end)
        } else {
            // The positions come from the edit: with mixed line endings the new text can join a
            // CR with an LF (a CRLF is one line ending), and then the lines are not counted by
            // `new_lines`.
            let edit = self.replace(start, old_end, &new_text)?;
            (edit.start, edit.end)
        };
        if caret_at_start {
            self.select_range(new_end, start)
        } else {
            self.select_range(start, new_end)
        }
    }

    /// `F_SORT_ASC` and `F_SORT_DESC`: sorts the selected lines (see
    /// [`selected_whole_lines`](Self::selected_whole_lines)). Nothing selected, or a selection
    /// inside the last line without a line ending: nothing happens (Sakura does not beep).
    ///
    /// The lines are compared with their line endings, by UTF-16 code unit as Sakura's `wmemcmp`
    /// does on Windows, and a line that is a prefix of another comes first (`CNativeW_comp`
    /// compares the terminating NUL too). The sort is stable, also for the descending order.
    pub(crate) fn sort_selected_lines(&mut self, ascending: bool) -> Result<()> {
        let Some((first, end)) = self.selected_whole_lines()? else {
            return Ok(());
        };
        let caret_at_start = self.caret().line == first;
        let mut keyed: Vec<(Vec<u16>, String)> = self
            .lines_with_eol(first, end)?
            .into_iter()
            .map(|line| (line.encode_utf16().collect(), line))
            .collect();
        keyed.sort_by(|a, b| {
            let order = a.0.cmp(&b.0);
            if ascending { order } else { order.reverse() }
        });
        let sorted: Vec<String> = keyed.into_iter().map(|(_, line)| line).collect();
        self.replace_whole_lines(first, end, &sorted, caret_at_start)
    }

    /// `F_MERGE`: removes each line that is the same as the line before it, with its line
    /// ending, from the selected lines. Nothing selected: nothing happens.
    pub(crate) fn merge_selected_lines(&mut self) -> Result<()> {
        let Some((first, end)) = self.selected_whole_lines()? else {
            return Ok(());
        };
        let caret_at_start = self.caret().line == first;
        let mut merged: Vec<String> = Vec::new();
        for line in self.lines_with_eol(first, end)? {
            if merged.last() != Some(&line) {
                merged.push(line);
            }
        }
        self.replace_whole_lines(first, end, &merged, caret_at_start)
    }

    /// `F_LTRIM` and `F_RTRIM`: removes the white space at the start (`left`) or the end of each
    /// line of the selection, counting only the selected part of the first and the last line.
    /// With a selection, the selection becomes the trimmed text, with the caret at its end.
    /// With no selection the row of the caret is trimmed and the caret goes to the end of it.
    pub(crate) fn trim_selection(&mut self, left: bool) -> Result<()> {
        if self.cursor.is_box() && self.cursor.has_selection() {
            // `Command_TRIM` converts the selection with `ConvSelectedArea`, which trims each
            // row of a box.
            return self.convert_box(|text, _, _| {
                if left {
                    text.trim_start_matches(is_blank).to_owned()
                } else {
                    text.trim_end_matches(is_blank).to_owned()
                }
            });
        }
        let selected = self.selection();
        let (from, to) = match selected {
            Some(range) => range,
            None => {
                let caret = self.caret();
                (
                    self.layout().row_start(self.doc(), caret)?,
                    self.layout().row_end(self.doc(), caret)?,
                )
            }
        };
        let mut trimmed = String::new();
        for line_no in from.line..=to.line {
            // no line: the empty line after a final line ending
            let (text, eol) = self
                .doc()
                .line(line_no)?
                .map_or_else(Default::default, |line| (line.text, line.eol));
            let start = if line_no == from.line { from.col } else { 0 };
            let stop = if line_no == to.line {
                to.col
            } else {
                text.len()
            };
            let piece = text.get(start..stop).unwrap_or_default();
            trimmed.push_str(if left {
                piece.trim_start_matches(is_blank)
            } else {
                piece.trim_end_matches(is_blank)
            });
            if line_no < to.line {
                trimmed.push_str(eol.map_or("", eol_text));
            }
        }
        // Sakura replaces the text even when nothing changes; here no edit is made then.
        let (from, new_end) = if self.doc().text_between(from, to)? == trimmed {
            (from, to)
        } else {
            // the positions come from the edit: the new text can join a CR before it with an LF
            let edit = self.replace(from, to, &trimmed)?;
            (edit.start, edit.end)
        };
        if selected.is_some() {
            self.select_range(from, new_end)
        } else {
            self.move_caret(new_end, false)
        }
    }
}
