//! Editing a box selection (Sakura's 矩形選択): copy, delete, typing, indent, conversions and
//! pasting a box.
//!
//! A box is the rows between two horizontal positions, `X` (see [`boxsel`](super::boxsel)). The edits here
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
//! * Typing into a box over several rows of one wrapped line (or the last row of one line and the
//!   first of the next) goes to places in the text that the box input keeps in the cursor
//!   (`Cursor::take_box_places`) until the box changes. Sakura takes the box's columns for every
//!   char, so the first char rewraps the line and the next ones go to other places in the second
//!   row, in the reverse order (a bug, type A). The rows of the box after the typing are the rows
//!   of the first and the last place.
//! * Overwrite mode matters only with the option `m_bOverWriteBoxDelete` (off by default,
//!   `sakura_core/env/CShareData.cpp`): typing then deletes the box first
//!   (`delete_box_keeping_selection`).

use matsu_core::buffer::{Edit, Pos};
use matsu_core::cursor::char_left;

use super::CommandContext;
use super::boxsel::Corner;
use crate::command::Result;
use crate::layout::X;

/// The corners of a box selection, as the rectangle that Sakura's `TwoPointToRect` makes.
pub(super) struct BoxRect {
    /// The first `X` of the box.
    pub left: X,
    /// The `X` after the last one of the box.
    pub right: X,
    /// The first position of each row of the box, top to bottom.
    pub rows: Vec<Pos>,
}

/// What an edit needs to know about a row.
struct RowInfo {
    /// The row is the empty line after a final line ending: there is no text in it.
    eof: bool,
    /// The end of the text of the row (before the line ending, or at the wrap point).
    end: Pos,
    /// The `X` of `end`.
    width: X,
}

/// One insertion of the text into a row of a box.
struct Insertion {
    /// Where the text goes, after `pad` spaces.
    at: Pos,
    pad: usize,
    /// The `X` where the text starts (after the spaces).
    x: X,
}

