// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CCodeBase.cpp (UnicodeToHex)
//   - sakura_core/charset/CShiftJis.cpp (UnicodeToHex)
//   - sakura_core/charset/CLatin1.cpp (UnicodeToHex)
//   - sakura_core/charset/CEuc.cpp (UnicodeToHex)
//   - sakura_core/charset/CJis.cpp (UnicodeToHex)
//   - sakura_core/charset/CUtf8.cpp (_UnicodeToHex)
//   - sakura_core/charset/CCesu8.h, CCodePage.cpp, CUnicode.h (use the one of CCodeBase)
//
// Modified: rewritten in Rust on UTF-8 strings and the codecs of this module. The original works
// on UTF-16 buffers and writes into a fixed buffer; here the text is a `&str` and the units of
// UTF-16 are only counted where the original looks at them (the surrogate pair, the variation
// selector). The bytes come from the codecs of `encoding`, so the result does not depend on the
// OS, and the stand-ins of invalid bytes (U+10FF00..) take the place of U+DC00..U+DCFF. The tests
// are new.
//
// Copyright (C) 2008, Uchi
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

//! The code of the character at the caret, as Sakura's status bar shows it
//! (`CCaret::ShowCaretPosInfo`, `sakura_core/view/CCaret.cpp`): the bytes of the character in the
//! encoding of the document, or its Unicode code point.

use super::{Encoding, INVALID_BYTE_BASE};

/// The check boxes of 共通設定 / ステータスバー / 表示文字コードの指定
/// (`CommonSetting_Statusbar`, `sakura_core/env/CommonSetting.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharCodeOptions {
    /// `m_bDispUniInSjis`: Shift_JIS and Latin1 show the Unicode code point.
    pub unicode_in_sjis: bool,
    /// `m_bDispUniInJis`: ISO-2022-JP shows the Unicode code point.
    pub unicode_in_jis: bool,
    /// `m_bDispUniInEuc`: EUC-JP shows the Unicode code point.
    pub unicode_in_euc: bool,
    /// `m_bDispUtf8Codepoint`: UTF-8 and CESU-8 show the code point, not the bytes.
    pub utf8_codepoint: bool,
    /// `m_bDispSPCodepoint`: a surrogate pair shows the code point (`U+1F600`), not the two units.
    pub surrogate_codepoint: bool,
}

impl Default for CharCodeOptions {
    /// The defaults of `CShareData::InitShareData` (`sakura_core/env/CShareData.cpp`).
    fn default() -> Self {
        Self {
            unicode_in_sjis: false,
            unicode_in_jis: false,
            unicode_in_euc: false,
            utf8_codepoint: true,
            surrogate_codepoint: true,
        }
    }
}

/// U+E0100..=U+E01EF, the variation selectors (`IsVariationSelector` in `charset/codechecker.h`
/// looks at the surrogate pair 0xDB40 0xDD00..=0xDDEF).
fn is_variation_selector(c: char) -> bool {
    ('\u{E0100}'..='\u{E01EF}').contains(&c)
}

/// The byte a stand-in char stands for.
fn stand_in_byte(c: char) -> Option<u8> {
    let n = u32::from(c);
    (INVALID_BYTE_BASE..INVALID_BYTE_BASE + 0x100)
        .contains(&n)
        .then(|| (n - INVALID_BYTE_BASE) as u8)
}

/// The code of the character that starts `text` (the rest of the line from the caret, without the
/// line ending), in the way Sakura shows it for `encoding`. Empty for an empty `text`.
///
/// Sakura's rules, by encoding:
/// * UTF-16, UTF-7 and the code pages show the code point ([`unicode_code`]).
/// * UTF-8 and CESU-8 show the code point or, with `utf8_codepoint` off, the bytes of the
///   character (a surrogate pair or a base character with its variation selector is one
///   character).
/// * Shift_JIS, Latin1, EUC-JP and ISO-2022-JP show the bytes of the first UTF-16 unit, or the
///   code point when their option is on. A character the encoding cannot write (a surrogate pair
///   is one) is shown as the code point too (`RESULT_LOSESOME` falls back to `CODE_UNICODE`).
/// * A byte that is invalid in the encoding (a stand-in) is `?` and the byte.
pub fn char_code(encoding: Encoding, text: &str, options: &CharCodeOptions) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let second = chars.next();
    let by_bytes = match encoding {
        Encoding::Utf8 | Encoding::Cesu8 => !options.utf8_codepoint,
        Encoding::ShiftJis | Encoding::Latin1 => !options.unicode_in_sjis,
        Encoding::EucJp => !options.unicode_in_euc,
        Encoding::Jis => !options.unicode_in_jis,
        _ => false,
    };
    if by_bytes && let Some(code) = bytes_code(encoding, first, second) {
        return code;
    }
    unicode_code(first, second, options)
}

