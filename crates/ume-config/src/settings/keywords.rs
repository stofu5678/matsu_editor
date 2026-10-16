// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/types/CType.cpp (CShareData::InitKeyword: the names, the case rule and the order of
//     the built-in keyword sets)
//   sakura_core/env/CKeyWordSetMgr.cpp, sakura_core/env/CKeyWordSetMgr.h (the keyword set manager)
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_KeyWords)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, jepro, MIK
//   Copyright (C) 2004, Moca, genta
//   Copyright (C) 2005, D.S.Koba
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

//! Keyword sets (強調キーワード): lists of words that the coloring draws in the keyword colors.
//!
//! A document type chooses up to 10 sets by their index ([`TypeSettings::keyword_set_idx`](super::TypeSettings),
//! `-1` is "none"); the sets themselves are shared by all types, as in Sakura. [`KeywordSets::builtin`]
//! has Sakura's 17 sets; [`KeywordSets::from_profile`] reads the section `[KeyWords]` of the settings
//! file over them.
//!
//! The words of the built-in sets come from the arrays `g_ppszKeywords*` of
//! `sakura_core/types/CType_*.cpp`; they are all in the Sakura clone (none is loaded from a file).

use std::cmp::Ordering;

use super::keyword_data as data;
use crate::profile::Profile;

/// `MAX_SETNUM`: how many keyword sets there can be.
pub const MAX_KEYWORD_SETS: usize = 100;
/// `MAX_SETNAMELEN`: the longest name of a set, in UTF-16 units.
pub const MAX_SET_NAME_LEN: usize = 32;
/// `MAX_KEYWORDLEN`: the longest keyword, in UTF-16 units.
pub const MAX_KEYWORD_LEN: usize = 63;

/// One keyword set (`CKeyWordSetMgr` keeps these side by side in arrays).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordSet {
    pub name: String,
    /// Letters are compared with their case (`m_bKEYWORDCASEArr`).
    pub case_sensitive: bool,
    /// The words, sorted the way Sakura sorts them before it searches (`SortKeyWord`).
    words: Vec<String>,
}

impl KeywordSet {
    /// A set of `words`. The words are sorted with `wcscmp` (case sensitive) or `_wcsicmp`.
    pub fn new(name: &str, case_sensitive: bool, words: impl IntoIterator<Item = String>) -> Self {
        let mut words: Vec<String> = words.into_iter().collect();
        words.sort_by(|a, b| compare(a, b, case_sensitive));
        Self {
            name: name.to_owned(),
            case_sensitive,
            words,
        }
    }

    /// The words in the order Sakura searches them.
    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// The length of the longest word, in `char`s (`KeywordMaxLen`; the coloring skips a word that
    /// is longer).
    pub fn max_len(&self) -> usize {
        self.words
            .iter()
            .map(|w| w.chars().count())
            .max()
            .unwrap_or(0)
    }

    /// Is `word` in the set? Whole word, compared with the case rule of the set.
    pub fn contains(&self, word: &str) -> bool {
        self.words
            .binary_search_by(|w| compare(w, word, self.case_sensitive))
            .is_ok()
    }
}

/// `wcscmp` or `_wcsicmp` (which compares the lower case of each UTF-16 unit).
fn compare(a: &str, b: &str, case_sensitive: bool) -> Ordering {
    if case_sensitive {
        a.encode_utf16().cmp(b.encode_utf16())
    } else {
        let lower = |s: &str| {
            s.chars()
                .flat_map(char::to_lowercase)
                .collect::<Vec<char>>()
                .into_iter()
                .flat_map(|c| {
                    let mut buf = [0u16; 2];
                    c.encode_utf16(&mut buf).to_vec()
                })
                .collect::<Vec<u16>>()
        };
        lower(a).cmp(&lower(b))
    }
}

/// All keyword sets, in Sakura's order (the index a type stores is the index here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordSets {
    sets: Vec<KeywordSet>,
}

