// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/recent/CRecentImp.cpp (AppendItem), sakura_core/recent/CRecentSearch.cpp,
//   sakura_core/recent/CRecentGrepFile.cpp, CRecentGrepFolder.cpp, CRecentExcludeFile.cpp,
//   CRecentExcludeFolder.cpp, CRecentTagjumpKeyword.cpp,
//   sakura_core/env/CSearchKeywordManager.cpp,
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_Keys, ShareData_IO_Grep, ShareData_IO_Other),
//   sakura_core/env/CDataProfile.h (IOProfileDataSet), sakura_core/config/maxdata.h
//
// Original copyright notices of those files:
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

//! The histories of the combo boxes of the search dialogs: the section `[Keys]` (`SEARCHKEY`,
//! `REPLACEKEY`), the section `[Grep]` (`GREPFILE`, `GREPFOLDER`, `GREPEXCLUDEFILE`,
//! `GREPEXCLUDEFOLDER`) and the keywords of the tag jump dialog, `TagJumpKeyword` of `[Other]`.
//!
//! Sakura keeps the strings of the find, replace and Grep dialogs in the shared data
//! (`m_sSearchKeywords`, `m_sTagJump`), newest first, and writes them as `_SEARCHKEY_Counts=N` and
//! `SEARCHKEY[00]=...` (`ShareData_IO_Keys`, `ShareData_IO_Grep`, `IOProfileDataSet`).
//! [`KeyList`] is such a list, [`SearchKeys`] holds all of them and reads and writes the sections.
//! Everything else in the file is left as it is.

use crate::profile::{Profile, ProfileError};
use crate::value::parse_long;

/// The section of the search and replace strings.
pub const SECTION: &str = "Keys";
/// The section of the Grep boxes (`ShareData_IO_Grep`).
pub const GREP_SECTION: &str = "Grep";
/// The section that has the tag jump keywords (`ShareData_IO_Other`).
pub const OTHER_SECTION: &str = "Other";
/// `MAX_SEARCHKEY` (`sakura_core/config/maxdata.h`).
pub const MAX_SEARCH_KEYS: usize = 30;
/// `MAX_REPLACEKEY`.
pub const MAX_REPLACE_KEYS: usize = 30;
/// `_MAX_PATH`: a string must be shorter than this many UTF-16 units to be kept
/// (`ValidateReceiveType`, and the `size() < _MAX_PATH` checks of the callers).
pub const MAX_KEY_UNITS: usize = 260;
/// `MAX_GREPFILE`, `MAX_GREPFOLDER`, `MAX_EXCLUDEFILE`, `MAX_EXCLUDEFOLDER`.
pub const MAX_GREP_ITEMS: usize = 30;
/// `MAX_GREP_PATH` and `MAX_EXCLUDE_PATH`: the strings of the Grep boxes are kept when shorter than
/// this (`CGrepFileString`, `CGrepFolderString`, `CExcludeFileString`, `CExcludeFolderString`).
pub const MAX_GREP_UNITS: usize = 512;
/// `MAX_TAGJUMP_KEYWORD`.
pub const MAX_TAG_KEYWORDS: usize = 30;

/// A history: newest first, at most `max` strings, none twice (`CRecentImp`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyList {
    items: Vec<String>,
    max: usize,
    /// A string must be shorter than this many UTF-16 units to be kept.
    max_units: usize,
    /// Whether two strings that differ in case are the same one (`_wcsicmp` in `CompareItem` of
    /// the Grep boxes; the others use `wcscmp`).
    ignore_case: bool,
}

impl KeyList {
    /// A list of case sensitive strings of fewer than [`MAX_KEY_UNITS`] units.
    pub fn new(max: usize) -> Self {
        Self::with(max, MAX_KEY_UNITS, false)
    }

    /// A list with its own length limit and its way of telling strings apart.
    pub fn with(max: usize, max_units: usize, ignore_case: bool) -> Self {
        Self {
            items: Vec::new(),
            max,
            max_units,
            ignore_case,
        }
    }

