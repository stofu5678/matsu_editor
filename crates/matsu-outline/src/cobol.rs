// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Cobol.cpp
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

//! The outline of a COBOL source: the labels (sections and paragraphs) under their divisions.
//!
//! Sakura: `CDocOutline::MakeTopicList_cobol` in `sakura_core/types/CType_Cobol.cpp`. It reads a
//! fixed format source: the 7th column marks a comment line (`*`), and a line whose 8th column
//! (area A) is not blank is a label, up to the first period. The name of an item is
//! `division::label`, which the tree (`SetTreeJava` without the class texts, see
//! [`crate::tree::build_java_tree`]) splits into a node for the division and the label under it.

use crate::chars::is_line_delimiter;
use crate::{FuncInfoArr, LineSource};

/// Lists the labels of a COBOL source. A line with a division header (`... DIVISION`) is not
/// listed: it sets the division of the labels that follow.
pub fn make_topic_list_cobol(src: &dyn LineSource) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let mut division = String::new();

    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        // a comment line
        if line.len() >= 7 && line[6] == '*' {
            continue;
        }
        // a label line
        if line.len() >= 8 && line[7] != ' ' {
            let mut label: Vec<char> = Vec::new();
            let mut i = 7usize;
            while i < line.len() {
                if line[i] == '.' || is_line_delimiter(line[i]) {
                    break;
                }
                label.push(line[i]);
                i += 1;
                // a run of blanks is one blank
                if line[i - 1] == ' ' {
                    while i < line.len() && line[i] == ' ' {
                        i += 1;
                    }
                }
            }

            // the header of a division: the first "division" in any case
            let keyword: Vec<char> = "division".chars().collect();
            let found = (0..=label.len().saturating_sub(keyword.len())).find(|&i| {
                label.len() >= keyword.len()
                    && label[i..i + keyword.len()]
                        .iter()
                        .zip(&keyword)
                        .all(|(a, b)| a.eq_ignore_ascii_case(b))
            });
            if let Some(i) = found {
                // Sakura copies `i + 7` characters here (`wcsncpy_s(..., i + nKeyWordLen - 1)`), which
                // drops the last letter: "IDENTIFICATION DIVISIO". The node would read like a typo
                // and nothing in the source says it is meant, so the whole word is kept (type A of
                // ADR-0018).
                division = label[..i + keyword.len()].iter().collect();
                continue;
            }
            let label: String = label.iter().collect();
            out.push(n + 1, format!("{division}::{label}"), 0, 0);
        }
    }
    out
}
