// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CDicMgr.cpp (CDicMgr::Search), sakura_core/view/CEditView_Search.cpp
//   (CEditView::KeySearchCore), sakura_core/window/CTipWnd.cpp (UnEscapeInfoText)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, jepro, genta
//   Copyright (C) 2001, asa-o
//   Copyright (C) 2002, GAE, aroka, Moca, genta
//   Copyright (C) 2003, Moca
//   Copyright (C) 2005, D.S.Koba, genta
//   Copyright (C) 2006, fon, ryoji
//   Copyright (C) 2007, kobake, ryoji
//   Copyright (C) 2008, kobake
//   Copyright (C) 2011, 2012, Moca
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

//! The lookup of the keyword help: a word in the dictionaries of the type, and the text of the tip.
//!
//! A dictionary is a text file. A line `key1,key2 /// meaning` gives the meaning of the keys (the
//! delimiter is exactly " /// "); a line that starts with `;` is a comment and a line without the
//! delimiter says nothing. [`search_lines`] finds the first line with a key that equals the word,
//! ignoring case (`CDicMgr::Search`); [`key_search`] goes through the dictionaries of the type
//! and builds the text of the tip as the settings of the type say (`CEditView::KeySearchCore`).

use std::sync::Arc;

use ume_config::settings::KeyHelpDict;

/// `STRNCMP_MAX`: how many characters of the word are compared (`CEditView_Search.cpp`).
pub(crate) const STRNCMP_MAX: usize = 100;

/// The delimiter between the keys and the meaning (`pszDelimit`).
const DELIMITER: &str = " /// ";

/// The line of a dictionary where the word hit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Hit {
    /// The key of the line that matched, as the line has it.
    pub key: String,
    /// The meaning: what follows the delimiter, up to the end of the line.
    pub meaning: String,
    /// The number of the line, from 1 (`pLine`).
    pub line: usize,
}

/// `towlower` of one UTF-16 unit; a unit that is part of a surrogate pair is left as it is.
fn lower(unit: u16) -> u16 {
    match char::from_u32(u32::from(unit)) {
        Some(c) => {
            let mut lowered = c.to_lowercase();
            match (lowered.next(), lowered.next()) {
                (Some(l), None) => u16::try_from(u32::from(l)).unwrap_or(unit),
                _ => unit,
            }
        }
        None => unit,
    }
}

/// `_wcsnicmp( a, b, n ) == 0`: the first `n` units are equal ignoring case; both strings
/// ending at the same place before that counts as equal, one ending first does not.
fn equal_n(a: &[u16], b: &[u16], n: usize) -> bool {
    for i in 0..n {
        match (a.get(i), b.get(i)) {
            (None, None) => return true,
            (Some(x), Some(y)) if lower(*x) == lower(*y) => {}
            _ => return false,
        }
    }
    true
}

/// `CDicMgr::Search` over the lines of a dictionary: the first line with a key equal to `word`
/// in the first `compare_len` UTF-16 units (case ignored). With the length of the word as
/// `compare_len` a key that starts with the word hits.
pub(crate) fn search_lines(lines: &[String], word: &str, compare_len: usize) -> Option<Hit> {
    let word: Vec<u16> = word.encode_utf16().collect();
    for (index, line) in lines.iter().enumerate() {
        // `szLine[0] != L';'`: a comment.
        if line.starts_with(';') {
            continue;
        }
        let Some(at) = line.find(DELIMITER) else {
            continue;
        };
        // `wcstok_s( szLine, L",\0" )`: empty tokens are skipped.
        let found = line[..at]
            .split(',')
            .filter(|t| !t.is_empty())
            .find(|token| {
                let token: Vec<u16> = token.encode_utf16().collect();
                equal_n(&word, &token, compare_len)
            });
        if let Some(key) = found {
            let meaning = &line[at + DELIMITER.len()..];
            // `IsLineDelimiterBasic`: the meaning ends at a line break.
            let meaning = meaning.split(['\r', '\n']).next().unwrap_or_default();
            return Some(Hit {
                key: key.to_owned(),
                meaning: meaning.to_owned(),
                line: index + 1,
            });
        }
    }
    None
}

/// `UnEscapeInfoText`: `\n` (a backslash and an n) is a line break and `\x5C` is a backslash.
pub(crate) fn unescape(text: &str) -> String {
    text.replace("\\n", "\n").replace("\\x5C", "\\")
}

/// What the type says about the lookup.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Options<'a> {
    /// `m_bUseKeyHelpAllSearch`
    pub all_search: bool,
    /// `m_bUseKeyHelpKeyDisp`
    pub key_disp: bool,
    /// `m_bUseKeyHelpPrefix`
    pub prefix: bool,
    /// `m_KeyHelpArr`
    pub dicts: &'a [KeyHelpDict],
}