    /// The strings, newest first.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// The newest string.
    pub fn newest(&self) -> Option<&str> {
        self.items.first().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Whether `text` can be kept: shorter than the limit in UTF-16 units
    /// (`ValidateReceiveType`).
    pub fn accepts(&self, text: &str) -> bool {
        text.encode_utf16().count() < self.max_units
    }

    /// `CompareItem`.
    fn same(&self, a: &str, b: &str) -> bool {
        if self.ignore_case {
            a == b || a.to_lowercase() == b.to_lowercase()
        } else {
            a == b
        }
    }

    /// `CRecentImp::AppendItem`: puts `text` first. A string that is already there moves to the
    /// front (with the case of the new one: `CopyItem`); when the list is full the oldest one goes.
    /// Returns whether the list changed (a string that was already first leaves it as it was), or
    /// `false` for a string that is too long.
    pub fn add(&mut self, text: &str) -> bool {
        if !self.accepts(text) {
            return false;
        }
        if let Some(i) = self.items.iter().position(|s| self.same(s, text)) {
            if i == 0 && self.items[0] == text {
                return false;
            }
            self.items.remove(i);
        } else if self.items.len() >= self.max {
            self.items.truncate(self.max - 1);
        }
        self.items.insert(0, text.to_owned());
        true
    }
}

/// The search strings and the replace strings, and the strings of the other combo boxes that
/// Sakura keeps in the same shared data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchKeys {
    pub search: KeyList,
    pub replace: KeyList,
    /// `m_aGrepFiles`: ファイル of the Grep dialog.
    pub grep_file: KeyList,
    /// `m_aGrepFolders`: フォルダー.
    pub grep_folder: KeyList,
    /// `m_aExcludeFiles`: 除外ファイル.
    pub exclude_file: KeyList,
    /// `m_aExcludeFolders`: 除外フォルダー.
    pub exclude_folder: KeyList,
    /// `m_aTagJumpKeywords`: the keyword of the tag jump dialog.
    pub tag_keyword: KeyList,
}

impl Default for SearchKeys {
    fn default() -> Self {
        // The Grep boxes compare without regard to case (`_wcsicmp`), the others do not.
        let grep = || KeyList::with(MAX_GREP_ITEMS, MAX_GREP_UNITS, true);
        Self {
            search: KeyList::new(MAX_SEARCH_KEYS),
            replace: KeyList::new(MAX_REPLACE_KEYS),
            grep_file: grep(),
            grep_folder: grep(),
            exclude_file: grep(),
            exclude_folder: grep(),
            tag_keyword: KeyList::new(MAX_TAG_KEYWORDS),
        }
    }
}

/// The lists that are not the search and replace strings, with the place of each in the file:
/// the section and the name. [`SearchKeys::other`] and [`SearchKeys::other_mut`] give the list.
const OTHER_LISTS: [(&str, &str); 5] = [
    (GREP_SECTION, "GREPFILE"),
    (GREP_SECTION, "GREPFOLDER"),
    (GREP_SECTION, "GREPEXCLUDEFILE"),
    (GREP_SECTION, "GREPEXCLUDEFOLDER"),
    (OTHER_SECTION, "TagJumpKeyword"),
];

impl SearchKeys {
    /// The list `i` of [`OTHER_LISTS`].
    fn other(&self, i: usize) -> &KeyList {
        match i {
            0 => &self.grep_file,
            1 => &self.grep_folder,
            2 => &self.exclude_file,
            3 => &self.exclude_folder,
            _ => &self.tag_keyword,
        }
    }

    fn other_mut(&mut self, i: usize) -> &mut KeyList {
        match i {
            0 => &mut self.grep_file,
            1 => &mut self.grep_folder,
            2 => &mut self.exclude_file,
            3 => &mut self.exclude_folder,
            _ => &mut self.tag_keyword,
        }
    }

