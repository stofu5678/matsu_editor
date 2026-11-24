//! Whole-line edits shared by the sort, merge and trim commands.
//!
//! Sakura: `CViewCommander::Command_SORT`, `Command_MERGE` and `Command_TRIM` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`, `CEditView::ConvSelectedArea` in
//! `sakura_core/view/CEditView.cpp` and `CConvert_Trim::DoConvert` in
//! `sakura_core/convert/CConvert_Trim.cpp`.

use matsu_core::buffer::Pos;
use matsu_core::model::eol_text;

use super::CommandContext;
use crate::command::Result;
use crate::layout::X;

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
        if self.cursor.is_box() {
            return self.sort_box_lines(ascending);
        }
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

    /// `F_SORT_ASC` and `F_SORT_DESC` with a box selection: sorts the lines from the top to the
    /// bottom line of the box (both included, also a last line without a line ending) by the text
    /// in the columns of the box. A box of width 0 takes the text from its column to the end of
    /// the line. A line that ends before the box has an empty key. The sort is stable.
    ///
    /// A last line without a line ending gets one for the sort and the line that ends up last
    /// loses its ending (`Command_SORT`, 2013.06.19 Moca). The box stays selected; the caret does
    /// not move to the start of the box as Sakura's does (nothing can see the difference
    /// except a later extending move).
    ///
    /// With a wrapping layout the box is made of rows of the screen, but the lines are sorted
    /// whole: the key of a line is its text in the columns of the box counted from the start of
    /// the line (`LineColumnToIndex( pcDocLine, nCF )` on the logical line; the columns of a box
    /// are at most the wrap width, so they are in the first row of the line). The corners keep
    /// their rows of the screen and their columns, as Sakura restores `sRangeA`, so a box that
    /// was drawn over the later rows of a line stays there.
    fn sort_box_lines(&mut self, ascending: bool) -> Result<()> {
        let Some(((a, a_col), (c, c_col))) = self.box_corner_xs() else {
            return Ok(());
        };
        // `IsTextSelected` is false when both corners are the same.
        if (a, a_col) == (c, c_col) {
            return Ok(());
        }
        let (left, right) = if a_col == c_col {
            (a_col, self.max_box_x())
        } else {
            (a_col.min(c_col), a_col.max(c_col))
        };
        let first = a.line.min(c.line);
        // lines that do not exist (the empty line after a final line ending) are not sorted
        let end = (a.line.max(c.line) + 1).min(self.doc().line_count());
        if end <= first {
            return Ok(());
        }
        self.refuse_in_view_mode()?;
        let new_line = eol_text(self.new_line());
        // Sakura restores the screen rows of the corners (`sRangeA`). The lines before the first
        // sorted one do not change, so each corner is saved as a number of rows from the start of
        // that line, and the rows of the lines before it may stay estimates (they are the same
        // before and after). Only the sorted lines are made exact, down to the lower corner.
        let last = a
            .line
            .max(c.line)
            .min(self.doc().line_count().saturating_sub(1));
        for line in first..=last {
            self.layout.make_exact(self.model.doc(), line);
        }
        let base = self.layout.row_index(self.doc(), Pos::new(first, 0))?;
        let a_off = self.layout.row_index(self.doc(), a)?.saturating_sub(base);
        let c_off = self.layout.row_index(self.doc(), c)?.saturating_sub(base);
        // the key, the line with its ending, and the length of the ending
        let mut keyed: Vec<(Vec<u16>, String, usize)> = Vec::with_capacity(end - first);
        let mut lacked_eol = false;
        for line_no in first..end {
            let Some(line) = self.doc().line(line_no)? else {
                continue;
            };
            let (from, to) = self.box_row_range(Pos::new(line_no, 0), left, right)?;
            let key: Vec<u16> = self.doc().text_between(from, to)?.encode_utf16().collect();
            let mut text = line.text;
            let eol = match line.eol {
                Some(eol) => eol_text(eol),
                None => {
                    // only the last line can lack one
                    lacked_eol = true;
                    new_line
                }
            };
            text.push_str(eol);
            keyed.push((key, text, eol.len()));
        }
        keyed.sort_by(|x, y| {
            let order = x.0.cmp(&y.0);
            if ascending { order } else { order.reverse() }
        });
        let mut sorted: Vec<String> = Vec::with_capacity(keyed.len());
        let mut last_eol_len = 0;
        for (_, line, eol_len) in keyed {
            sorted.push(line);
            last_eol_len = eol_len;
        }
        if lacked_eol && let Some(last) = sorted.last_mut() {
            // the ending of the line that ends up last, whatever it is (CR, LF, CRLF, NEL, ...)
            last.truncate(last.len() - last_eol_len);
        }
        let start = Pos::new(first, 0);
        // a last line without an ending is replaced up to its end
        let old_end = if lacked_eol {
            self.doc().end_pos()?
        } else {
            Pos::new(end, 0)
        };
        let new_text = sorted.concat();
        // Sakura replaces the lines even when they are the same; no edit is made here then.
        if self.doc().text_between(start, old_end)? != new_text {
            let lock = self.cursor.selecting_lock();
            // `replace` refuses while a box is selected
            self.cursor.clear_selection();
            self.replace(start, old_end, &new_text)?;
            // the lines moved: the corners keep their rows and display columns
            // With mixed line endings the new text can join a CR and an LF into one CRLF, so the
            // document can have fewer lines than before: the rows are clamped to the new end.
            // (Sakura inserts the sorted lines as separate `CDocLine`s, see `CSearchAgent::ReplaceData`,
            // `pInsData` loop, so it likely keeps the row count;
            // the saved bytes are the same.)
            let anchor = self.box_corner_below(first, a_off, a_col)?;
            let caret = self.box_corner_below(first, c_off, c_col)?;
            self.set_box((anchor, a_col), (caret, c_col), lock);
        }
        Ok(())
    }

    /// The box corner at `x` of the row `offset` rows below the start of line `first` (the last
    /// row if the document ends sooner). After a sort of very many lines the layout only estimates
    /// the rows of the new lines (one row each): each line is made exact as the walk enters it, so
    /// the work is proportional to the offset. The line above `first` is made exact too, as an
    /// edit makes it inexact (its ending can change).
    fn box_corner_below(&mut self, first: usize, offset: usize, x: X) -> Result<Pos> {
        let line = first.min(self.doc().end_pos()?.line);
        if line > 0 {
            self.layout.make_exact(self.model.doc(), line - 1);
        }
        self.layout.make_exact(self.model.doc(), line);
        let mut start = self.layout.row_start(self.doc(), Pos::new(line, 0))?;
        let mut at = line;
        for _ in 0..offset {
            let Some(next) = self.layout.next_row_start(self.doc(), start)? else {
                break;
            };
            if next.line != at {
                at = next.line;
                self.layout.make_exact(self.model.doc(), at);
            }
            start = next;
        }
        self.box_corner_pos(start, x)
    }

    /// `F_MERGE`: removes each line that is the same as the line before it, with its line
    /// ending, from the selected lines. Nothing selected: nothing happens.
    ///
    /// With a box selection it returns at once, without a beep and without an error
    /// (`Command_MERGE` in `CViewCommander_Edit_advanced.cpp`: `if IsBoxSelecting() return`).
    pub(crate) fn merge_selected_lines(&mut self) -> Result<()> {
        if self.cursor.is_box() && self.cursor.has_selection() {
            return Ok(());
        }
        let Some((first, end)) = self.selected_whole_lines()? else {
            return Ok(());
        };
        let caret_at_start = self.caret().line == first;
        let lines_before = self.doc().line_count();
        let mut merged: Vec<String> = Vec::new();
        for line in self.lines_with_eol(first, end)? {
            if merged.last() != Some(&line) {
                merged.push(line);
            }
        }
        self.replace_whole_lines(first, end, &merged, caret_at_start)?;
        // `STR_ERR_DLGEDITVWCMDNW7` and `STR_ERR_DLGEDITVWCMDNW8`: the number of lines is the
        // difference of the line counts of the document (`j` in `Command_MERGE`).
        let merged_lines = lines_before.saturating_sub(self.doc().line_count());
        self.changes.message_box = Some(if merged_lines > 0 {
            format!("{merged_lines}行をマージしました。")
        } else {
            "マージ可能な行がみつかりませんでした。".to_owned()
        });
        Ok(())
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
                // the row that the caret is drawn on
                let row = self.caret_row_pos()?;
                (
                    self.layout().row_start(self.doc(), row)?,
                    self.layout().row_end(self.doc(), row)?,
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
