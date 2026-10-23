// Sakura Editor's input completion (入力補完), ported to Rust.
//
// Source: Sakura Editor, commit 48db6c7c25e2d98bdf47baab201826b4da8c7303
//   https://github.com/sakura-editor/sakura
//   sakura_core/view/CEditView_CmdHokan.cpp   CEditView::HokanSearchByFile
//   sakura_core/view/CViewParser.cpp          CViewParser::GetLeftWord
//   sakura_core/env/CHokanMgr.cpp             CHokanMgr::Search, HokanSearchByKeyword (the limit,
//                                             the unique list, the keywords of the type)
//   sakura_core/env/CDicMgr.cpp               CDicMgr::HokanSearch (the word file)
//   sakura_core/io/CTextStream.cpp            CTextInputStream (how the word file is read)
//
// This file is derived from the files above and was modified: it is written in Rust, it works on
// `char`s instead of UTF-16 units and on lines given by the caller instead of the document of the
// view, and it leaves out the plugins.
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2000, jepro, genta
// Copyright (C) 2001, asa-o
// Copyright (C) 2002, YAZAKI
// Copyright (C) 2003, Moca, KEITA
// Copyright (C) 2004, genta, Moca, novice
// Copyright (C) 2005, genta
// Copyright (C) 2007, ryoji
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

//! The words that input completion offers: the word before the caret (the *key*), and the words of
//! the document that start with it (the *candidates*).
//!
//! Sakura's help (`help/sakura/res/HLP000104.html`) says: the candidates are the words of the
//! file; a symbol is in principle not part of a candidate (except `_`, and the full-width `ヽヾゝゞ〃仝々〆〇ー`);
//! a word ends where the kind of char changes, but a hiragana stays with the word before it.
//!
//! # Differences from Sakura
//!
//! * Lengths are counted in `char`s, not UTF-16 units (the limit of a word, 1020, and the length
//!   of the key).
//! * The case-insensitive comparison (`wmemicmp`) compares the lowercase of each char.
//! * The plugins (`nHokanType`) are left out. The word file, the words of the document and the
//!   highlighted keywords of the type are searched by [`word_file_candidates`], [`candidates`] and
//!   [`keyword_candidates`], and put in Sakura's order by [`merge_candidates`].

use matsu_core::encoding::Encoding;

use super::word::{CharKind, char_kind, merge_kinds, size_of_char, where_current_word};

/// The most candidates taken from the document (`HokanSearchByFile` is called with 1024 in
/// `CHokanMgr::Search`: "the data being edited, so the number is limited").
pub const MAX_CANDIDATES: usize = 1024;

/// The longest word that is a candidate (`CDicMgr` limits the length of an entry).
const MAX_WORD_LEN: usize = 1020;

/// Chars that start a word of their own kind: `$@#\` (`wcschr(L"$@#\\", c)`).
fn is_mark(c: char) -> bool {
    matches!(c, '$' | '@' | '#' | '\\')
}

/// `IS_KEYWORD_CHAR`: the ASCII chars of the kind `Csym` or `Udef` (letters, digits, `_`, `$@#\`).
fn is_keyword_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || is_mark(c)
}

/// The word before the caret, the key of the completion (`CViewParser::GetLeftWord`): the part of
/// the word before the caret that holds the char before the caret. `line` is the logical line and
/// `caret` the index of the char at the caret. `None` if there is no key: the caret is at the
/// start of the line, or after a space, a tab or a full-width space.
///
/// Sakura looks at the line of the layout, and at the start of a row goes to the row before; here
/// the rows are not looked at, so at the start of a logical line there is no key (the line before
/// ends with a line ending, where Sakura also stops).
pub fn left_word(line: &[char], caret: usize) -> Option<String> {
    let caret = caret.min(line.len());
    let prev = caret.checked_sub(1)?;
    // `IsWordDelimiter`: a space, a tab or the full-width space is not part of a word
    if matches!(line[prev], ' ' | '\t' | '\u{3000}') {
        return None;
    }
    let (from, _) = where_current_word(line, 0, prev)?;
    Some(line[from..caret].iter().collect())
}

/// Whether two chars are equal, ignoring case if asked (`wmemcmp` and `wmemicmp`).
fn same_char(a: char, b: char, ignore_case: bool) -> bool {
    a == b || (ignore_case && a.to_lowercase().eq(b.to_lowercase()))
}

