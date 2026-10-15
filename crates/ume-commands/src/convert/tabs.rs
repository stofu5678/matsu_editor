// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/convert/CConvert_TabToSpace.cpp
//   sakura_core/convert/CConvert_SpaceToTab.cpp
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (UTF-16 buffers
// edited in place) was rewritten in Rust on UTF-8 strings, the width of a char is a parameter
// (Sakura asks its font width cache), and the tests are new.
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

//! Tabs to spaces and spaces to tabs (`F_TABTOSPACE`, `F_SPACETOTAB`).
//!
//! Both work on text that starts at display column `start_col` (the column of the start of the
//! selection) and on tab stops every `tab_width` columns. The chars other than the tab and the
//! space take `char_width(c)` columns (Sakura: 2 for `WCODE::IsZenkaku`, else 1). Line endings
//! (CR, LF, CRLF) are kept as they are, and the column goes back to 0 after each of them. Only
//! the first line starts at `start_col`.

/// The columns of a char other than a tab: 2 for East Asian wide chars, else 1. An approximation
/// of Sakura's `WCODE::IsZenkaku` (`sakura_core/charset/charcode.cpp`, which asks the font cache),
/// the same as the one of `PlainLayout`.
pub fn char_width(c: char) -> usize {
    match u32::from(c) {
        0x1100..=0x115F
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1F64F
        | 0x20000..=0x3FFFD => 2,
        _ => 1,
    }
}

/// Splits `text` into lines and their line endings (CR, LF or CRLF; the last line may have none).
fn lines(text: &str) -> impl Iterator<Item = (&str, &str)> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let end = rest.find(['\r', '\n']).unwrap_or(rest.len());
        let eol_len = if rest[end..].starts_with("\r\n") {
            2
        } else {
            usize::from(end < rest.len())
        };
        let (line, tail) = rest.split_at(end + eol_len);
        rest = tail;
        Some(line.split_at(end))
    })
}

/// Replaces each tab by the spaces up to the next tab stop (`CConvert_TabToSpace::DoConvert`).
pub fn tab_to_space(
    text: &str,
    tab_width: usize,
    start_col: usize,
    char_width: impl Fn(char) -> usize,
) -> String {
    let w = tab_width.max(1);
    let mut out = String::with_capacity(text.len());
    for (i, (line, eol)) in lines(text).enumerate() {
        let mut pos = if i == 0 { start_col } else { 0 };
        for c in line.chars() {
            if c == '\t' {
                let n = w - pos % w;
                out.extend(std::iter::repeat_n(' ', n));
                pos += n;
            } else {
                out.push(c);
                pos += char_width(c);
            }
        }
        out.push_str(eol);
    }
    out
}

/// Replaces runs of spaces and tabs by as many tabs as fit, and spaces for the rest
/// (`CConvert_SpaceToTab::DoConvert`). A lone space between other chars stays a space.
pub fn space_to_tab(
    text: &str,
    tab_width: usize,
    start_col: usize,
    char_width: impl Fn(char) -> usize,
) -> String {
    let w = tab_width.max(1);
    let mut out = String::with_capacity(text.len());
    for (i, (line, eol)) in lines(text).enumerate() {
        let mut pos = if i == 0 { start_col } else { 0 };
        // the start column of the current run of spaces and tabs, if there is one
        let mut run: Option<usize> = None;
        let mut prev = '\0';
        for c in line.chars() {
            if c == ' ' || c == '\t' {
                run.get_or_insert(pos);
                pos += if c == ' ' { 1 } else { w - pos % w };
            } else {
                if let Some(start) = run.take() {
                    flush_run(&mut out, start, pos, prev == ' ', w);
                }
                pos += char_width(c);
                out.push(c);
            }
            prev = c;
        }
        if let Some(start) = run {
            flush_run(&mut out, start, pos, prev == ' ', w);
        }
        out.push_str(eol);
    }
    out
}

