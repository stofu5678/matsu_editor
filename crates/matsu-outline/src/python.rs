// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Python.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2014, the authors named in the original files (genta, zenryaku, Moca,
//     aroka, hor, YAZAKI, little YOSHI, ryoji, syat and others)
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a UTF-16 buffer
// walked with `CDocLineMgr`, the results put in `CFuncInfoArr`, a Windows dialog) was rewritten in
// Rust on the lines of `LineSource`, the positions in the layout are left to the window, and the
// tests are new.
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! The outline of a Python program: the `class` and `def` lines, nested by their indentation.
//!
//! Sakura: `CDocOutline::MakeFuncList_python` and the scanner `COutlinePython` in
//! `sakura_core/types/CType_Python.cpp` (2007.02.08 genta). Strings and comments are skipped, so
//! a line inside a long string is not taken for a definition.

use crate::chars::{at, is_line_delimiter, is_space, starts_at, string_of};
use crate::{FuncInfoArr, LineSource};

/// The deepest nesting that is told apart (`MAX_DEPTH`).
const MAX_DEPTH: usize = 10;

/// `STR_OUTLINE_PYTHON_UNDEFINED`: the name of a definition without a name.
const UNDEFINED: &str = "名称未定";
/// `STR_OUTLINE_PYTHON_CLASS`: what a class gets after its name.
const CLASS_SUFFIX: &str = " クラス";

/// `Python_IsWordChar`.
fn is_word_char(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// A normal line, the start of a logical line included.
    Normal,
    /// In a string.
    String,
    /// After a continuation mark: the next line is not the start of a logical line.
    Continue,
}

/// `COutlinePython`: the state of the scan between lines.
struct Scanner {
    state: State,
    quote: char,
    /// Escape characters are not special (a string with `r` before it).
    raw: bool,
    /// A string with three quotes at both ends.
    long: bool,
}

impl Scanner {
    fn new() -> Self {
        Self {
            state: State::Normal,
            quote: '\0',
            raw: false,
            long: false,
        }
    }

    fn is_logical_line_top(&self) -> bool {
        self.state == State::Normal
    }

    /// `EnterString`: decides the kind of the string at `start`, which is a quote.
    fn enter_string(&mut self, data: &[char], start: usize) -> usize {
        let mut col = start;
        let c = at(data, col);
        if c == '"' || c == '\'' {
            self.state = State::String;
            self.quote = c;
            self.raw = col >= 1 && (data[col - 1] == 'r' || data[col - 1] == 'R');
            if col + 2 < data.len() && data[col + 1] == c && data[col + 2] == c {
                self.long = true;
                col += 2;
            } else {
                self.long = false;
            }
            col += 1;
        }
        col
    }

    /// `ScanNormal`: the program text, up to a string, a comment, a continuation or the end.
    fn scan_normal(&mut self, data: &[char], start: usize) -> usize {
        let len = data.len();
        let mut col = start;
        while col < len {
            let c = data[col];
            if c == '#' {
                // a comment is the same as the end of the line
                self.state = State::Normal;
                break;
            } else if c == '"' || c == '\'' {
                return self.enter_string(data, col);
            } else if c == '\\' {
                // a continuation line, maybe: the line ends in CRLF here. (`linelen - 1 == col`
                // can never hold with a line ending after it; Sakura has the same test.)
                if (len >= 2
                    && len - 2 == col
                    && at(data, col + 1) == '\r'
                    && at(data, col + 2) == '\n')
                    || (len >= 1 && len - 1 == col && is_line_delimiter(at(data, col + 1)))
                {
                    self.state = State::Continue;
                    break;
                }
            }
            col += 1;
        }
        len
    }

