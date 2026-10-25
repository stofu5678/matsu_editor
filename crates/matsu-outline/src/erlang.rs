// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Erlang.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 2009, genta
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

//! The outline of an Erlang source: the functions, each as `name/arity`, a list.
//!
//! Sakura: `COutlineErlang` and `CDocOutline::MakeFuncList_Erlang` in
//! `sakura_core/types/CType_Erlang.cpp` (genta, 2009.08.10).
//!
//! A function declaration starts in the first column. The scan takes an atom there, counts the
//! arguments between the parentheses (nested brackets and quoted text are taken into account),
//! and calls it a function when `->` or `when` follows (it may be on a later line). A `%` comment
//! is skipped to the end of the line.

use crate::{FuncInfoArr, LineSource};

/// The size of the buffer of the name with its arity (`m_func[64]`), with the null.
const FUNC_BUFFER: usize = 64;

/// The depth of the parentheses that are followed (`m_parenthesis[32]`).
const PARENTHESIS_MAX: usize = 32;

/// What the scan is doing (`m_state`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Not in a function.
    Normal,
    /// Something that looks like a function (the atom in the first column) is read.
    FuncCandidateFin,
    /// Looking at the first argument.
    FuncArgs1,
    /// Looking at the second argument and the ones after it.
    FuncArgs,
    /// The arguments are read.
    FuncArgsFin,
    /// A function is found.
    FuncFound,
}

/// `IS_ATOM_HEAD`
fn is_atom_head(c: char) -> bool {
    c.is_ascii_lowercase() || c == '_' || c == '@'
}

/// `IS_ALNUM`
fn is_alnum(c: char) -> bool {
    is_atom_head(c) || c.is_ascii_uppercase() || c.is_ascii_digit()
}

/// `IS_COMMENT`
fn is_comment(c: char) -> bool {
    c == '%'
}

/// `IS_SPACE`
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// The state machine (`COutlineErlang`).
struct Erlang {
    state: State,
    /// The name with the arity, as the list shows it (`m_func`).
    func: String,
    /// The line of the function, 0 origin (`m_lnum`).
    line: usize,
    /// The arguments found (`m_argcount`).
    argcount: usize,
    /// The opening brackets that are not closed (`m_parenthesis`).
    parenthesis: [char; PARENTHESIS_MAX],
    /// The depth of the brackets (`m_parenthesis_ptr`); it may pass the size of the array.
    parenthesis_ptr: usize,
}

impl Erlang {
    fn new() -> Self {
        Self {
            state: State::Normal,
            func: String::new(),
            line: 0,
            argcount: 0,
            parenthesis: ['\0'; PARENTHESIS_MAX],
            parenthesis_ptr: 0,
        }
    }

    /// `ScanFuncName`: the name of the function, an atom. An atom starts with a lowercase letter,
    /// `_` or `@` and goes on with letters and digits, or it is text between single quotes.
    fn scan_func_name(&mut self, buf: &[char], p: usize) -> usize {
        let end = buf.len();
        if p > 0 || p >= end || !(is_atom_head(buf[p]) || buf[p] == '\'') {
            return end;
        }
        let mut p = p;
        if buf[p] == '\'' {
            loop {
                p += 1;
                if p >= end || buf[p] == '\'' {
                    break;
                }
            }
            if p >= end {
                // not a valid atom
                return p;
            }
            p += 1;
        } else {
            loop {
                p += 1;
                if p >= end || !is_alnum(buf[p]) {
                    break;
                }
            }
        }
        let name: &[char] = if buf[0] == '\'' {
            &buf[1..p - 1]
        } else {
            &buf[..p]
        };
        // `wcsncpy_s` into 64 units: a longer name does not fit; it is cut here
        self.func = name.iter().take(FUNC_BUFFER - 1).collect();
        self.state = State::FuncCandidateFin;
        p
    }

    /// `EnterArgs`: the parenthesis that starts the arguments.
    fn enter_args(&mut self, buf: &[char], mut p: usize) -> usize {
        let end = buf.len();
        while p < end && is_space(buf[p]) {
            p += 1;
        }
        if p >= end {
            return end;
        }
        if is_comment(buf[p]) {
            return end;
        }
        if buf[p] == '(' {
            self.state = State::FuncArgs1;
            self.argcount = 0;
            self.parenthesis_ptr = 1;
            self.parenthesis[0] = buf[p];
            return p + 1;
        }
        // not a function
        self.state = State::Normal;
        end
    }

