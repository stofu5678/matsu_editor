// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/convert/convert_util.cpp (the tables and the conversions)
//   sakura_core/convert/CConvert_ToHankaku.cpp, CConvert_ToZenkata.cpp, CConvert_ToZenhira.cpp,
//   CConvert_HankataToZenkata.cpp, CConvert_HankataToZenhira.cpp, CConvert_ZenkataToHankata.cpp,
//   CConvert_HaneisuToZeneisu.cpp, CConvert_ZeneisuToHaneisu.cpp (which of them to chain)
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

//! Width and kana conversions (`F_TOHANKAKU`, `F_TOZENKAKUKATA`, `F_TOZENKAKUHIRA`,
//! `F_HANKATATOZENKATA`, `F_HANKATATOZENHIRA`, `F_TOHANKATA`, `F_TOZENEI`, `F_TOHANEI`).
//!
//! The tables are Sakura's. Where a half-width table has the same char twice (`ｲ`, `ｴ`, `ﾜ`, `ｶ`,
//! `ｹ` stand in for `ヰ`, `ヱ`, `ヮ`, `ヵ`, `ヶ`), the first match wins, as in Sakura (`wcschr`).

// ---- tables (sakura_core/convert/convert_util.cpp) ----

/// Full-width katakana. `ヰ` and `ヱ` become `ｲ` and `ｴ`; the last three are `ヮ`, `ヵ`, `ヶ`.
const ZENKATA_NORMAL: &str = "アイウエオカキクケコサシスセソタチツテトナニヌネノ\
ハヒフヘホマミムメモヤユヨラリルレロワヰヱヲンァィゥェォッャュョ\u{30ee}\u{30f5}\u{30f6}";
const HANKATA_NORMAL: &str = "ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉ\
ﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜｲｴｦﾝｧｨｩｪｫｯｬｭｮﾜｶｹ";

/// With a dakuten. The last four are `ヷ`, `ヸ`, `ヹ`, `ヺ` (U+30F7-30FA).
const ZENKATA_DAKUTEN: &str =
    "ヴガギグゲゴザジズゼゾダヂヅデドバビブベボ\u{30f7}\u{30f8}\u{30f9}\u{30fa}";
const HANKATA_DAKUTEN: &str = "ｳｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾊﾋﾌﾍﾎﾜｲｴｦ";

const ZENKATA_HANDAKUTEN: &str = "パピプペポ";
const HANKATA_HANDAKUTEN: &str = "ﾊﾋﾌﾍﾎ";

const ZENKATA_CHO: &str = "ー";
const HANKATA_CHO: &str = "ｰ";

/// `゛゜` and the combining dakuten and handakuten.
const ZENKATA_DAKU: &str = "゛゜\u{3099}\u{309a}";
const HANKATA_DAKU: &str = "ﾞﾟﾞﾟ";

const ZENKATA_KIGO: &str = "。、「」・";
const HANKATA_KIGO: &str = "｡､｢｣･";

/// Full-width symbols that go with the half-width ones in [`HAN_KIGO`], index by index. The
/// backslash is mapped to `￥` and the space to the ideographic space.
const ZEN_KIGO: &str = "\u{3000}，．＋－＊／％＝｜＆＾￥＠；：”‘’＜＞（）｛｝［］！？＃＄￣＿";
const HAN_KIGO: &str = " ,.+-*/%=|&^\\@;:\"`'<>(){}[]!?#$~_";

const DAKUTEN: char = 'ﾞ';
const HANDAKUTEN: char = 'ﾟ';

/// The index of `c` in `table`, counted in chars (`wcschr_idx`).
fn idx(table: &str, c: char) -> Option<usize> {
    table.chars().position(|t| t == c)
}

/// The char at `i` of `table`. The tables of a pair have the same length (tested).
fn at(table: &str, i: usize) -> char {
    table.chars().nth(i).expect("tables of a pair are as long")
}

// ---- single chars ----

/// Hiragana to katakana (`ZenhiraToZenkata_`): `ぁ`-`ゖ` and `ゝ`-`ゞ`.
fn zenhira_to_zenkata_char(c: char) -> char {
    match c {
        'ぁ'..='\u{3096}' | 'ゝ'..='ゞ' => shift(c, 'ぁ', 'ァ'),
        _ => c,
    }
}