/// Finds the words of the document that start with `key`, in the order of the document, without
/// duplicates, at most [`MAX_CANDIDATES`]. `line_at(i)` gives line `i` without its line ending
/// (`None` past the last line). `caret` is the line and the char index of the caret: the word the
/// caret is in (or touches the end of) is not a candidate, so a word being typed does not offer
/// itself.
///
/// Sakura: `CEditView::HokanSearchByFile`.
pub fn candidates(
    mut line_at: impl FnMut(usize) -> Option<String>,
    key: &str,
    ignore_case: bool,
    caret: (usize, usize),
) -> Vec<String> {
    let key: Vec<char> = key.chars().collect();
    let mut found: Vec<String> = Vec::new();
    let Some(&key_first) = key.first() else {
        return found;
    };
    let key_starts_with_mark = is_mark(key_first);

    let mut line_no = 0;
    while let Some(text) = line_at(line_no) {
        // `line_at` gives the line without its ending, so only a stray CR or LF is dropped; NEL, LS
        // and PS are chars of the line unless the document counts them as endings, and then they
        // are not in `text`.
        let line: Vec<char> = text
            .chars()
            .filter(|&c| !matches!(c, '\r' | '\n'))
            .collect();
        let len = line.len();
        let mut j = 0;
        let mut width;
        while j < len {
            width = size_of_char(&line, j);
            let first = line[j];

            // a half-width symbol is not part of a candidate
            if u32::from(first) < 0xC0 && !is_keyword_char(first) {
                j += width;
                continue;
            }
            // a word that starts with a mark is a candidate only if the key starts with one
            if !key_starts_with_mark && is_mark(first) {
                j += width;
                continue;
            }
            let mut kind_pre = char_kind(&line, 0, j);
            // a full-width symbol is not part of a candidate
            if matches!(
                kind_pre,
                CharKind::ZenSpace
                    | CharKind::ZenNobasu
                    | CharKind::ZenDaku
                    | CharKind::ZenKigo
                    | CharKind::ZenSkigo
            ) {
                j += width;
                continue;
            }
            let word_starts_with_mark = is_mark(first);

            let word_begin = j;
            let mut word_len = width;
            // the end of the word without the okurigana (the hiragana after a kanji are kept,
            // those after anything else cut off); `None` is "not set"
            let mut word_len_stop: Option<usize> = None;
            // Whether the loop below stopped at a char that Sakura steps over (see after it).
            let mut step_over = true;
            j += width;
            while j < len {
                width = size_of_char(&line, j);
                let c = line[j];
                if u32::from(c) < 0xC0 && !is_keyword_char(c) {
                    break;
                }
                let kind_cur = char_kind(&line, 0, j);
                if matches!(
                    kind_cur,
                    CharKind::ZenSpace | CharKind::ZenKigo | CharKind::ZenSkigo
                ) {
                    break;
                }
                let mut merged = merge_kinds(kind_pre, kind_cur);
                if merged == CharKind::Null {
                    if kind_cur == CharKind::Hira {
                        // a hiragana goes on; only a kanji before it makes it part of the word
                        merged = kind_cur;
                        if kind_pre != CharKind::ZenEtc {
                            word_len_stop = Some(word_len);
                        }
                    } else if key_starts_with_mark
                        && word_starts_with_mark
                        && kind_pre == CharKind::Udef
                    {
                        // a word that starts with a mark is not held to the kinds
                        merged = kind_cur;
                    } else {
                        // the kind changes: the word ends, and the next one starts here
                        step_over = false;
                        break;
                    }
                }
                kind_pre = merged;
                word_len += width;
                j += width;
            }
            // Sakura's loops step over the char they stopped at (`for (...; j += nCharSize)`):
            // the char is a symbol or a space in the cases that matter, which starts no word.
            // Where the kind changed, Sakura steps back first, so the next word starts there.
            if step_over {
                j += width;
            }

            if let Some(stop) = word_len_stop.filter(|&s| s > 0) {
                word_len = stop;
            }
            if word_len > MAX_WORD_LEN || key.len() > word_len {
                continue;
            }
            let word = &line[word_begin..word_begin + word_len];
            if !key
                .iter()
                .zip(word)
                .all(|(&k, &w)| same_char(k, w, ignore_case))
            {
                continue;
            }
            // the word at the caret is left out
            let (caret_line, caret_idx) = caret;
            if caret_line == line_no
                && word_begin <= caret_idx
                && caret_idx <= word_begin + word_len
            {
                continue;
            }
            let word: String = word.iter().collect();
            if !found.contains(&word) {
                found.push(word);
            }
            if found.len() >= MAX_CANDIDATES {
                return found;
            }
        }
        line_no += 1;
    }
    found
}

