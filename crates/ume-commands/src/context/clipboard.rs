//! What the clipboard commands share: copy, cut and paste of the selection or of the row.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Clipboard.cpp` (`Command_CUT`, `Command_COPY`,
//! `Command_PASTE`, `Command_INSTEXT`, `Command_COPYLINES*`), `CEditView::CopyCurLine` and
//! `CEditView::GetSelectedData` in `sakura_core/view/CEditView_Command.cpp` and `CEditView.cpp`,
//! `CViewCommander::ConvertEol` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Three of the settings that these functions read in Sakura (`m_Common.m_sEdit`) come from
//! [`crate::edit_options`] (Sakura's defaults, `sakura_core/env/CShareData.cpp`, `InitShareData`,
//! until the application sets them); the others are constants.

use ume_core::buffer::{Eol, Pos};
use ume_core::model::eol_text;

use super::CommandContext;
use crate::clipboard::{ClipData, ClipKind};
use crate::command::{CommandError, Result};

/// `bEnableNoSelectCopy`: copy and cut without a selection take the row. Default on.
fn enable_no_select_copy() -> bool {
    crate::edit_options::get().enable_no_select_copy
}
/// `bEnableLineModePaste`: a copy of a row is pasted at the start of the row. Default on.
fn enable_line_mode_paste() -> bool {
    crate::edit_options::get().enable_line_mode_paste
}
/// `bConvertEOLPaste`: paste converts the line endings to the document's. Default off.
fn convert_eol_paste() -> bool {
    crate::edit_options::get().convert_eol_paste
}
/// `bAutoColumnPaste`: the text of a box copy is pasted as a box. Default on.
const AUTO_COLUMN_PASTE: bool = true;
/// `m_szInyouKigou`: the quote of `F_COPYLINESASPASSAGE` (`m_sFormat`). Default `"> "`.
pub(crate) const QUOTE: &str = "> ";
/// `LINENUMWIDTH_MIN` in `sakura_core/config/app_constants.h`: the least number of digits of the
/// line numbers of `F_COPYLINESWITHLINENUMBER`.
const LINE_NUMBER_MIN_DIGITS: usize = 2;

/// The bits of the argument of `F_PASTE` (`Command_PASTE`): the macro can force each setting.
pub(crate) mod paste_option {
    /// Convert the line endings.
    pub const CONVERT_EOL: i64 = 0x01;
    /// Do not convert the line endings.
    pub const KEEP_EOL: i64 = 0x02;
    /// Paste a row copy at the start of the row.
    pub const LINE_MODE: i64 = 0x04;
    /// Paste a row copy at the caret.
    pub const NO_LINE_MODE: i64 = 0x08;
    /// Paste the text of a box copy as a box.
    pub const BOX: i64 = 0x10;
    /// Paste the text of a box copy as normal text.
    pub const NO_BOX: i64 = 0x20;
}

/// Whether `c` ends a line (Sakura: `WCODE::IsLineDelimiter(c, bEnableExtEol)`). `ext_eol` counts
/// U+0085, U+2028 and U+2029: for text of the document it is the setting of the document, as the
/// buffer treats them; for text from the clipboard it is the setting now.
fn is_eol_char(c: char, ext_eol: bool) -> bool {
    matches!(c, '\r' | '\n') || (ext_eol && matches!(c, '\u{85}' | '\u{2028}' | '\u{2029}'))
}

/// Replaces every line ending in `text` with `eol`. `extended` also counts U+0085, U+2028 and
/// U+2029 as line endings (Sakura: `m_bEnableExtEol`, off by default, for paste; a selection
/// taken from the document only has the endings the document counts).
pub(crate) fn convert_eols(text: &str, eol: &str, extended: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str(eol);
            }
            '\n' => out.push_str(eol),
            '\u{85}' | '\u{2028}' | '\u{2029}' if extended => out.push_str(eol),
            _ => out.push(c),
        }
    }
    out
}