impl KeywordSets {
    /// `CShareData::InitKeyword`: the 17 sets Sakura starts with.
    pub fn builtin() -> Self {
        // (name, case sensitive, words), in the order of `InitKeyword`.
        let table: [(&str, bool, &[&str]); 17] = [
            ("C/C++", true, data::cpp::WORDS),
            ("HTML", false, data::html::WORDS),
            ("PL/SQL", false, data::plsql::WORDS),
            ("COBOL", true, data::cobol::WORDS),
            ("Java", true, data::java::WORDS),
            ("CORBA IDL", true, data::corba_idl::WORDS),
            ("AWK", true, data::awk::WORDS),
            ("MS-DOS batch", false, data::bat::WORDS),
            ("Pascal", false, data::pascal::WORDS),
            ("TeX", true, data::tex::WORDS),
            ("TeX2", true, data::tex2::WORDS),
            ("Perl", true, data::perl::WORDS),
            ("Perl2", true, data::perl2::WORDS),
            ("Visual Basic", false, data::vb::WORDS),
            ("Visual Basic2", false, data::vb2::WORDS),
            ("Rich Text", true, data::rtf::WORDS),
            ("Python", true, data::python::WORDS),
        ];
        Self {
            sets: table
                .iter()
                .map(|(name, case, words)| {
                    KeywordSet::new(name, *case, words.iter().map(|w| (*w).to_owned()))
                })
                .collect(),
        }
    }

    /// The sets of the section `[KeyWords]` (`ShareData_IO_KeyWords`, read side). Like Sakura, a
    /// file without `nKeyWordSetNum` keeps the built-in sets; with it, the file's sets replace all
    /// of them.
    pub fn from_profile(profile: &Profile) -> Self {
        const S: &str = "KeyWords";
        let Some(count) = profile.get_i32(S, "nKeyWordSetNum") else {
            return Self::builtin();
        };
        let count = usize::try_from(count).unwrap_or(0).min(MAX_KEYWORD_SETS);
        let sets = (0..count)
            .map(|i| {
                let name = profile.get(S, &format!("szSN[{i:02}]")).unwrap_or_default();
                let name = truncate_units(name, MAX_SET_NAME_LEN);
                let case = profile
                    .get_bool(S, &format!("nCASE[{i:02}]"))
                    .unwrap_or(false);
                let num = profile
                    .get_i32(S, &format!("nKWN[{i:02}]"))
                    .and_then(|n| usize::try_from(n).ok())
                    .unwrap_or(0);
                let words = profile
                    .get(S, &format!("szKW[{i:02}]"))
                    .map(|v| split_words(v, num))
                    .unwrap_or_default();
                KeywordSet::new(&name, case, words)
            })
            .collect();
        Self { sets }
    }

    /// How many sets there are.
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// The set at `index`; `None` for `-1` ("none") and for an index that does not exist.
    pub fn get(&self, index: i32) -> Option<&KeywordSet> {
        usize::try_from(index).ok().and_then(|i| self.sets.get(i))
    }

    pub fn iter(&self) -> impl Iterator<Item = &KeywordSet> {
        self.sets.iter()
    }

    /// The index of the set called `name`.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.sets.iter().position(|s| s.name == name)
    }
}

impl Default for KeywordSets {
    fn default() -> Self {
        Self::builtin()
    }
}

/// `SetKeyWordArr` (the string form): words separated by tabs; at most `max` words; reading stops
/// at the end of the string, so a final tab does not make an empty word.
fn split_words(value: &str, max: usize) -> Vec<String> {
    let mut words = Vec::new();
    let mut rest = value;
    while words.len() < max && !rest.is_empty() {
        let (word, tail) = rest.split_once('\t').unwrap_or((rest, ""));
        words.push(truncate_units(word, MAX_KEYWORD_LEN));
        rest = tail;
    }
    words
}