/// The lines of a word file (`CTextInputStream`): UTF-8 when the file starts with the BOM,
/// Shift_JIS otherwise; a line ends with CRLF, LF or CR (the ending is not part of the line).
pub fn word_file_lines(bytes: &[u8]) -> Vec<String> {
    let (bytes, encoding) = match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => (rest, Encoding::Utf8),
        None => (bytes, Encoding::ShiftJis),
    };
    let mut text = String::new();
    encoding.codec().decode(bytes, &mut text);
    let mut lines = Vec::new();
    let mut rest = text.as_str();
    while !rest.is_empty() {
        let end = rest.find(['\r', '\n']).unwrap_or(rest.len());
        lines.push(rest[..end].to_owned());
        let after = &rest[end..];
        rest = after
            .strip_prefix("\r\n")
            .or_else(|| after.strip_prefix('\r'))
            .or_else(|| after.strip_prefix('\n'))
            .unwrap_or(after);
    }
    lines
}

/// The lines of the word file that start with `key`, in the order of the file, as they are
/// (a word may be there twice). A line that starts with `;` is a comment, and a line shorter than
/// the key cannot match.
///
/// Sakura: `CDicMgr::HokanSearch`.
pub fn word_file_candidates(lines: &[String], key: &str, ignore_case: bool) -> Vec<String> {
    let key: Vec<char> = key.chars().collect();
    lines
        .iter()
        .filter(|line| !line.starts_with(';') && starts_with_key(line, &key, ignore_case))
        .cloned()
        .collect()
}

/// The words that start with `key`, once each, in the order they are given.
///
/// Sakura: `CHokanMgr::HokanSearchByKeyword`, which is given the words of the keyword sets of the
/// type in the order of the slots.
pub fn keyword_candidates<'a>(
    words: impl IntoIterator<Item = &'a str>,
    key: &str,
    ignore_case: bool,
) -> Vec<String> {
    let key: Vec<char> = key.chars().collect();
    let mut found: Vec<String> = Vec::new();
    for word in words {
        if starts_with_key(word, &key, ignore_case) && !found.iter().any(|w| w == word) {
            found.push(word.to_owned());
        }
    }
    found
}

/// Adds `more` to `found` without the words `found` has (`vector_ex::push_back_unique`). With a
/// `limit`, nothing is added once `found` has that many words (the search of the document stops at
/// [`MAX_CANDIDATES`] in all, the word file and the keywords have no limit).
pub fn merge_candidates(
    mut found: Vec<String>,
    more: Vec<String>,
    limit: Option<usize>,
) -> Vec<String> {
    for word in more {
        if limit.is_some_and(|limit| found.len() >= limit) {
            break;
        }
        if !found.contains(&word) {
            found.push(word);
        }
    }
    found
}

