// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/parse/CWordParse.cpp, sakura_core/parse/CWordParse.h (WhatKindOfChar, WhatKindOfTwoChars4KW,
//     SearchNextWordPosition4KW)
//   sakura_core/charset/charcode.cpp, sakura_core/charset/charcode.h (gm_keyword_char, IS_KEYWORD_CHAR, the
//     character class tests)
//   sakura_core/mem/CNativeW.cpp (GetSizeOfChar)
//   sakura_core/charset/codechecker.h (IsVariationSelector)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2006, D. S. Koba, genta
//   Copyright (C) 2007, kobake
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

//! The kind of a character and the word breaks of the keyword coloring.
//!
//! Positions are byte offsets in a line of UTF-8 text; Sakura counts UTF-16 units. A "character"
//! is what `CNativeW::GetSizeOfChar` takes as one: a code point, with the ideographic variation
//! selector that follows a BMP character.

use unicode_width::UnicodeWidthChar;

/// `ECharKind` (`sakura_core/parse/CWordParse.h`). The order matters: the keyword coloring skips
/// everything up to [`CharKind::Space`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CharKind {
    Null,
    Tab,
    Cr,
    Lf,
    Ctrl,
    Space,
    Csym,
    Kata,
    Latin,
    Udef,
    Etc,
    ZenSpace,
    ZenNobasu,
    ZenDaku,
    ZenCsym,
    ZenKigo,
    Hira,
    ZenKata,
    Greek,
    ZenRos,
    ZenSkigo,
    ZenEtc,
}

use CharKind as K;

/// `gm_keyword_char` (`sakura_core/charset/charcode.cpp`) for the ASCII characters.
fn ascii_kind(b: u8) -> CharKind {
    match b {
        0x09 => K::Tab,
        0x0A => K::Lf,
        0x0D => K::Cr,
        0x00..=0x1F | 0x7F => K::Ctrl,
        b' ' => K::Space,
        b'#' | b'$' | b'@' | b'\\' => K::Udef,
        b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_' => K::Csym,
        _ => K::Etc,
    }
}

/// Whether the character is a keyword character: `IS_KEYWORD_CHAR`. Only ASCII letters, digits,
/// `_`, `#`, `$`, `@` and `\`.
pub fn is_keyword_char(c: char) -> bool {
    c.is_ascii() && matches!(ascii_kind(c as u8), K::Csym | K::Udef)
}

/// `IsZenkakuKigou`'s table (`sakura_core/charset/charcode.h`).
const ZENKAKU_KIGOU: &str = "　、。，．・：；？！´｀¨＾￣＿〃―‐／＼～∥｜…‥‘’“”（）〔〕［］｛｝〈〉《》「」『』【】＋－±×÷＝≠＜＞≦≧∞∴♂♀°′″℃￥＄￠￡％＃＆＊＠§☆★○●◎◇◆□■△▲▽▼※〒→←↑↓〓∈∋⊆⊇⊂⊃∪∩∧∨￢⇒⇔∀∃∠⊥⌒∂∇≡≒≪≫√∽∝∵∫∬Å‰♯♭♪†‡¶◯";

/// Whether `c` is a variation selector that follows a BMP character (`IsVariationSelector`:
/// U+E0100 to U+E01EF).
fn is_variation_selector(c: char) -> bool {
    ('\u{E0100}'..='\u{E01EF}').contains(&c)
}

/// How many bytes the character at `idx` takes, counting a variation selector after a BMP
/// character with it (`CNativeW::GetSizeOfChar`). 0 at the end of the text.
pub fn char_len_at(s: &str, idx: usize) -> usize {
    let Some(c) = s.get(idx..).and_then(|rest| rest.chars().next()) else {
        return 0;
    };
    let mut len = c.len_utf8();
    if (c as u32) < 0x10000
        && let Some(next) = s[idx + len..].chars().next()
        && is_variation_selector(next)
    {
        len += next.len_utf8();
    }
    len
}

