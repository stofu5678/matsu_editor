//! The C/C++ smart indent that `Command_WCHAR` runs after it typed a char.
//!
//! Sakura: `CEditView::SmartIndent_CPP` in `sakura_core/types/CType_Cpp.cpp`. It reacts to a line
//! break and to `:`, `{`, `}`, `(` and `)`. It looks backwards for the bracket that the char opens
//! or closes and gives the line the indentation of the line of that bracket: one more level
//! after a line break, `{` and `(`, the same one for `}`, `)` and `:`. The extra level is a tab,
//! or the spaces up to the next tab stop if the type inserts spaces for the Tab key
//! (`m_bInsSpace`).
//!
//! Not done: the state at the start of a line that earlier lines leave (`GetColorTypePrev`: a
//! `/* ... */` comment that goes on from the line before); the line is scanned from its start. The
//! three options of the "支援" page (`m_bIndentCppStringIgnore`, `m_bIndentCppCommentIgnore`,
//! `m_bIndentCppUndoSep`) are taken as their defaults (on, on, off).

use matsu_core::buffer::Pos;

use super::CommandContext;
use crate::command::Result;

/// The line ending of a line stands for itself as one char: Sakura's line strings end with it
/// (`GetDocLineStrWithEOL`).
const EOL_MARK: char = '\n';