/// Whether `word` starts with `key` (`wmemcmp` and `wmemicmp` over the length of the key).
fn starts_with_key(word: &str, key: &[char], ignore_case: bool) -> bool {
    let mut chars = word.chars();
    key.iter()
        .all(|&k| chars.next().is_some_and(|w| same_char(k, w, ignore_case)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    /// The candidates for the key before `|` in `text` (lines are separated by `\n`).
    fn complete(text: &str, ignore_case: bool) -> Vec<String> {
        let (line_no, idx, lines) = split_caret(text);
        let key = left_word(&chars(&lines[line_no]), idx).expect("a key");
        candidates(|i| lines.get(i).cloned(), &key, ignore_case, (line_no, idx))
    }

    fn split_caret(text: &str) -> (usize, usize, Vec<String>) {
        let mut caret = (0, 0);
        let mut lines = Vec::new();
        for (n, l) in text.split('\n').enumerate() {
            if let Some(at) = l.find('|') {
                caret = (n, l[..at].chars().count());
            }
            lines.push(l.replacen('|', "", 1));
        }
        (caret.0, caret.1, lines)
    }

    fn key(text: &str) -> Option<String> {
        let (n, idx, lines) = split_caret(text);
        left_word(&chars(&lines[n]), idx)
    }

    #[test]
    fn the_key_is_the_part_of_the_word_before_the_caret() {
        assert_eq!(key("foo qu|x"), Some("qu".into()));
        assert_eq!(key("foo bar|"), Some("bar".into()));
        assert_eq!(key("foo_bar1|"), Some("foo_bar1".into()));
        // the kinds of char make the word: kanji, hiragana and katakana are separate words
        assert_eq!(key("漢字かな|"), Some("かな".into()));
        assert_eq!(key("カタカナ漢|字"), Some("漢".into()));
        // the prolonged sound mark goes with the katakana before it
        assert_eq!(key("コーヒー|"), Some("コーヒー".into()));
    }

    #[test]
    fn there_is_no_key_after_a_space_or_at_the_start_of_a_line() {
        assert_eq!(key("foo |"), None);
        assert_eq!(key("foo\t|"), None);
        assert_eq!(key("foo\u{3000}|"), None);
        assert_eq!(key("|foo"), None);
        assert_eq!(key("abc\n|def"), None);
        assert_eq!(key("|"), None);
    }

    #[test]
    fn a_symbol_before_the_caret_is_the_key() {
        // Sakura only refuses the spaces; a symbol is a word of its own kind
        assert_eq!(key("a.|"), Some(".".into()));
        assert_eq!(key("a(|"), Some("(".into()));
    }

    #[test]
    fn the_candidates_start_with_the_key_in_the_order_of_the_text() {
        let got = complete("apple apricot banana\napple avocado ap|", false);
        assert_eq!(got, ["apple", "apricot"]);
    }

    #[test]
    fn the_word_at_the_caret_is_not_a_candidate() {
        // only "apple" elsewhere: "ap|" itself is the word at the caret
        assert_eq!(complete("apple ap|", false), ["apple"]);
        assert_eq!(complete("ap|ple", false), Vec::<String>::new());
        // the caret inside another word with the same start
        assert_eq!(complete("apple\napp|", false), ["apple"]);
    }

    #[test]
    fn a_word_is_listed_once() {
        assert_eq!(complete("abc abc abcd abc\nab|", false), ["abc", "abcd"]);
    }

    #[test]
    fn case_matters_unless_asked() {
        let text = "Apple apple APPLE\nap|";
        assert_eq!(complete(text, false), ["apple"]);
        assert_eq!(complete(text, true), ["Apple", "apple", "APPLE"]);
    }

    #[test]
    fn no_candidate() {
        assert!(complete("apple\nzz|", false).is_empty());
    }

    #[test]
    fn one_candidate() {
        assert_eq!(complete("hello\nhe|", false), ["hello"]);
    }

    #[test]
    fn japanese_words_split_where_the_kind_changes() {
        // 漢字 (kanji) | ひらがな (hiragana) | カタカナ (katakana)
        let text = "漢字ひらがな カタカナ 漢字\n漢|";
        // a hiragana after a kanji stays with the word
        assert_eq!(complete(text, false), ["漢字ひらがな", "漢字"]);
        let text = "カタカナ漢字\nカタ|";
        assert_eq!(complete(text, false), ["カタカナ"]);
    }

    #[test]
    fn hiragana_after_something_else_than_a_kanji_is_cut_off() {
        // "abcあい": the hiragana are not part of the candidate
        assert_eq!(complete("abcあい\nab|", false), ["abc"]);
        // a word of hiragana only is whole
        assert_eq!(complete("あいうえお\nあい|", false), ["あいうえお"]);
    }

    #[test]
    fn symbols_end_a_word_and_are_not_candidates() {
        assert_eq!(complete("foo.bar(baz)\nba|", false), ["bar", "baz"]);
        // full-width symbols too
        assert_eq!(complete("名前、値。\n名|", false), ["名前"]);
    }

    #[test]
    fn words_that_start_with_a_mark_need_a_key_with_a_mark() {
        // "$name" is offered for "$" (a word that starts with a mark may go on with other kinds),
        // but for "na" the candidate is "name", not "$name"
        assert_eq!(complete("$name name\n$|", false), ["$name"]);
        assert_eq!(complete("$name name\nna|", false), ["name"]);
        // the mark is a word of its own, not part of the key
        assert_eq!(key("$na|"), Some("na".into()));
    }

    #[test]
    fn a_nel_inside_a_line_is_not_dropped() {
        // With `bEnableExtEol` off it is a char of the line: "foo" and "bar" stay two words.
        let lines = ["foo\u{85}bar".to_owned(), "fo".to_owned()];
        let got = candidates(|i| lines.get(i).cloned(), "fo", false, (1, 2));
        assert_eq!(got, ["foo"]);
    }

    #[test]
    fn the_candidates_stop_at_the_limit() {
        let text: String = (0..2000).map(|i| format!("w{i} ")).collect();
        let lines = [text, "w|".to_owned()];
        let got = candidates(
            |i| lines.get(i).map(|l| l.replace('|', "")),
            "w",
            false,
            (1, 1),
        );
        assert_eq!(got.len(), MAX_CANDIDATES);
        assert_eq!(got[0], "w0");
    }

    #[test]
    fn a_word_that_ends_before_a_char_with_a_variation_selector_does_not_underflow() {
        // "漢" + IVS is one unit of two chars; the ASCII word before it ends at its start
        let got = candidates(
            |i| (i == 0).then(|| "a漢\u{E0100} ab".to_owned()),
            "a",
            false,
            (1, 0),
        );
        assert_eq!(got, ["a", "ab"]);
    }

    #[test]
    fn a_line_ending_does_not_join_words() {
        assert_eq!(complete("foo\r\nfoo2\r\nfo|", false), ["foo", "foo2"]);
    }

    #[test]
    fn a_word_longer_than_the_limit_is_left_out() {
        let long = "a".repeat(MAX_WORD_LEN + 1);
        let text = format!("{long} abc\na|");
        assert_eq!(complete(&text, false), ["abc"]);
    }

    #[test]
    fn a_word_file_is_read_as_utf8_with_a_bom_and_as_shift_jis_without() {
        let mut utf8 = vec![0xEF, 0xBB, 0xBF];
        utf8.extend_from_slice("alpha\r\nあいう\nbeta\rgamma".as_bytes());
        assert_eq!(word_file_lines(&utf8), ["alpha", "あいう", "beta", "gamma"]);
        // "あ" in Shift_JIS is 82 A0
        assert_eq!(word_file_lines(b"ab\r\n\x82\xA0c\r\n"), ["ab", "あc"]);
        assert!(word_file_lines(b"").is_empty());
        // an empty line is a line
        assert_eq!(word_file_lines(b"a\r\n\r\nb"), ["a", "", "b"]);
    }

    #[test]
    fn the_word_file_gives_its_lines_that_start_with_the_key() {
        let lines: Vec<String> = [
            "apple",
            "; apple pie",
            "Apricot",
            "ap",
            "apple",
            "banana",
            "",
        ]
        .map(str::to_owned)
        .to_vec();
        // the order of the file, a comment is not a word, a word twice is listed twice
        assert_eq!(
            word_file_candidates(&lines, "ap", false),
            ["apple", "ap", "apple"]
        );
        assert_eq!(
            word_file_candidates(&lines, "AP", true),
            ["apple", "Apricot", "ap", "apple"]
        );
        // a line as long as the key matches; a shorter one cannot
        assert_eq!(
            word_file_candidates(&lines, "apple", false),
            ["apple", "apple"]
        );
        assert!(word_file_candidates(&lines, "apples", false).is_empty());
        // a key that starts with `;` finds nothing in a comment
        assert!(word_file_candidates(&lines, ";", false).is_empty());
    }

    #[test]
    fn the_keywords_are_listed_once_and_case_follows_the_setting() {
        let words = ["if", "else", "Iffy", "if", "ifdef"];
        assert_eq!(keyword_candidates(words, "if", false), ["if", "ifdef"]);
        assert_eq!(
            keyword_candidates(words, "IF", true),
            ["if", "Iffy", "ifdef"]
        );
        assert!(keyword_candidates(words, "x", false).is_empty());
    }

    #[test]
    fn the_sources_are_merged_in_sakuras_order_without_repeats() {
        let file = vec!["foo".to_owned(), "foo".to_owned(), "fa".to_owned()];
        let document = vec!["fa".to_owned(), "fob".to_owned()];
        let merged = merge_candidates(file, document, Some(MAX_CANDIDATES));
        // the word file is kept as it is; the document adds what is new
        assert_eq!(merged, ["foo", "foo", "fa", "fob"]);
        let merged = merge_candidates(merged, vec!["fob".to_owned(), "for".to_owned()], None);
        assert_eq!(merged, ["foo", "foo", "fa", "fob", "for"]);
        // the limit of the document counts everything; the keywords have none
        let full: Vec<String> = (0..MAX_CANDIDATES).map(|i| format!("w{i}")).collect();
        let more = vec!["x".to_owned()];
        assert_eq!(
            merge_candidates(full.clone(), more.clone(), Some(MAX_CANDIDATES)),
            full
        );
        assert_eq!(merge_candidates(full, more, None).len(), MAX_CANDIDATES + 1);
    }
}
