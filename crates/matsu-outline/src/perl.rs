// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Perl.cpp
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

//! The outline of a Perl program (簡易版): the lines that match `/^\s*sub\s+(\w+)/`.
//!
//! Sakura: `CDocOutline::MakeFuncList_Perl` in `sakura_core/types/CType_Perl.cpp`. Nesting is not
//! considered; a package qualifier (`::` and `'`) is part of the name (2005.06.18 genta).

use crate::chars::is_line_delimiter;
use crate::{FuncInfoArr, LineSource};

/// The longest name (`nMaxWordLeng`).
const MAX_WORD_LENGTH: usize = 70;

/// What the scan of a line is doing (`nMode`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Before `sub`.
    Start,
    /// After `sub`, before the name.
    AfterSub,
    /// Reading the name.
    Word,
}

pub fn make_func_list_perl(src: &dyn LineSource) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        let mut mode = Mode::Start;
        let mut word = String::new();
        let mut i = 0;
        while i < line.len() {
            let c = line[i];
            // only the one-unit characters are processed
            if crate::chars::is_wide(c) {
                break;
            }
            match mode {
                Mode::Start => {
                    if c == '\t' || c == ' ' || is_line_delimiter(c) {
                        i += 1;
                        continue;
                    }
                    if c != 's' {
                        break;
                    }
                    // maybe the "s" of "sub"
                    if line.len() - i < 4 {
                        break;
                    }
                    if !crate::chars::starts_at(&line, i, "sub") {
                        break;
                    }
                    let after = line[i + 3];
                    if after == ' ' || after == '\t' {
                        mode = Mode::AfterSub;
                        i += 3;
                    } else {
                        break;
                    }
                }
                Mode::AfterSub => {
                    if c == '\t' || c == ' ' || is_line_delimiter(c) {
                        i += 1;
                        continue;
                    }
                    if c == '_' || c.is_ascii_alphanumeric() {
                        // the start of the name
                        word.clear();
                        word.push(c);
                        mode = Mode::Word;
                        i += 1;
                        continue;
                    }
                    break;
                }
                Mode::Word => {
                    if c == '_' || c.is_ascii_alphanumeric() || c == ':' || c == '\'' {
                        // `nWordIdx` counts from 0 for the first character
                        if word.chars().count() >= MAX_WORD_LENGTH {
                            break;
                        }
                        word.push(c);
                    } else {
                        out.push(n + 1, std::mem::take(&mut word), 0, 0);
                        break;
                    }
                }
            }
            i += 1;
        }
    }
    out
}