    /// `AddToSearchKeyArr`. An empty string is not kept: Sakura's dialogs never add one except
    /// Grep, which adds it so that its own field starts empty (`CDlgGrep::GetData`); a blank line
    /// in the drop-down is of no use. Returns whether the list changed.
    pub fn add_search(&mut self, text: &str) -> bool {
        !text.is_empty() && self.search.add(text)
    }

    /// `AddToReplaceKeyArr`. An empty string is kept, as in Sakura: replacing with nothing is a
    /// replacement the user picks again.
    pub fn add_replace(&mut self, text: &str) -> bool {
        self.replace.add(text)
    }

    /// `AddToGrepFileArr`. An empty string is not kept, for the reason of
    /// [`add_search`](Self::add_search): Sakura adds the empty 除外ファイル of most searches, and
    /// the drop-down would show a blank line on top.
    pub fn add_grep_file(&mut self, text: &str) -> bool {
        !text.is_empty() && self.grep_file.add(text)
    }

    /// `AddToGrepFolderArr`.
    pub fn add_grep_folder(&mut self, text: &str) -> bool {
        !text.is_empty() && self.grep_folder.add(text)
    }

    /// `AddToExcludeFileArr`.
    pub fn add_exclude_file(&mut self, text: &str) -> bool {
        !text.is_empty() && self.exclude_file.add(text)
    }

    /// `AddToExcludeFolderArr`.
    pub fn add_exclude_folder(&mut self, text: &str) -> bool {
        !text.is_empty() && self.exclude_folder.add(text)
    }

    /// `CRecentTagjumpKeyword::AppendItem` (`CDlgTagJumpList::GetData`).
    pub fn add_tag_keyword(&mut self, text: &str) -> bool {
        !text.is_empty() && self.tag_keyword.add(text)
    }

    /// Takes in the strings of `other` (the file, as another window or process saved it): they go
    /// on top of ours, in their order, and the strings only we have stay below them. Over the
    /// limit the oldest (the bottom) goes. A string we have that `other` also has takes
    /// `other`'s place, as it was used more recently there.
    pub fn merge_from(&mut self, other: &SearchKeys) {
        fn merge(list: &mut KeyList, from: &KeyList) {
            for text in from.items().iter().rev() {
                list.add(text);
            }
        }
        merge(&mut self.search, &other.search);
        merge(&mut self.replace, &other.replace);
        merge(&mut self.grep_file, &other.grep_file);
        merge(&mut self.grep_folder, &other.grep_folder);
        merge(&mut self.exclude_file, &other.exclude_file);
        merge(&mut self.exclude_folder, &other.exclude_folder);
        merge(&mut self.tag_keyword, &other.tag_keyword);
    }

    /// Reads `[Keys]`, `[Grep]` and the keywords of `[Other]`. A missing section or entry means no
    /// strings.
    pub fn from_profile(profile: &Profile) -> Self {
        let mut keys = Self::default();
        keys.search.items = read_set(profile, SECTION, "SEARCHKEY", MAX_SEARCH_KEYS)
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        keys.replace.items = read_set(profile, SECTION, "REPLACEKEY", MAX_REPLACE_KEYS);
        for (i, (section, name)) in OTHER_LISTS.into_iter().enumerate() {
            let list = keys.other_mut(i);
            // Sakura's Grep leaves "" in these lists; a blank line is not shown.
            let items = read_set(profile, section, name, list.max);
            list.items = items.into_iter().filter(|s| !s.is_empty()).collect();
        }
        keys
    }

    /// Writes the lists (the count and the strings), leaving the rest of the file alone. A string
    /// the file format cannot hold (a line break) is skipped.
    pub fn write_to(&self, profile: &mut Profile) -> Result<(), ProfileError> {
        write_set(profile, SECTION, "SEARCHKEY", self.search.items())?;
        write_set(profile, SECTION, "REPLACEKEY", self.replace.items())?;
        for (i, (section, name)) in OTHER_LISTS.into_iter().enumerate() {
            write_set(profile, section, name, self.other(i).items())?;
        }
        Ok(())
    }
}

