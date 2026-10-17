// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for UME Editor.
//
// Original files: sakura_core/doc/CDocOutline.cpp
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

//! The list of the bookmarked lines (ブックマークの一覧).
//!
//! Sakura: `CDocOutline::MakeFuncList_BookMark` in `sakura_core/doc/CDocOutline.cpp`.

use crate::chars::{is_blank, is_line_delimiter, is_wide};
use crate::{FuncInfoArr, LineSource};

/// Lists the bookmarked lines (0 origin, in increasing order) with their text: without the blanks
/// at both ends. With `ignore_blank_lines` (the check box 空行を無視する, `m_bMarkUpBlankLineEnable`)
/// a line with nothing but blanks is left out.
///
/// The blank test compares the length of the line with the length of the line ending that new
/// lines get (`m_cNewLineCode.GetLen()`), not that of the ending the line has.
pub fn make_bookmark_list(
    src: &dyn LineSource,
    bookmarks: impl IntoIterator<Item = usize>,
    ignore_blank_lines: bool,
) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let new_line_len = src.new_line_len();
    for n in bookmarks {
        let Some(line) = src.line(n) else {
            // `GetDocLineStrWithEOL` is null: the loop ends
            break;
        };
        let len = line.len();
        // 2002.01.19 aroka: a line that is only a line ending
        if ignore_blank_lines && len <= new_line_len {
            continue;
        }
        // LTrim
        let left = line.iter().position(|&c| !is_blank(c)).unwrap_or(len);
        if ignore_blank_lines && (left + new_line_len >= len || left >= len) {
            continue;
        }
        // RTrim, from the left (2005.10.11 ryoji): the end of the last character that is not a
        // line ending, space or tab. A character of two units does not move the end in Sakura.
        let mut end = left;
        for (k, &c) in line.iter().enumerate().skip(left) {
            if !is_wide(c) && !(is_line_delimiter(c) || is_blank(c) || c == '\0') {
                end = k + 1;
            }
        }
        out.push(n + 1, line[left..end].iter().collect::<String>(), 0, 0);
    }
    out
}
