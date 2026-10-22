// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Text.cpp
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

//! The outline of a text: the topics that start with a heading mark (`MakeTopicList_txt`), and a
//! text with levels in the style of the WZ editor (`MakeTopicList_wztxt`).
//!
//! Sakura: `CDocOutline::MakeTopicList_txt` and `MakeTopicList_wztxt` in
//! `sakura_core/types/CType_Text.cpp`.

use crate::chars::{at, is_blank, is_line_delimiter};
use crate::{FuncInfoArr, LineSource, NO_CLIP_TEXT};

/// The default of 見出し記号 (`m_szMidashiKigou`, `CShareData::InitShareData`): a line that
/// starts with one of these characters is a topic.
pub const DEFAULT_HEADING_MARKS: &str = "１２３４５６７８９０（(［[「『【■□▲△▼▽◆◇○◎●§・※☆★第①②③④⑤⑥⑦⑧⑨⑩⑪⑫⑬⑭⑮⑯⑰⑱⑲⑳ⅠⅡⅢⅣⅤⅥⅦⅧⅨⅩ一二三四五六七八九十壱弐参伍";

/// `STR_NO_TITLE1`: the name of an item that the levels skip.
pub const NO_TITLE: &str = "(無題)";

/// The deepest level of the topics (`nMaxStack`).
const MAX_STACK: usize = 32;

fn in_range(c: char, front: char, back: char) -> bool {
    c >= front && c <= back
}

/// The kind of heading that a line starts with: the title that decides the level. `None` for a
/// line that is not a topic (a `(` that is not followed by a digit or a letter).
fn heading_title(line: &[char], i: usize, b278a: &mut bool) -> Option<String> {
    let c = line[i];
    let title = if c == '(' {
        let next = at(line, i + 1);
        if in_range(next, '0', '9') {
            "(0)".to_owned()
        } else if in_range(next, 'A', 'Z') {
            "(A)".to_owned()
        } else if in_range(next, 'a', 'z') {
            "(a)".to_owned()
        } else {
            // 「(」の次が英数字で無い場合、見出しとみなさない
            return None;
        }
    } else if in_range(c, '０', '９') {
        "０".to_owned()
    } else if in_range(c, '①', '⑳')
        || c == '\u{24ea}'
        || in_range(c, '\u{3251}', '\u{325f}')
        || in_range(c, '\u{32b1}', '\u{32bf}')
    {
        "①".to_owned()
    } else if in_range(c, 'Ⅰ', '\u{216f}') || in_range(c, 'ⅰ', '\u{217f}') {
        "Ⅰ".to_owned()
    } else if in_range(c, '\u{2474}', '\u{2487}') {
        "\u{2474}".to_owned()
    } else if in_range(c, '\u{2488}', '\u{249b}') {
        "\u{2488}".to_owned()
    } else if in_range(c, '\u{249c}', '\u{24b5}') {
        "\u{249c}".to_owned()
    } else if in_range(c, '\u{24b6}', '\u{24cf}') {
        "\u{24b6}".to_owned()
    } else if in_range(c, '\u{24d0}', '\u{24e9}') {
        "\u{24d0}".to_owned()
    } else if in_range(c, '\u{24eb}', '\u{24f4}') {
        // ●11-●20
        if *b278a { "\u{278a}" } else { "\u{2776}" }.to_owned()
    } else if in_range(c, '\u{24f5}', '\u{24fe}') {
        "\u{24f5}".to_owned()
    } else if in_range(c, '\u{2776}', '\u{277f}') {
        "\u{2776}".to_owned()
    } else if in_range(c, '\u{2780}', '\u{2789}') {
        "\u{2780}".to_owned()
    } else if in_range(c, '\u{278a}', '\u{2793}') {
        // ●1-●10 (SANS-SERIF)
        *b278a = true;
        "\u{278a}".to_owned()
    } else if in_range(c, '\u{3220}', '\u{3229}') {
        // Sakura's `L"ꌢ0"` is U+A322 followed by a zero: kept, so that the levels are the same.
        "\u{a322}0".to_owned()
    } else if in_range(c, '\u{3280}', '\u{3289}') {
        "\u{3220}".to_owned()
    } else if in_range(c, '\u{32d0}', '\u{32fe}') {
        "\u{32d0}".to_owned()
    } else if "〇一二三四五六七八九十百零壱弐参伍".contains(c) {
        "一".to_owned()
    } else {
        c.to_string()
    };
    Some(title)
}

/// テキスト・トピックリスト作成: the lines that start with one of the `marks` (after blanks) are
/// topics; the level is found from the kind of the first character: a heading of a kind that was
/// seen goes to the level of that kind, a new kind goes one level deeper.
///
/// Sakura: `CDocOutline::MakeTopicList_txt`.
pub fn make_topic_list_txt(src: &dyn LineSource, marks: &str) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let marks: Vec<char> = marks.chars().collect();
    // The kinds of the headings seen, by depth (`pszStack`).
    let mut stack: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut b278a = false;
    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        // skip the blanks at the start
        let Some(i) = line.iter().position(|&c| !is_blank(c)) else {
            continue;
        };
        // a topic starts with one of the marks
        if !marks.contains(&line[i]) {
            continue;
        }
        let Some(title) = heading_title(&line, i, &mut b278a) else {
            continue;
        };

        // the text without the line ending
        let end = line[i..]
            .iter()
            .position(|&c| is_line_delimiter(c))
            .map_or(line.len(), |p| i + p);
        let text: String = line[i..end].iter().collect();

        // the depth
        let mut append = true;
        match stack.iter().take(depth).position(|s| *s == title) {
            Some(k) => {
                // a heading of this kind was seen: the same level
                depth = k;
            }
            None if depth < MAX_STACK => {
                // a new kind: remember it at this depth
                if stack.len() <= depth {
                    stack.resize(depth + 1, String::new());
                }
                stack[depth] = title;
            }
            None => {
                // nDepth = nMaxStack; (the heading is dropped, 2002.11.03 Moca)
                append = false;
            }
        }
        if append {
            out.push(n + 1, text, 0, depth);
            depth += 1;
        }
    }
    out
}

/// 階層付きテキスト アウトライン解析: a line that starts with dots is a topic, as deep as the dots
/// are many. A level that skips levels gets items called (無題).
///
/// Sakura: `CDocOutline::MakeTopicList_wztxt`.
pub fn make_topic_list_wztxt(src: &dyn LineSource) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let mut level_prev = 0usize;
    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        if at(&line, 0) != '.' {
            continue;
        }
        // the dots are the depth
        let mut pos = 1;
        while at(&line, pos) == '.' {
            pos += 1;
        }
        let level = pos;
        // 2003.06.27 Moca: when the level is two or more deeper, add items without a title
        // (not for the tag list)
        if level_prev < level && level != level_prev + 1 {
            for dummy in (level_prev + 1)..level {
                out.push(n + 1, NO_TITLE, NO_CLIP_TEXT, dummy - 1);
            }
        }
        level_prev = level;

        let mut title = format!("{level} - ");
        // szTitle[1024]: the text is cut 2 before the end of the buffer
        let room = 1022usize.saturating_sub(title.chars().count());
        for &c in line[pos..].iter().take(room) {
            if is_line_delimiter(c) || c == '\0' {
                break;
            }
            title.push(c);
        }
        out.push(n + 1, title, 0, level - 1);
    }
    out
}