/// Whether the text of the range `start..end` of the document ends with a line ending: it ends at
/// the start of a later line. This reads the endings the document has (Sakura copies with the
/// ending recorded in each line, `CDocLine::m_cEol`), not the setting now.
fn ends_with_break(start: Pos, end: Pos) -> bool {
    end.col == 0 && end.line > start.line
}

impl CommandContext<'_> {
    /// The text from `from` to `to` with every line ending of the document replaced by `eol`
    /// (Sakura: `GetSelectedData` writes the new ending where the layout line has its own). The
    /// endings are the ones the document has, found by lines; characters that look like a line
    /// ending but are not one in this document (NEL, LS and PS with the setting off when the text
    /// was made) stay as they are.
    pub fn text_with_eol(&self, from: Pos, to: Pos, eol: &str) -> Result<String> {
        let doc = self.doc();
        let (from, to) = (doc.clamp_pos(from)?, doc.clamp_pos(to)?);
        let mut out = String::new();
        for line in from.line..=to.line {
            let start = if line == from.line { from.col } else { 0 };
            let end = if line == to.line { to.col } else { usize::MAX };
            out.push_str(&doc.text_between(Pos::new(line, start), Pos::new(line, end))?);
            if line < to.line {
                // The ending itself is what lies between the end of this line and the next one.
                let line_end = Pos::new(line, usize::MAX);
                if !doc
                    .text_between(line_end, Pos::new(line + 1, 0))?
                    .is_empty()
                {
                    out.push_str(eol);
                }
            }
        }
        Ok(out)
    }

    /// The text on the clipboard, or `None` if there is none. Fails if this context has no
    /// clipboard.
    pub fn clipboard_get(&mut self) -> Result<Option<ClipData>> {
        match self.clipboard.as_deref_mut() {
            Some(clipboard) => Ok(clipboard.get()),
            None => Err(CommandError::Unavailable("the clipboard")),
        }
    }

    /// Replaces the content of the clipboard. Returns `false` if it could not be written.
    /// Fails if this context has no clipboard.
    pub fn clipboard_set(&mut self, text: String, kind: ClipKind) -> Result<bool> {
        match self.clipboard.as_deref_mut() {
            Some(clipboard) => Ok(clipboard.set(ClipData { text, kind })),
            None => Err(CommandError::Unavailable("the clipboard")),
        }
    }

    /// Copies `text` and beeps if the clipboard refuses it (Sakura: `ErrorBeep()` after a failed
    /// `MySetClipboardData`). Returns whether it worked.
    fn copy_text(&mut self, text: String, kind: ClipKind) -> Result<bool> {
        let done = self.clipboard_set(text, kind)?;
        if !done {
            self.beep();
        }
        Ok(done)
    }

    /// The text of the row that has the caret, with its line ending, or `None` if there is no row
    /// (the caret is on the empty line after a final line ending; Sakura: no layout there).
    fn caret_row_text(&self) -> Result<Option<(Pos, Pos, String)>> {
        let caret = self.caret();
        if caret.line >= self.doc().line_count() {
            return Ok(None);
        }
        let start = self.layout().row_start(self.doc(), caret)?;
        let end = match self.layout().next_row_start(self.doc(), caret)? {
            Some(next) => next,
            None => self.layout().row_end(self.doc(), caret)?,
        };
        let text = self.doc().text_between(start, end)?;
        Ok(Some((start, end, text)))
    }

    /// The selected text with the line endings changed to `eol` if given (Sakura:
    /// `GetSelectedData`), and how it is marked on the clipboard. A box selection gives the text
    /// of its rows, each followed by CRLF (see [`box_text`](Self::box_text)); `eol` and `add_eol`
    /// do not apply to it. `add_eol` (Sakura: `bAddCRLFWhenCopy`) ends every row that does not
    /// end the line, a wrapped row, with the line ending of the document or `eol`.
    fn selected_text(
        &mut self,
        eol: Option<Eol>,
        add_eol: bool,
    ) -> Result<Option<(String, ClipKind)>> {
        let Some((from, to)) = self.selection() else {
            return Ok(None);
        };
        if self.cursor().is_box() {
            return Ok(self.box_text()?.map(|text| (text, ClipKind::Box)));
        }
        if add_eol {
            let text = self.text_with_row_endings(from, to, eol)?;
            return Ok(Some((text, ClipKind::Normal)));
        }
        let text = match eol {
            Some(eol) => self.text_with_eol(from, to, eol_text(eol))?,
            None => self.doc().text_between(from, to)?,
        };
        Ok(Some((text, ClipKind::Normal)))
    }

    /// The text from `from` to `to` where every row that the selection takes to its end gets a
    /// line ending if it has none (Sakura: the `bAddCRLFWhenCopy` part of `GetSelectedData`).
    /// Rows without text are left out, as there.
    fn text_with_row_endings(&self, from: Pos, to: Pos, eol: Option<Eol>) -> Result<String> {
        let new_line = eol_text(eol.unwrap_or_else(|| self.new_line()));
        let mut out = String::new();
        let mut row = self.layout().row_start(self.doc(), from)?;
        while row < to {
            let next = self.layout().next_row_start(self.doc(), row)?;
            let row_end = match next {
                Some(next) => next,
                None => self.layout().row_end(self.doc(), row)?,
            };
            let (start, end) = (from.max(row), to.min(row_end));
            if start < end {
                if end == row_end && !ends_with_break(start, end) {
                    out.push_str(&self.doc().text_between(start, end)?);
                    out.push_str(new_line);
                } else if let Some(eol) = eol {
                    out.push_str(&self.text_with_eol(start, end, eol_text(eol))?);
                } else {
                    out.push_str(&self.doc().text_between(start, end)?);
                }
            }
            match next {
                Some(next) => row = next,
                None => break,
            }
        }
        Ok(out)
    }

    /// `Command_COPY`: copies the selection, or the row if nothing is selected. `eol` changes the
    /// line endings of the copy (`F_COPY_CRLF`). `add_eol` ends the rows that wrap with a line
    /// ending (`F_COPY_ADDCRLF`; Sakura's `bAddCRLFWhenCopy`, off by default). A box is copied as
    /// a box.
    pub fn copy(&mut self, eol: Option<Eol>, add_eol: bool) -> Result<()> {
        if self.selection().is_none() {
            if !enable_no_select_copy() {
                return Ok(()); // Sakura: nothing, not even a beep
            }
            // `CopyCurLine`: no row, no copy and no beep.
            if let Some((start, end, mut text)) = self.caret_row_text()? {
                if let Some(eol) = eol {
                    text = self.text_with_eol(start, end, eol_text(eol))?;
                }
                // A row that wraps has no line ending: `CopyCurLine` adds CRLF, or the ending
                // that was asked for.
                if add_eol && !ends_with_break(start, end) {
                    text.push_str(eol.map_or("\r\n", eol_text));
                }
                let kind = if enable_line_mode_paste() {
                    ClipKind::Line
                } else {
                    ClipKind::Normal
                };
                self.copy_text(text, kind)?;
            }
            return Ok(());
        }
        if let Some((text, kind)) = self.selected_text(eol, add_eol)? {
            self.copy_text(text, kind)?;
        }
        Ok(())
    }

    /// `Command_CUT`: copies the selection and deletes it. Without a selection it cuts the row.
    /// A box is cut as a box.
    pub fn cut(&mut self) -> Result<()> {
        self.refuse_in_view_mode()?;
        if self.selection().is_none() {
            if !enable_no_select_copy() {
                return Ok(());
            }
            return self.cut_row();
        }
        let Some((text, kind)) = self.selected_text(None, false)? else {
            return Ok(());
        };
        if self.copy_text(text, kind)? {
            self.delete_selection()?;
        }
        Ok(())
    }

    /// `Command_CUT_LINE`: copies the row of the caret and deletes it. Beeps with a selection
    /// and when there is no row.
    pub fn cut_row(&mut self) -> Result<()> {
        self.refuse_in_view_mode()?;
        let Some((start, end, text)) = self.caret_row_text()? else {
            self.beep();
            return Ok(());
        };
        if self.selection().is_some() {
            self.beep();
            return Ok(());
        }
        let kind = if enable_line_mode_paste() {
            ClipKind::Line
        } else {
            ClipKind::Normal
        };
        // `CopyCurLine` beeps when the clipboard fails, and the row is deleted anyway.
        self.copy_text(text, kind)?;
        // The same as `F_DELETE_LINE`: the caret keeps its display column on the row that
        // takes the place of the deleted one.
        let col = self.caret_display_col()?;
        self.delete_range(start, end)?;
        if start.line < self.doc().line_count() {
            let pos = self.layout().pos_at_col(self.doc(), start, col)?;
            self.move_caret(pos, false)?;
        }
        Ok(())
    }

    /// `Command_LineCutToStart`: cuts from the start of the line to the caret.
    pub fn cut_to_line_start(&mut self) -> Result<()> {
        self.refuse_in_view_mode()?;
        if self.selection().is_some() {
            return self.cut();
        }
        let caret = self.caret();
        let start = Pos::new(caret.line, 0);
        if caret.line >= self.doc().line_count() || caret == start {
            self.beep();
            return Ok(());
        }
        self.select_range(start, caret)?;
        self.cut()
    }

    /// `Command_LineCutToEnd`: cuts from the caret to the end of the text of the line.
    pub fn cut_to_line_end(&mut self) -> Result<()> {
        self.refuse_in_view_mode()?;
        if self.selection().is_some() {
            return self.cut();
        }
        let caret = self.caret();
        let Some(line) = self.doc().line(caret.line)? else {
            self.beep();
            return Ok(());
        };
        let end = Pos::new(caret.line, line.text.len());
        if caret.col >= end.col {
            self.beep();
            return Ok(());
        }
        self.select_range(caret, end)?;
        self.cut()
    }

    /// `CEditView::CopySelectedAllLines`: widens the selection to whole rows and copies them,
    /// each with the quote and the line number if asked. Nothing happens without a selection.
    pub fn copy_selected_lines(&mut self, quote: Option<&str>, numbered: bool) -> Result<()> {
        let Some((from, to)) = self.selection() else {
            return Ok(());
        };
        if self.cursor().is_box() {
            self.beep();
            return Ok(());
        }
        // From the start of the first row to the start of the row after the last one. A
        // selection that ends at the start of a row does not take that row. Past the last row
        // the end of the document counts (`GetAdjustCursorPos`).
        let from = self.layout().row_start(self.doc(), from)?;
        let to = if to > self.layout().row_start(self.doc(), to)? {
            match self.layout().next_row_start(self.doc(), to)? {
                Some(next) => next,
                None => self.layout().row_end(self.doc(), to)?,
            }
        } else {
            to
        };
        self.select_range(from, to)?;

        // The width of the line numbers: the digits of the number of lines, at least
        // `LINE_NUMBER_MIN_DIGITS`, plus the blank that the line number area has
        // (`DetectWidthOfLineNumberArea_calculate`) and one more (`GetSelectedData`).
        let width = self
            .doc()
            .line_count()
            .max(1)
            .to_string()
            .len()
            .max(LINE_NUMBER_MIN_DIGITS)
            + 2;
        let new_line = eol_text(self.new_line());
        let quote = quote.filter(|q| !q.is_empty());
        let mut out = String::new();
        let mut pos = from;
        while pos < to {
            let end = match self.layout().next_row_start(self.doc(), pos)? {
                Some(next) => next,
                None => self.layout().row_end(self.doc(), pos)?,
            };
            if end <= pos {
                break;
            }
            let text = self.doc().text_between(pos, end)?;
            // an empty row is skipped (`nIdxTo - nIdxFrom == 0`)
            if !text.is_empty() {
                if let Some(quote) = quote {
                    out.push_str(quote);
                }
                if numbered {
                    // Sakura numbers the rows of the layout; this numbers the lines.
                    let number = format!(" {}:", pos.line + 1);
                    out.push_str(&format!("{number:>width$}"));
                }
                out.push_str(&text);
                // A row that does not end the line (the last line, or a wrapped row) gets a
                // line ending when it is quoted or numbered.
                if !ends_with_break(pos, end) && (quote.is_some() || numbered) {
                    out.push_str(new_line);
                }
            }
            pos = end;
        }
        self.copy_text(out, ClipKind::Normal)?;
        Ok(())
    }

    /// `Command_PASTE`: pastes the clipboard text. `option` is the argument of `F_PASTE`
    /// (the bits of `paste_option`). A row copy goes to the start of the row, and the caret stays on the
    /// character it was on. Beeps when the clipboard has no text.
    pub fn paste(&mut self, option: i64) -> Result<()> {
        use paste_option::{CONVERT_EOL, KEEP_EOL, LINE_MODE, NO_LINE_MODE};
        let Some(clip) = self.clipboard_get()? else {
            self.beep();
            return Ok(());
        };
        let line_mode = if option & LINE_MODE != 0 {
            true
        } else if option & NO_LINE_MODE != 0 {
            false
        } else {
            enable_line_mode_paste()
        };
        let convert = if option & CONVERT_EOL != 0 {
            true
        } else if option & KEEP_EOL != 0 {
            false
        } else {
            convert_eol_paste()
        };
        let as_box = if option & paste_option::BOX != 0 {
            true
        } else if option & paste_option::NO_BOX != 0 {
            false
        } else {
            AUTO_COLUMN_PASTE
        };
        // The text of a box copy is pasted as a box (`Command_PASTE`, `bAutoColumnPaste`). The
        // line endings are not converted.
        if as_box && clip.kind == ClipKind::Box {
            return self.paste_box(&clip.text);
        }
        let new_line = eol_text(self.new_line());
        // `CViewCommander::ConvertEol` and `Command_PASTE` read the setting, not the document.
        let ext_eol = crate::edit_options::get().ext_eol;
        let mut text = clip.text;
        // A row copy of a wrapped row does not end with a line ending; add one.
        let by_line = line_mode && clip.kind == ClipKind::Line;
        if by_line && !text.is_empty() && !text.ends_with(|c| is_eol_char(c, ext_eol)) {
            text.push_str(new_line);
        }
        if convert {
            text = convert_eols(&text, new_line, ext_eol);
        }
        // With a box selected the first line goes into every row of the box, a row copy too
        // (`Command_INSTEXT`).
        if !by_line || (self.cursor().is_box() && self.cursor().has_selection()) {
            self.insert_text(&text)?;
            return Ok(());
        }
        // Sakura deletes the selection, goes to the start of the row, inserts, and comes back
        // to the character the caret was on.
        self.delete_selection()?;
        self.cursor.clear_selection();
        let caret = self.caret();
        let row = self.layout().row_start(self.doc(), caret)?;
        let delta = caret.col.saturating_sub(row.col);
        let edit = self.replace(row, row, &text)?;
        self.move_caret(Pos::new(edit.end.line, edit.end.col + delta), false)
    }

    /// The full path of the document, or `None` with a beep if it has no file.
    pub fn path_text(&mut self) -> Option<String> {
        let path = self.path().map(|p| p.to_string_lossy().into_owned());
        if path.is_none() {
            self.beep();
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_eols_joins_crlf_into_one() {
        assert_eq!(convert_eols("a\r\nb\rc\nd", "\n", false), "a\nb\nc\nd");
        assert_eq!(convert_eols("a\nb", "\r\n", false), "a\r\nb");
        assert_eq!(convert_eols("a\r\n\r\nb", "\r", false), "a\r\rb");
    }

    #[test]
    fn convert_eols_extended_endings_are_optional() {
        assert_eq!(
            convert_eols("a\u{85}b\u{2028}c", "\n", false),
            "a\u{85}b\u{2028}c"
        );
        assert_eq!(
            convert_eols("a\u{85}b\u{2028}c\u{2029}", "\n", true),
            "a\nb\nc\n"
        );
    }
}
