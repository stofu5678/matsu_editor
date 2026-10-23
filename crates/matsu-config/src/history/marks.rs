// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/docplus/CBookmarkManager.cpp (GetBookMarks, SetBookMarks)
//
// Original copyright notices of those files:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The text Sakura keeps the bookmarked lines in (`MRU[nn].szMark2`).
//!
//! Version 2 (Moca, 2014): `:0` and then, for each bookmarked line, the distance from the one
//! before in base 32 (`CBookmarkManager::GetBookMarks`). A distance is the number of lines
//! between the two (`0` for the next line). The last digit is one of `0-9a-v`; the digits before
//! it are `w-zA-Z+-`. The text is at most 1023 characters (`MAX_MARKLINES_LEN`); lines that do not
//! fit are left out. The old format (version 1, `szMark`) is the line numbers (0 based) separated
//! by `,` or space.

/// `MAX_MARKLINES_LEN` (`config/maxdata.h`).
pub const MAX_MARKLINES_LEN: usize = 1023;

/// The text for the bookmarked lines (0 based, ascending).
pub fn encode_marks(lines: impl IntoIterator<Item = usize>) -> String {
    let mut text = String::from(":0");
    let mut previous: Option<usize> = None;
    for line in lines {
        let mut diff = match previous {
            Some(p) => line.saturating_sub(p + 1),
            None => line,
        };
        previous = Some(line);
        let mut digits = Vec::new();
        if diff == 0 {
            digits.push('0');
        } else {
            let mut column = 0;
            while diff != 0 {
                let d = (diff % 32) as u8;
                digits.push(match (column, d) {
                    (0, 0..=9) => char::from(b'0' + d),
                    (0, _) => char::from(b'a' + d - 10),
                    (_, 0..=3) => char::from(b'w' + d),
                    (_, 4..=29) => char::from(b'A' + d - 4),
                    (_, 30) => '+',
                    _ => '-',
                });
                column += 1;
                diff /= 32;
            }
            digits.reverse();
        }
        if text.len() + digits.len() > MAX_MARKLINES_LEN {
            break;
        }
        text.extend(digits);
    }
    text
}

/// The bookmarked lines in `text` (0 based). Text of an unknown version gives none.
pub fn decode_marks(text: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    if let Some(rest) = text.strip_prefix(':') {
        let Some(digits) = rest.strip_prefix('0') else {
            return lines;
        };
        let mut line = 0usize;
        let mut part = 0usize;
        for c in digits.chars() {
            let (value, last) = match c {
                '0'..='9' => (c as usize - '0' as usize, true),
                'a'..='v' => (c as usize - 'a' as usize + 10, true),
                'w'..='z' => (c as usize - 'w' as usize, false),
                'A'..='Z' => (c as usize - 'A' as usize + 4, false),
                '+' => (30, false),
                '-' => (31, false),
                _ => break,
            };
            part = part.saturating_add(value);
            if last {
                line = line.saturating_add(part);
                lines.push(line);
                line = line.saturating_add(1);
                part = 0;
            } else {
                part = part.saturating_mul(32);
            }
        }
    } else {
        // Version 1: `strtok` over ", ", then `_wtol` (a number that fails is 0).
        for word in text.split([',', ' ']).filter(|w| !w.is_empty()) {
            let digits: String = word.chars().take_while(char::is_ascii_digit).collect();
            lines.push(digits.parse().unwrap_or(0));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_examples_of_sakuras_comment() {
        // `:00123x0` => 1, 3, 6, 10, 43rd lines (0 based 0, 2, 5, 9, 42).
        assert_eq!(decode_marks(":00123x0"), [0, 2, 5, 9, 42]);
        // `:0a` => 10 (0 based), after nothing.
        assert_eq!(decode_marks(":0a"), [10]);
        assert_eq!(decode_marks(":0"), Vec::<usize>::new());
    }

    #[test]
    fn encoding_and_decoding_agree() {
        for lines in [
            vec![],
            vec![0],
            vec![0, 1, 2],
            vec![5, 40, 41, 1000, 100_000],
            vec![31, 32, 33, 1023, 1024],
        ] {
            let text = encode_marks(lines.clone());
            assert!(text.starts_with(":0"));
            assert_eq!(decode_marks(&text), lines, "{text}");
        }
        assert_eq!(encode_marks([0, 2, 5, 9, 42]), ":00123x0");
        assert_eq!(encode_marks([10]), ":0a");
        // 32 needs two digits: `x` is 1, then 0.
        assert_eq!(encode_marks([32]), ":0x0");
    }

    #[test]
    fn the_text_is_cut_at_the_limit() {
        // Every other line: one digit each.
        let lines = (0..2000).map(|i| i * 2);
        let text = encode_marks(lines);
        assert_eq!(text.len(), MAX_MARKLINES_LEN);
        assert_eq!(decode_marks(&text).len(), MAX_MARKLINES_LEN - 2);
    }

    #[test]
    fn the_old_format_is_line_numbers() {
        assert_eq!(decode_marks("3,10, 12"), [3, 10, 12]);
        assert_eq!(decode_marks(""), Vec::<usize>::new());
        // An unknown version.
        assert_eq!(decode_marks(":1abc"), Vec::<usize>::new());
    }
}