/// `CWordParse::WhatKindOfChar`: the kind of the character at `idx` (`Null` at the end).
pub fn kind_at(s: &str, idx: usize) -> CharKind {
    let Some(c) = s.get(idx..).and_then(|rest| rest.chars().next()) else {
        return K::Null;
    };
    let len = char_len_at(s, idx);
    if len > c.len_utf8() {
        // An ideographic variation sequence.
        return K::ZenEtc;
    }
    let code = c as u32;
    if code >= 0x10000 {
        // CJK extension planes count as full-width, other characters outside the BMP as half-width.
        return if (0x20000..=0x2FFFF).contains(&code) {
            K::ZenEtc
        } else {
            K::Etc
        };
    }
    if c.is_ascii() {
        return ascii_kind(c as u8);
    }
    if (0xFF61..=0xFF9F).contains(&code) {
        return K::Kata;
    }
    if (0x00C0..0x0180).contains(&code) && code != 0x00D7 && code != 0x00F7 {
        return K::Latin;
    }
    if c == '\u{3000}' {
        return K::ZenSpace;
    }
    if c == 'ー' {
        return K::ZenNobasu;
    }
    if c == '゛' || c == '゜' {
        return K::ZenDaku;
    }
    if c == '＿'
        || ('０'..='９').contains(&c)
        || ('Ａ'..='Ｚ').contains(&c)
        || ('ａ'..='ｚ').contains(&c)
    {
        return K::ZenCsym;
    }
    if ZENKAKU_KIGOU.contains(c) {
        return K::ZenKigo;
    }
    if (0x3041..=0x3096).contains(&code) || (0x309D..=0x309E).contains(&code) {
        return K::Hira;
    }
    if (0x30A1..=0x30FA).contains(&code) || (0x30FD..=0x30FE).contains(&code) {
        return K::ZenKata;
    }
    if (0x0391..=0x03C9).contains(&code) {
        return K::Greek;
    }
    if (0x0400..=0x052F).contains(&code)
        || (0x2DE0..=0x2DFF).contains(&code)
        || (0xA640..=0xA69F).contains(&code)
    {
        return K::ZenRos;
    }
    if (0x2500..=0x257F).contains(&code) {
        return K::ZenSkigo;
    }
    // `IsHankaku` asks the font in Sakura. 0x7F to 0xA0 are half-width there; for the rest this
    // follows East Asian Width, as the layout does.
    if (0x7F..=0xA0).contains(&code) || c.width() != Some(2) {
        K::Etc
    } else {
        K::ZenEtc
    }
}

/// `CWordParse::WhatKindOfTwoChars4KW`: the kind of two neighbours taken together, `Null` if
/// they are of different kinds.
pub fn two_chars_kind(pre: CharKind, cur: CharKind) -> CharKind {
    if pre == cur {
        return cur;
    }
    // A long vowel mark or a voiced sound mark follows the hiragana or katakana next to it.
    let nobasu = |k| k == K::ZenNobasu || k == K::ZenDaku;
    let kana = |k| k == K::ZenKata || k == K::Hira;
    if nobasu(pre) && kana(cur) {
        return cur;
    }
    if nobasu(cur) && kana(pre) {
        return pre;
    }
    if nobasu(pre) && nobasu(cur) {
        return cur;
    }
    let pre = if matches!(pre, K::Latin | K::Udef) {
        K::Csym
    } else {
        pre
    };
    let cur = if matches!(cur, K::Latin | K::Udef) {
        K::Csym
    } else {
        cur
    };
    if pre == cur { cur } else { K::Null }
}

/// The start of the character before `idx`.
pub fn prev_char_start(s: &str, idx: usize) -> usize {
    let mut i = idx.min(s.len());
    if i == 0 {
        return 0;
    }
    i -= 1;
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    // A variation selector belongs to the character before it.
    if is_variation_selector(s[i..].chars().next().unwrap_or(' ')) && i > 0 {
        let mut j = i - 1;
        while !s.is_char_boundary(j) {
            j -= 1;
        }
        if char_len_at(s, j) > s[j..].chars().next().map_or(0, char::len_utf8) {
            return j;
        }
    }
    i
}

