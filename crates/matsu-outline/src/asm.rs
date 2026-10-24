// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Asm.cpp, sakura_core/util/string_ex.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2026, Sakura Editor Organization (the part from string_ex.cpp)
//   Copyright (C) 2018-2022, Sakura Editor Organization (the part from CType_Asm.cpp)
//   The part from CType_Asm.cpp was written by MIK (2004.04.12).
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

//! The outline of an assembler source: its labels and procedures.
//!
//! Sakura: `CDocOutline::MakeTopicList_asm` in `sakura_core/types/CType_Asm.cpp` (2004.04.12 MIK
//! made it again). A line is cut at the first `;` (a comment) and at the first NUL, and the first
//! two tokens are read (`my_strtok` of `sakura_core/util/string_ex.cpp`: blanks separate them, but
//! not inside double quotes). A line with a token that has a quote or a backslash in it is not
//! analyzed past that token.
//!
//! `FuncInfo::info` is 50 for `name PROC`, 51 for a label (`name:`) and 52 for `name ENDP`.

use crate::{FuncInfoArr, LineSource};

/// How many tokens are read (`MAX_ASM_TOKEN`).
const MAX_TOKENS: usize = 2;

/// The characters that separate tokens.
fn is_delimiter(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// `my_strtok`: the next token of `buf` from `*offset`, or `None` at the end. A delimiter inside
/// double quotes does not separate; an empty token is skipped.
fn next_token(buf: &[char], offset: &mut usize) -> Option<String> {
    let mut i = *offset;
    loop {
        if i >= buf.len() {
            return None;
        }
        let start = i;
        let mut quoted = false;
        let mut end = buf.len();
        while i < buf.len() {
            if buf[i] == '"' {
                quoted = !quoted;
            }
            if !quoted && is_delimiter(buf[i]) {
                end = i;
                i += 1;
                break;
            }
            i += 1;
        }
        *offset = i;
        if end > start {
            return Some(buf[start..end].iter().collect());
        }
    }
}

/// Lists the labels and the procedures of an assembler source.
pub fn make_topic_list_asm(src: &dyn LineSource) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    for n in 0..src.line_count() {
        let Some(mut line) = src.line(n) else {
            break;
        };
        // what follows a NUL is not known; a comment is cut off
        if let Some(p) = line.iter().position(|&c| c == '\0' || c == ';') {
            line.truncate(p);
        }

        let mut offset = 0usize;
        let mut tokens: Vec<String> = Vec::new();
        for _ in 0..MAX_TOKENS {
            let Some(token) = next_token(&line, &mut offset) else {
                break;
            };
            // a token that is not an assembler word ends the reading
            if token.contains(['"', '\\', '\'']) {
                break;
            }
            tokens.push(token);
        }

        let Some(first) = tokens.first() else {
            continue;
        };
        let (id, name) = if first.chars().count() >= 2 && first.ends_with(':') {
            // a label: the colon is cut off
            (51, first[..first.len() - 1].to_owned())
        } else if let Some(second) = tokens.get(1) {
            if second.eq_ignore_ascii_case("proc") {
                (50, first.clone())
            } else if second.eq_ignore_ascii_case("endp") {
                (52, first.clone())
            } else {
                continue;
            }
        } else {
            continue;
        };
        out.push(n + 1, name, id, 0);
    }
    out
}