/// Katakana to hiragana (`ZenkataToZenhira_`): `ァ`-`ヴ` and `ヽ`-`ヾ`. `ヵ` and `ヶ` stay
/// (2012.06.17 syat in Sakura).
fn zenkata_to_zenhira_char(c: char) -> char {
    match c {
        'ァ'..='ヴ' | 'ヽ'..='ヾ' => shift(c, 'ァ', 'ぁ'),
        _ => c,
    }
}

/// `c - from + to`.
fn shift(c: char, from: char, to: char) -> char {
    char::from_u32(u32::from(c) - u32::from(from) + u32::from(to)).unwrap_or(c)
}

/// Full-width letters, digits and symbols to ASCII (`ZeneisuToHaneisu_`).
fn zeneisu_to_haneisu_char(c: char) -> char {
    match c {
        'Ａ'..='Ｚ' => shift(c, 'Ａ', 'A'),
        'ａ'..='ｚ' => shift(c, 'ａ', 'a'),
        '０'..='９' => shift(c, '０', '0'),
        _ => idx(ZEN_KIGO, c).map_or(c, |n| at(HAN_KIGO, n)),
    }
}

/// ASCII letters, digits and symbols to full width (`HaneisuToZeneisu_`).
fn haneisu_to_zeneisu_char(c: char) -> char {
    match c {
        'A'..='Z' => shift(c, 'A', 'Ａ'),
        'a'..='z' => shift(c, 'a', 'ａ'),
        '0'..='9' => shift(c, '0', '０'),
        _ => idx(HAN_KIGO, c).map_or(c, |n| at(ZEN_KIGO, n)),
    }
}

// ---- strings ----

/// Half-width letters, digits and some symbols to full width (`F_TOZENEI`). The length does not
/// change; the space becomes the ideographic space.
pub fn haneisu_to_zeneisu(text: &str) -> String {
    text.chars().map(haneisu_to_zeneisu_char).collect()
}

/// Full-width letters, digits and some symbols to half width (`F_TOHANEI`).
pub fn zeneisu_to_haneisu(text: &str) -> String {
    text.chars().map(zeneisu_to_haneisu_char).collect()
}

/// Full-width katakana to half-width katakana (`F_TOHANKATA`, `Convert_ZenkataToHankata`). A char
/// with a dakuten becomes two chars. A long vowel mark or a lone dakuten follows only if the char
/// before it was converted (2013.08.28 in Sakura).
pub fn zenkata_to_hankata(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // the previous char was a katakana, not counting the dakuten and handakuten
    let mut in_kata_normal = false;
    // the previous char was a katakana (any kind)
    let mut in_kata = false;
    for c in text.chars() {
        if let Some(n) = idx(ZENKATA_NORMAL, c) {
            out.push(at(HANKATA_NORMAL, n));
            (in_kata_normal, in_kata) = (true, true);
        } else if let Some(n) = idx(ZENKATA_DAKUTEN, c) {
            out.push(at(HANKATA_DAKUTEN, n));
            out.push(DAKUTEN);
            (in_kata_normal, in_kata) = (false, true);
        } else if let Some(n) = idx(ZENKATA_HANDAKUTEN, c) {
            out.push(at(HANKATA_HANDAKUTEN, n));
            out.push(HANDAKUTEN);
            (in_kata_normal, in_kata) = (false, true);
        } else if let Some(n) = idx(ZENKATA_CHO, c) {
            out.push(if in_kata { at(HANKATA_CHO, n) } else { c });
            in_kata_normal = false;
        } else if let Some(n) = idx(ZENKATA_DAKU, c) {
            out.push(if in_kata_normal {
                at(HANKATA_DAKU, n)
            } else {
                c
            });
            (in_kata_normal, in_kata) = (false, true);
        } else if let Some(n) = idx(ZENKATA_KIGO, c) {
            out.push(at(HANKATA_KIGO, n));
            (in_kata_normal, in_kata) = (false, false);
        } else {
            out.push(c);
            (in_kata_normal, in_kata) = (false, false);
        }
    }
    out
}