/// `CCodeBase::UnicodeToHex`: the code point. A base character with a variation selector shows
/// both (`845B, U+E0100`), a surrogate pair shows the code point or the two units.
pub fn unicode_code(first: char, second: Option<char>, options: &CharCodeOptions) -> String {
    let unit = |c: char| match stand_in_byte(c) {
        // U+DC00..=U+DCFF
        Some(b) => 0xDC00 + u32::from(b),
        None => u32::from(c),
    };
    if first.len_utf16() == 1
        && let Some(vs) = second.filter(|&c| is_variation_selector(c))
    {
        return if options.surrogate_codepoint {
            format!("{:04X}, U+{:05X}", unit(first), u32::from(vs))
        } else {
            let (hi, lo) = utf16_pair(vs);
            format!("{:04X}, {hi:04X}{lo:04X}", unit(first))
        };
    }
    if first.len_utf16() == 2 && stand_in_byte(first).is_none() {
        return if options.surrogate_codepoint {
            format!("U+{:05X}", u32::from(first))
        } else {
            let (hi, lo) = utf16_pair(first);
            format!("{hi:04X}{lo:04X}")
        };
    }
    format!("U+{:04X}", unit(first))
}

fn utf16_pair(c: char) -> (u16, u16) {
    let mut units = [0u16; 2];
    c.encode_utf16(&mut units);
    (units[0], units[1])
}