fn is_tab_or_space(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// `IsHeadCppKeyword`: the text starts with a keyword that has a `:` after it.
fn starts_with_label_keyword(rest: &[char]) -> bool {
    let starts = |word: &str| {
        let mut chars = rest.iter();
        word.chars().all(|w| chars.next() == Some(&w))
    };
    starts("case")
        || starts("default:")
        || starts("public:")
        || starts("private:")
        || starts("protected:")
}

/// The ranges of a line that are inside a string, a char literal or a comment (`SCommentBlock`):
/// both ends are included.
fn comment_blocks(line: &[char]) -> Vec<(usize, usize)> {
    let len = line.len();
    let mut blocks = Vec::new();
    // 0: code, 1: "...", 2: '...', 3: /* ... */
    let mut mode = 0;
    let mut begin: isize = -1;
    let mut n = 0;
    while n < len {
        let c = line[n];
        let next = line.get(n + 1).copied();
        match mode {
            0 => {
                if c == '"' {
                    mode = 1;
                    begin = n as isize;
                } else if c == '\'' {
                    mode = 2;
                    begin = n as isize;
                } else if c == '/' && next == Some('*') {
                    mode = 3;
                    begin = n as isize;
                    n += 1;
                } else if c == '/' && next == Some('/') {
                    blocks.push((n, len - 1));
                    n = len - 1;
                }
            }
            1 | 2 => {
                let quote = if mode == 1 { '"' } else { '\'' };
                if c == '\\' {
                    n += 1;
                } else if c == quote {
                    blocks.push((begin as usize, n));
                    begin = -1;
                    mode = 0;
                }
            }
            _ => {
                if c == '*' && next == Some('/') {
                    blocks.push((begin as usize, n + 1));
                    begin = -1;
                    mode = 0;
                    n += 1;
                }
            }
        }
        n += 1;
    }
    if begin > 0 {
        // a string or a comment that does not end
        blocks.push((begin as usize, len));
    }
    blocks
}

fn in_block(blocks: &[(usize, usize)], at: usize) -> bool {
    blocks
        .iter()
        .any(|&(first, last)| first <= at && at <= last)
}

impl CommandContext<'_> {
    /// The chars of a line with a mark for its line ending; empty for the empty last line.
    fn chars_with_eol(&self, line: usize) -> Result<Vec<char>> {
        let Some(l) = self.doc().line(line)? else {
            return Ok(Vec::new());
        };
        let mut chars: Vec<char> = l.text.chars().collect();
        if l.eol.is_some() {
            chars.push(EOL_MARK);
        }
        Ok(chars)
    }

    /// The indentation of the level after the chars `indent` (all tabs and spaces): one tab, or
    /// the spaces up to the next tab stop if the Tab key inserts spaces.
    fn one_level_deeper(&self, indent: &[char]) -> String {
        let mut out: String = indent.iter().collect();
        if self.is_ins_space() {
            let width = self.layout().tab_width().max(1);
            let mut col = 0;
            for &c in indent {
                col += if c == '\t' { width - col % width } else { 1 };
            }
            out.push_str(&" ".repeat(width - col % width));
        } else {
            out.push('\t');
        }
        out
    }

    /// `SmartIndent_CPP( typed )`, run after `typed` was inserted and the caret is behind it (for
    /// a line break, at the start of the new line, after the copied indentation).
    pub(crate) fn smart_indent_cpp(&mut self, typed: char) -> Result<()> {
        if typed != '\r' && !":{}()".contains(typed) {
            return Ok(());
        }
        let original = typed;
        let mut typed = typed;
        let caret = self.caret();
        let y = caret.line;
        let line = self.chars_with_eol(y)?;
        let text = self.doc().line(y)?.map(|l| l.text).unwrap_or_default();
        let caret_chars = text.get(..caret.col).unwrap_or(&text).chars().count();
        // `nCaretPosX_PHY` and `nWork`, the place the work is based on.
        let mut caret_x = caret_chars;
        let mut work = caret_chars.saturating_sub(1);
        let range_to = if line.is_empty() {
            // only the end of the document
            if original != '\r' {
                return Ok(());
            }
            0
        } else {
            if original == '\r' {
                // The identifier behind the caret is indented: `}` and `)` as if they were typed.
                let mut i = caret_chars;
                while i < line.len() && is_tab_or_space(line[i]) {
                    i += 1;
                }
                if i < line.len() {
                    if line[i] == ')' || line[i] == '}' {
                        typed = line[i];
                    }
                    caret_x = i;
                }
                work = if i < line.len() { i } else { caret_x };
            }
            let work = work.min(line.len());
            let mut i = 0;
            while i < work && is_tab_or_space(line[i]) {
                i += 1;
            }
            if i < work {
                let ok = (typed == ':' && starts_with_label_keyword(&line[i..]))
                    || (typed == '{' && line[i] != '#')
                    || (typed == '(' && line[i] != '#');
                if !ok {
                    return Ok(());
                }
            } else if typed == ':' {
                return Ok(());
            }
            i
        };

        // Look for the bracket that the char opens or closes.
        let mut level: i32 = 0;
        let mut data: Option<String> = None;
        let mut j = y as isize;
        while j >= 0 {
            let row = j as usize;
            let other = if row == y {
                line.clone()
            } else {
                self.chars_with_eol(row)?
            };
            let mut k: isize;
            if row == y {
                if other.is_empty() {
                    // the empty last line has nothing to look at
                    j -= 1;
                    continue;
                }
                k = work.min(other.len()).saturating_sub(1) as isize;
            } else {
                if other.is_empty() {
                    break;
                }
                k = other.len() as isize - 1;
            }
            let blocks = comment_blocks(&other);
            if row == y && original != '\r' && in_block(&blocks, caret_chars) {
                // the char that was typed is inside a comment or a string
                return Ok(());
            }
            while k >= 0 {
                let ku = k as usize;
                let c = other[ku];
                // a bracket in a char literal such as '{'
                let quoted = ku > 0
                    && other[ku - 1] == '\''
                    && other.len() - 1 > ku
                    && other[ku + 1] == '\'';
                let counts = !quoted && !in_block(&blocks, ku);
                if (c == ')' || c == '}') && counts {
                    if row == y {
                        if typed == '{' && c == '}' {
                            typed = '}';
                            level -= 1;
                        }
                        if typed == '(' && c == ')' {
                            typed = ')';
                            level -= 1;
                        }
                    }
                    level += 1;
                }
                if (c == '(' || c == '{') && counts {
                    if row == y && ((typed == '{' && c == '{') || (typed == '(' && c == '(')) {
                        return Ok(());
                    }
                    if level == 0 {
                        break;
                    }
                    level -= 1;
                }
                k -= 1;
            }
            if k < 0 {
                // not on this line
                j -= 1;
                continue;
            }
            let indent_len = other.iter().take_while(|&&c| is_tab_or_space(c)).count();
            let indent = &other[..indent_len];
            data = Some(if typed == '\r' || typed == '{' || typed == '(' {
                self.one_level_deeper(indent)
            } else {
                indent.iter().collect()
            });
            break;
        }
        let data = match data {
            Some(data) => data,
            None if j < 0 && typed == '\r' => return Ok(()),
            None => String::new(),
        };

        // Replace the indentation, and put the caret where it was relative to the text.
        let source: String = line[..range_to].iter().collect();
        let after = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .nth(caret_x)
            .unwrap_or(text.len());
        let kept_to_caret = after.saturating_sub(source.len());
        let new_col = data.len() + kept_to_caret;
        if !((data.is_empty() && range_to == 0) || data == source) {
            self.replace(Pos::new(y, 0), Pos::new(y, source.len()), &data)?;
        }
        self.move_caret(Pos::new(y, new_col), false)
    }
}
