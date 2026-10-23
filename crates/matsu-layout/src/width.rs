// Derived from Sakura Editor (zlib License):
//   sakura_core/charset/charcode.cpp (WCODE::IsHankaku, CCharWidthCache::QueryPixelWidth)
//   sakura_core/mem/CNativeW.cpp (CNativeW::GetKetaOfChar, GetHabaOfChar)
// Modified: Sakura asks the font for the width of a character; this version decides it from
// Unicode data alone, so that every OS gives the same layout. It is a new rule, not a port.
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The width of a character in columns, without a font.

use unicode_width::UnicodeWidthChar;

/// The first of the stand-ins for invalid bytes (`INVALID_BYTE_BASE` in the codec contract of
/// `matsu-core`). Byte `b` decodes to `U+10FF00 + b`.
pub const INVALID_BYTE_BASE: u32 = 0x10FF00;

/// The Latin-1 characters that JIS X 0208 has and that Japanese fonts draw full-width, such as
/// MS Gothic: `§ ¨ ° ± ´ ¶ × ÷`. All are East Asian Ambiguous.
const JIS_LATIN1: [char; 8] = ['§', '¨', '°', '±', '´', '¶', '×', '÷'];

/// How wide a character is, in columns: 1 (half-width) or 2 (full-width).
///
/// Sakura measures each character with the font and calls it half-width when it is no wider
/// than the average half-width letter (`CalcHankakuByFont`, `charcode.cpp`), so the result
/// depends on the font and the OS. This rule gives what Sakura shows with its default Japanese
/// fonts (MS Gothic and the like) in most cases:
///
/// | characters | columns | Sakura |
/// |---|---|---|
/// | U+0000..=U+007E, U+007F..=U+00A0, U+FF61..=U+FF9F | 1 | the same, fixed in `IsHankaku` |
/// | U+00A1..=U+02FF | 1, except `§ ¨ ° ± ´ ¶ × ÷` which are 2 | by the font |
/// | U+E000..=U+E8FF (private use) | 2 | the same, as U+E000 (`IsHankaku`) |
/// | East Asian Wide and Fullwidth | 2 | by the font |
/// | Greek and Cyrillic letters of JIS X 0208 | 2 (or 1, as below) | by the font |
/// | East Asian Ambiguous elsewhere | 2 (or 1, see [`WidthRule::ambiguous_wide`]) | by the font |
/// | combining marks, zero-width characters | 1 | by the font |
/// | other BMP characters | 1 | by the font |
/// | outside the BMP (surrogate pairs) | 2 | the same, `GetKetaOfChar` |
/// | stand-ins for invalid bytes | [`WidthRule::invalid_byte`] (2) | 1 in `GetKetaOfChar` |
///
/// Where it differs from Sakura is listed in the crate documentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidthRule {
    /// Draws East Asian Ambiguous characters (Greek, Cyrillic, box drawing, `○ ● ■ ※ ①` and
    /// the like) in two columns, as Japanese fonts do. The default is `true`.
    pub ambiguous_wide: bool,
    /// The columns of a stand-in for an invalid byte (U+10FF00..=U+10FFFF), which the UI draws
    /// as a box with two hex digits. The default is 2.
    pub invalid_byte: u32,
}

impl Default for WidthRule {
    fn default() -> Self {
        Self {
            ambiguous_wide: true,
            invalid_byte: 2,
        }
    }
}