/// Everything that can be half width to half width (`F_TOHANKAKU`, `Convert_ToHankaku`):
/// full-width alphanumerics and symbols, hiragana and katakana, kana symbols.
pub fn to_hankaku(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let d = zeneisu_to_haneisu_char(c);
        if d != c {
            out.push(d);
            continue;
        }
        // The small `ゝ` and `ゞ` can become full-width katakana but have no half-width form, so
        // they stay (and so do the unassigned U+3097-3098 and `ゟ`). `ゕ` and `ゖ` and the
        // dakuten marks can be converted.
        if matches!(c, '\u{3097}' | '\u{3098}' | '\u{309d}'..='\u{309f}') {
            out.push(c);
            continue;
        }
        let c = zenhira_to_zenkata_char(c);
        if let Some(n) = idx(ZENKATA_NORMAL, c) {
            out.push(at(HANKATA_NORMAL, n));
        } else if let Some(n) = idx(ZENKATA_DAKUTEN, c) {
            out.push(at(HANKATA_DAKUTEN, n));
            out.push(DAKUTEN);
        } else if let Some(n) = idx(ZENKATA_HANDAKUTEN, c) {
            out.push(at(HANKATA_HANDAKUTEN, n));
            out.push(HANDAKUTEN);
        } else if let Some(n) = idx(ZENKATA_CHO, c) {
            out.push(at(HANKATA_CHO, n));
        } else if let Some(n) = idx(ZENKATA_DAKU, c) {
            out.push(at(HANKATA_DAKU, n));
        } else if let Some(n) = idx(ZENKATA_KIGO, c) {
            out.push(at(HANKATA_KIGO, n));
        } else {
            out.push(c);
        }
    }
    out
}

/// Half-width katakana to full-width katakana (`F_HANKATATOZENKATA`). A char followed by a
/// dakuten or handakuten that has a combined form becomes one char; the length may shrink.
pub fn hankata_to_zenkata(text: &str) -> String {
    hankata_to_zen(text, false)
}

/// Half-width katakana to full-width hiragana (`F_HANKATATOZENHIRA`). Full-width katakana that is
/// already in the text stays katakana.
pub fn hankata_to_zenhira(text: &str) -> String {
    hankata_to_zen(text, true)
}

/// The common part of `Convert_HankataToZenkata` and `Convert_HankataToZenhira`.
fn hankata_to_zen(text: &str, hiragana: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        // `ﾜ` and `ｦ` with a dakuten have a full-width katakana char, but no hiragana one: the
        // dakuten is converted by itself
        let split_wo = hiragana && matches!(c, 'ﾜ' | 'ｦ');
        let converted = if split_wo {
            Some(if c == 'ﾜ' { 'ワ' } else { 'ヲ' })
        } else if next == Some(DAKUTEN) && idx(HANKATA_DAKUTEN, c).is_some() {
            i += 1;
            idx(HANKATA_DAKUTEN, c).map(|n| at(ZENKATA_DAKUTEN, n))
        } else if next == Some(HANDAKUTEN) && idx(HANKATA_HANDAKUTEN, c).is_some() {
            i += 1;
            idx(HANKATA_HANDAKUTEN, c).map(|n| at(ZENKATA_HANDAKUTEN, n))
        } else if let Some(n) = idx(HANKATA_NORMAL, c) {
            Some(at(ZENKATA_NORMAL, n))
        } else if let Some(n) = idx(HANKATA_CHO, c) {
            Some(at(ZENKATA_CHO, n))
        } else if let Some(n) = idx(HANKATA_DAKU, c) {
            Some(at(ZENKATA_DAKU, n))
        } else {
            idx(HANKATA_KIGO, c).map(|n| at(ZENKATA_KIGO, n))
        };
        out.push(match converted {
            Some(k) if hiragana => zenkata_to_zenhira_char(k),
            Some(k) => k,
            None => c,
        });
        i += 1;
    }
    out
}

/// As much as possible to full-width katakana (`F_TOZENKAKUKATA`): half-width katakana, then
/// hiragana, then alphanumerics and symbols.
pub fn to_zenkata(text: &str) -> String {
    let kata = hankata_to_zenkata(text);
    haneisu_to_zeneisu(
        &kata
            .chars()
            .map(zenhira_to_zenkata_char)
            .collect::<String>(),
    )
}

