//! Moves and selections by words and lines, shared by `word_left`, `select_word`, `word_delete`
//! and their relatives. The rules of what a word is are in [`crate::text::word`].
//!
//! Sakura searches words in the *logical* line (`CSearchAgent::PrevOrNextWord`), not in the row
//! of the layout, so wrapping does not change where a word move stops.

use matsu_core::buffer::Pos;

use super::CommandContext;
use crate::command::Result;
use crate::text::word;

/// A logical line as `char`s with its line ending, and the byte column of every `char`.
struct LineChars {
    chars: Vec<char>,
    /// `offsets[i]` is the byte column of `chars[i]`; the last entry is the end of the line
    /// including the line ending.
    offsets: Vec<usize>,
    /// How many chars at the end of `chars` are the line ending the document recorded for the
    /// line (0 for none).
    eol_len: usize,
}

impl LineChars {
    /// The index of the char that starts at byte column `col`, if `col` is at a char boundary.
    fn index_of(&self, col: usize) -> Option<usize> {
        self.offsets.binary_search(&col).ok()
    }
}

impl CommandContext<'_> {
    /// The line `line` with its line ending, or `None` if there is no such line (the empty line
    /// after a final line ending is none).
    ///
    /// Of a very long line (ADR-0020, 9.10) only the part around the column `col` is taken, so
    /// that a move in it does not decode it all: the words that are searched are those in it.
    fn line_chars(&self, line: usize, col: usize) -> Result<Option<LineChars>> {
        /// How far from the column a very long line is taken, in bytes.
        const AROUND: usize = 4096;
        let (text, base, eol) = if self.doc().is_long_line(line) {
            let Some(slice) = self.doc().line_slice(
                line,
                col.saturating_sub(AROUND),
                col.saturating_add(AROUND),
            )?
            else {
                return Ok(None);
            };
            // the line ending is at the end of the line: only if the part reaches it
            let reaches_end =
                self.doc().line_len(line)? == Some(slice.start_col + slice.text.len());
            let eol = if reaches_end {
                self.doc().line_eol(line)?
            } else {
                None
            };
            (slice.text, slice.start_col, eol)
        } else {
            let Some(l) = self.doc().line(line)? else {
                return Ok(None);
            };
            (l.text, 0, l.eol)
        };
        let l = (text, eol);
        let mut chars = Vec::with_capacity(l.0.len() + 2);
        let mut offsets = Vec::with_capacity(l.0.len() + 3);
        let mut off = base;
        let eol = l.1.map_or("", matsu_core::model::eol_text);
        for c in l.0.chars().chain(eol.chars()) {
            chars.push(c);
            offsets.push(off);
            off += c.len_utf8();
        }
        offsets.push(off);
        Ok(Some(LineChars {
            chars,
            offsets,
            eol_len: eol.chars().count(),
        }))
    }

    /// The range of the word at `pos` (Sakura: `WhereCurrentWord`), or `None` on a line ending,
    /// at the end of a line, or on a line that does not exist.
    pub fn word_range_at(&self, pos: Pos) -> Result<Option<(Pos, Pos)>> {
        let Some(line) = self.line_chars(pos.line, pos.col)? else {
            return Ok(None);
        };
        let Some(idx) = line.index_of(pos.col) else {
            return Ok(None);
        };
        Ok(
            word::where_current_word(&line.chars, line.eol_len, idx).map(|(from, to)| {
                (
                    Pos::new(pos.line, line.offsets[from]),
                    Pos::new(pos.line, line.offsets[to]),
                )
            }),
        )
    }

    /// Selects the word at the caret, with the caret at its end, and drops the selection that
    /// was there. Returns `false` (and selects nothing) if there is no word.
    pub fn select_word(&mut self) -> Result<bool> {
        self.cursor.clear_selection();
        match self.word_range_at(self.caret())? {
            Some((from, to)) => {
                self.select_range(from, to)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Selects the logical line of the caret with its line ending: the anchor at the start of the
    /// line and the caret at the start of the next one (at the end of the document for the last
    /// line). On the empty line after a final line ending there is nothing to select.
    pub fn select_line(&mut self) -> Result<()> {
        let line = self.caret().line;
        let start = Pos::new(line, 0);
        if line >= self.doc().line_count() {
            return self.move_caret(start, false);
        }
        // `select_range` clamps the position after the last line to the end of the document
        self.select_range(start, Pos::new(line + 1, 0))
    }

    /// Moves to the start of the word before the caret, or one char left if there is none
    /// (Sakura: `Command_WORDLEFT`). Without `extend` the selection is dropped first and the move
    /// starts from the caret, not from the start of the selection.
    pub fn move_word_left(&mut self, extend: bool) -> Result<()> {
        let extend = self.extends(extend);
        if !extend {
            self.cursor.clear_selection();
        }
        let caret = self.caret();
        if let Some(line) = self.line_chars(caret.line, caret.col)?
            && let Some(idx) = line.index_of(caret.col)
            && let Some(to) =
                word::search_prev_word(&line.chars, line.eol_len, idx, stops_both_ends())
        {
            return self.move_caret(Pos::new(caret.line, line.offsets[to]), extend);
        }
        self.move_left(extend)
    }

    /// Moves to the start of the next word, or one char right if there is none (Sakura:
    /// `Command_WORDRIGHT`). At the end of a line this goes to the start of the next line, and
    /// there on to the first word if the line starts with a space or a tab.
    pub fn move_word_right(&mut self, extend: bool) -> Result<()> {
        let extend = self.extends(extend);
        if !extend {
            self.cursor.clear_selection();
        }
        let mut try_again = false;
        loop {
            let caret = self.caret();
            let Some(line) = self.line_chars(caret.line, caret.col)? else {
                return Ok(());
            };
            if try_again {
                let row = self.layout.row_start(self.doc(), caret)?;
                let first = line
                    .index_of(row.col)
                    .and_then(|i| line.chars.get(i).copied());
                if !matches!(first, Some(' ' | '\t')) {
                    return Ok(());
                }
            }
            let to = line
                .index_of(caret.col)
                .filter(|&i| {
                    i < line.chars.len() && !word::is_line_ending_at(&line.chars, line.eol_len, i)
                })
                .and_then(|i| {
                    word::search_next_word(&line.chars, line.eol_len, i, stops_both_ends())
                });
            if let Some(to) = to {
                return self.move_caret(Pos::new(caret.line, line.offsets[to]), extend);
            }
            self.move_right(extend)?;
            if try_again {
                return Ok(());
            }
            try_again = true;
        }
    }
}

/// Sakura's option `m_bStopsBothEndsWhenSearchWord` ("stop at both ends of a word when moving by
/// words"); off by default (`sakura_core/env/CShareData.cpp`). See [`crate::edit_options`].
fn stops_both_ends() -> bool {
    crate::edit_options::get().stops_both_ends_word
}

#[cfg(test)]
mod tests {
    use crate::testing::check;

    // The commands that use these methods have their own tests; these are the cases of the
    // methods themselves.

    #[test]
    fn word_right_goes_on_to_the_next_line_and_skips_its_indent() {
        // Command_WORDRIGHT: at the end of a line the first step is one char right, and a line
        // that starts with a space or a tab gets a second search
        check("word_right", "abc|\n  def", "abc\n  |def");
        check("word_right", "abc|\n\tdef", "abc\n\t|def");
        check("word_right", "abc|\ndef", "abc\n|def");
        check("word_right", "abc |\n  def", "abc \n  |def");
    }

    #[test]
    fn nel_ls_and_ps_are_word_chars_unless_the_document_split_the_line_there() {
        use crate::testing::Editor;
        for c in ['\u{85}', '\u{2028}', '\u{2029}'] {
            // The setting is off: the char at the end of the last line is a char of the line, and
            // it can be selected as a word.
            let mut editor = Editor::new_with_ext_eol(&format!("|ab {c}"), false);
            editor.with_context(|cx| {
                let before = matsu_core::buffer::Pos::new(0, 3);
                assert!(cx.word_range_at(before).unwrap().is_some());
            });
            // The setting is on: the line ends there, and there is no word on the line ending.
            let mut editor = Editor::new_with_ext_eol(&format!("|ab {c}"), true);
            editor.with_context(|cx| {
                let before = matsu_core::buffer::Pos::new(0, 3);
                assert_eq!(cx.word_range_at(before).unwrap(), None);
            });
        }
    }

    #[test]
    fn select_line_at_the_end() {
        check("select_line", "a|b\ncd", "[ab\n|cd");
        check("select_line", "ab\ncd|", "ab\n[cd|");
    }
}