impl WidthRule {
    /// The columns of `c`, 1 or 2 (a tab is not handled here, see `LayoutConfig::columns_of`).
    pub fn columns(&self, c: char) -> u32 {
        let cp = c as u32;
        // charcode.cpp, IsHankaku: "wc<=0x007E", and "0x7F ~ 0xA0 are half-width too".
        if cp <= 0xA0 {
            return 1;
        }
        // charcode.cpp, IsHankaku: half-width katakana, FF61..=FF9F.
        if (0xFF61..=0xFF9F).contains(&cp) {
            return 1;
        }
        if cp > 0xFFFF {
            // CNativeW.cpp, GetKetaOfChar: "outside the BMP is full-width" (2008/7/5 Uchi).
            return if cp >= INVALID_BYTE_BASE {
                self.invalid_byte
            } else {
                2
            };
        }
        // charcode.cpp, IsHankaku: private use, E000..=E8FF, is measured as U+E000.
        if (0xE000..=0xE8FF).contains(&cp) {
            return 2;
        }
        if cp <= 0x02FF {
            return if self.ambiguous_wide && JIS_LATIN1.contains(&c) {
                2
            } else {
                1
            };
        }
        // unicode-width leaves Greek and Cyrillic letters narrow even in its CJK mode, but the
        // JIS X 0208 ones (rows 6 and 7) are full-width in Japanese fonts. They are East Asian
        // Ambiguous in UAX #11.
        if self.ambiguous_wide
            && matches!(cp,
                0x0391..=0x03A1 | 0x03A3..=0x03A9 | 0x03B1..=0x03C1 | 0x03C3..=0x03C9
                | 0x0401 | 0x0410..=0x044F | 0x0451)
        {
            return 2;
        }
        let width = if self.ambiguous_wide {
            c.width_cjk()
        } else {
            c.width()
        };
        // Zero-width marks and controls get a cell of their own: Sakura draws each UTF-16 unit
        // at its own position and never joins them.
        match width {
            Some(2) => 2,
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_latin1_controls_are_half_width() {
        let rule = WidthRule::default();
        // sakura_core/charset/charcode.cpp, IsHankaku: wc<=0x007E and 0x7F..=0xA0.
        for c in ['a', ' ', '~', '\t', '\0', '\u{7F}', '\u{80}', '\u{A0}'] {
            assert_eq!(rule.columns(c), 1, "{c:?}");
        }
        // IsHankaku: half-width katakana.
        assert_eq!(rule.columns('ｱ'), 1);
        assert_eq!(rule.columns('ﾟ'), 1);
    }

    #[test]
    fn japanese_is_full_width() {
        let rule = WidthRule::default();
        for c in [
            'あ', 'ア', '漢', '，', '　', 'Ａ', '한', '\u{E000}', '\u{E8FF}',
        ] {
            assert_eq!(rule.columns(c), 2, "{c:?}");
        }
    }

    #[test]
    fn non_bmp_is_full_width() {
        let rule = WidthRule::default();
        // CNativeW.cpp, GetKetaOfChar: surrogate pairs are 2.
        for c in ['😀', '𠮷', '\u{10000}', '\u{1F1EF}'] {
            assert_eq!(rule.columns(c), 2, "{c:?}");
        }
    }

    #[test]
    fn invalid_byte_stand_ins_are_configurable() {
        let rule = WidthRule::default();
        assert_eq!(rule.columns('\u{10FF00}'), 2);
        assert_eq!(rule.columns('\u{10FFFF}'), 2);
        let one = WidthRule {
            invalid_byte: 1,
            ..rule
        };
        assert_eq!(one.columns('\u{10FF80}'), 1);
        // The char just below the range is an ordinary astral char.
        assert_eq!(one.columns('\u{10FEFF}'), 2);
    }

    #[test]
    fn ambiguous_follows_the_setting() {
        let wide = WidthRule::default();
        let narrow = WidthRule {
            ambiguous_wide: false,
            ..wide
        };
        // Greek, Cyrillic, box drawing, geometric shapes, curly quotes, and the JIS symbols of
        // Latin-1 are drawn full-width by Japanese fonts.
        for c in ['α', 'Я', '─', '○', '※', '“', '°', '×', '①'] {
            assert_eq!(wide.columns(c), 2, "{c:?}");
            assert_eq!(narrow.columns(c), 1, "{c:?}");
        }
        // Latin letters with accents are half-width in Japanese fonts too, ambiguous or not.
        for c in ['é', 'ü', 'ñ', '¡', '¿', '½', '·'] {
            assert_eq!(wide.columns(c), 1, "{c:?}");
        }
    }

    #[test]
    fn marks_and_zero_width_take_a_cell() {
        let rule = WidthRule::default();
        for c in ['\u{0301}', '\u{200B}', '\u{200D}', '\u{FE0F}', '\u{3099}'] {
            assert_eq!(rule.columns(c), 1, "{c:?}");
        }
    }
}
