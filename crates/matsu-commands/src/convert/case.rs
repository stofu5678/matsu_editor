// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/convert/CConvert_ToLower.cpp
//   sakura_core/convert/CConvert_ToUpper.cpp
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (UTF-16 buffers
// edited in place) was rewritten in Rust on UTF-8 strings, and the tests are new.
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

//! Lower and upper case, the way Sakura does it (`F_TOLOWER`, `F_TOUPPER`).
//!
//! Sakura does not use the Unicode case mapping. It converts by hand-written ranges: ASCII,
//! full-width Latin, Greek, Cyrillic, Latin-1 Supplement, Latin Extended-A, B and Additional.
//! Chars outside the ranges (and everything outside the BMP) stay as they are, and the length
//! never changes. The standard library's `to_lowercase` would give other results (`ß`, `ς`, the
//! Deseret letters), so this module does not use it.

/// Lower-cases every char by Sakura's ranges (`CConvert_ToLower::DoConvert`).
pub fn to_lower(text: &str) -> String {
    text.chars().map(lower_char).collect()
}

/// Upper-cases every char by Sakura's ranges (`CConvert_ToUpper::DoConvert`).
pub fn to_upper(text: &str) -> String {
    text.chars().map(upper_char).collect()
}

fn in_ranges(c: u32, ranges: &[(u32, u32)]) -> bool {
    ranges.iter().any(|&(lo, hi)| (lo..=hi).contains(&c))
}

/// The ranges of Latin Extended-B where the upper-case char is the even one and the lower-case
/// char is the odd one after it; Sakura converts them by setting or clearing the lowest bit.
const LATIN_B_PAIRS: &[(u32, u32)] = &[
    (0x0180, 0x0185),
    (0x0198, 0x0199),
    (0x01A0, 0x01A5),
    (0x01AC, 0x01AD),
    (0x01B8, 0x01B9),
    (0x01BC, 0x01BD),
    (0x01DE, 0x01EF),
    (0x01F4, 0x01F5),
    (0x01F8, 0x01FF),
];

/// The ranges of Latin Extended-B where the upper-case char is the odd one and the lower-case
/// char is the even one after it.
const LATIN_B_ODD_UPPER: &[(u32, u32)] = &[
    (0x0187, 0x018C),
    (0x0191, 0x0192),
    (0x01A7, 0x01A8),
    (0x01AF, 0x01B0),
    (0x01B3, 0x01B6),
    (0x01CD, 0x01DC),
];

fn to_char(c: u32) -> char {
    // every result is in the BMP and is not a surrogate
    char::from_u32(c).expect("a converted char is a valid scalar value")
}

fn lower_char(c: char) -> char {
    let u = u32::from(c);
    let r = match u {
        0x0041..=0x005A => 0x0061 + (u - 0x0041),
        // full-width Latin
        0xFF21..=0xFF3A => 0xFF41 + (u - 0xFF21),
        // Greek
        0x0391..=0x03A9 => 0x03B1 + (u - 0x0391),
        // Cyrillic
        0x0410..=0x042F => 0x0430 + (u - 0x0410),
        // Cyrillic (extended)
        0x0400..=0x040F => 0x0450 + (u - 0x0400),
        // Latin-1 Supplement (not the multiplication sign)
        0x00C0..=0x00DE if u != 0x00D7 => 0x00E0 + (u - 0x00C0),
        0x0178 => 0x00FF,
        // Latin Extended-A. U+0130 goes to a plain `i` (2014.11.25 in Sakura).
        0x0130 => 0x0069,
        0x0100..=0x0137 | 0x014A..=0x0177 => u | 1,
        0x0139..=0x0148 | 0x0179..=0x017E if u & 1 == 1 => u + 1,
        // Latin Extended-B
        _ if in_ranges(u, LATIN_B_PAIRS) => u | 1,
        _ if in_ranges(u, LATIN_B_ODD_UPPER) && u & 1 == 1 => u + 1,
        0x01C4 | 0x01C7 | 0x01CA | 0x01F1 => u + 2,
        // title case letters
        0x01C5 | 0x01C8 | 0x01CB | 0x01F2 => u + 1,
        0x018E => 0x01DD,
        0x01F6 => 0x0195,
        0x01F7 => 0x01BF,
        // Latin Extended Additional
        0x1E00..=0x1E95 | 0x1EA0..=0x1EFF => u | 1,
        _ => u,
    };
    to_char(r)
}

fn upper_char(c: char) -> char {
    let u = u32::from(c);
    let r = match u {
        0x0061..=0x007A => 0x0041 + (u - 0x0061),
        0xFF41..=0xFF5A => 0xFF21 + (u - 0xFF41),
        0x03B1..=0x03C9 => 0x0391 + (u - 0x03B1),
        0x0430..=0x044F => 0x0410 + (u - 0x0430),
        0x0450..=0x045F => 0x0400 + (u - 0x0450),
        0x00E0..=0x00FE if u != 0x00F7 => 0x00C0 + (u - 0x00E0),
        0x00FF => 0x0178,
        // U+0131 goes to a plain `I` (2014.11.25 in Sakura).
        0x0131 => 0x0049,
        0x0100..=0x0137 | 0x014A..=0x0177 => u & !1,
        0x0139..=0x0148 | 0x0179..=0x017E if u & 1 == 0 => u - 1,
        _ if in_ranges(u, LATIN_B_PAIRS) => u & !1,
        _ if in_ranges(u, LATIN_B_ODD_UPPER) && u & 1 == 0 => u - 1,
        0x01C6 | 0x01C9 | 0x01CC | 0x01F3 => u - 2,
        0x01C5 | 0x01C8 | 0x01CB | 0x01F2 => u - 1,
        0x01DD => 0x018E,
        0x0195 => 0x01F6,
        0x01BF => 0x01F7,
        0x1E00..=0x1E95 | 0x1EA0..=0x1EFF => u & !1,
        _ => u,
    };
    to_char(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_full_width_latin() {
        // CConvert_ToLower.cpp: A-Z, Ａ-Ｚ
        assert_eq!(to_lower("ABC xyz ＡＢＣ ０"), "abc xyz ａｂｃ ０");
        assert_eq!(to_upper("abc XYZ ａｂｃ ０"), "ABC XYZ ＡＢＣ ０");
    }

    #[test]
    fn greek_and_cyrillic() {
        assert_eq!(to_lower("ΑΒΓΩ"), "αβγω");
        assert_eq!(to_upper("αβγω"), "ΑΒΓΩ");
        // Sakura's range is the block 0x0391-0x03A9 including the unassigned 0x03A2, which
        // goes to 0x03C2 (final sigma) by the arithmetic
        assert_eq!(to_lower("\u{03A2}"), "\u{03C2}");
        // Ё is 0x0401, in the "extended" range
        assert_eq!(to_lower("АЯЁ"), "аяё");
        assert_eq!(to_upper("аяё"), "АЯЁ");
    }

    #[test]
    fn latin_1_supplement_skips_the_multiplication_and_division_signs() {
        assert_eq!(to_lower("ÀÞ×Ö"), "àþ×ö");
        assert_eq!(to_upper("àþ÷ö"), "ÀÞ÷Ö");
        // ÿ and Ÿ are far apart
        assert_eq!(to_upper("ÿ"), "Ÿ");
        assert_eq!(to_lower("Ÿ"), "ÿ");
        // Sakura does not know ß
        assert_eq!(to_upper("ß"), "ß");
        assert_eq!(to_lower("ß"), "ß");
    }

    #[test]
    fn latin_extended_a_with_even_and_odd_pairs() {
        assert_eq!(to_lower("ĀĂĄ"), "āăą");
        assert_eq!(to_upper("āăą"), "ĀĂĄ");
        // 0x0139-0x0148 and 0x0179-0x017E: the upper-case char is the odd one
        assert_eq!(to_lower("ĹŃŹ"), "ĺńź");
        assert_eq!(to_upper("ĺńź"), "ĹŃŹ");
        // the dotted and the dotless i go to the plain ASCII letter (2014.11.25 in Sakura)
        assert_eq!(to_lower("İ"), "i");
        assert_eq!(to_upper("ı"), "I");
        // 0x0138 (ĸ) and 0x0149 (ŉ) are outside both ranges
        assert_eq!(to_upper("ĸŉ"), "ĸŉ");
        assert_eq!(to_lower("ĸŉ"), "ĸŉ");
    }

    #[test]
    fn latin_extended_b_and_the_digraphs() {
        assert_eq!(
            to_lower("\u{01C4}\u{01C7}\u{01CA}\u{01F1}"),
            "\u{01C6}\u{01C9}\u{01CC}\u{01F3}"
        );
        assert_eq!(
            to_upper("\u{01C6}\u{01C9}\u{01CC}\u{01F3}"),
            "\u{01C4}\u{01C7}\u{01CA}\u{01F1}"
        );
        // title case letters
        assert_eq!(to_lower("\u{01C5}\u{01C8}"), "\u{01C6}\u{01C9}");
        assert_eq!(to_upper("\u{01C5}\u{01C8}"), "\u{01C4}\u{01C7}");
        assert_eq!(
            to_lower("\u{018E}\u{01F6}\u{01F7}"),
            "\u{01DD}\u{0195}\u{01BF}"
        );
        assert_eq!(
            to_upper("\u{01DD}\u{0195}\u{01BF}"),
            "\u{018E}\u{01F6}\u{01F7}"
        );
        assert_eq!(to_lower("\u{0187}\u{01CD}"), "\u{0188}\u{01CE}");
        assert_eq!(to_upper("\u{0188}\u{01CE}"), "\u{0187}\u{01CD}");
        assert_eq!(to_lower("\u{01DE}"), "\u{01DF}");
    }

    #[test]
    fn latin_extended_additional() {
        assert_eq!(
            to_lower("\u{1E00}\u{1EA0}\u{1EFE}"),
            "\u{1E01}\u{1EA1}\u{1EFF}"
        );
        assert_eq!(
            to_upper("\u{1E01}\u{1EA1}\u{1EFF}"),
            "\u{1E00}\u{1EA0}\u{1EFE}"
        );
        // outside the ranges
        assert_eq!(to_lower("\u{1E96}\u{1E9E}"), "\u{1E96}\u{1E9E}");
    }

    #[test]
    fn other_chars_surrogate_pairs_and_line_endings_stay() {
        // the standard library would change Deseret U+10400; Sakura only handles the BMP
        assert_eq!(to_lower("A\u{10400}\r\nB😀"), "a\u{10400}\r\nb😀");
        assert_eq!(to_upper("a\u{10428}\nb"), "A\u{10428}\nB");
        assert_eq!(to_lower("日本語ひらがなカタカナ"), "日本語ひらがなカタカナ");
        assert_eq!(to_lower(""), "");
    }
}
