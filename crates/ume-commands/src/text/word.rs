// Sakura Editor's rules for words, ported to Rust.
//
// Source: Sakura Editor, commit 48db6c7c25e2d98bdf47baab201826b4da8c7303
//   https://github.com/sakura-editor/sakura
//   sakura_core/parse/CWordParse.cpp   WhatKindOfChar, WhatKindOfTwoChars, WhereCurrentWord_2,
//                                      SearchNextWordPosition, SearchPrevWordPosition
//   sakura_core/parse/CWordParse.h     ECharKind
//   sakura_core/charset/charcode.cpp   gm_keyword_char
//   sakura_core/charset/charcode.h     IsHankakuKatakana, IsZenkakuKigou, IsHiragana, ...
//   sakura_core/charset/codechecker.h  IsVariationSelector
//   sakura_core/mem/CNativeW.cpp       GetSizeOfChar, GetCharPrev
//   sakura_core/agent/CSearchAgent.cpp PrevOrNextWord (what it checks before the two searches)
//
// This file is derived from the files above and was modified: it is written in Rust, it works
// on `char`s instead of UTF-16 units, and it leaves out the parts for keywords and URLs.
//
// Copyright (C) 2007, kobake
// Copyright (C) 2008, kobake
// Copyright (C) 2018-2026, Sakura Editor Organization
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

//! What a *word* is in Sakura: a run of chars of one *kind* ([`CharKind`]).
//!
//! The functions work on one logical line as a slice of `char`s **with its line ending**, because
//! Sakura searches `GetDocLineStrWithEOL`: the line ending is a char of its own kind, so a word
//! never runs into it and a search stops in front of it. Indexes are `char` indexes.
//!
//! # Differences from Sakura
//!
//! * Sakura counts UTF-16 units; here a surrogate pair is one `char`. The kind of a char outside
//!   the BMP is `ZenEtc` for U+20000..=U+2FFFF (CJK extensions) and `Etc` for the rest, as in
//!   `WhatKindOfChar`. A BMP char followed by a variation selector (U+E0100..=U+E01EF, an IVS) is
//!   one unit of kind `ZenEtc`, as in Sakura. Stepping back goes one `char` at a time and does
//!   not know about IVS, as `GetCharPrev` does not.
//! * `IsHankaku` in Sakura measures the glyph in the font of the view. Here a char that no earlier
//!   rule has classified is `ZenEtc` if it is of East Asian full width (see `is_zenkaku`) and
//!   `Etc` otherwise. Kanji, hangul and the full-width forms come out the same as in Sakura with a
//!   Japanese font; symbols whose width depends on the font may differ.
//! * Sakura treats CR and LF as line ending chars, and NEL, LS and PS as well when
//!   `bEnableExtEol` is on (it is off by default). The slice does not say which setting made it,
//!   so NEL, LS and PS are line endings only as the last char of the slice (where the line ending
//!   of a line is) and ordinary chars elsewhere; they are given the kind of LF. A NEL, LS or PS
//!   that ends an unterminated last line is taken for a line ending though it is a char.
//! * The option "stop at both ends of a word" (`m_bStopsBothEndsWhenSearchWord`) is a parameter;
//!   its default in Sakura is off (`sakura_core/env/CShareData.cpp`).

/// The kind of a char (Sakura: `ECharKind` in `sakura_core/parse/CWordParse.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharKind {
    /// Nothing: past the end of the line, or a char that has no kind.
    Null,
    Tab,
    Cr,
    Lf,
    /// A control char other than tab, CR and LF.
    Ctrl,
    /// The half-width space.
    Space,
    /// Letters, digits and `_` (Sakura: chars of identifiers).
    Csym,
    /// Half-width katakana.
    Kata,
    /// Letters of Latin-1 Supplement and Latin Extended-A, without U+00D7 and U+00F7.
    Latin,
    /// `#`, `$`, `@` and `\`.
    Udef,
    /// Other half-width chars.
    Etc,
    /// The full-width space U+3000.
    ZenSpace,
    /// The prolonged sound mark U+30FC.
    ZenNobasu,
    /// The full-width voiced sound marks U+309B and U+309C.
    ZenDaku,
    /// Full-width letters, digits and `_`.
    ZenCsym,
    /// Full-width symbols.
    ZenKigo,
    Hira,
    ZenKata,
    Greek,
    /// Cyrillic.
    ZenRos,
    /// Box drawing chars.
    ZenSkigo,
    /// Other full-width chars (kanji and so on).
    ZenEtc,
}

