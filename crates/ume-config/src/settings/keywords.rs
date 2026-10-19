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

use super::extra_types::ExtraSet;
use super::keyword_data as data;
use crate::profile::{Profile, ProfileError};

/// How many keyword sets Sakura starts with (`CShareData::InitKeyword`).
pub const SAKURA_KEYWORD_SET_COUNT: usize = 17;
/// `MAX_SETNUM`: how many keyword sets there can be.
pub const MAX_KEYWORD_SETS: usize = 100;
/// `MAX_SETNAMELEN`: the longest name of a set, in UTF-16 units.
pub const MAX_SET_NAME_LEN: usize = 32;
/// `MAX_KEYWORDLEN`: the longest keyword, in UTF-16 units.
pub const MAX_KEYWORD_LEN: usize = 63;

/// `[KeyWords]` of the settings file.
pub const SECTION: &str = "KeyWords";
/// The key that tells that UME Editor wrote the sets, with its own among them (ADR-0017): they are all in the
/// file then, and a set that is not there was deleted. Sakura does not know the key.
pub const EXTRA_SETS_IN_FILE_KEY: &str = "bUmeExtraSetsInFile";

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

/// How the words of a set are ordered under its case rule (`wcscmp` or `_wcsicmp`).
pub fn compare_words(a: &str, b: &str, case_sensitive: bool) -> Ordering {
    compare(a, b, case_sensitive)
}

/// All keyword sets, in Sakura's order (the index a type stores is the index here), then UME Editor's own
/// sets ([`ExtraSet`]) in the numbers after the sets of the file (ADR-0017).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordSets {
    sets: Vec<KeywordSet>,
    /// Where each of [`ExtraSet::ALL`] is, or -1 when there was no number left for it.
    extra: Vec<i32>,
}

impl KeywordSets {
    /// What the editor has without a settings file: the 17 sets of Sakura (`CShareData::InitKeyword`), then
    /// UME Editor's own.
    pub fn builtin() -> Self {
        Self::with_extras(Self::sakura_sets(), false)
    }

    /// Finds UME Editor's own sets among `sets` (by name), and puts the ones that are missing after `sets`, as
    /// long as there are numbers left (`MAX_KEYWORD_SETS`). A file with fewer than 17 sets gets none: the
    /// numbers up to 16 must stay free, because the types of Sakura still refer to them (as to sets that do not
    /// exist).
    ///
    /// A file that [UME Editor wrote](EXTRA_SETS_IN_FILE_KEY) has the sets itself, so a missing one was deleted
    /// by the user and stays gone (`in_file`).
    fn with_extras(mut sets: Vec<KeywordSet>, in_file: bool) -> Self {
        let room = !in_file && sets.len() >= SAKURA_KEYWORD_SET_COUNT;
        let extra = ExtraSet::ALL
            .iter()
            .map(|s| {
                if let Some(at) = sets.iter().position(|set| set.name == s.name()) {
                    i32::try_from(at).unwrap_or(-1)
                } else if room && sets.len() < MAX_KEYWORD_SETS {
                    sets.push(s.to_set());
                    i32::try_from(sets.len() - 1).unwrap_or(-1)
                } else {
                    -1
                }
            })
            .collect();
        Self { sets, extra }
    }

    /// The index of one of UME Editor's own sets, or -1 when it did not fit.
    pub fn extra_index(&self, set: ExtraSet) -> i32 {
        self.extra.get(set.ordinal()).copied().unwrap_or(-1)
    }

    /// `CShareData::InitKeyword`: the 17 sets Sakura starts with.
    fn sakura_sets() -> Vec<KeywordSet> {
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
        table
            .iter()
            .map(|(name, case, words)| {
                KeywordSet::new(name, *case, words.iter().map(|w| (*w).to_owned()))
            })
            .collect()
    }

