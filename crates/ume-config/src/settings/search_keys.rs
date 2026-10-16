// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/recent/CRecentImp.cpp (AppendItem), sakura_core/recent/CRecentSearch.cpp,
//   sakura_core/env/CSearchKeywordManager.cpp, sakura_core/env/CShareData_IO.cpp (ShareData_IO_Keys),
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

//! The history of the search and replace strings: the section `[Keys]` (`SEARCHKEY`, `REPLACEKEY`).
//!
//! Sakura keeps the strings of the find, replace and Grep dialogs in the shared data
//! (`m_sSearchKeywords.m_aSearchKeys`, `m_aReplaceKeys`), newest first, and writes them to `[Keys]`
//! as `_SEARCHKEY_Counts=N` and `SEARCHKEY[00]=...` (`ShareData_IO_Keys`, `IOProfileDataSet`).
//! [`KeyList`] is such a list, [`SearchKeys`] is the pair of them and reads and writes the section.
//! Everything else in the file is left as it is.

use crate::profile::{Profile, ProfileError};
use crate::value::parse_long;

/// The section of the file.
pub const SECTION: &str = "Keys";
/// `MAX_SEARCHKEY` (`sakura_core/config/maxdata.h`).
pub const MAX_SEARCH_KEYS: usize = 30;
/// `MAX_REPLACEKEY`.
pub const MAX_REPLACE_KEYS: usize = 30;
/// `_MAX_PATH`: a string must be shorter than this many UTF-16 units to be kept
/// (`ValidateReceiveType`, and the `size() < _MAX_PATH` checks of the callers).
pub const MAX_KEY_UNITS: usize = 260;

/// A history: newest first, at most `max` strings, none twice (`CRecentImp`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyList {
    items: Vec<String>,
    max: usize,
}

impl KeyList {
    pub fn new(max: usize) -> Self {
        Self {
            items: Vec::new(),
            max,
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

    /// Whether `text` can be kept: shorter than `_MAX_PATH` UTF-16 units.
    pub fn accepts(text: &str) -> bool {
        text.encode_utf16().count() < MAX_KEY_UNITS
    }

    /// `CRecentImp::AppendItem`: puts `text` first. A string that is already there moves to the
    /// front; when the list is full the oldest one goes. Returns whether the list changed (a
    /// string that was already first leaves it as it was), or `false` for a string that is too long.
    pub fn add(&mut self, text: &str) -> bool {
        if !Self::accepts(text) {
            return false;
        }
        if let Some(i) = self.items.iter().position(|s| s == text) {
            if i == 0 {
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

/// The search strings and the replace strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchKeys {
    pub search: KeyList,
    pub replace: KeyList,
}

impl Default for SearchKeys {
    fn default() -> Self {
        Self {
            search: KeyList::new(MAX_SEARCH_KEYS),
            replace: KeyList::new(MAX_REPLACE_KEYS),
        }
    }
}

impl SearchKeys {
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

    /// Reads `[Keys]`. A missing section or entry means no strings.
    pub fn from_profile(profile: &Profile) -> Self {
        let mut keys = Self::default();
        keys.search.items = read_set(profile, "SEARCHKEY", MAX_SEARCH_KEYS)
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        keys.replace.items = read_set(profile, "REPLACEKEY", MAX_REPLACE_KEYS);
        keys
    }

    /// Writes `[Keys]` (the count and the strings), leaving the rest of the file alone. A string
    /// the file format cannot hold (a line break) is skipped.
    pub fn write_to(&self, profile: &mut Profile) -> Result<(), ProfileError> {
        write_set(profile, "SEARCHKEY", self.search.items())?;
        write_set(profile, "REPLACEKEY", self.replace.items())
    }
}

/// `IOProfileDataSet`, reading: `_<name>_Counts` is the number of strings (limited to `max`); if
/// it is missing all `max` places are read and the empty ones at the end are dropped.
fn read_set(profile: &Profile, name: &str, max: usize) -> Vec<String> {
    let count_key = format!("_{name}_Counts");
    let (count, missing) = match profile.get(SECTION, &count_key).and_then(parse_long) {
        Some(n) => (usize::try_from(n.max(0)).unwrap_or(0).min(max), false),
        None => (max, true),
    };
    let mut items: Vec<String> = (0..count)
        .map(|i| {
            profile
                .get(SECTION, &format!("{name}[{i:02}]"))
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
fn write_set(profile: &mut Profile, name: &str, items: &[String]) -> Result<(), ProfileError> {
    let items: Vec<&String> = items.iter().filter(|s| !s.contains(['\r', '\n'])).collect();
    let count = i32::try_from(items.len()).unwrap_or(i32::MAX);
    profile.set_i32(SECTION, &format!("_{name}_Counts"), count)?;
    for (i, item) in items.iter().enumerate() {
        profile.set(SECTION, &format!("{name}[{i:02}]"), item)?;
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
        assert!(out.starts_with("[Other]\r\nk=  v\r\n\r\n[Keys]\r\n"));
        assert!(out.ends_with("\r\n[Last]\r\nz=1\r\n"));
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
}