/// Whether `c` ends a line (see "Differences from Sakura" in the [module documentation](self)).
pub fn is_line_delimiter(c: char) -> bool {
    matches!(c, '\r' | '\n' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// Whether the char `i` of `line` (a line with its line ending) is part of the line ending: CR or
/// LF anywhere, and NEL, LS or PS only as the last char, where the line ending of a line is.
pub fn is_line_ending_at(line: &[char], i: usize) -> bool {
    match line.get(i) {
        Some('\r' | '\n') => true,
        Some('\u{85}' | '\u{2028}' | '\u{2029}') => i + 1 == line.len(),
        _ => false,
    }
}

/// Whether `c` is an IVS variation selector.
fn is_variation_selector(c: char) -> bool {
    ('\u{E0100}'..='\u{E01EF}').contains(&c)
}

/// The number of `char`s of the unit that starts at `i`: 2 for a BMP char with an IVS selector,
/// 1 for anything else, 0 at the end (Sakura: `CNativeW::GetSizeOfChar`).
pub fn size_of_char(line: &[char], i: usize) -> usize {
    match line.get(i) {
        None => 0,
        Some(&c)
            if u32::from(c) <= 0xFFFF
                && line.get(i + 1).is_some_and(|&n| is_variation_selector(n)) =>
        {
            2
        }
        Some(_) => 1,
    }
}

/// Sakura's `gm_keyword_char` for the ASCII chars.
fn ascii_kind(c: char) -> CharKind {
    use CharKind::*;
    match c {
        '\t' => Tab,
        '\n' => Lf,
        '\r' => Cr,
        '\0'..='\x1F' | '\x7F' => Ctrl,
        ' ' => Space,
        '#' | '$' | '@' | '\\' => Udef,
        '0'..='9' | 'A'..='Z' | 'a'..='z' | '_' => Csym,
        _ => Etc,
    }
}

/// `WCODE::IsZenkakuKigou` (the table is the one in `charcode.h`, without the full-width space).
fn is_zenkaku_kigou(c: char) -> bool {
    const TABLE: &str = "、。，．・：；？！´｀¨＾￣＿〃―‐／＼～∥｜…‥‘’“”（）〔〕［］｛｝〈〉《》「」『』【】＋－±×÷＝≠＜＞≦≧∞∴♂♀°′″℃￥＄￠￡％＃＆＊＠§☆★○●◎◇◆□■△▲▽▼※〒→←↑↓〓∈∋⊆⊇⊂⊃∪∩∧∨￢⇒⇔∀∃∠⊥⌒∂∇≡≒≪≫√∽∝∵∫∬Å‰♯♭♪†‡¶◯";
    TABLE.contains(c)
}

/// Whether a BMP char that no earlier rule of [`char_kind`] caught is full-width. An approximation
/// of `IsHankaku` of Sakura, which asks the font (`sakura_core/charset/charcode.cpp`).
fn is_zenkaku(c: char) -> bool {
    matches!(u32::from(c),
        0x1100..=0x115F
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6)
}

/// The kind of one BMP char (`WhatKindOfChar` for a one-unit char).
fn bmp_kind(c: char) -> CharKind {
    use CharKind::*;
    let u = u32::from(c);
    if u < 0x80 {
        return ascii_kind(c);
    }
    if (0xFF61..=0xFF9F).contains(&u) {
        return Kata;
    }
    if (0x00C0..0x0180).contains(&u) && u != 0x00D7 && u != 0x00F7 {
        return Latin;
    }
    if u == 0x3000 {
        return ZenSpace;
    }
    if c == 'ー' {
        return ZenNobasu;
    }
    if c == '゛' || c == '゜' {
        return ZenDaku;
    }
    if c == '＿'
        || ('０'..='９').contains(&c)
        || ('Ａ'..='Ｚ').contains(&c)
        || ('ａ'..='ｚ').contains(&c)
    {
        return ZenCsym;
    }
    if is_zenkaku_kigou(c) {
        return ZenKigo;
    }
    if (0x3041..=0x3096).contains(&u) || (0x309D..=0x309E).contains(&u) {
        return Hira;
    }
    if (0x30A1..=0x30FA).contains(&u) || (0x30FD..=0x30FE).contains(&u) {
        return ZenKata;
    }
    if (0x0391..=0x03C9).contains(&u) {
        return Greek;
    }
    if (0x0400..=0x052F).contains(&u)
        || (0x2DE0..=0x2DFF).contains(&u)
        || (0xA640..=0xA69F).contains(&u)
    {
        return ZenRos;
    }
    if (0x2500..=0x257F).contains(&u) {
        return ZenSkigo;
    }
    if is_zenkaku(c) { ZenEtc } else { Etc }
}

/// The kind of the unit that starts at `i`; [`CharKind::Null`] past the end
/// (Sakura: `CWordParse::WhatKindOfChar`).
pub fn char_kind(line: &[char], i: usize) -> CharKind {
    match size_of_char(line, i) {
        0 => CharKind::Null,
        // a BMP char with an IVS selector
        2 => CharKind::ZenEtc,
        _ => {
            let c = line[i];
            if is_line_ending_at(line, i) {
                // CR and LF are in the ASCII table; the others are given the kind of LF
                return if c == '\r' {
                    CharKind::Cr
                } else {
                    CharKind::Lf
                };
            }
            if u32::from(c) > 0xFFFF {
                // outside the BMP: CJK extensions are full-width, the rest is not
                if (0x20000..=0x2FFFF).contains(&u32::from(c)) {
                    CharKind::ZenEtc
                } else {
                    CharKind::Etc
                }
            } else {
                bmp_kind(c)
            }
        }
    }
}

/// The kind of two neighbours taken together, or [`CharKind::Null`] if they are not one word
/// (Sakura: `CWordParse::WhatKindOfTwoChars`).
pub fn merge_kinds(pre: CharKind, cur: CharKind) -> CharKind {
    use CharKind::*;
    if pre == cur {
        return cur;
    }
    // The prolonged sound mark and the voiced sound marks go with the hiragana and katakana
    // before and after them.
    if matches!(pre, ZenNobasu | ZenDaku) && matches!(cur, ZenKata | Hira) {
        return cur;
    }
    if matches!(cur, ZenNobasu | ZenDaku) && matches!(pre, ZenKata | Hira) {
        return pre;
    }
    // a run of the two marks is one kind
    if matches!(pre, ZenNobasu | ZenDaku) && matches!(cur, ZenNobasu | ZenDaku) {
        return cur;
    }
    let normalize = |k| match k {
        Latin => Csym,
        Udef | Ctrl => Etc,
        k => k,
    };
    let (pre, cur) = (normalize(pre), normalize(cur));
    if pre == cur { cur } else { Null }
}

/// The range `(from, to)` of the word that contains the char at `idx`, or `None` if there is no
/// word there: past the end, or on a line ending (Sakura: `CWordParse::WhereCurrentWord_2`).
/// `to` is the index after the last char.
pub fn where_current_word(line: &[char], idx: usize) -> Option<(usize, usize)> {
    if idx >= line.len() || is_line_ending_at(line, idx) {
        return None;
    }
    let mut kind = char_kind(line, idx);

    // back until the kind changes
    let mut from = idx;
    while from > 0 {
        let merged = merge_kinds(char_kind(line, from - 1), kind);
        if merged == CharKind::Null {
            break;
        }
        kind = merged;
        from -= 1;
    }

    // forward until the kind changes
    let mut to = idx;
    let mut width = size_of_char(line, to);
    while width > 0 {
        to += width;
        let merged = merge_kinds(char_kind(line, to), kind);
        if merged == CharKind::Null {
            break;
        }
        kind = merged;
        width = size_of_char(line, to);
    }
    Some((from, to))
}

/// The start of the next word after `idx`: where the kind changes, skipping spaces and tabs
/// (Sakura: `CWordParse::SearchNextWordPosition`). With `stops_both_ends` it also stops where a
/// run of spaces or tabs starts. `None` if the line ends first.
pub fn search_next_word(line: &[char], idx: usize, stops_both_ends: bool) -> Option<usize> {
    use CharKind::{Space, Tab};
    let mut kind = char_kind(line, idx);
    let mut next = idx;
    let mut width = size_of_char(line, next);
    while width > 0 {
        next += width;
        let next_kind = char_kind(line, next);
        if matches!(next_kind, Tab | Space) {
            if stops_both_ends && kind != next_kind {
                return Some(next);
            }
            kind = next_kind;
        } else {
            let merged = merge_kinds(kind, next_kind);
            if merged == CharKind::Null {
                return Some(next);
            }
            kind = merged;
        }
        width = size_of_char(line, next);
    }
    None
}

/// The start of the word before `idx` (Sakura: `CWordParse::SearchPrevWordPosition`). Spaces and
/// tabs before `idx` are skipped. With `stops_both_ends` it stops at the end of the word before
/// the spaces. `None` if `idx` is 0.
pub fn search_prev_word(line: &[char], idx: usize, stops_both_ends: bool) -> Option<usize> {
    use CharKind::{Null, Space, Tab};
    let mut kind = char_kind(line, idx);
    if idx == 0 {
        return None;
    }
    let mut count = 0;
    let mut i = idx;
    while i > 0 {
        let after = i;
        i -= 1;
        let prev_kind = char_kind(line, i);
        let mut merged = merge_kinds(prev_kind, kind);
        if merged == Null {
            if matches!(kind, Tab | Space) {
                // started on (or has come to) spaces: the word before them ends here
                if stops_both_ends && count > 0 {
                    i = after;
                    break;
                }
                merged = prev_kind;
            } else if count == 0 {
                merged = prev_kind;
            } else {
                i = after;
                break;
            }
        }
        kind = merged;
        count += 1;
    }
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::CharKind::*;
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    /// The word at the `n`th char of `s`, as text.
    fn word_at(s: &str, n: usize) -> Option<String> {
        let line = chars(s);
        where_current_word(&line, n).map(|(a, b)| line[a..b].iter().collect())
    }

    fn next(s: &str, n: usize, both: bool) -> Option<usize> {
        search_next_word(&chars(s), n, both)
    }

    fn prev(s: &str, n: usize, both: bool) -> Option<usize> {
        search_prev_word(&chars(s), n, both)
    }

    #[test]
    fn kinds_of_ascii_chars_follow_gm_keyword_char() {
        // sakura_core/charset/charcode.cpp, gm_keyword_char
        let kind = |c| char_kind(&[c], 0);
        assert_eq!(kind('\t'), Tab);
        assert_eq!(kind('\n'), Lf);
        assert_eq!(kind('\r'), Cr);
        assert_eq!(kind('\x01'), Ctrl);
        assert_eq!(kind('\x7F'), Ctrl);
        assert_eq!(kind(' '), Space);
        for c in ['#', '$', '@', '\\'] {
            assert_eq!(kind(c), Udef, "{c}");
        }
        for c in ['0', '9', 'A', 'Z', 'a', 'z', '_'] {
            assert_eq!(kind(c), Csym, "{c}");
        }
        for c in [
            '!', '"', '%', '(', '-', '/', ':', '[', ']', '^', '`', '{', '|', '~',
        ] {
            assert_eq!(kind(c), Etc, "{c}");
        }
    }

    #[test]
    fn kinds_of_other_chars_follow_what_kind_of_char() {
        let kind = |c| char_kind(&[c], 0);
        assert_eq!(kind('ｱ'), Kata);
        assert_eq!(kind('é'), Latin);
        // not Latin, but it is in the table of symbols
        assert_eq!(kind('×'), ZenKigo);
        assert_eq!(kind('\u{3000}'), ZenSpace);
        assert_eq!(kind('ー'), ZenNobasu);
        assert_eq!(kind('゛'), ZenDaku);
        assert_eq!(kind('Ａ'), ZenCsym);
        assert_eq!(kind('７'), ZenCsym);
        assert_eq!(kind('＿'), ZenCsym);
        assert_eq!(kind('、'), ZenKigo);
        assert_eq!(kind('「'), ZenKigo);
        assert_eq!(kind('あ'), Hira);
        assert_eq!(kind('ゝ'), Hira);
        assert_eq!(kind('ア'), ZenKata);
        assert_eq!(kind('ヾ'), ZenKata);
        assert_eq!(kind('α'), Greek);
        assert_eq!(kind('Ж'), ZenRos);
        assert_eq!(kind('─'), ZenSkigo);
        assert_eq!(kind('漢'), ZenEtc);
        assert_eq!(kind('々'), ZenEtc);
        assert_eq!(kind('한'), ZenEtc);
        // outside the BMP: the CJK extensions are full-width, the rest is not
        assert_eq!(kind('\u{20000}'), ZenEtc);
        assert_eq!(kind('😀'), Etc);
    }

    #[test]
    fn a_bmp_char_with_an_ivs_selector_is_one_zen_etc_unit() {
        let line = chars("葛\u{E0100}a");
        assert_eq!(size_of_char(&line, 0), 2);
        assert_eq!(char_kind(&line, 0), ZenEtc);
        assert_eq!(size_of_char(&line, 2), 1);
        // the word is the unit and the kanji next to it
        assert_eq!(where_current_word(&line, 0), Some((0, 2)));
        // outside the BMP there is no IVS check
        assert_eq!(size_of_char(&chars("😀\u{E0100}"), 0), 1);
    }

    #[test]
    fn two_kinds_merge_like_what_kind_of_two_chars() {
        assert_eq!(merge_kinds(Csym, Csym), Csym);
        assert_eq!(merge_kinds(Latin, Csym), Csym);
        assert_eq!(merge_kinds(Csym, Latin), Csym);
        assert_eq!(merge_kinds(Udef, Etc), Etc);
        assert_eq!(merge_kinds(Ctrl, Etc), Etc);
        assert_eq!(merge_kinds(Udef, Csym), Null);
        assert_eq!(merge_kinds(ZenNobasu, ZenKata), ZenKata);
        assert_eq!(merge_kinds(Hira, ZenDaku), Hira);
        assert_eq!(merge_kinds(ZenNobasu, ZenDaku), ZenDaku);
        assert_eq!(merge_kinds(ZenNobasu, ZenEtc), Null);
        assert_eq!(merge_kinds(Hira, ZenKata), Null);
        assert_eq!(merge_kinds(Tab, Space), Null);
        assert_eq!(merge_kinds(Cr, Lf), Null);
    }

    #[test]
    fn the_word_around_an_ascii_char() {
        assert_eq!(word_at("hello world", 0).as_deref(), Some("hello"));
        assert_eq!(word_at("hello world", 4).as_deref(), Some("hello"));
        assert_eq!(word_at("hello world", 5).as_deref(), Some(" "));
        assert_eq!(word_at("hello world", 6).as_deref(), Some("world"));
        assert_eq!(word_at("foo_bar1", 3).as_deref(), Some("foo_bar1"));
        // a space and a tab are different kinds
        assert_eq!(word_at("a \tb", 1).as_deref(), Some(" "));
        assert_eq!(word_at("a \tb", 2).as_deref(), Some("\t"));
    }

    #[test]
    fn symbols_make_words_of_their_own() {
        assert_eq!(word_at("a+-b", 1).as_deref(), Some("+-"));
        assert_eq!(word_at("a+-b", 2).as_deref(), Some("+-"));
        // `#` is a user-defined char and does not join the identifier after it
        assert_eq!(word_at("#include", 0).as_deref(), Some("#"));
        assert_eq!(word_at("#include", 3).as_deref(), Some("include"));
        // but it does join the other symbols
        assert_eq!(word_at("#+", 0).as_deref(), Some("#+"));
    }

    #[test]
    fn japanese_words() {
        assert_eq!(word_at("こんにちはworld", 0).as_deref(), Some("こんにちは"));
        assert_eq!(word_at("漢字ひらがなカタカナ", 0).as_deref(), Some("漢字"));
        assert_eq!(
            word_at("漢字ひらがなカタカナ", 3).as_deref(),
            Some("ひらがな")
        );
        assert_eq!(
            word_at("漢字ひらがなカタカナ", 7).as_deref(),
            Some("カタカナ")
        );
        // the prolonged sound mark goes with the kana before and after it
        assert_eq!(word_at("コーヒーを", 2).as_deref(), Some("コーヒー"));
        assert_eq!(word_at("らーめん漢", 1).as_deref(), Some("らーめん"));
        assert_eq!(word_at("ーアー", 0).as_deref(), Some("ーアー"));
        // half-width katakana
        assert_eq!(word_at("ｱｲｳ abc", 1).as_deref(), Some("ｱｲｳ"));
        // punctuation is a kind of its own
        assert_eq!(word_at("あ、。い", 1).as_deref(), Some("、。"));
        // full-width letters and digits are not the half-width ones
        assert_eq!(
            word_at("ＡＢＣ１２３abc", 0).as_deref(),
            Some("ＡＢＣ１２３")
        );
        assert_eq!(word_at("ＡＢＣ１２３abc", 7).as_deref(), Some("abc"));
        assert_eq!(word_at("a　　b", 1).as_deref(), Some("　　"));
        assert_eq!(word_at("αβγ абв", 1).as_deref(), Some("αβγ"));
        assert_eq!(word_at("αβγ абв", 5).as_deref(), Some("абв"));
        // Latin letters with accents are identifier chars
        assert_eq!(word_at("naïve café", 2).as_deref(), Some("naïve"));
    }

    #[test]
    fn there_is_no_word_on_a_line_ending_or_past_the_end() {
        assert_eq!(word_at("ab\r\n", 1).as_deref(), Some("ab"));
        assert_eq!(word_at("ab\r\n", 2), None);
        assert_eq!(word_at("ab\r\n", 3), None);
        assert_eq!(word_at("ab\n", 2), None);
        assert_eq!(word_at("ab", 2), None);
        assert_eq!(word_at("", 0), None);
        // the line ending does not join a word
        assert_eq!(word_at("ab\n", 0).as_deref(), Some("ab"));
    }

    #[test]
    fn nel_ls_and_ps_are_line_endings_only_at_the_end_of_the_line() {
        // As the last char they are the line ending of the line.
        assert_eq!(word_at("ab\u{85}", 2), None);
        assert_eq!(word_at("ab\u{2028}", 1).as_deref(), Some("ab"));
        // Inside the line (the document does not count them as endings) they are chars.
        assert!(word_at("ab\u{85}cd", 2).is_some());
        assert!(is_line_ending_at(&chars("a\r\n"), 1));
        assert!(!is_line_ending_at(&chars("a\u{2029}b"), 1));
        assert!(is_line_ending_at(&chars("a\u{2029}"), 1));
    }

    #[test]
    fn next_word_skips_spaces() {
        assert_eq!(next("foo bar", 0, false), Some(4));
        assert_eq!(next("foo  \t bar", 1, false), Some(7));
        assert_eq!(next("foo+bar", 0, false), Some(3));
        assert_eq!(next("foo+bar", 3, false), Some(4));
        assert_eq!(next("日本語です", 0, false), Some(3));
        // the full-width space is a kind of its own and is not skipped
        assert_eq!(next("コーヒー　を", 0, false), Some(4));
    }

    #[test]
    fn next_word_stops_before_the_line_ending_or_at_the_end() {
        assert_eq!(next("foo\r\nbar", 0, false), Some(3));
        assert_eq!(next("foo \r\n", 0, false), Some(4));
        assert_eq!(next("foo", 0, false), Some(3));
        assert_eq!(next("foo", 1, false), Some(3));
        // nothing to search from the end
        assert_eq!(next("foo", 3, false), None);
    }

    #[test]
    fn next_word_stops_at_both_ends_when_asked() {
        assert_eq!(next("foo bar", 0, true), Some(3));
        assert_eq!(next("foo bar", 3, true), Some(4));
        assert_eq!(next("foo  bar", 3, true), Some(5));
        assert_eq!(next("foo \tbar", 3, true), Some(4));
    }

    #[test]
    fn prev_word_skips_spaces() {
        assert_eq!(prev("foo bar", 7, false), Some(4));
        assert_eq!(prev("foo bar", 5, false), Some(4));
        assert_eq!(prev("foo bar", 4, false), Some(0));
        assert_eq!(prev("foo  bar", 5, false), Some(0));
        assert_eq!(prev("foo bar\r\n", 7, false), Some(4));
        assert_eq!(prev("foo+bar", 4, false), Some(3));
        assert_eq!(prev("日本語です", 5, false), Some(3));
        assert_eq!(prev("日本語です", 3, false), Some(0));
        assert_eq!(prev("foo bar", 1, false), Some(0));
        assert_eq!(prev("foo", 0, false), None);
        assert_eq!(prev("", 0, false), None);
    }

    #[test]
    fn prev_word_stops_at_both_ends_when_asked() {
        // from the start of "bar": the end of "foo", not the start of it
        assert_eq!(prev("foo  bar", 5, true), Some(3));
        assert_eq!(prev("foo  bar", 3, true), Some(0));
        assert_eq!(prev("foo bar", 4, true), Some(3));
        // the first step never stops (count is 0)
        assert_eq!(prev("foo bar", 7, true), Some(4));
    }
}