/// The result of a lookup: the text of the tip and the place of the first hit, which the menu
/// item 「キーワード辞書を開く」 opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Found {
    pub text: String,
    /// `m_nSearchDict`: the index of the first dictionary that hit.
    pub dict: usize,
    /// `m_nSearchLine`: the line of the hit in it, from 1.
    pub line: usize,
}

/// `STR_ERR_DLGEDITVW5`: before the name of a dictionary after the first text.
const SEPARATOR_THEN_MARK: &str = "\n--------------------\n■";
/// `STR_ERR_DLGEDITVW6`: before the name of the first dictionary.
const MARK: &str = "■";
/// The line between the hits when only the first hit of each dictionary is shown.
const SEPARATOR: &str = "\n--------------------\n";

/// The name of a dictionary file without the folder and the extension (`_wsplitpath_s`); both
/// kinds of separators end a folder, as the files come from Windows.
fn file_stem(path: &str) -> &str {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    match name.rfind('.') {
        Some(dot) => &name[..dot],
        None => name,
    }
}

/// `CEditView::KeySearchCore`: looks for `word` in the dictionaries that are used, in order, and
/// returns the text of the tip. `lines_of` gives the lines of the file of a dictionary (`None`
/// when it cannot be read).
pub(crate) fn key_search(
    options: &Options,
    word: &str,
    mut lines_of: impl FnMut(&str) -> Option<Arc<Vec<String>>>,
) -> Option<Found> {
    let mut text = String::new();
    // The first line of the tip: the word.
    if options.key_disp {
        text.push_str("[ ");
        text.push_str(word);
        text.push_str(" ]");
    }
    // A prefix search compares as many units as the word has.
    let compare_len = if options.prefix {
        word.encode_utf16().count()
    } else {
        STRNCMP_MAX
    };
    let mut first: Option<(usize, usize)> = None;
    for (i, dict) in options.dicts.iter().enumerate() {
        if !dict.used {
            continue;
        }
        let Some(lines) = lines_of(&dict.path) else {
            continue;
        };
        let Some(hit) = search_lines(&lines, word, compare_len) else {
            continue;
        };
        if options.all_search {
            // Every dictionary that hits adds its meaning, with the name of the dictionary.
            text.push_str(if text.is_empty() {
                MARK
            } else {
                SEPARATOR_THEN_MARK
            });
            text.push_str(file_stem(&dict.path));
            text.push('\n');
            if options.prefix {
                text.push_str(&hit.key);
                text.push_str(" >>\n");
            }
            text.push_str(&unescape(&hit.meaning));
            first.get_or_insert((i, hit.line));
        } else {
            // The first hit is all.
            if !text.is_empty() {
                text.push_str(SEPARATOR);
            }
            if options.prefix {
                text.push_str(&hit.key);
                text.push_str(" >>\n");
            }
            text.push_str(&unescape(&hit.meaning));
            return Some(Found {
                text,
                dict: i,
                line: hit.line,
            });
        }
    }
    first.map(|(dict, line)| Found { text, dict, line })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    fn dict(path: &str, used: bool) -> KeyHelpDict {
        KeyHelpDict {
            used,
            about: String::new(),
            path: path.to_owned(),
        }
    }

    #[test]
    fn a_key_equal_to_the_word_ignoring_case_hits() {
        let d = lines("; comment /// no\nfoo,Bar /// the meaning\nbaz /// other");
        let hit = search_lines(&d, "BAR", STRNCMP_MAX).unwrap();
        assert_eq!(hit.key, "Bar");
        assert_eq!(hit.meaning, "the meaning");
        assert_eq!(hit.line, 2);
        // the whole key must be equal, not just start with the word
        assert!(search_lines(&d, "fo", STRNCMP_MAX).is_none());
        assert!(search_lines(&d, "bars", STRNCMP_MAX).is_none());
    }

    #[test]
    fn a_prefix_search_takes_a_key_that_starts_with_the_word() {
        let d = lines("foobar /// long\nfoo /// short");
        let hit = search_lines(&d, "FOO", 3).unwrap();
        // the first line that starts with the word, not the best one
        assert_eq!((hit.key.as_str(), hit.line), ("foobar", 1));
        // a key shorter than the word does not hit
        assert!(search_lines(&d, "foobarx", 7).is_none());
    }

    #[test]
    fn the_delimiter_is_exact_and_comments_and_other_lines_say_nothing() {
        let d = lines("a ///b\na/// b\n;a /// c\nplain line\na /// d");
        let hit = search_lines(&d, "a", STRNCMP_MAX).unwrap();
        assert_eq!((hit.meaning.as_str(), hit.line), ("d", 5));
        // an empty key between commas is skipped, and a space is part of a key
        let d = lines(",,a, b /// x");
        assert!(search_lines(&d, "a", STRNCMP_MAX).is_some());
        assert!(search_lines(&d, "b", STRNCMP_MAX).is_none());
        assert!(search_lines(&d, " b", STRNCMP_MAX).is_some());
    }

    #[test]
    fn japanese_and_wide_letters_are_found() {
        let d = lines("辞書,じしょ /// dictionary\nＡＢ /// wide");
        assert_eq!(search_lines(&d, "じしょ", STRNCMP_MAX).unwrap().line, 1);
        // case is ignored for the full-width letters too
        assert_eq!(search_lines(&d, "ａｂ", STRNCMP_MAX).unwrap().line, 2);
        // outside the BMP: the length is in UTF-16 units, as `wcslen` counts it
        let d = lines("𠮷野家 /// shop");
        assert!(search_lines(&d, "𠮷野", "𠮷野".encode_utf16().count()).is_some());
        assert!(search_lines(&d, "𠮷野x", "𠮷野x".encode_utf16().count()).is_none());
    }

    #[test]
    fn the_text_is_unescaped() {
        assert_eq!(unescape("a\\nb\\x5Cn"), "a\nb\\n");
    }

    fn files(
        files: &[(&'static str, &'static str)],
    ) -> impl FnMut(&str) -> Option<Arc<Vec<String>>> {
        let files: Vec<_> = files.iter().map(|(p, t)| (*p, lines(t))).collect();
        move |path| {
            files
                .iter()
                .find(|(p, _)| *p == path)
                .map(|(_, l)| Arc::new(l.clone()))
        }
    }

    #[test]
    fn the_first_hit_is_the_tip_and_a_dictionary_not_used_is_skipped() {
        let dicts = [
            dict("a.txt", false),
            dict("b.txt", true),
            dict("c.txt", true),
        ];
        let options = Options {
            all_search: false,
            key_disp: false,
            prefix: false,
            dicts: &dicts,
        };
        let mut read = files(&[
            ("a.txt", "x /// from a"),
            ("b.txt", "x /// from b"),
            ("c.txt", "x /// from c"),
        ]);
        let found = key_search(&options, "x", &mut read).unwrap();
        assert_eq!(found.text, "from b");
        assert_eq!((found.dict, found.line), (1, 1));
        assert!(key_search(&options, "y", &mut read).is_none());
    }

    #[test]
    fn the_key_line_and_the_prefix_hit_are_shown() {
        let dicts = [dict("a.txt", true)];
        let options = Options {
            all_search: false,
            key_disp: true,
            prefix: true,
            dicts: &dicts,
        };
        let mut read = files(&[("a.txt", "foobar /// line1\\nline2")]);
        let found = key_search(&options, "Foo", &mut read).unwrap();
        assert_eq!(
            found.text,
            "[ Foo ]\n--------------------\nfoobar >>\nline1\nline2"
        );
        // no hit: the key line alone is not a tip
        assert!(key_search(&options, "none", &mut read).is_none());
    }

    #[test]
    fn all_dictionaries_add_their_meanings_with_their_names() {
        let dicts = [
            dict("C:\\dict\\first.txt", true),
            dict("miss.txt", true),
            dict("/home/u/second.v2.txt", true),
        ];
        let options = Options {
            all_search: true,
            key_disp: false,
            prefix: false,
            dicts: &dicts,
        };
        let mut read = files(&[
            ("C:\\dict\\first.txt", "k /// one"),
            ("miss.txt", "z /// no"),
            ("/home/u/second.v2.txt", "a /// a\nk /// two"),
        ]);
        let found = key_search(&options, "k", &mut read).unwrap();
        assert_eq!(
            found.text,
            "■first\none\n--------------------\n■second.v2\ntwo"
        );
        // the menu opens the first dictionary that hit
        assert_eq!((found.dict, found.line), (0, 1));
        // with the key line, the first name comes after a separator
        let options = Options {
            key_disp: true,
            ..options
        };
        let found = key_search(&options, "k", &mut read).unwrap();
        assert!(
            found
                .text
                .starts_with("[ k ]\n--------------------\n■first\none")
        );
    }

    #[test]
    fn a_file_that_cannot_be_read_is_skipped() {
        let dicts = [dict("gone.txt", true), dict("b.txt", true)];
        let options = Options {
            all_search: false,
            key_disp: false,
            prefix: false,
            dicts: &dicts,
        };
        let mut read = files(&[("b.txt", "x /// b")]);
        assert_eq!(key_search(&options, "x", &mut read).unwrap().dict, 1);
        let none: [KeyHelpDict; 0] = [];
        let options = Options {
            dicts: &none,
            ..options
        };
        assert!(key_search(&options, "x", &mut read).is_none());
    }
}
