// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Java.cpp
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

//! The outline of a Java program: its classes and interfaces, the methods and the declarations
//! in them. The names are `Outer\Inner::method`; the tree splits them at `\` and `::`.
//!
//! Sakura: `CDocOutline::MakeFuncList_Java` in `sakura_core/types/CType_Java.cpp`.

use crate::chars::{is_blank, is_control_code, is_csym, is_line_delimiter};
use crate::func_obj;
use crate::{FuncInfoArr, LineSource};

/// `STR_OUTLINE_JAVA_DEFPOS`: the name that stands for the place of a class definition.
const DEFPOS: &str = "定義位置";

/// The characters of ASCII that cannot be in a name (`szJavaKigou`); `_:~.$` can.
const SYMBOLS: &str = "!\"#%&'()=-^|\\`@[{+;*}]<,>?/";

/// The longest word (`nMaxWordLeng`).
const MAX_WORD_LENGTH: usize = 70;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Word,
    Comment,
    SingleQuote,
    DoubleQuote,
    TooLongWord,
}

/// What the parentheses of a method seen after a class give (`nNestLevel2Arr`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Method {
    /// Nothing yet.
    None,
    /// The `(` of a method: its name is in `func_name`.
    Open,
    /// The `)` is followed by `{`, `;` or a name (`throws`, `abstract`...): a method.
    Closed,
}

/// A class or interface being read.
struct Class {
    /// The nesting level of the `{` that holds it (`nClassNestArr`).
    nest_level: i32,
    method: Method,
}

fn is_word_char(c: char) -> bool {
    !is_blank(c) && !is_line_delimiter(c) && !is_control_code(c) && !SYMBOLS.contains(c)
}

/// The names of control statements: a `(` after one of them is not a method.
fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "sizeof" | "if" | "for" | "do" | "while" | "catch" | "switch" | "return"
    )
}

/// `auto_snprintf_s(szWork, 256, ...)`: a name that does not fit is not added.
fn fits(name: &str) -> bool {
    name.encode_utf16().count() < 256
}