    /// `ScanArgs1`: the first argument, to tell no argument from one or more.
    fn scan_args1(&mut self, buf: &[char], mut p: usize) -> usize {
        let end = buf.len();
        while p < end && is_space(buf[p]) {
            p += 1;
        }
        if p >= end {
            return end;
        }
        if buf[p] == ')' {
            // no argument
            self.state = State::FuncArgsFin;
            p += 1;
        } else if is_comment(buf[p]) {
            return end;
        } else {
            // an argument
            self.state = State::FuncArgs;
            self.argcount += 1;
        }
        p
    }

    /// `ScanArgs`: reads the arguments and counts them. The commas, the brackets and the quotes
    /// matter; a quote does not go over the end of a line.
    fn scan_args(&mut self, buf: &[char], mut p: usize) -> usize {
        let end = buf.len();
        let mut quote = '\0';
        let head = p;
        while p < end {
            let c = buf[p];
            if quote != '\0' {
                if c == quote {
                    quote = '\0';
                }
            } else if matches!(c, '(' | '[' | '{') {
                // level up
                if self.parenthesis_ptr < PARENTHESIS_MAX {
                    self.parenthesis[self.parenthesis_ptr] = c;
                }
                self.parenthesis_ptr += 1;
            } else if matches!(c, ')' | ']' | '}') {
                let open = match c {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                // level down; a bracket that does not match is skipped
                self.parenthesis_ptr -= 1;
                while self.parenthesis_ptr >= 1 && self.parenthesis_ptr < PARENTHESIS_MAX {
                    if self.parenthesis[self.parenthesis_ptr] != open {
                        self.parenthesis_ptr -= 1;
                    } else {
                        break;
                    }
                }
                if self.parenthesis_ptr == 0 {
                    self.state = State::FuncArgsFin;
                    return p + 1;
                }
            } else if c == ',' && self.parenthesis_ptr == 1 {
                self.argcount += 1;
            } else if c == ';' {
                // A semicolon separates statements; it is not in the arguments, so the scan has
                // failed (a missing closing bracket, maybe): start again.
                self.state = State::Normal;
                return end;
            } else if c == '.' {
                // A period ends an expression, or is a decimal point.
                if p > head && buf[p - 1].is_ascii_digit() {
                    // maybe a decimal point: it stays
                } else {
                    // the end of a statement in the arguments: the scan has failed
                    self.state = State::Normal;
                    return end;
                }
            } else if c == '"' {
                quote = '"';
            } else if c == '\'' {
                quote = '\'';
            } else if is_comment(c) {
                return end;
            }
            p += 1;
        }
        p
    }

    /// `EnterCond`: the arrow of the body (`->`) or the keyword of a condition (`when`) makes it a
    /// function; anything else means it was not one.
    fn enter_cond(&mut self, buf: &[char], mut p: usize) -> usize {
        let end = buf.len();
        while p < end && is_space(buf[p]) {
            p += 1;
        }
        if p >= end {
            return end;
        }
        let arrow = p + 1 < end && buf[p] == '-' && buf[p + 1] == '>';
        let when = p + 3 < end && buf[p..p + 4] == ['w', 'h', 'e', 'n'];
        if arrow || when {
            self.state = State::FuncFound;
        } else if is_comment(buf[p]) {
            return end;
        } else {
            self.state = State::Normal;
        }
        end
    }

    /// `build_arity`: `/arity` after the name. As much as fits is written (63 units).
    fn build_arity(&mut self) {
        let arity = format!("/{}", self.argcount);
        let room = (FUNC_BUFFER - 1).saturating_sub(self.func.chars().count());
        self.func.extend(arity.chars().take(room));
    }

    /// `parse`: a line. Returns whether a function was found (its name is in `func`).
    fn parse(&mut self, buf: &[char], line_number: usize) -> bool {
        let end = buf.len();
        let mut pos = 0;
        if self.state == State::FuncFound {
            self.state = State::Normal;
        }
        if self.state == State::Normal {
            pos = self.scan_func_name(buf, pos);
            if self.state != State::Normal {
                self.line = line_number;
            }
        }
        while pos < end {
            match self.state {
                State::FuncCandidateFin => pos = self.enter_args(buf, pos),
                State::FuncArgs1 => pos = self.scan_args1(buf, pos),
                State::FuncArgs => pos = self.scan_args(buf, pos),
                State::FuncArgsFin => pos = self.enter_cond(buf, pos),
                // `PleaseReportToAuthor` in Sakura: the states cannot be these here
                State::Normal | State::FuncFound => break,
            }
            if self.state == State::FuncFound {
                self.build_arity();
                break;
            }
        }
        self.state == State::FuncFound
    }
}

/// `MakeFuncList_Erlang`.
pub fn make_func_list_erlang(src: &dyn LineSource) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let mut machine = Erlang::new();
    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        if machine.parse(&line, n) {
            out.push(machine.line + 1, machine.func.clone(), 0, 0);
        }
    }
    out
}