/// `IOProfileDataSet`, reading: `_<name>_Counts` is the number of strings (limited to `max`); if
/// it is missing all `max` places are read and the empty ones at the end are dropped.
fn read_set(profile: &Profile, section: &str, name: &str, max: usize) -> Vec<String> {
    let count_key = format!("_{name}_Counts");
    let (count, missing) = match profile.get(section, &count_key).and_then(parse_long) {
        Some(n) => (usize::try_from(n.max(0)).unwrap_or(0).min(max), false),
        None => (max, true),
    };
    let mut items: Vec<String> = (0..count)
        .map(|i| {
            profile
                .get(section, &format!("{name}[{i:02}]"))
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    if missing {
        while items.last().is_some_and(String::is_empty) {
            items.pop();
        }
    }
    items
}

/// `IOProfileDataSet`, writing.
fn write_set(
    profile: &mut Profile,
    section: &str,
    name: &str,
    items: &[String],
) -> Result<(), ProfileError> {
    let items: Vec<&String> = items.iter().filter(|s| !s.contains(['\r', '\n'])).collect();
    let count = i32::try_from(items.len()).unwrap_or(i32::MAX);
    profile.set_i32(section, &format!("_{name}_Counts"), count)?;
    for (i, item) in items.iter().enumerate() {
        profile.set(section, &format!("{name}[{i:02}]"), item)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(max: usize, items: &[&str]) -> KeyList {
        let mut l = KeyList::new(max);
        for s in items.iter().rev() {
            l.add(s);
        }
        l
    }

    #[test]
    fn the_newest_string_is_first() {
        let mut l = KeyList::new(30);
        assert!(l.add("a") && l.add("b") && l.add("c"));
        assert_eq!(l.items(), ["c", "b", "a"]);
        assert_eq!(l.newest(), Some("c"));
    }

    #[test]
    fn a_repeated_string_moves_to_the_front() {
        let mut l = list(30, &["c", "b", "a"]);
        assert!(l.add("a"));
        assert_eq!(l.items(), ["a", "c", "b"]);
        // already first: nothing changes
        assert!(!l.add("a"));
        assert_eq!(l.items(), ["a", "c", "b"]);
    }

    #[test]
    fn the_oldest_string_goes_when_the_list_is_full() {
        let mut l = KeyList::new(3);
        for s in ["1", "2", "3", "4"] {
            l.add(s);
        }
        assert_eq!(l.items(), ["4", "3", "2"]);
        // a repeated one does not push anything out
        l.add("2");
        assert_eq!(l.items(), ["2", "4", "3"]);
    }

    #[test]
    fn the_limit_is_thirty() {
        let mut keys = SearchKeys::default();
        for i in 0..40 {
            keys.add_search(&i.to_string());
            keys.add_replace(&i.to_string());
        }
        assert_eq!(keys.search.len(), 30);
        assert_eq!(keys.replace.len(), 30);
        assert_eq!(keys.search.newest(), Some("39"));
        assert_eq!(keys.search.items()[29], "10");
    }

    #[test]
    fn a_string_of_260_units_or_more_is_not_kept() {
        let mut l = KeyList::new(30);
        assert!(l.add(&"a".repeat(259)));
        assert!(!l.add(&"a".repeat(260)));
        // counted in UTF-16 units: a character outside the BMP is two
        assert!(!l.add(&"😀".repeat(130)));
        assert!(l.add(&"😀".repeat(129)));
        assert_eq!(l.len(), 2);
    }

    #[test]
    fn an_empty_search_string_is_not_kept_but_an_empty_replacement_is() {
        let mut keys = SearchKeys::default();
        assert!(!keys.add_search(""));
        assert!(keys.search.is_empty());
        assert!(keys.add_replace(""));
        assert_eq!(keys.replace.items(), [""]);
    }

    fn sample() -> SearchKeys {
        let mut keys = SearchKeys::default();
        for s in [
            "b+",
            "検索 文字列",
            "a=b;c",
            "[x]",
            " lead",
            "trail ",
            "^\\s*$",
        ] {
            keys.add_search(s);
        }
        keys.add_replace("置換後");
        keys.add_replace("");
        keys.add_replace("\\1\\t");
        keys
    }

    #[test]
    fn the_strings_survive_the_file() {
        let keys = sample();
        let mut profile = Profile::new();
        keys.write_to(&mut profile).unwrap();
        let back = Profile::from_bytes(&profile.to_bytes());
        assert_eq!(SearchKeys::from_profile(&back), keys);
    }

    #[test]
    fn the_file_has_sakuras_keys() {
        let mut keys = SearchKeys::default();
        keys.add_search("old");
        keys.add_search("b+");
        keys.add_replace("x");
        let mut profile = Profile::new();
        keys.write_to(&mut profile).unwrap();
        assert_eq!(profile.get("Keys", "_SEARCHKEY_Counts"), Some("2"));
        assert_eq!(profile.get("Keys", "SEARCHKEY[00]"), Some("b+"));
        assert_eq!(profile.get("Keys", "SEARCHKEY[01]"), Some("old"));
        assert_eq!(profile.get("Keys", "_REPLACEKEY_Counts"), Some("1"));
        assert_eq!(profile.get("Keys", "REPLACEKEY[00]"), Some("x"));
    }

    #[test]
    fn a_sakura_file_is_read() {
        let text = "[Keys]\r\n_SEARCHKEY_Counts=2\r\nSEARCHKEY[00]=foo\r\nSEARCHKEY[01]=bar\r\nSEARCHKEY[02]=ignored\r\nREPLACEKEY[00]=one\r\nREPLACEKEY[01]=two\r\n\r\n[Other]\r\nk=v\r\n";
        let keys = SearchKeys::from_profile(&Profile::from_bytes(text.as_bytes()));
        assert_eq!(keys.search.items(), ["foo", "bar"]);
        // no count: the places are read and the empty ones at the end are dropped
        assert_eq!(keys.replace.items(), ["one", "two"]);
    }

    #[test]
    fn a_wrong_count_is_corrected() {
        let text =
            "[Keys]\r\n_SEARCHKEY_Counts=-5\r\n_REPLACEKEY_Counts=99\r\nREPLACEKEY[00]=a\r\n";
        let keys = SearchKeys::from_profile(&Profile::from_bytes(text.as_bytes()));
        assert!(keys.search.is_empty());
        assert_eq!(keys.replace.len(), 30);
        assert_eq!(keys.replace.items()[0], "a");
    }

    #[test]
    fn no_section_means_no_strings() {
        let keys = SearchKeys::from_profile(&Profile::new());
        assert!(keys.search.is_empty() && keys.replace.is_empty());
    }

    #[test]
    fn empty_search_strings_in_the_file_are_not_shown() {
        // Sakura's Grep leaves "" in the list.
        let text = "[Keys]\r\n_SEARCHKEY_Counts=3\r\nSEARCHKEY[00]=\r\nSEARCHKEY[01]=a\r\nSEARCHKEY[02]=\r\n";
        let keys = SearchKeys::from_profile(&Profile::from_bytes(text.as_bytes()));
        assert_eq!(keys.search.items(), ["a"]);
    }

    #[test]
    fn writing_leaves_the_rest_of_the_file_alone() {
        let text = "[Other]\r\nk=  v\r\n\r\n[Keys]\r\n_SEARCHKEY_Counts=1\r\nSEARCHKEY[00]=old\r\n\r\n[Last]\r\nz=1\r\n";
        let mut profile = Profile::from_bytes(text.as_bytes());
        let mut keys = SearchKeys::from_profile(&profile);
        keys.add_search("new");
        keys.write_to(&mut profile).unwrap();
        let out = String::from_utf8(profile.to_bytes()).unwrap();
        // the keywords of the tag jump are written to [Other], after what was there
        assert!(out.starts_with("[Other]\r\nk=  v\r\n"));
        assert!(out.contains("\r\n[Keys]\r\n"));
        assert!(out.contains("\r\n[Last]\r\nz=1\r\n"));
        assert_eq!(profile.get("Keys", "SEARCHKEY[00]"), Some("new"));
        assert_eq!(profile.get("Keys", "SEARCHKEY[01]"), Some("old"));
    }

    #[test]
    fn a_string_with_a_line_break_is_skipped_in_the_file() {
        let mut keys = SearchKeys::default();
        keys.add_search("a\nb");
        keys.add_search("ok");
        let mut profile = Profile::new();
        keys.write_to(&mut profile).unwrap();
        assert_eq!(profile.get("Keys", "_SEARCHKEY_Counts"), Some("1"));
        assert_eq!(profile.get("Keys", "SEARCHKEY[00]"), Some("ok"));
    }

    // ---- the Grep boxes and the tag jump keywords ----

    #[test]
    fn the_grep_boxes_ignore_case_and_the_search_strings_do_not() {
        let mut keys = SearchKeys::default();
        keys.add_grep_file("*.TXT");
        keys.add_grep_file("*.rs");
        // the same string in another case moves to the front and takes the new case
        assert!(keys.add_grep_file("*.txt"));
        assert_eq!(keys.grep_file.items(), ["*.txt", "*.rs"]);
        // only the case differs from the first: the list still changes
        assert!(keys.add_grep_file("*.TXT"));
        assert_eq!(keys.grep_file.items(), ["*.TXT", "*.rs"]);
        assert!(!keys.add_grep_file("*.TXT"));
        keys.add_search("Abc");
        keys.add_search("abc");
        assert_eq!(keys.search.items(), ["abc", "Abc"]);
        keys.add_tag_keyword("Main");
        keys.add_tag_keyword("main");
        assert_eq!(keys.tag_keyword.items(), ["main", "Main"]);
    }

    #[test]
    fn the_limits_of_the_grep_boxes_and_the_keywords() {
        let mut keys = SearchKeys::default();
        for i in 0..40 {
            keys.add_grep_folder(&format!("C:\\dir{i}"));
            keys.add_exclude_file(&format!("x{i}"));
            keys.add_exclude_folder(&format!("y{i}"));
            keys.add_tag_keyword(&format!("k{i}"));
        }
        assert_eq!(keys.grep_folder.len(), 30);
        assert_eq!(keys.exclude_file.len(), 30);
        assert_eq!(keys.exclude_folder.len(), 30);
        assert_eq!(keys.tag_keyword.len(), 30);
        assert_eq!(keys.tag_keyword.newest(), Some("k39"));
        assert_eq!(keys.tag_keyword.items()[29], "k10");
        // MAX_GREP_PATH is 512, the keyword is _MAX_PATH
        assert!(keys.add_grep_file(&"a".repeat(511)));
        assert!(!keys.add_grep_file(&"a".repeat(512)));
        assert!(keys.add_tag_keyword(&"a".repeat(259)));
        assert!(!keys.add_tag_keyword(&"a".repeat(260)));
    }

    #[test]
    fn an_empty_string_is_not_kept_in_the_grep_boxes() {
        let mut keys = SearchKeys::default();
        assert!(!keys.add_grep_file("") && !keys.add_grep_folder(""));
        assert!(!keys.add_exclude_file("") && !keys.add_exclude_folder(""));
        assert!(!keys.add_tag_keyword(""));
    }

    #[test]
    fn the_grep_lists_have_sakuras_sections_and_keys() {
        let mut keys = SearchKeys::default();
        keys.add_grep_file("*.c");
        keys.add_grep_file("*.h");
        keys.add_grep_folder("C:\\src");
        keys.add_exclude_file("*.obj");
        keys.add_exclude_folder("Debug");
        keys.add_tag_keyword("main");
        let mut profile = Profile::new();
        keys.write_to(&mut profile).unwrap();
        assert_eq!(profile.get("Grep", "_GREPFILE_Counts"), Some("2"));
        assert_eq!(profile.get("Grep", "GREPFILE[00]"), Some("*.h"));
        assert_eq!(profile.get("Grep", "GREPFILE[01]"), Some("*.c"));
        assert_eq!(profile.get("Grep", "GREPFOLDER[00]"), Some("C:\\src"));
        assert_eq!(profile.get("Grep", "GREPEXCLUDEFILE[00]"), Some("*.obj"));
        assert_eq!(profile.get("Grep", "_GREPEXCLUDEFOLDER_Counts"), Some("1"));
        assert_eq!(profile.get("Grep", "GREPEXCLUDEFOLDER[00]"), Some("Debug"));
        assert_eq!(profile.get("Other", "_TagJumpKeyword_Counts"), Some("1"));
        assert_eq!(profile.get("Other", "TagJumpKeyword[00]"), Some("main"));
        let back = Profile::from_bytes(&profile.to_bytes());
        assert_eq!(SearchKeys::from_profile(&back), keys);
    }

    #[test]
    fn a_sakura_file_with_grep_histories_is_read() {
        let text = "[Grep]\r\n_GREPFILE_Counts=2\r\nGREPFILE[00]=*.cpp;*.h\r\nGREPFILE[01]=*.*\r\n\
            GREPFOLDER[00]=C:\\work\r\nGREPFOLDER[01]=\r\n_GREPEXCLUDEFILE_Counts=2\r\n\
            GREPEXCLUDEFILE[00]=\r\nGREPEXCLUDEFILE[01]=*.obj\r\n\
            [Other]\r\nnExecFlgOpt=1\r\n_TagJumpKeyword_Counts=1\r\nTagJumpKeyword[00]=WinMain\r\n\
            m_bTagJumpICase=1\r\n";
        let keys = SearchKeys::from_profile(&Profile::from_bytes(text.as_bytes()));
        assert_eq!(keys.grep_file.items(), ["*.cpp;*.h", "*.*"]);
        // no count: the places are read and the empty ones at the end are dropped
        assert_eq!(keys.grep_folder.items(), ["C:\\work"]);
        // a blank line is not shown
        assert_eq!(keys.exclude_file.items(), ["*.obj"]);
        assert!(keys.exclude_folder.is_empty());
        assert_eq!(keys.tag_keyword.items(), ["WinMain"]);
    }

    #[test]
    fn the_grep_lists_leave_the_rest_of_the_file_alone() {
        let text = "[Other]\r\nm_bTagJumpICase=1\r\n\r\n[Grep]\r\nGREPFILE[00]=old\r\n\r\n[Last]\r\nz=1\r\n";
        let mut profile = Profile::from_bytes(text.as_bytes());
        let mut keys = SearchKeys::from_profile(&profile);
        keys.add_grep_file("new");
        keys.add_tag_keyword("kw");
        keys.write_to(&mut profile).unwrap();
        assert_eq!(profile.get("Other", "m_bTagJumpICase"), Some("1"));
        assert_eq!(profile.get("Grep", "GREPFILE[00]"), Some("new"));
        assert_eq!(profile.get("Grep", "GREPFILE[01]"), Some("old"));
        assert_eq!(profile.get("Last", "z"), Some("1"));
    }

    #[test]
    fn merging_takes_in_the_grep_lists_too() {
        let mut ours = SearchKeys::default();
        ours.add_grep_folder("A");
        ours.add_tag_keyword("local");
        let mut theirs = SearchKeys::default();
        theirs.add_grep_folder("a");
        theirs.add_grep_folder("B");
        theirs.add_tag_keyword("remote");
        ours.merge_from(&theirs);
        assert_eq!(ours.grep_folder.items(), ["B", "a"]);
        assert_eq!(ours.tag_keyword.items(), ["remote", "local"]);
    }
}
