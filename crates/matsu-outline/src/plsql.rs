// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Sql.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 2008, kobake
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

//! The outline of a PL/SQL source: its functions, procedures and packages.
//!
//! Sakura: `CDocOutline::MakeFuncList_PLSQL` in `sakura_core/types/CType_Sql.cpp`.
//!
//! `FuncInfo::info` tells what the item is: 10 a function declaration (ended by `;`), 11 a
//! function body (`IS`), 20 a procedure declaration, 21 a procedure body, 31 a package
//! specification and 41 a package body (`IS` or `AS`). The line of an item is the line of the
//! keyword (`FUNCTION`, `PROCEDURE`, `PACKAGE`), and its name is the next word.

use crate::chars::{is_line_delimiter, is_wcntrl, is_wide, is_wspace};
use crate::{FuncInfoArr, LineSource};

/// The longest word kept (`nMaxWordLeng`); a longer word is skipped. Counted in UTF-16 units.
const MAX_WORD_LENGTH: usize = 70;

/// A character that a word is made of (2013.05.08: Japanese too).
fn is_word_char(c: char) -> bool {
    c == '_'
        || c == '~'
        || c.is_ascii_alphanumeric()
        || ((c as u32) >= 0xa1 && !is_wcntrl(c) && !is_wspace(c))
}

/// A character that ends a run of symbols.
fn ends_symbols(c: char) -> bool {
    is_word_char(c)
        || c == '\t'
        || c == ' '
        || is_line_delimiter(c)
        || matches!(c, '{' | '}' | '(' | ')' | ';' | '\'' | '/' | '-')
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Word,
    Symbols,
    /// The inside of a text in single quotes (`nMode` 20).
    Text,
    /// The inside of a `/* */` comment (`nMode` 8).
    Comment,
    TooLongWord,
}

/// What the keyword before the name was (`nFuncOrProc`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    None,
    Function,
    Procedure,
    Package,
    PackageBody,
}

fn eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Lists the functions, procedures and packages of a PL/SQL source.
pub fn make_func_list_plsql(src: &dyn LineSource) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let mut word = String::new();
    let mut word_len = 0usize;
    let mut mode = Mode::Normal;
    let mut name = String::new();
    let mut func_line = 0usize;
    let mut unit = Unit::None;
    // 0: nothing, 1: the keyword was read, 2: the name was read
    let mut parse_count = 0u32;

    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        let mut i = 0usize;
        while i < line.len() {
            let c = line[i];
            let wide = is_wide(c);
            match mode {
                // inside a text in single quotes: '' is a quote in the text
                Mode::Text => {
                    if c == '\'' {
                        if line.get(i + 1) == Some(&'\'') {
                            i += 1;
                        } else {
                            mode = Mode::Normal;
                        }
                    }
                }
                Mode::Comment => {
                    if c == '*' && line.get(i + 1) == Some(&'/') {
                        i += 1;
                        mode = Mode::Normal;
                    }
                }
                Mode::Word => {
                    if is_word_char(c) || wide {
                        if word_len >= MAX_WORD_LENGTH {
                            mode = Mode::TooLongWord;
                        } else {
                            word.push(c);
                            word_len += c.len_utf16();
                        }
                    } else {
                        if parse_count == 0 && eq(&word, "FUNCTION") {
                            unit = Unit::Function;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 0 && eq(&word, "PROCEDURE") {
                            unit = Unit::Procedure;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 0 && eq(&word, "PACKAGE") {
                            unit = Unit::Package;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 1 && unit == Unit::Package && eq(&word, "BODY") {
                            unit = Unit::PackageBody;
                            parse_count = 1;
                        } else if parse_count == 1 {
                            // the name (every unit is one of the four here)
                            parse_count = 2;
                            name = word.clone();
                        } else if parse_count == 2 {
                            if eq(&word, "IS") {
                                // the body of a function or a procedure, the package
                                let id = match unit {
                                    Unit::Function => 11,
                                    Unit::Procedure => 21,
                                    Unit::Package => 31,
                                    Unit::PackageBody => 41,
                                    Unit::None => 0,
                                };
                                out.push(func_line, name.clone(), id, 0);
                                parse_count = 0;
                            }
                            if eq(&word, "AS") {
                                // a package takes `AS` as well as `IS`
                                match unit {
                                    Unit::Package => {
                                        out.push(func_line, name.clone(), 31, 0);
                                        parse_count = 0;
                                    }
                                    Unit::PackageBody => {
                                        out.push(func_line, name.clone(), 41, 0);
                                        parse_count = 0;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        word.clear();
                        word_len = 0;
                        mode = Mode::Normal;
                        // process this character again
                        continue;
                    }
                }
                Mode::Symbols => {
                    if ends_symbols(c) || wide {
                        word.clear();
                        word_len = 0;
                        mode = Mode::Normal;
                        continue;
                    } else if word_len >= MAX_WORD_LENGTH {
                        mode = Mode::TooLongWord;
                    } else {
                        word.push(c);
                        word_len += c.len_utf16();
                    }
                }
                // a word that is too long is skipped up to a blank
                Mode::TooLongWord => {
                    if c == '\t' || c == ' ' || is_line_delimiter(c) {
                        mode = Mode::Normal;
                    }
                }
                Mode::Normal => {
                    if c == '\t' || c == ' ' || is_line_delimiter(c) {
                        // blanks
                    } else if c == '-' && line.get(i + 1) == Some(&'-') {
                        // a comment: the rest of the line
                        break;
                    } else if c == '/' && line.get(i + 1) == Some(&'*') {
                        i += 1;
                        mode = Mode::Comment;
                    } else if c == '\'' {
                        mode = Mode::Text;
                    } else if c == ';' {
                        if parse_count == 2 {
                            // the declaration of a function; a procedure and a package too
                            let id = if unit == Unit::Function { 10 } else { 20 };
                            out.push(func_line, name.clone(), id, 0);
                            parse_count = 0;
                        }
                    } else {
                        word.clear();
                        word.push(c);
                        word_len = c.len_utf16();
                        mode = if is_word_char(c) || wide {
                            Mode::Word
                        } else {
                            Mode::Symbols
                        };
                    }
                }
            }
            i += 1;
        }
    }
    out
}