/// Writes a run of spaces and tabs that covers the columns `start..pos`.
fn flush_run(out: &mut String, start: usize, pos: usize, ends_with_space: bool, w: usize) {
    if pos - start == 1 && ends_with_space {
        out.push(' ');
        return;
    }
    let mut at = start;
    for _ in start / w..pos / w {
        out.push('\t');
        at += w - at % w;
    }
    out.extend(std::iter::repeat_n(' ', pos.saturating_sub(at)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t2s(text: &str, w: usize, col: usize) -> String {
        tab_to_space(text, w, col, char_width)
    }

    fn s2t(text: &str, w: usize, col: usize) -> String {
        space_to_tab(text, w, col, char_width)
    }

    #[test]
    fn splits_lines_with_their_endings() {
        let v: Vec<_> = lines("a\r\nb\rc\n\nd").collect();
        assert_eq!(
            v,
            [
                ("a", "\r\n"),
                ("b", "\r"),
                ("c", "\n"),
                ("", "\n"),
                ("d", "")
            ]
        );
        assert_eq!(lines("").count(), 0);
        assert_eq!(lines("\r\n").collect::<Vec<_>>(), [("", "\r\n")]);
    }

    #[test]
    fn tab_to_space_goes_to_the_next_tab_stop() {
        // CConvert_TabToSpace.cpp: width - (pos % width)
        assert_eq!(t2s("\t", 4, 0), "    ");
        assert_eq!(t2s("a\tb", 4, 0), "a   b");
        assert_eq!(t2s("abcd\te", 4, 0), "abcd    e");
        assert_eq!(t2s("a\t\tb", 8, 0), format!("a{}b", " ".repeat(15)));
        assert_eq!(t2s("a\tb", 1, 0), "a b");
    }

    #[test]
    fn tab_to_space_counts_wide_chars_and_the_start_column() {
        assert_eq!(t2s("日\tx", 4, 0), "日  x");
        assert_eq!(t2s("日a\tx", 4, 0), "日a x");
        // only the first line starts at start_col (the column of the selection start)
        assert_eq!(t2s("\tx\r\n\ty", 4, 2), "  x\r\n    y");
        assert_eq!(t2s("\tx\n\ty", 4, 3), " x\n    y");
        // a char outside the BMP and not wide is one column
        assert_eq!(t2s("\u{10400}\tx", 4, 0), "\u{10400}   x");
    }

    #[test]
    fn tab_to_space_keeps_line_endings() {
        assert_eq!(t2s("a\t\r\nb\t\nc\t\r\r", 4, 0), "a   \r\nb   \nc   \r\r");
        assert_eq!(t2s("", 4, 0), "");
        assert_eq!(t2s("\n", 4, 0), "\n");
    }

    #[test]
    fn space_to_tab_uses_tabs_for_runs_that_reach_a_tab_stop() {
        // CConvert_SpaceToTab.cpp
        assert_eq!(s2t("    x", 4, 0), "\tx");
        assert_eq!(s2t("        x", 4, 0), "\t\tx");
        assert_eq!(s2t("      x", 4, 0), "\t  x");
        assert_eq!(s2t("a       b", 4, 0), "a\t\tb");
        // a run that ends before the next tab stop stays spaces
        assert_eq!(s2t("a  b", 4, 0), "a  b");
        assert_eq!(s2t("ab   c", 4, 0), "ab\t c");
        assert_eq!(s2t("abc  d", 4, 0), "abc\t d");
    }

    #[test]
    fn space_to_tab_keeps_a_lone_space() {
        assert_eq!(s2t("a b", 4, 0), "a b");
        // a single column that is made by a tab is not a lone space
        assert_eq!(s2t("abc\td", 4, 0), "abc\td");
        // a lone space just before a tab stop: the run is one column long, so it stays a space
        assert_eq!(s2t("abc d", 4, 0), "abc d");
        assert_eq!(s2t(" ", 4, 0), " ");
    }

    #[test]
    fn space_to_tab_at_the_end_of_a_line_and_with_mixed_runs() {
        assert_eq!(s2t("x    \r\ny", 4, 0), "x\t \r\ny");
        assert_eq!(s2t("x   ", 4, 0), "x\t");
        assert_eq!(s2t("  \t x", 4, 0), "\t x");
        assert_eq!(s2t("\t\t", 4, 0), "\t\t");
        assert_eq!(s2t("    \n    \n", 4, 0), "\t\n\t\n");
    }

    #[test]
    fn space_to_tab_counts_wide_chars_and_the_start_column() {
        assert_eq!(s2t("日    x", 4, 0), "日\t  x");
        assert_eq!(s2t("日  x", 4, 0), "日\tx");
        // start at column 2: two spaces reach the stop at 4
        assert_eq!(s2t("  x", 4, 2), "\tx");
        // only the first line starts at start_col
        assert_eq!(s2t("  x\n  y", 4, 2), "\tx\n  y");
    }

    #[test]
    fn the_round_trip_keeps_the_columns() {
        for text in ["a\tb\tc", "  \t  x   y", "日\t本    語", "\t  \t"] {
            let back = t2s(&s2t(&t2s(text, 4, 0), 4, 0), 4, 0);
            assert_eq!(back, t2s(text, 4, 0), "{text:?}");
        }
    }
}