pub fn make_func_list_java(src: &dyn LineSource) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let mut nest_level: i32 = 0;
    let mut word_prev = String::new();
    let mut word = String::new();
    let mut word_len = 0usize;
    let mut mode = Mode::Normal;
    let mut func_name = String::new();
    let mut func_line = 0usize;
    let mut class_name = String::new();
    let mut classes: Vec<Class> = Vec::new();

    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        let len = line.len();
        let mut i = 0usize;
        while i < len {
            let c = line[i];
            // escape sequences are always removed
            if c == '\\' {
                i += 2;
                continue;
            }
            match mode {
                // in a character constant
                Mode::SingleQuote => {
                    if c == '\'' {
                        mode = Mode::Normal;
                    }
                }
                // in a string
                Mode::DoubleQuote => {
                    if c == '"' {
                        mode = Mode::Normal;
                    }
                }
                // in a comment
                Mode::Comment => {
                    if i + 1 < len && c == '*' && line[i + 1] == '/' {
                        i += 1;
                        mode = Mode::Normal;
                    }
                }
                // reading a word
                Mode::Word => {
                    // 2011.09.16 syat: Japanese text is part of a name
                    if is_word_char(c) {
                        let units = c.len_utf16();
                        if word_len + units >= MAX_WORD_LENGTH {
                            mode = Mode::TooLongWord;
                        } else {
                            word.push(c);
                            word_len += units;
                        }
                    } else {
                        // found a class (or interface, 2002.10.10 genta) declaration
                        if word_prev == "class" || word_prev == "interface" {
                            classes.push(Class {
                                nest_level,
                                method: Method::None,
                            });
                            if nest_level > 0 {
                                class_name.push('\\');
                            }
                            class_name.push_str(&word);
                            let name = format!("{class_name}::{DEFPOS}");
                            if fits(&name) {
                                out.push(n + 1, name, func_obj::DEFINITION, 0);
                            }
                        }
                        mode = Mode::Normal;
                        // process this character again
                        continue;
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
                        // skip blanks
                    } else if i + 1 < len && c == '/' && line[i + 1] == '/' {
                        // a line comment: the rest of the line
                        break;
                    } else if i + 1 < len && c == '/' && line[i + 1] == '*' {
                        i += 1;
                        mode = Mode::Comment;
                    } else if c == '\'' {
                        mode = Mode::SingleQuote;
                    } else if c == '"' {
                        mode = Mode::DoubleQuote;
                    } else if c == '{' {
                        if let Some(class) = classes.last()
                            && class.method == Method::Closed
                        {
                            // 2002.10.10 genta: a method is not defined in a method, only a
                            // class or interface holds one (the level is just inside it)
                            if class.nest_level == nest_level - 1 && !is_keyword(&func_name) {
                                let name = format!("{class_name}::{func_name}");
                                if fits(&name) {
                                    out.push(func_line, name, func_obj::FUNCTION, 0);
                                }
                            }
                        }
                        if let Some(class) = classes.last_mut() {
                            class.method = Method::None;
                        }
                        nest_level += 1;
                        mode = Mode::Normal;
                    } else if c == '}' {
                        if let Some(class) = classes.last_mut() {
                            class.method = Method::None;
                        }
                        nest_level -= 1;
                        if classes.last().is_some_and(|c| c.nest_level == nest_level) {
                            classes.pop();
                            // the last name of the class path goes
                            let cut = class_name.rfind('\\').unwrap_or(0);
                            class_name.truncate(cut);
                        }
                        mode = Mode::Normal;
                    } else if c == '(' {
                        if !classes.is_empty() && word_prev != "new" {
                            func_name = word.clone();
                            func_line = n + 1;
                            if let Some(class) = classes.last_mut() {
                                class.method = Method::Open;
                            }
                        }
                        mode = Mode::Normal;
                    } else if c == ')' {
                        // what follows the ")" decides whether this is a method
                        let mut owned: Option<Vec<char>> = None;
                        let mut n2 = n;
                        let mut k = i + 1;
                        let mut in_comment = false;
                        loop {
                            let line2: &[char] = owned.as_deref().unwrap_or(&line);
                            while k < line2.len() {
                                if !in_comment {
                                    let c2 = line2[k];
                                    if c2 != ' ' && c2 != '\t' && !is_line_delimiter(c2) {
                                        if k + 1 < line2.len() && c2 == '/' && line2[k + 1] == '*' {
                                            in_comment = true;
                                            k += 1;
                                        } else if k + 1 < line2.len()
                                            && c2 == '/'
                                            && line2[k + 1] == '/'
                                        {
                                            k = line2.len() + 1;
                                            break;
                                        } else {
                                            break;
                                        }
                                    }
                                } else if k + 1 < line2.len()
                                    && line2[k] == '*'
                                    && line2[k + 1] == '/'
                                {
                                    in_comment = false;
                                    k += 1;
                                }
                                k += 1;
                            }
                            if k >= line2.len() {
                                // the next line
                                k = 0;
                                n2 += 1;
                                match src.line(n2) {
                                    Some(next) => {
                                        owned = Some(next);
                                        continue;
                                    }
                                    None => {
                                        if let Some(class) = classes.last_mut() {
                                            class.method = Method::None;
                                        }
                                    }
                                }
                            } else {
                                // 2002.10.10 genta: abstract methods too
                                let c2 = line2[k];
                                if let Some(class) = classes.last_mut() {
                                    if c2 == '{' || c2 == ';' || is_csym(c2) {
                                        if class.method == Method::Open {
                                            class.method = Method::Closed;
                                        }
                                    } else {
                                        class.method = Method::None;
                                    }
                                }
                            }
                            break;
                        }
                        mode = Mode::Normal;
                    } else if c == ';' {
                        if let Some(class) = classes.last()
                            && class.method == Method::Closed
                            && class.nest_level == nest_level - 1
                            && !is_keyword(&func_name)
                        {
                            let name = format!("{class_name}::{func_name}");
                            if fits(&name) {
                                out.push(func_line, name, func_obj::DECLARE, 0);
                            }
                        }
                        if let Some(class) = classes.last_mut() {
                            class.method = Method::None;
                        }
                        mode = Mode::Normal;
                    } else if is_word_char(c) {
                        word_prev = std::mem::take(&mut word);
                        word.push(c);
                        word_len = c.len_utf16();
                        mode = Mode::Word;
                    } else {
                        mode = Mode::Normal;
                    }
                }
            }
            i += 1;
        }
    }
    out
}
