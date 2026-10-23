// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Vb.cpp
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

//! The outline of a Visual Basic program: its functions, procedures, properties, constants,
//! enumerations, types and events (簡易版).
//!
//! Sakura: `CDocOutline::MakeFuncList_VisualBasic` in `sakura_core/types/CType_Vb.cpp`
//! (2001.06.23 N.Nakatani; 2003.07.10 little YOSHI made it read the declarations).
//!
//! `FuncInfo::info` has the kind in the low 4 bits (1 function, 2 procedure, 3 property get,
//! 4 property let, 5 property set, 6 constant, 7 enumeration, 8 type, 9 event), the access in bits
//! 4 to 7 (1 public, 2 private, 3 friend), bit 8 for Static and bit 9 for a `Declare`.

use crate::chars::{is_line_delimiter, is_wcntrl, is_wide, is_wspace, string_of};
use crate::{FuncInfoArr, LineSource};

/// The longest name (`nMaxWordLeng`, 2003.08.07 little YOSHI: the rule of VB).
const MAX_WORD_LENGTH: usize = 255;

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
        || matches!(c, '{' | '}' | '(' | ')' | ';' | '\'' | '/' | '-' | '#')
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Word,
    Symbols,
    /// The inside of a text in double quotes.
    Text,
    TooLongWord,
}

fn eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Lists the declarations of a Visual Basic program. `file_path` tells a class module (`.cls`),
/// where `Type` and `Const` are private.
pub fn make_func_list_visual_basic(src: &dyn LineSource, file_path: Option<&str>) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    // Type and Const behave differently in a class module
    let is_class = file_path.is_some_and(|p| {
        let chars: Vec<char> = p.chars().collect();
        chars.len() > 4 && eq(&string_of(&chars, chars.len() - 4, chars.len()), ".cls")
    });

    let mut word_prev = String::new();
    let mut word = String::new();
    let mut word_len = 0usize;
    let mut mode = Mode::Normal;
    let mut parse_count = 0u32;
    let mut in_procedure = false;
    let mut func_line = 0usize;
    // the last character of the previous line
    let mut previous_last: Option<char> = None;

    for n in 0..src.line_count() {
        if previous_last.is_some_and(|c| c != '_') {
            parse_count = 0;
        }
        let Some(line) = src.line(n) else {
            break;
        };
        previous_last = line.last().copied();
        let mut func_id = 0u32;
        let mut double_quote = false;
        let mut i = 0usize;
        while i < line.len() {
            let c = line[i];
            let wide = is_wide(c);
            match mode {
                // reading a word
                Mode::Word => {
                    if is_word_char(c) || wide {
                        if word_len >= MAX_WORD_LENGTH {
                            mode = Mode::TooLongWord;
                            i += 1;
                            continue;
                        }
                        word.push(c);
                        word_len += c.len_utf16();
                    } else if c == '"' {
                        // 2003.08.07 little YOSHI: the inside of a text is ignored
                        mode = Mode::Text;
                    } else {
                        if parse_count == 0 && eq(&word, "Public") {
                            func_id |= 0x10;
                        } else if parse_count == 0 && eq(&word, "Private") {
                            func_id |= 0x20;
                        } else if parse_count == 0 && eq(&word, "Friend") {
                            func_id |= 0x30;
                        } else if parse_count == 0 && eq(&word, "Static") {
                            func_id |= 0x100;
                        } else if parse_count == 0 && eq(&word, "Function") {
                            if eq(&word_prev, "End") {
                                in_procedure = false;
                            } else if !eq(&word_prev, "Exit") {
                                if eq(&word_prev, "Declare") {
                                    // a reference to a DLL
                                    func_id |= 0x200;
                                } else {
                                    in_procedure = true;
                                }
                                func_id |= 0x01;
                                parse_count = 1;
                                func_line = n + 1;
                            }
                        } else if parse_count == 0 && eq(&word, "Sub") {
                            if eq(&word_prev, "End") {
                                in_procedure = false;
                            } else if !eq(&word_prev, "Exit") {
                                if eq(&word_prev, "Declare") {
                                    func_id |= 0x200;
                                } else {
                                    in_procedure = true;
                                }
                                func_id |= 0x02;
                                parse_count = 1;
                                func_line = n + 1;
                            }
                        } else if parse_count == 0 && eq(&word, "Get") && eq(&word_prev, "Property")
                        {
                            in_procedure = true;
                            func_id |= 0x03;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 0 && eq(&word, "Let") && eq(&word_prev, "Property")
                        {
                            in_procedure = true;
                            func_id |= 0x04;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 0 && eq(&word, "Set") && eq(&word_prev, "Property")
                        {
                            in_procedure = true;
                            func_id |= 0x05;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 0 && eq(&word, "Const") && !eq(&word_prev, "#") {
                            if is_class || in_procedure || (func_id >> 4) & 0x0f == 0 {
                                // private in a class module and in a procedure, and without
                                // Public
                                func_id &= 0x0f2f;
                                func_id |= 0x20;
                            }
                            func_id |= 0x06;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 0 && eq(&word, "Enum") {
                            func_id |= 0x207;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 0 && eq(&word, "Type") {
                            if is_class {
                                func_id &= 0x0f2f;
                                func_id |= 0x20;
                            }
                            func_id |= 0x208;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 0 && eq(&word, "Event") {
                            func_id |= 0x209;
                            parse_count = 1;
                            func_line = n + 1;
                        } else if parse_count == 0 && eq(&word, "Property") && eq(&word_prev, "End")
                        {
                            in_procedure = false;
                        } else if parse_count == 1 {
                            out.push(func_line, word.clone(), func_id, 0);
                            parse_count = 0;
                            // the flags are or-ed, so they start again
                            func_id = 0;
                        }
                        word_prev = std::mem::take(&mut word);
                        word_len = 0;
                        mode = Mode::Normal;
                        // process this character again
                        continue;
                    }
                }
                // reading symbols
                Mode::Symbols => {
                    // 2003.07.10 little YOSHI: "#" is told apart to tell "#Const" from "Const"
                    if ends_symbols(c) || wide {
                        word_prev = std::mem::take(&mut word);
                        word_len = 0;
                        mode = Mode::Normal;
                        continue;
                    } else if c == '"' {
                        mode = Mode::Text;
                    } else if word_len >= MAX_WORD_LENGTH {
                        mode = Mode::TooLongWord;
                        i += 1;
                        continue;
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
                    } else if c == '\'' {
                        // a comment: the rest of the line
                        break;
                    } else if c == '"' {
                        mode = Mode::Text;
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
                // up to the closing quote of a text
                Mode::Text => {
                    // a doubled quote is part of the text
                    if c == '"' {
                        double_quote = !double_quote;
                    } else if double_quote {
                        // a character after a quote that is not a quote: the text ended
                        mode = Mode::Normal;
                        double_quote = false;
                        continue;
                    }
                }
            }
            i += 1;
        }
    }
    out
}