    /// `ScanString`: the inside of a string, up to its end.
    fn scan_string(&mut self, data: &[char], start: usize) -> usize {
        let len = data.len();
        let quote = self.quote;
        let mut col = start;
        while col < len {
            let c = data[col];
            if c == '\\' && col + 1 < len {
                let key = data[col + 1];
                if !self.raw && (key == '\\' || key == '"' || key == '\'') {
                    col += 2;
                    continue;
                }
                if is_line_delimiter(key) {
                    // \r\n has to count as one \n
                    if col + 1 >= len || at(data, col + 2) == key {
                        // the end of the line
                        col += 2;
                        continue;
                    } else if at(data, col + 2) == '\n' {
                        col += 2; // CRLF
                    }
                }
            } else if is_line_delimiter(c) {
                // a short string ends at the end of the line, if it was not continued
                if !self.long {
                    self.state = State::Normal;
                    return col + 1;
                }
            } else if c == quote {
                if !self.long {
                    self.state = State::Normal;
                    return col + 1;
                }
                if col + 2 < len && data[col + 1] == quote && data[col + 2] == quote {
                    self.state = State::Normal;
                    return col + 3;
                }
            }
            col += 1;
        }
        len
    }

    /// `DoScanLine`: from `start` to the end of the line, to find the state at its end.
    fn scan_line(&mut self, data: &[char], start: usize) {
        let mut col = start;
        while col < data.len() {
            col = match self.state {
                State::Normal | State::Continue => self.scan_normal(data, col),
                State::String => self.scan_string(data, col),
            };
        }
    }
}

/// Lists the classes and functions of a Python program. The depth is the number of definitions
/// the indentation puts it in; a `class` has `info` 4 and its name ends with " クラス", a `def`
/// has `info` 1.
pub fn make_func_list_python(src: &dyn LineSource) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let mut scanner = Scanner::new();
    // the column of each level of the indentation
    let mut indent_level = [0usize; MAX_DEPTH];
    let mut depth_index = 0usize;
    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        let len = line.len();
        let mut col = 0usize;
        if scanner.is_logical_line_top() {
            // the indentation: a tab moves to the next multiple of 8
            let mut depth = 0usize;
            while col < len {
                match line[col] {
                    ' ' => depth += 1,
                    '\t' => depth = (depth + 8) & !7,
                    _ => break,
                }
                col += 1;
            }
            let c = at(&line, col);
            if is_line_delimiter(c) || c == '\0' || c == '#' {
                // blank lines and comment lines are ignored
                continue;
            }
            let item_id;
            if len - col > 3 + 1 && starts_at(&line, col, "def") {
                item_id = 1;
                col += 3;
            } else if len - col > 5 + 1 && starts_at(&line, col, "class") {
                item_id = 4;
                col += 5;
            } else {
                scanner.scan_line(&line, col);
                continue;
            }
            // 2006.02.28 genta: `define` and `classic` are not definitions, a space or a tab
            // follows the keyword
            let c = at(&line, col);
            if c != ' ' && c != '\t' {
                scanner.scan_line(&line, col);
                continue;
            }
            // the nesting from the indentation, once the line is known to be a definition
            for i in (0..=depth_index).rev() {
                if depth == indent_level[i] {
                    depth_index = i;
                    break;
                } else if depth > indent_level[i] && i < MAX_DEPTH - 1 {
                    depth_index = i + 1;
                    indent_level[depth_index] = depth;
                    break;
                }
            }
            while col < len && is_space(line[col]) {
                col += 1;
            }
            let mut w_end = col;
            while w_end < len && is_word_char(line[w_end]) {
                w_end += 1;
            }
            let name_len = w_end - col;
            let name = if name_len > 0 {
                // szWord[512]; the name of a class is cut so that the suffix fits
                let cap = if item_id == 4 { 512 - 8 } else { 511 };
                let mut name = string_of(&line, col, col + name_len.min(cap));
                if item_id == 4 {
                    name.push_str(CLASS_SUFFIX);
                }
                name
            } else {
                // `wcscpy(szWord + 8, ...)` writes the suffix after the end of the short name:
                // a class without a name is shown as undefined only
                UNDEFINED.to_owned()
            };
            out.push(n + 1, name, item_id, depth_index);
            // the rest of the definition is scanned from here
            col = w_end;
        }
        scanner.scan_line(&line, col);
    }
    out
}