/// Cuts `s` to at most `max` UTF-16 units, at a character boundary.
fn truncate_units(s: &str, max: usize) -> String {
    let mut units = 0;
    let mut out = String::new();
    for c in s.chars() {
        units += c.len_utf16();
        if units > max {
            break;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    #[test]
    fn the_builtin_sets_have_sakuras_names_and_case_rules() {
        let sets = KeywordSets::builtin();
        assert_eq!(sets.len(), 17);
        let names: Vec<&str> = sets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(&names[..3], ["C/C++", "HTML", "PL/SQL"]);
        assert_eq!(names[16], "Python");
        // `InitKeyword`: HTML, PL/SQL, batch, Pascal and Visual Basic ignore case.
        for (i, case) in [
            (0, true),
            (1, false),
            (2, false),
            (7, false),
            (8, false),
            (13, false),
        ] {
            assert_eq!(sets.get(i).unwrap().case_sensitive, case, "set {i}");
        }
        assert_eq!(sets.get(-1), None);
        assert_eq!(sets.get(17), None);
        assert_eq!(sets.index_of("Perl2"), Some(12));
    }

    #[test]
    fn the_words_are_those_of_the_original_arrays() {
        let sets = KeywordSets::builtin();
        let cpp = sets.get(0).unwrap();
        assert_eq!(cpp.words().len(), 158);
        assert!(cpp.contains("while"));
        assert!(cpp.contains("#ifdef"));
        assert!(!cpp.contains("WHILE"), "C/C++ is case sensitive");
        let sql = sets.get(2).unwrap();
        assert_eq!(sql.words().len(), 80);
        assert!(sql.contains("SELECT") && sql.contains("select"));
        // Words with a backslash (RTF) and in Japanese keep their characters.
        assert!(sets.get(15).unwrap().contains("\\ansi"));
        assert_eq!(sets.get(16).unwrap().words().len(), 38);
        assert!(sets.get(16).unwrap().contains("lambda"));
    }

    #[test]
    fn a_set_is_sorted_with_its_case_rule() {
        let case = KeywordSet::new("a", true, ["b", "B", "a"].map(String::from));
        assert_eq!(case.words(), ["B", "a", "b"]);
        let nocase = KeywordSet::new("a", false, ["b", "B", "a"].map(String::from));
        assert_eq!(nocase.words()[0], "a");
        assert!(nocase.contains("A"));
        assert_eq!(nocase.max_len(), 1);
    }

    #[test]
    fn a_file_without_the_count_keeps_the_builtin_sets() {
        let p = profile("[KeyWords]\r\nszSN[00]=X\r\n");
        assert_eq!(KeywordSets::from_profile(&p), KeywordSets::builtin());
    }

    #[test]
    fn the_sets_of_the_file_replace_the_builtin_ones() {
        let p = profile(
            "[KeyWords]\r\nnKeyWordSetNum=2\r\nszSN[00]=Mine\r\nnCASE[00]=1\r\nnKWN[00]=2\r\n\
             szKW[00]=foo\tBar\t\r\nszSN[01]=Other\r\nnCASE[01]=0\r\nnKWN[01]=5\r\nszKW[01]=x\t\t\r\n",
        );
        let sets = KeywordSets::from_profile(&p);
        assert_eq!(sets.len(), 2);
        let mine = sets.get(0).unwrap();
        assert_eq!(mine.name, "Mine");
        assert!(mine.case_sensitive);
        assert!(mine.contains("foo") && mine.contains("Bar") && !mine.contains("bar"));
        // The number of words limits what is read, and the string end stops it.
        assert_eq!(sets.get(1).unwrap().words().len(), 2);
        assert_eq!(sets.get(2), None);
    }

    #[test]
    fn long_names_and_words_are_cut() {
        assert_eq!(truncate_units(&"a".repeat(40), MAX_SET_NAME_LEN).len(), 32);
        // A pair of UTF-16 units is not split.
        assert_eq!(truncate_units("a😀", 2), "a");
        assert_eq!(split_words("a\tb\tc", 2), ["a", "b"]);
    }
}