    /// The sets of the section `[KeyWords]` (`ShareData_IO_KeyWords`, read side). Like Sakura, a
    /// file without `nKeyWordSetNum` keeps the built-in sets; with it, the file's sets replace all
    /// of them.
    pub fn from_profile(profile: &Profile) -> Self {
        const S: &str = "KeyWords";
        let Some(count) = profile.get_i32(S, "nKeyWordSetNum") else {
            return Self::builtin();
        };
        let in_file = profile.get_bool(S, EXTRA_SETS_IN_FILE_KEY) == Some(true);
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
        Self::with_extras(sets, in_file)
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

/// A keyword set as the settings page edits it: the words stay in the order of the list (they are not sorted
/// until the coloring reads them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetDraft {
    pub name: String,
    pub case_sensitive: bool,
    pub words: Vec<String>,
}

impl KeywordSets {
    /// The sets as the page starts with them.
    pub fn drafts(&self) -> Vec<SetDraft> {
        self.sets
            .iter()
            .map(|s| SetDraft {
                name: s.name.clone(),
                case_sensitive: s.case_sensitive,
                words: s.words.clone(),
            })
            .collect()
    }
}

/// Writes `drafts` as the section `[KeyWords]` (`ShareData_IO_KeyWords`, write side): the number of sets, and
/// for each set its name `szSN`, `nCASE`, the number of words `nKWN`, and the words `szKW` (each followed by a
/// tab) with the length of that string `szKW.Size`. The keys of sets that are gone are removed; the other keys
/// of the section (`nCurrentKeyWordSetIdx`) stay. A key that already has the value is left as it is.
///
/// The sets of UME Editor's own are in `drafts` too: the key [`EXTRA_SETS_IN_FILE_KEY`] says so. Returns whether
/// the file changed.
pub fn store_sets(profile: &mut Profile, drafts: &[SetDraft]) -> Result<bool, ProfileError> {
    let old_count = profile
        .get_i32(SECTION, "nKeyWordSetNum")
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0)
        .min(MAX_KEYWORD_SETS);
    let mut changed = false;
    let mut put = |profile: &mut Profile, key: &str, value: &str| {
        if profile.get(SECTION, key) != Some(value) {
            profile.set(SECTION, key, value)?;
            changed = true;
        }
        Ok::<(), ProfileError>(())
    };
    put(profile, "nKeyWordSetNum", &drafts.len().to_string())?;
    put(profile, EXTRA_SETS_IN_FILE_KEY, "1")?;
    for (i, set) in drafts.iter().enumerate() {
        let mut words = String::new();
        for word in &set.words {
            words.push_str(word);
            words.push('\t');
        }
        // `nMemLen`: the length of the string and its end.
        let size = words.encode_utf16().count() + 1;
        put(profile, &format!("szSN[{i:02}]"), &set.name)?;
        put(
            profile,
            &format!("nCASE[{i:02}]"),
            if set.case_sensitive { "1" } else { "0" },
        )?;
        put(
            profile,
            &format!("nKWN[{i:02}]"),
            &set.words.len().to_string(),
        )?;
        put(profile, &format!("szKW[{i:02}].Size"), &size.to_string())?;
        put(profile, &format!("szKW[{i:02}]"), &words)?;
    }
    for i in drafts.len()..old_count {
        for key in [
            format!("szSN[{i:02}]"),
            format!("nCASE[{i:02}]"),
            format!("nKWN[{i:02}]"),
            format!("szKW[{i:02}]"),
            format!("szKW[{i:02}].Size"),
        ] {
            changed |= profile.remove(SECTION, &key);
        }
    }
    Ok(changed)
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
        // Sakura's 17, then UME Editor's own (ADR-0017).
        assert_eq!(sets.len(), 17 + ExtraSet::ALL.len());
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
        assert_eq!(sets.get(27), None);
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
        // A file with fewer than 17 sets leaves the numbers up to 16 as they are: none of UME Editor's sets.
        assert_eq!(sets.len(), 2);
        assert_eq!(sets.extra_index(ExtraSet::Rust), -1);
        let mine = sets.get(0).unwrap();
        assert_eq!(mine.name, "Mine");
        assert!(mine.case_sensitive);
        assert!(mine.contains("foo") && mine.contains("Bar") && !mine.contains("bar"));
        // The number of words limits what is read, and the string end stops it.
        assert_eq!(sets.get(1).unwrap().words().len(), 2);
        assert_eq!(sets.get(2), None);
    }

    #[test]
    fn the_sets_of_ume_editor_are_found_by_name_and_a_deleted_one_stays_gone() {
        let mut p = Profile::new();
        let mut drafts = KeywordSets::builtin().drafts();
        assert_eq!(drafts.len(), 27);
        assert!(store_sets(&mut p, &drafts).unwrap());
        // The file has all 27 sets; UME Editor's own are found by name and not added a second time.
        let read = KeywordSets::from_profile(&p);
        assert_eq!(read.len(), 27);
        assert_eq!(read.extra_index(ExtraSet::Rust), 22);
        // The user deletes "Rust": the key says that the file has the sets, so it stays gone.
        drafts.remove(22);
        store_sets(&mut p, &drafts).unwrap();
        let read = KeywordSets::from_profile(&p);
        assert_eq!(read.len(), 26);
        assert_eq!(read.extra_index(ExtraSet::Rust), -1);
        assert_eq!(read.extra_index(ExtraSet::Go), 22);
        // Without the key (Sakura wrote the file) a missing set of UME Editor's own is added at the end.
        p.remove(SECTION, EXTRA_SETS_IN_FILE_KEY);
        let read = KeywordSets::from_profile(&p);
        assert_eq!(read.len(), 27);
        assert_eq!(read.extra_index(ExtraSet::Rust), 26);
        // A set of Sakura's file that has the name of one of UME Editor's is that set.
        let p = profile(
            "[KeyWords]\r\nnKeyWordSetNum=18\r\nszSN[17]=Rust\r\nnKWN[17]=1\r\nszKW[17]=mine\t\r\n",
        );
        let read = KeywordSets::from_profile(&p);
        assert_eq!(read.extra_index(ExtraSet::Rust), 17);
        assert!(read.get(17).unwrap().contains("mine"));
        assert_eq!(read.len(), 18 + ExtraSet::ALL.len() - 1);
    }

    #[test]
    fn the_sets_are_stored_as_sakura_stores_them() {
        let mut p = profile(
            "[KeyWords]\r\nnCurrentKeyWordSetIdx=2\r\nnKeyWordSetNum=3\r\nszSN[02]=gone\r\n",
        );
        let drafts = vec![
            SetDraft {
                name: "C".to_owned(),
                case_sensitive: true,
                words: vec!["if".to_owned(), "あ".to_owned()],
            },
            SetDraft {
                name: "empty".to_owned(),
                case_sensitive: false,
                words: Vec::new(),
            },
        ];
        assert!(store_sets(&mut p, &drafts).unwrap());
        assert_eq!(p.get(SECTION, "nKeyWordSetNum"), Some("2"));
        assert_eq!(p.get(SECTION, "szKW[00]"), Some("if\tあ\t"));
        // The length counts UTF-16 units and the end of the string.
        assert_eq!(p.get(SECTION, "szKW[00].Size"), Some("6"));
        assert_eq!(p.get(SECTION, "nKWN[00]"), Some("2"));
        assert_eq!(p.get(SECTION, "nCASE[00]"), Some("1"));
        assert_eq!(p.get(SECTION, "szKW[01]"), Some(""));
        assert_eq!(p.get(SECTION, "szKW[01].Size"), Some("1"));
        // The set that is gone loses its keys; the chosen set stays.
        assert_eq!(p.get(SECTION, "szSN[02]"), None);
        assert_eq!(p.get(SECTION, "nCurrentKeyWordSetIdx"), Some("2"));
        assert!(!store_sets(&mut p, &drafts).unwrap());
        assert!(compare_words("a", "B", false).is_lt());
        assert!(compare_words("a", "B", true).is_gt());
    }

    #[test]
    fn long_names_and_words_are_cut() {
        assert_eq!(truncate_units(&"a".repeat(40), MAX_SET_NAME_LEN).len(), 32);
        // A pair of UTF-16 units is not split.
        assert_eq!(truncate_units("a😀", 2), "a");
        assert_eq!(split_words("a\tb\tc", 2), ["a", "b"]);
    }
}
