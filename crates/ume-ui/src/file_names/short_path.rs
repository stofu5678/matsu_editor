// Derived from Sakura Editor (zlib License):
//   sakura_core/util/file.cpp (GetShortViewPath and the function there that cuts a text to a width)
//
// Copyright (C) 2002, SUI
// Copyright (C) 2003, MIK
// Copyright (C) 2008, kobake
// Copyright (C) 2018-2026, Sakura Editor Organization
//
// Modified: rewritten in Rust on character vectors. Sakura measures the text in pixels with the
// font of the window (`CTextWidthCalc`); here the width is counted in columns, a half-width
// character being one and a full-width character two, and the limit is the number of columns
// (Sakura's limit is the width of that many `x` of the same font).
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

//! The short form of a long path: `C:\sub1\sub2\sub3\file.ext` becomes `C:\...\sub3\file.ext`
//! (`GetShortViewPath`, 2014.06.12 Moca).

use unicode_width::UnicodeWidthChar;

/// The width of a text in columns. A character with no width (a combining mark) takes none.
fn width(text: &[char]) -> usize {
    text.iter().map(|c| c.width().unwrap_or(0)).sum()
}

fn is_separator(c: char) -> bool {
    c == '\\' || c == '/'
}

fn collect(chars: &[char]) -> String {
    chars.iter().collect()
}

/// The function of `file.cpp` that cuts a text to a width: the text cut so that it, with `...`, fits in `limit` columns. The left
/// part is kept as long as possible (`\\server\dir...`).
fn truncate(path: &[char], limit: usize) -> String {
    if width(path) <= limit {
        return collect(path);
    }
    // `strTempOld`: the longest start that was found to fit, one character behind the one that
    // is tried.
    let mut old = 0;
    for pos in 0..path.len() {
        if limit < width(&path[..pos]) + 3 {
            return format!("{}...", collect(&path[..old]));
        }
        old = pos;
    }
    collect(path)
}

/// `GetShortViewPath`: shortens `path` to `limit` columns by leaving out the folders in the
/// middle. The first folder (after `\\server\`, `\\?\`, `scheme://`) and the last names stay.
/// With `fit` a result that still does not fit is cut further, keeping the extension; without it
/// the result may stay longer than `limit`.
pub(super) fn short_path(path: &str, limit: usize, fit: bool) -> String {
    let path: Vec<char> = path.chars().collect();
    if width(&path) <= limit {
        return collect(&path);
    }
    let at = |i: usize| path.get(i).copied();
    let mut left = 0;
    let mut skip_level = 1;
    if at(0) == Some('\\') && at(1) == Some('\\') {
        if at(2) == Some('?') && at(4) == Some('\\') {
            // [\\?\A:\]
            left = 4;
        } else {
            // [\\server\dir\]: two levels stay.
            skip_level = 2;
            left = 2;
        }
    } else {
        // http://server/ and ftp://server/ stay.
        let mut top = 0;
        while at(top).is_some_and(|c| c != '/') {
            top += 1;
        }
        if top > 0 && path[top - 1] == ':' {
            while at(top) == Some('/') {
                top += 1;
            }
            left = top;
        }
    }
    for level in 0..skip_level {
        while at(left).is_some_and(|c| !is_separator(c)) {
            left += 1;
        }
        if at(left).is_some() {
            if level + 1 < skip_level {
                left += 1;
            }
        } else {
            // Nothing to leave out.
            return if fit {
                truncate(&path, limit)
            } else {
                collect(&path)
            };
        }
    }
    // `right` is where the part to show starts; it is at a separator.
    let mut right = left;
    while at(right).is_some() {
        let mut next = right + 1;
        while at(next).is_some_and(|c| !is_separator(c)) {
            next += 1;
        }
        if at(next).is_none() {
            break;
        }
        // The folders in between are left out: C:\...\dir\file.ext
        // (When `right` is `left`, the separator is in `path[right..]` already: Sakura repeats it.)
        let mut shown: Vec<char> = path[..if left == right { left } else { left + 1 }].to_vec();
        if left + 1 < right {
            shown.extend("...".chars());
        }
        shown.extend_from_slice(&path[right..]);
        if width(&shown) <= limit {
            return collect(&shown);
        }
        // The path of a folder: C:\...\dir\
        if at(next + 1).is_none() {
            return if fit {
                truncate(&shown, limit)
            } else {
                collect(&shown)
            };
        }
        right = next;
    }
    // Nothing is after `right` but a name: the file name. C:\...\file.ext
    let mut left_len = left;
    if left_len > 0 && left_len != right {
        left_len += 1;
    }
    let mut left_part: Vec<char> = path[..left_len.min(path.len())].to_vec();
    if left != right {
        left_part.extend("...".chars());
    }
    let mut shown = left_part.clone();
    shown.extend_from_slice(&path[right.min(path.len())..]);
    if !fit || width(&shown) <= limit {
        return collect(&shown);
    }
    // The name (or the left part) is too long. The extension stays, the title is cut.
    let ext = (right..path.len()).rev().find(|&i| path[i] == '.');
    let Some(ext) = ext else {
        return truncate(&shown, limit);
    };
    let ext_width = width(&path[ext..]);
    let room = limit
        .checked_sub(width(&left_part) + ext_width)
        .filter(|room| *room > 0);
    if room.is_none() {
        // No room for a name: the left side is kept.
        return truncate(&shown, limit);
    }
    // C:\...\longf...ext (Sakura joins what follows the dot, so the dot is the last of the `...`).
    let mut title = left_part;
    title.extend_from_slice(&path[right..ext]);
    let mut result = truncate(&title, limit - ext_width);
    result.push_str(&collect(&path[ext + 1..]));
    result
}
