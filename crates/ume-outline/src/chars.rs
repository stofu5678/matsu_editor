// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for UME Editor.
//
// Original files: sakura_core/charset/charcode.h, sakura_core/types/CType.h
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

//! The character tests that Sakura's analyzers use (`WCODE::*` in `sakura_core/charset/charcode.h`,
//! `C_IsSpace` in `sakura_core/types/CType.h`).

/// `WCODE::IsBlank`: a space, a tab or an ideographic space.
pub(crate) fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\u{3000}'
}

/// `WCODE::IsLineDelimiter` with `m_bEnableExtEol` off.
pub(crate) fn is_line_delimiter(c: char) -> bool {
    c == '\r' || c == '\n'
}

/// `C_IsSpace`: a tab, a space or a line ending.
pub(crate) fn is_space(c: char) -> bool {
    c == '\t' || c == ' ' || is_line_delimiter(c)
}

/// `WCODE::IsControlCode`: the control characters of ASCII that are not a tab or a line ending
/// (`CK_CTRL` of `gm_keyword_char`).
pub(crate) fn is_control_code(c: char) -> bool {
    let n = c as u32;
    (n < 0x20 && c != '\t' && c != '\n' && c != '\r') || n == 0x7f
}

/// `__iscsym`: a letter or digit of ASCII, or an underscore.
pub(crate) fn is_csym(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

/// `iswcntrl`.
pub(crate) fn is_wcntrl(c: char) -> bool {
    c.is_control()
}

/// `iswspace`.
pub(crate) fn is_wspace(c: char) -> bool {
    c.is_whitespace()
}

/// A character that takes two UTF-16 units: where Sakura tests `GetSizeOfChar(...) == 2`.
pub(crate) fn is_wide(c: char) -> bool {
    (c as u32) > 0xFFFF
}

/// The character at `i`, or NUL past the end: Sakura's lines are NUL-terminated buffers and some
/// analyzers read one or two characters beyond the length.
pub(crate) fn at(line: &[char], i: usize) -> char {
    line.get(i).copied().unwrap_or('\0')
}

/// Whether `line[i..]` starts with `lit` (`wcsncmp_literal`).
pub(crate) fn starts_at(line: &[char], i: usize, lit: &str) -> bool {
    for (k, c) in (i..).zip(lit.chars()) {
        if k >= line.len() || line[k] != c {
            return false;
        }
    }
    true
}

/// The text of `chars[from..to]` (clamped).
pub(crate) fn string_of(chars: &[char], from: usize, to: usize) -> String {
    let to = to.min(chars.len());
    let from = from.min(to);
    chars[from..to].iter().collect()
}