/// `CWordParse::SearchNextWordPosition4KW` with both ends: where the word that starts at `start`
/// ends (the next change of kind, a run of spaces counting as one thing). `None` if `start` is
/// not inside the text.
pub fn next_word_break(s: &str, start: usize) -> Option<usize> {
    let mut kind = kind_at(s, start);
    let mut next = start;
    let mut step = char_len_at(s, next);
    while step > 0 {
        next += step;
        let kind_next = kind_at(s, next);
        if kind_next == K::Tab || kind_next == K::Space {
            if kind != kind_next {
                return Some(next);
            }
            kind = kind_next;
        } else {
            let merged = two_chars_kind(kind, kind_next);
            if merged == K::Null {
                return Some(next);
            }
            kind = merged;
        }
        step = char_len_at(s, next);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_of_ascii_characters() {
        assert_eq!(kind_at("a", 0), K::Csym);
        assert_eq!(kind_at("_", 0), K::Csym);
        assert_eq!(kind_at("7", 0), K::Csym);
        assert_eq!(kind_at("#", 0), K::Udef);
        assert_eq!(kind_at("\\", 0), K::Udef);
        assert_eq!(kind_at("+", 0), K::Etc);
        assert_eq!(kind_at(" ", 0), K::Space);
        assert_eq!(kind_at("\t", 0), K::Tab);
        assert_eq!(kind_at("\u{7f}", 0), K::Ctrl);
        assert_eq!(kind_at("a", 1), K::Null);
    }

    #[test]
    fn kinds_of_japanese_characters() {
        assert_eq!(kind_at("あ", 0), K::Hira);
        assert_eq!(kind_at("ア", 0), K::ZenKata);
        assert_eq!(kind_at("ｱ", 0), K::Kata);
        assert_eq!(kind_at("漢", 0), K::ZenEtc);
        assert_eq!(kind_at("ー", 0), K::ZenNobasu);
        assert_eq!(kind_at("、", 0), K::ZenKigo);
        assert_eq!(kind_at("Ａ", 0), K::ZenCsym);
        assert_eq!(kind_at("\u{3000}", 0), K::ZenSpace);
        assert_eq!(kind_at("é", 0), K::Latin);
        assert_eq!(kind_at("α", 0), K::Greek);
        assert_eq!(kind_at("Я", 0), K::ZenRos);
        assert_eq!(kind_at("─", 0), K::ZenSkigo);
        // CJK Extension B is full-width; other characters outside the BMP are half-width.
        assert_eq!(kind_at("𠮷", 0), K::ZenEtc);
        assert_eq!(kind_at("😀", 0), K::Etc);
    }

    #[test]
    fn a_variation_selector_goes_with_its_character() {
        let s = "葛\u{E0100}x";
        assert_eq!(char_len_at(s, 0), 3 + 4);
        assert_eq!(kind_at(s, 0), K::ZenEtc);
        assert_eq!(prev_char_start(s, 7), 0);
        assert_eq!(prev_char_start(s, 8), 7);
    }

    #[test]
    fn two_chars_of_the_same_word() {
        assert_eq!(two_chars_kind(K::Csym, K::Csym), K::Csym);
        assert_eq!(two_chars_kind(K::Latin, K::Csym), K::Csym);
        assert_eq!(two_chars_kind(K::Udef, K::Csym), K::Csym);
        assert_eq!(two_chars_kind(K::Csym, K::Etc), K::Null);
        assert_eq!(two_chars_kind(K::ZenNobasu, K::ZenKata), K::ZenKata);
        assert_eq!(two_chars_kind(K::Hira, K::ZenDaku), K::Hira);
        assert_eq!(two_chars_kind(K::Hira, K::ZenKata), K::Null);
    }

    #[test]
    fn word_breaks() {
        // `foo` ends where the space begins; the space and the word after it are the next steps.
        assert_eq!(next_word_break("foo bar", 0), Some(3));
        assert_eq!(next_word_break("foo bar", 3), Some(4));
        assert_eq!(next_word_break("foo bar", 4), Some(7));
        assert_eq!(next_word_break("a+b", 0), Some(1));
        assert_eq!(next_word_break("#define", 0), Some(7));
        assert_eq!(next_word_break("日本語です", 0), Some(9));
        assert_eq!(next_word_break("abc", 3), None);
    }

    #[test]
    fn keyword_characters() {
        for c in ['a', 'Z', '0', '_', '#', '$', '@', '\\'] {
            assert!(is_keyword_char(c), "{c}");
        }
        for c in [' ', '-', '.', 'あ', 'é'] {
            assert!(!is_keyword_char(c), "{c}");
        }
    }
}