/// The bytes of the character as hexadecimal, or `None` when the encoding cannot write it.
fn bytes_code(encoding: Encoding, first: char, second: Option<char>) -> Option<String> {
    let lower = encoding == Encoding::Latin1;
    let utf8 = matches!(encoding, Encoding::Utf8 | Encoding::Cesu8);
    let stand_in = stand_in_byte(first).is_some();
    // The characters that are converted: UTF-8 takes a surrogate pair and a base character with
    // its variation selector; the others only the first UTF-16 unit.
    let mut text = String::new();
    text.push(first);
    if utf8 && first.len_utf16() == 1 && second.is_some_and(is_variation_selector) {
        text.extend(second);
    } else if !utf8 && first.len_utf16() == 2 && !stand_in {
        // A lone high surrogate does not convert.
        return None;
    }
    let mut bytes = Vec::new();
    encoding.codec().encode(&text, &mut bytes).ok()?;
    if encoding == Encoding::Jis {
        // The escape sequences (ESC ... a capital letter) are not shown.
        let mut hex = String::new();
        let mut in_escape = false;
        for b in bytes {
            if b == 0x1B {
                in_escape = true;
            } else if in_escape {
                in_escape = !b.is_ascii_uppercase();
            } else {
                hex.push_str(&format!("{b:02X}"));
            }
        }
        return (!hex.is_empty()).then_some(hex);
    }
    let hex = |b: &u8| {
        if lower {
            format!("{b:02x}")
        } else {
            format!("{b:02X}")
        }
    };
    if stand_in {
        return Some(format!("?{}", hex(bytes.first()?)));
    }
    Some(bytes.iter().map(hex).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(encoding: Encoding, text: &str) -> String {
        char_code(encoding, text, &CharCodeOptions::default())
    }

    fn bytes(encoding: Encoding, text: &str) -> String {
        let options = CharCodeOptions {
            unicode_in_sjis: false,
            unicode_in_jis: false,
            unicode_in_euc: false,
            utf8_codepoint: false,
            surrogate_codepoint: true,
        };
        char_code(encoding, text, &options)
    }

    fn all_unicode() -> CharCodeOptions {
        CharCodeOptions {
            unicode_in_sjis: true,
            unicode_in_jis: true,
            unicode_in_euc: true,
            utf8_codepoint: true,
            surrogate_codepoint: true,
        }
    }

    #[test]
    fn utf8_shows_the_code_point_by_default() {
        assert_eq!(code(Encoding::Utf8, "a"), "U+0061");
        assert_eq!(code(Encoding::Utf8, "あい"), "U+3042");
        assert_eq!(code(Encoding::Cesu8, "あ"), "U+3042");
        assert_eq!(code(Encoding::Utf16Le, "あ"), "U+3042");
        assert_eq!(code(Encoding::Utf16Be, "a"), "U+0061");
        assert_eq!(code(Encoding::Utf7, "a"), "U+0061");
        assert_eq!(code(Encoding::Utf8, ""), "");
    }

    #[test]
    fn utf8_shows_the_bytes_when_the_option_is_off() {
        assert_eq!(bytes(Encoding::Utf8, "a"), "61");
        assert_eq!(bytes(Encoding::Utf8, "あ"), "E38182");
        assert_eq!(bytes(Encoding::Utf8, "😀"), "F09F9880");
        // CESU-8 writes a surrogate pair as two three-byte units.
        assert_eq!(bytes(Encoding::Cesu8, "😀"), "EDA0BDEDB880");
    }

    #[test]
    fn utf8_bytes_of_a_base_character_include_its_variation_selector() {
        // 葛 U+845B + U+E0100: E8 91 9B and F3 A0 84 80
        assert_eq!(bytes(Encoding::Utf8, "葛\u{E0100}"), "E8919BF3A08480");
    }

    #[test]
    fn shift_jis_shows_the_bytes() {
        assert_eq!(code(Encoding::ShiftJis, "a"), "61");
        assert_eq!(code(Encoding::ShiftJis, "あ"), "82A0");
        assert_eq!(code(Encoding::ShiftJis, "ｱ"), "B1");
        assert_eq!(code(Encoding::ShiftJis, "漢字"), "8ABF");
    }

    #[test]
    fn shift_jis_shows_unicode_with_the_option() {
        let options = CharCodeOptions {
            unicode_in_sjis: true,
            ..CharCodeOptions::default()
        };
        assert_eq!(char_code(Encoding::ShiftJis, "あ", &options), "U+3042");
        assert_eq!(char_code(Encoding::Latin1, "a", &options), "U+0061");
    }

    #[test]
    fn a_character_the_encoding_cannot_write_shows_the_code_point() {
        assert_eq!(code(Encoding::ShiftJis, "😀"), "U+1F600");
        assert_eq!(code(Encoding::EucJp, "😀"), "U+1F600");
        assert_eq!(code(Encoding::Jis, "😀"), "U+1F600");
        assert_eq!(code(Encoding::Latin1, "あ"), "U+3042");
        // A surrogate pair shows the two units when that option is off.
        let options = CharCodeOptions {
            surrogate_codepoint: false,
            ..CharCodeOptions::default()
        };
        assert_eq!(char_code(Encoding::ShiftJis, "😀", &options), "D83DDE00");
    }

    #[test]
    fn latin1_is_lower_case() {
        assert_eq!(code(Encoding::Latin1, "é"), "e9");
        assert_eq!(code(Encoding::Latin1, "A"), "41");
    }

    #[test]
    fn euc_jp_shows_the_bytes() {
        assert_eq!(code(Encoding::EucJp, "a"), "61");
        assert_eq!(code(Encoding::EucJp, "あ"), "A4A2");
        assert_eq!(code(Encoding::EucJp, "ｱ"), "8EB1");
        assert_eq!(char_code(Encoding::EucJp, "あ", &all_unicode()), "U+3042");
    }

    #[test]
    fn jis_shows_the_bytes_without_the_escape_sequences() {
        assert_eq!(code(Encoding::Jis, "a"), "61");
        assert_eq!(code(Encoding::Jis, "あ"), "2422");
        assert_eq!(code(Encoding::Jis, "漢"), "3441");
        assert_eq!(char_code(Encoding::Jis, "あ", &all_unicode()), "U+3042");
    }

    #[test]
    fn a_surrogate_pair_is_one_code_point() {
        assert_eq!(code(Encoding::Utf8, "😀x"), "U+1F600");
        assert_eq!(code(Encoding::Utf16Le, "𠮷"), "U+20BB7");
        // five digits at least
        assert_eq!(code(Encoding::Utf8, "\u{10000}"), "U+10000");
        let options = CharCodeOptions {
            surrogate_codepoint: false,
            ..CharCodeOptions::default()
        };
        assert_eq!(char_code(Encoding::Utf8, "😀", &options), "D83DDE00");
    }

    #[test]
    fn a_variation_selector_is_shown_with_its_base() {
        assert_eq!(code(Encoding::Utf8, "葛\u{E0100}"), "845B, U+E0100");
        let options = CharCodeOptions {
            surrogate_codepoint: false,
            ..CharCodeOptions::default()
        };
        assert_eq!(
            char_code(Encoding::Utf8, "葛\u{E0100}", &options),
            "845B, DB40DD00"
        );
        // Without a selector after it, the base alone.
        assert_eq!(code(Encoding::Utf8, "葛x"), "U+845B");
    }

    #[test]
    fn a_combining_character_is_a_character_of_its_own() {
        // The caret before "e" shows "e"; the accent after it shows U+0301.
        assert_eq!(code(Encoding::Utf8, "e\u{301}"), "U+0065");
        assert_eq!(code(Encoding::Utf8, "\u{301}"), "U+0301");
    }

    #[test]
    fn an_invalid_byte_shows_a_question_mark_and_the_byte() {
        let stand_in = char::from_u32(INVALID_BYTE_BASE + 0x81).expect("a scalar value");
        let text = stand_in.to_string();
        assert_eq!(code(Encoding::ShiftJis, &text), "?81");
        assert_eq!(code(Encoding::EucJp, &text), "?81");
        assert_eq!(code(Encoding::Latin1, &text), "?81");
        assert_eq!(bytes(Encoding::Utf8, &text), "?81");
        // The code point: the stand-in of Sakura is U+DC81.
        assert_eq!(code(Encoding::Utf8, &text), "U+DC81");
        assert_eq!(code(Encoding::Utf16Le, &text), "U+DC81");
    }
}