/// Where the texts of `lengths` (bytes, none with a line break) start in the text after they
/// went in at `places` (top to bottom, the places of the text before): a text is pushed right by
/// the ones before it in the same line. One step for each place, however the places are spread
/// over the lines.
pub(super) fn places_after_typing(places: &[Pos], lengths: &[usize]) -> Vec<Pos> {
    let mut pushed = 0;
    let mut line = None;
    places
        .iter()
        .zip(lengths)
        .map(|(place, len)| {
            if line != Some(place.line) {
                line = Some(place.line);
                pushed = 0;
            }
            let start = Pos::new(place.line, place.col + pushed);
            pushed += len;
            start
        })
        .collect()
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
        let Some(bounds) = self.box_bounds()? else {
            return Ok(None);
        };
        let first = self.layout.row_start(self.doc(), bounds.top)?;
        let last = self.layout.row_start(self.doc(), bounds.bottom)?;
        let mut rows = vec![first];
        while let Some(&row) = rows.last()
            && row < last
            && let Some(next) = self.layout.next_row_start(self.doc(), row)?
        {
            rows.push(next);
        }
        Ok(Some(BoxRect {
            left: bounds.left,
            right: bounds.right,
            rows,
        }))
    }

    /// The bytes that the text of the box selection takes in the document, counted by the
    /// positions of its rows without reading the text, up to `limit` (the count stops there).
    /// Without a box it is 0. The rows are not listed, as a box can cover the whole document
    /// ([`box_rect`](Self::box_rect) lists them).
    pub(super) fn box_bytes(&self, limit: usize) -> Result<usize> {
        let Some(bounds) = self.box_bounds()? else {
            return Ok(0);
        };
        let (left, right) = (bounds.left, bounds.right);
        let last = self.layout.row_start(self.doc(), bounds.bottom)?;
        let mut row = self.layout.row_start(self.doc(), bounds.top)?;
        let mut bytes = 0usize;
        loop {
            let (from, to) = self.box_row_range(row, left, right)?;
            bytes = bytes.saturating_add(self.doc().raw_len_between(from, to)?);
            if bytes >= limit || row >= last {
                return Ok(bytes);
            }
            match self.layout.next_row_start(self.doc(), row)? {
                Some(next) => row = next,
                None => return Ok(bytes),
            }
        }
    }

    fn box_row_info(&self, row: Pos) -> Result<RowInfo> {
        if row.line >= self.doc().line_count() {
            return Ok(RowInfo {
                eof: true,
                end: row,
                width: X::ZERO,
            });
        }
        let end = self.layout.row_end(self.doc(), row)?;
        let width = self.row_text_end_x(row)?;
        Ok(RowInfo {
            eof: false,
            end,
            width,
        })
    }

    /// The text of `row` from the char that covers `left` to the char that covers `right`, as
    /// positions, without the line ending (Sakura: `LineColumnToIndex`).
    pub(super) fn box_row_range(&self, row: Pos, left: X, right: X) -> Result<(Pos, Pos)> {
        if self.box_row_info(row)?.eof {
            return Ok((row, row));
        }
        let from = self.layout.pos_at_x(self.doc(), row, left)?;
        let to = self.layout.pos_at_x(self.doc(), row, right)?;
        Ok((from, to.max(from)))
    }

    /// The box corner at `x` of `row`: the char that covers it, the end of the text if `x` is
    /// past it. On a wrapped row, `x` at the end of its text or past it is in the gap after the
    /// row (the corner is on the row, and its position is the start of the next one); `x` inside
    /// its last char is the left edge of that char.
    pub(super) fn box_corner_pos(&self, row: Pos, x: X) -> Result<Corner> {
        let info = self.box_row_info(row)?;
        let mut corner = Corner {
            pos: row,
            x,
            gap_last: None,
        };
        if info.eof {
            return Ok(corner);
        }
        corner.pos = self.layout.pos_at_x(self.doc(), row, x)?;
        if corner.pos == info.end
            && self.row_is_wrapped(row)?
            && let Some(last) = char_left(self.doc(), info.end)?
            && last >= row
        {
            if x >= info.width {
                corner.gap_last = Some(last);
            } else {
                corner.pos = last;
            }
        }
        Ok(corner)
    }

    /// Makes a box selection with the given corners, keeping the lock of the plain moves as it
    /// was.
    pub(super) fn set_box(&mut self, anchor: Corner, caret: Corner, lock: bool) {
        self.cursor.clear_selection();
        self.cursor.move_to(anchor.pos, false);
        self.begin_box_at(anchor.x, lock);
        if anchor.in_gap() {
            self.set_free_x(Some(anchor.x));
        }
        if caret.in_gap() {
            self.set_box_gap_caret(caret.pos, caret.x);
        } else {
            self.set_box_caret(caret.pos, caret.x);
        }
    }

    // ---- copy and delete ----

    /// The text of the box for the clipboard: the text of each row of it, each followed by CRLF
    /// whatever the line ending of the document is, and an empty line for a row without text
    /// (Sakura: `GetSelectedData`). `None` if no box covers anything.
    pub fn box_text(&self) -> Result<Option<String>> {
        self.box_text_with_eol("\r\n")
    }

    /// Like [`box_text`](Self::box_text) with `eol` after every row. Sakura always uses CRLF;
    /// the MATSU-only copy commands for LF and CR (`F_COPY_LF`, `F_COPY_CR`) ask for their own.
    pub fn box_text_with_eol(&self, eol: &str) -> Result<Option<String>> {
        let Some(rect) = self.box_rect()? else {
            return Ok(None);
        };
        let mut out = String::new();
        for &row in &rect.rows {
            let (from, to) = self.box_row_range(row, rect.left, rect.right)?;
            out.push_str(&self.doc().text_between(from, to)?);
            out.push_str(eol);
        }
        Ok(Some(out))
    }

    /// The text of the first row of the box, without a line ending: all that the macro function
    /// `ExpandParameter` (`$C`) takes of a box. Unlike [`box_text`](Self::box_text) it does not
    /// read the other rows. `None` if no box covers anything.
    pub fn box_first_row_text(&self) -> Result<Option<String>> {
        // The rows are not listed: only the first one is needed, and a box can cover the whole
        // document.
        let Some(bounds) = self.box_bounds()? else {
            return Ok(None);
        };
        let row = self.layout.row_start(self.doc(), bounds.top)?;
        let (from, to) = self.box_row_range(row, bounds.left, bounds.right)?;
        Ok(Some(self.doc().text_between(from, to)?))
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
        // The caret goes to the left column of the top row; past the text it stays at the end. At
        // the left column in the gap after a wrapped row it stays in the gap, on that row.
        let top_left = self.box_corner_pos(rect.rows[0], rect.left)?;
        let caret = ranges[0].0;
        self.cursor.clear_selection();
        for &(from, to) in ranges.iter().rev() {
            if from < to {
                self.replace(from, to, "")?;
            }
        }
        self.end_box_edit_at(caret, top_left)
    }

    /// Ends a box edit that leaves the caret at the top left corner of the box (`caret`, the
    /// position of the corner's column on its row): the selection is gone, and a corner that was in
    /// the gap after a wrapped row keeps the caret in that gap (without the free cursor mode the
    /// column is brought back to the end of the text). Delete and the conversions (case, tabs and
    /// spaces, trimming, ...) end here.
    fn end_box_edit_at(&mut self, caret: Pos, top_left: Corner) -> Result<()> {
        self.cursor.move_to(caret, false);
        let mark = (top_left.in_gap() && caret == top_left.pos).then_some(top_left.x);
        self.keep_gap_mark(mark)
    }

    /// `Command_INDENT` with `m_bOverWriteBoxDelete` in overwrite mode: deletes the text of the
    /// box and selects the same corners again (`DeleteData`, then `GetSelect() = sSelectOld` and
    /// `SetBoxSelect(true)`), so that the typing goes in front of the now empty box.
    pub(super) fn delete_box_keeping_selection(&mut self) -> Result<()> {
        let Some(rect) = self.box_rect()? else {
            return Ok(());
        };
        let Some((anchor, caret)) = self.box_corners()? else {
            return Ok(());
        };
        let lock = self.cursor.selecting_lock();
        let anchor_is_top = anchor.row_pos() <= caret.row_pos();
        let top_index = self.layout.row_index(self.doc(), rect.rows[0])?;
        let bottom_index = self
            .layout
            .row_index(self.doc(), rect.rows[rect.rows.len() - 1])?;
        self.delete_box()?;
        let (anchor_row, caret_row) = if anchor_is_top {
            (top_index, bottom_index)
        } else {
            (bottom_index, top_index)
        };
        // The rows of the box are the same rows (by number) after the delete, as Sakura keeps the
        // layout coordinates; where a long line wraps differently now, they start elsewhere.
        let anchor = self.box_corner_pos(self.row_start_numbered(anchor_row)?, anchor.x)?;
        let caret = self.box_corner_pos(self.row_start_numbered(caret_row)?, caret.x)?;
        self.set_box(anchor, caret, lock);
        Ok(())
    }

    // ---- inserting ----

    /// Inserts `text` at the box caret when the caret is to the right of the text of its row, the
    /// way Sakura pads the row with spaces (`InsertData_CEditView`). Ends the box.
    pub(super) fn insert_in_empty_space(&mut self, text: &str) -> Result<Edit> {
        self.refuse_in_view_mode()?;
        let caret = self.caret();
        let gap = self.box_caret_x()?.saturating_sub(self.caret_display_x()?);
        let pad = self.spaces_to_fill(gap);
        self.cursor.clear_selection();
        let padded = format!("{}{text}", " ".repeat(pad));
        let edit = self.replace(caret, caret, &padded)?;
        self.cursor.move_to(edit.end, false);
        Ok(edit)
    }

    /// `Command_WCHAR` with a box that was just begun (`F_BEGIN_BOX`, no corner moved since): it
    /// covers nothing, but `BeginSelectArea` made a range that `IsTextSelected` counts, so the
    /// char takes the box input, `Command_INDENT`, which inserts in both modes
    /// (`cmd/CViewCommander_Edit.cpp`, `view/CViewSelect.cpp`). A box of one row and no width
    /// inserts at its column, padding the row with spaces when the column is right of the text.
    /// The box stays begun, at the end of the text, as `Command_INDENT` leaves it moved right.
    pub(super) fn insert_in_begun_box(&mut self, text: &str) -> Result<Edit> {
        self.refuse_in_view_mode()?;
        let first = text.split(['\r', '\n']).next().unwrap_or_default();
        let caret = self.caret();
        if first.is_empty() {
            return Ok(Edit {
                start: caret,
                end: caret,
            });
        }
        // The row and the column of the box before the insertion: the box stays on its row and
        // moves right by the width of the char, as `insert_into_box_rows` does (a char that
        // reaches the wrap ends on the next row: the width is then to the right edge).
        let row = self.layout.row_start(self.doc(), self.caret_row_pos()?)?;
        let x0 = self.box_caret_x()?;
        let edit = if self.caret_is_in_empty_space()? {
            self.insert_in_empty_space(first)?
        } else {
            // (the padding reads the mark of the caret, so it comes before the box ends)
            self.pad_free_caret()?;
            self.cursor.clear_selection();
            let at = self.caret();
            let edit = self.replace(at, at, first)?;
            self.cursor.move_to(edit.end, false);
            edit
        };
        // `Command_INDENT` ends the selection (`DisableSelectArea` also drops the lock of the
        // plain moves) and selects the box again, moved right.
        let end_x = self.layout.display_x(self.doc(), edit.end)?;
        let max = self.max_box_x();
        let width = if end_x >= x0 {
            end_x - x0
        } else {
            max.saturating_sub(x0)
        };
        let corner = self.box_corner_pos(row, (x0 + width).min(max))?;
        self.cursor.move_to(corner.pos, false);
        self.begin_box_at(corner.x, false);
        if corner.in_gap() {
            self.set_free_x(Some(corner.x));
        }
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
        self.insert_into_box_rows(first, None, false)?;
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
            let (at, pad, x) = if left >= info.width {
                // the spaces fill the gap and may reach a little past `left`
                let pad = self.spaces_to_fill(left - info.width);
                (info.end, pad, info.width + self.width_of_spaces(pad))
            } else {
                (from, 0, self.layout.display_x(self.doc(), from)?)
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
                if align && (x == left || on_indent_char) {
                    aligned = true;
                    continue;
                }
            }
            plan.push(Insertion { at, pad, x });
        }
        Ok((plan, aligned))
    }

    /// The core of `Command_INDENT` for a box: puts `text` (typed text, or the indent) into each
    /// row, then moves the box right by the width of the text, and leaves it selected with the
    /// caret at its top left corner. `indent` is `Some` for the tab and the space indents. With
    /// `soft_tab` the text is not `text` but the spaces up to the next tab stop from where it goes
    /// into each row (`stabData.Len(ptInsert.x)`).
    fn insert_into_box_rows(
        &mut self,
        text: &str,
        indent: Option<char>,
        soft_tab: bool,
    ) -> Result<()> {
        self.refuse_in_view_mode()?;
        // The places that the last typing left (see `Cursor::take_box_places`). Only typing uses
        // them (an indent starts from the box), only while the text is the one they were made
        // for, and only if each of them is still a place in it.
        let mut places = match self.cursor.take_box_places() {
            Some((version, places))
                if indent.is_none()
                    && version == self.model.text_version()
                    && self.places_are_valid(&places)? =>
            {
                places
            }
            _ => Vec::new(),
        };
        // `Command_INDENT` (typing, the tab and space indents, `InsText`): with
        // `m_bOverWriteBoxDelete` a box typed over in overwrite mode is deleted first and stays
        // selected.
        if self.overwrite
            && self.cursor.has_selection()
            && crate::edit_options::get().overwrite_box_delete
        {
            // the text under the places changes
            places.clear();
            self.delete_box_keeping_selection()?;
        }
        let Some(rect) = self.box_rect()? else {
            return Ok(());
        };
        let lock = self.cursor.selecting_lock();
        // The left side of a full-width char is aligned when the box starts on a tab stop (tab)
        // or is one column wide (space). If that skips every row, the indent is done again
        // without it. (`alignFullWidthChar` in `Command_INDENT`.)
        // Sakura: `0 == from.x % GetTabSpace()` and `1 == to.x - from.x` (`Command_INDENT`). The
        // second one compares with 1 `X` and not with a column, which is a pixel in Sakura and
        // can only be meant as "one column wide": that is what is checked here.
        let align = match indent {
            Some('\t') => rect.left % self.layout.tab_x() == X::ZERO,
            Some(' ') => rect.right - rect.left == self.layout.column_x(),
            _ => false,
        };
        // Typing into a box over rows of a wrapped line keeps the places in the text: each char
        // rewraps the line, and the columns of the box would then point elsewhere (Sakura goes
        // wrong there, and types in the reverse order). The places of the last typing are used
        // as they are, whatever rows the box covers now.
        let tracked =
            indent.is_none() && (!places.is_empty() || self.box_has_wrapped_rows(&rect)?);
        let (mut plan, mut aligned) = if !places.is_empty() {
            (self.plan_at_places(&places)?, false)
        } else {
            self.plan_box_insertions(&rect, indent, align)?
        };
        if plan.is_empty() && align {
            (plan, aligned) = self.plan_box_insertions(&rect, indent, false)?;
        }
        places = plan.iter().map(|ins| ins.at).collect();
        // the length of what went into each place
        let mut lengths = vec![0; plan.len()];
        // a text that broke the line (a line break that the first line of the text did not cut)
        // moves the places below it onto another line: they are not kept then
        let mut broken = false;
        self.cursor.clear_selection();
        // From the bottom up: the rows above do not move, and neither do the places above.
        // `shift` is how far the text pushed the chars right, the least over the rows (0 if a
        // row was left alone for the alignment); the box moves by it.
        let mut shift = if aligned { Some(X::ZERO) } else { None };
        for i in (0..plan.len()).rev() {
            let ins = &plan[i];
            let text = if soft_tab {
                self.soft_tab_at(ins.x)
            } else {
                text.to_owned()
            };
            let padded = format!("{}{text}", " ".repeat(ins.pad));
            let edit = self.replace(places[i], places[i], &padded)?;
            lengths[i] = padded.len();
            broken |= edit.end.line != places[i].line;
            let end_x = self.layout.display_x(self.doc(), edit.end)?;
            // The text reached the wrap point: the end is on the next row. Sakura then counts
            // the width to the right edge (`GetMaxLineLayout()` minus the start).
            let width = if end_x >= ins.x {
                end_x - ins.x
            } else {
                self.max_box_x().saturating_sub(ins.x)
            };
            shift = Some(shift.map_or(width, |s: X| s.min(width)));
        }
        // Where each text starts and ends in the text as it is now.
        let starts = places_after_typing(&places, &lengths);
        let shift = shift.unwrap_or(X::ZERO);
        let max = self.max_box_x();
        let (left, right) = ((rect.left + shift).min(max), (rect.right + shift).min(max));
        // A tracked box covers the rows of the first and the last place: where the rows start now.
        let (top, bottom) = if tracked
            && !broken
            && let (Some(&first), Some(&last)) = (starts.first(), starts.last())
        {
            (
                self.layout.row_start(self.doc(), first)?,
                self.layout.row_start(self.doc(), last)?,
            )
        } else {
            (rect.rows[0], rect.rows[rect.rows.len() - 1])
        };
        let caret = self.box_corner_pos(top, left)?;
        let anchor = self.box_corner_pos(bottom, right)?;
        self.set_box(anchor, caret, lock);
        if tracked && !broken {
            // the places are the ends of the texts
            let ends = starts
                .iter()
                .zip(&lengths)
                .map(|(start, len)| Pos::new(start.line, start.col + len))
                .collect();
            self.cursor.set_box_places(self.model.text_version(), ends);
        }
        Ok(())
    }

    /// Whether the box covers more than one row and one of them belongs to a wrapped line (it
    /// continues a line, or wraps into the next row).
    fn box_has_wrapped_rows(&self, rect: &BoxRect) -> Result<bool> {
        if rect.rows.len() < 2 {
            return Ok(false);
        }
        for &row in &rect.rows {
            if row.col > 0 || self.row_is_wrapped(row)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether every one of `places` is a place in the text (inside it, on a char boundary), and
    /// they are in the order of the text, one for each row: the places of an older text cannot
    /// be trusted to be.
    fn places_are_valid(&self, places: &[Pos]) -> Result<bool> {
        for (i, &place) in places.iter().enumerate() {
            if self.doc().clamp_pos(place)? != place || (i > 0 && places[i - 1] >= place) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The insertions that go right at the places that the last typing left, without padding.
    fn plan_at_places(&self, places: &[Pos]) -> Result<Vec<Insertion>> {
        places
            .iter()
            .map(|&at| {
                Ok(Insertion {
                    at,
                    pad: 0,
                    x: self.layout.display_x(self.doc(), at)?,
                })
            })
            .collect()
    }

    /// The start of the row with the number `index`, or the end of the document if the document
    /// has fewer rows now.
    fn row_start_numbered(&self, index: usize) -> Result<Pos> {
        Ok(self
            .layout
            .row_start_at(self.doc(), index)?
            .unwrap_or_else(|| Pos::new(self.doc().line_count().saturating_sub(1), 0)))
    }

    /// `Command_INDENT` for a box: `F_INDENT_TAB` and `F_INDENT_SPACE`. A box of no width first
    /// just grows to the right edge (nothing is inserted; a second press indents). Otherwise
    /// `unit` goes before the box in each row that has text there (a soft tab is spaces up to the
    /// next tab stop, counted per row).
    pub(super) fn indent_box(&mut self, unit: char) -> Result<()> {
        // A box that covers nothing has corners too (`box_corner_xs` has none then): only
        // whether they are the same is asked, which the stored numbers answer.
        if self.cursor.box_anchor_col() == self.cursor.box_caret_col() {
            let row = self.layout.row_start(self.doc(), self.caret_row_pos()?)?;
            return self.box_move_to_x(row, self.max_box_x());
        }
        let mut utf8 = [0; 4];
        let soft_tab = self.is_soft_tab(unit);
        self.insert_into_box_rows(unit.encode_utf8(&mut utf8), Some(unit), soft_tab)
    }

    /// `Command_PASTEBOX`: pastes `text` as a box with its top left corner at the caret. The
    /// selection, a box or not, is deleted first. Each line of `text` goes into the next row at
    /// the caret's `X`; a row shorter than that is padded with spaces, and a row that
    /// does not exist is added (a row that has no line ending gets one, as Sakura does for the
    /// last line). The caret ends where it started.
    pub(crate) fn paste_box(&mut self, text: &str) -> Result<()> {
        self.refuse_in_view_mode()?;
        // A box whose top left corner is in the gap after a wrapped row pastes there: the caret
        // that the delete leaves cannot say the column without the free cursor mode.
        let mut at_gap = None;
        if self.cursor.is_box() {
            if self.cursor.has_selection() {
                if let Some(rect) = self.box_rect()? {
                    let top_left = self.box_corner_pos(rect.rows[0], rect.left)?;
                    at_gap = top_left.in_gap().then_some((rect.rows[0], rect.left));
                }
                self.delete_box()?;
            }
        } else {
            self.delete_selection()?;
        }
        let (x, first_row) = if let Some((row, left)) = at_gap {
            (left, row)
        } else {
            let x = if self.cursor.is_box() {
                self.box_caret_x()?
            } else {
                // the free cursor mode: the rectangle starts where the caret is shown
                self.caret_x_with_free()?
            };
            (x, self.layout.row_start(self.doc(), self.caret_row_pos()?)?)
        };
        self.cursor.clear_selection();
        let new_line = matsu_core::model::eol_text(self.new_line());

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
                self.insert_at_x(row, x, piece)?;
            }
            // Sakura: a row that does not end with a line ending gets one (the last line of the
            // document, the empty row at the end, and also a wrapped row).
            let info = self.box_row_info(row)?;
            // (The ending is read from the line index: a very long line is not decoded for it.)
            let has_eol =
                !info.eof && !self.row_is_wrapped(row)? && self.doc().line_eol(row.line)?.is_some();
            if !has_eol {
                let end = if info.eof { row } else { info.end };
                self.replace(end, end, new_line)?;
            }
        }
        let caret = self.box_corner_pos(first_row, x)?;
        self.cursor.move_to(caret.pos, false);
        if caret.in_gap() {
            self.set_gap_free_x(x)?;
        }
        Ok(())
    }

    /// Inserts `text` into `row` at `x`, after the padding that an `X` past the end of the text
    /// needs (Sakura: `InsertData_CEditView`, counted by [`spaces_to_fill`](Self::spaces_to_fill)).
    fn insert_at_x(&mut self, row: Pos, x: X, text: &str) -> Result<Edit> {
        let info = self.box_row_info(row)?;
        let (at, pad) = if x >= info.width {
            (info.end, self.spaces_to_fill(x - info.width))
        } else {
            (self.layout.pos_at_x(self.doc(), row, x)?, 0)
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
        let top_left = self.box_corner_pos(rect.rows[0], rect.left)?;
        let caret = ranges[0].0;
        self.cursor.clear_selection();
        for &(from, to) in ranges.iter().rev() {
            if from >= to {
                continue;
            }
            let old = self.doc().text_between(from, to)?;
            let start_col = self.whole_columns(self.layout.display_x(self.doc(), from)?);
            let new = convert(&old, tab_width, start_col);
            if new != old {
                self.replace(from, to, &new)?;
            }
        }
        self.end_box_edit_at(caret, top_left)
    }
}