/// As much as possible to full-width hiragana (`F_TOZENKAKUHIRA`): half-width katakana, then
/// katakana, then alphanumerics and symbols.
pub fn to_zenhira(text: &str) -> String {
    let kata = hankata_to_zenkata(text);
    haneisu_to_zeneisu(
        &kata
            .chars()
            .map(zenkata_to_zenhira_char)
            .collect::<String>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_of_a_pair_have_the_same_length() {
        for (a, b) in [
            (ZENKATA_NORMAL, HANKATA_NORMAL),
            (ZENKATA_DAKUTEN, HANKATA_DAKUTEN),
            (ZENKATA_HANDAKUTEN, HANKATA_HANDAKUTEN),
            (ZENKATA_CHO, HANKATA_CHO),
            (ZENKATA_DAKU, HANKATA_DAKU),
            (ZENKATA_KIGO, HANKATA_KIGO),
            (ZEN_KIGO, HAN_KIGO),
        ] {
            assert_eq!(a.chars().count(), b.chars().count(), "{a} {b}");
        }
    }

    #[test]
    fn alphanumerics_and_symbols() {
        // convert_util.cpp: tableHanKigo / tableZenKigo
        assert_eq!(haneisu_to_zeneisu("Az09 ,."), "Ａｚ０９　，．");
        assert_eq!(haneisu_to_zeneisu("\\~\"`'"), "￥￣”‘’");
        assert_eq!(zeneisu_to_haneisu("Ａｚ０９　，．"), "Az09 ,.");
        assert_eq!(zeneisu_to_haneisu("￥￣”‘’（）"), "\\~\"`'()");
        // kana and kanji are not touched
        assert_eq!(haneisu_to_zeneisu("ｱ日本"), "ｱ日本");
        assert_eq!(zeneisu_to_haneisu("ア日本。"), "ア日本。");
        // the C++ code works on UTF-16 units; a surrogate pair is never changed
        assert_eq!(haneisu_to_zeneisu("a😀"), "ａ😀");
    }

    #[test]
    fn zenkata_to_hankata_splits_the_dakuten() {
        assert_eq!(zenkata_to_hankata("アイウエオ"), "ｱｲｳｴｵ");
        assert_eq!(zenkata_to_hankata("ガギグ"), "ｶﾞｷﾞｸﾞ");
        assert_eq!(zenkata_to_hankata("ヴ"), "ｳﾞ");
        assert_eq!(zenkata_to_hankata("パピ"), "ﾊﾟﾋﾟ");
        assert_eq!(zenkata_to_hankata("\u{30f7}\u{30fa}"), "ﾜﾞｦﾞ");
        // ヰ ヱ ヮ ヵ ヶ use the same half-width char as イ エ ワ カ ケ
        assert_eq!(zenkata_to_hankata("ヰヱヮヵヶ"), "ｲｴﾜｶｹ");
        assert_eq!(zenkata_to_hankata("。、「」・"), "｡､｢｣･");
        assert_eq!(zenkata_to_hankata("ァィゥェォッャュョ"), "ｧｨｩｪｫｯｬｭｮ");
    }

    #[test]
    fn zenkata_to_hankata_long_vowel_and_loose_dakuten() {
        // 2013.08.28 in Sakura: the long vowel after a dakuten char is converted too
        assert_eq!(zenkata_to_hankata("ガー"), "ｶﾞｰ");
        assert_eq!(zenkata_to_hankata("カー"), "ｶｰ");
        // but not after a hiragana, a symbol or at the start
        assert_eq!(zenkata_to_hankata("あー"), "あー");
        assert_eq!(zenkata_to_hankata("。ー"), "｡ー");
        assert_eq!(zenkata_to_hankata("ー"), "ー");
        // a dakuten mark follows a plain katakana only
        assert_eq!(zenkata_to_hankata("カ゛カ゜"), "ｶﾞｶﾟ");
        assert_eq!(zenkata_to_hankata("カ\u{3099}"), "ｶﾞ");
        assert_eq!(zenkata_to_hankata("ガ゛"), "ｶﾞ゛");
        assert_eq!(zenkata_to_hankata("あ゛"), "あ゛");
    }

    #[test]
    fn hankata_to_zenkata_joins_the_dakuten() {
        assert_eq!(hankata_to_zenkata("ｱｲｳｴｵ"), "アイウエオ");
        assert_eq!(hankata_to_zenkata("ｶﾞｷﾞｸﾞ"), "ガギグ");
        assert_eq!(hankata_to_zenkata("ｳﾞ"), "ヴ");
        assert_eq!(hankata_to_zenkata("ﾊﾟﾋﾟ"), "パピ");
        // ﾜ ｲ ｴ ｦ have a dakuten form of their own (U+30F7-30FA)
        assert_eq!(
            hankata_to_zenkata("ﾜﾞｲﾞｴﾞｦﾞ"),
            "\u{30f7}\u{30f8}\u{30f9}\u{30fa}"
        );
        // a dakuten after a char that has no such form is converted alone
        assert_eq!(hankata_to_zenkata("ｱﾞ"), "ア゛");
        assert_eq!(hankata_to_zenkata("ﾞﾟ"), "゛゜");
        assert_eq!(hankata_to_zenkata("ｶﾟ"), "カ゜");
        assert_eq!(hankata_to_zenkata("ｰ｡､｢｣･"), "ー。、「」・");
        // others stay
        assert_eq!(hankata_to_zenkata("ABC あ カ 日"), "ABC あ カ 日");
    }

    #[test]
    fn hankata_to_zenhira_leaves_full_width_katakana_alone() {
        assert_eq!(hankata_to_zenhira("ｱｲｳｴｵ"), "あいうえお");
        assert_eq!(hankata_to_zenhira("ｶﾞﾊﾟ"), "がぱ");
        assert_eq!(hankata_to_zenhira("ｳﾞ"), "ゔ");
        assert_eq!(hankata_to_zenhira("ｯｬ"), "っゃ");
        // convert_util.cpp: ﾜﾞ and ｦﾞ cannot be hiragana, so the dakuten is cut off
        assert_eq!(hankata_to_zenhira("ﾜﾞｦﾞ"), "わ゛を゛");
        assert_eq!(hankata_to_zenhira("ﾜｦ"), "わを");
        // ヸ (U+30F8) is outside the katakana-to-hiragana range
        assert_eq!(hankata_to_zenhira("ｲﾞ"), "\u{30f8}");
        // the long vowel and the symbols have no hiragana form
        assert_eq!(hankata_to_zenhira("ｰ｡｢"), "ー。「");
        // already full width: unchanged
        assert_eq!(hankata_to_zenhira("カｶ"), "カか");
    }

    #[test]
    fn to_hankaku_converts_what_it_can() {
        assert_eq!(to_hankaku("ＡＢＣ１２３　！"), "ABC123 !");
        assert_eq!(to_hankaku("ひらがな"), "ﾋﾗｶﾞﾅ");
        assert_eq!(to_hankaku("カタカナ"), "ｶﾀｶﾅ");
        assert_eq!(to_hankaku("ゔヴ"), "ｳﾞｳﾞ");
        assert_eq!(to_hankaku("ぱガ"), "ﾊﾟｶﾞ");
        assert_eq!(to_hankaku("ゕゖ"), "ｶｹ");
        assert_eq!(to_hankaku("゛゜"), "ﾞﾟ");
        assert_eq!(to_hankaku("ー。、「」・"), "ｰ｡､｢｣･");
        // ゝゞ have no half-width form; ヽヾ are not in the tables
        assert_eq!(to_hankaku("ゝゞヽヾ"), "ゝゞヽヾ");
        // kanji, half width and surrogate pairs stay
        assert_eq!(to_hankaku("日本語ｱ😀\u{10400}"), "日本語ｱ😀\u{10400}");
    }

    #[test]
    fn to_zenkata_chains_three_conversions() {
        // half-width kana, hiragana and alphanumerics (the space too) become full width katakana
        assert_eq!(to_zenkata("ｶﾞ ひらがな ab1"), "ガ　ヒラガナ　ａｂ１");
        assert_eq!(to_zenkata("ゔゝゞ"), "ヴヽヾ");
        assert_eq!(to_zenkata("日本\r\n語"), "日本\r\n語");
    }

    #[test]
    fn to_zenhira_chains_three_conversions() {
        assert_eq!(to_zenhira("ｶﾞ ヒラ ab1"), "が　ひら　ａｂ１");
        assert_eq!(to_zenhira("ヴヽヾ"), "ゔゝゞ");
        // ヵ and ヶ stay (2012.06.17 syat in Sakura)
        assert_eq!(to_zenhira("ヵヶ"), "ヵヶ");
        assert_eq!(to_zenhira("ｰ"), "ー");
    }
}
