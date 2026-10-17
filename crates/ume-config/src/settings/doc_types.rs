// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CDocTypeManager.cpp (GetDocumentTypeOfPath, GetDocumentTypeOfExt, IsFileNameMatch)
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_Types: the number of types and the types that are
//     added to the built-in ones)
//
// Original copyright notices of those files:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The document types (タイプ別設定) of a settings file, and which type a file has.
//!
//! [`DocumentTypes`] is the list of types in the order of the index `N` of `Types(N)`: the 17
//! built-in types of Sakura ([`builtin_types`]) with what the file `ume.ini` overrides, then the
//! types the file adds, and UME Editor's own types ([`extra_types`](super::extra_types), ADR-0017) in the
//! numbers after 17 that the file does not use. It also has the [keyword sets](KeywordSets) that the types refer to.
//!
//! # For the coloring engine
//!
//! Take the type of a document with [`DocumentTypes::type_index_for_file_name`] (or the user's
//! choice), then read the rules from [`DocumentTypes::get`]:
//!
//! * comments: [`TypeSettings::line_comments`] and [`TypeSettings::block_comments`]
//!   (matching rules in [`LineComment`](super::LineComment) and [`BlockComment`](super::BlockComment));
//!   the color is the entry `CMT` of [`TypeSettings::colors`]
//! * strings: [`TypeSettings::string_type`], [`TypeSettings::string_line_only`],
//!   [`TypeSettings::string_end_line`]; the colors `SQT` and `WQT` (a string is colored only when its
//!   entry is shown, `disp`)
//! * numbers: [`TypeSettings::colors_numbers`] (the entry `NUM`)
//! * keywords: [`DocumentTypes::keyword_sets_of`], the sets the type chose, with the case rule;
//!   the entry of the n-th chosen set is `KW1` to `KWA`
//! * regular expression keywords: [`TypeSettings::use_regex_keyword`], [`TypeSettings::regex_keywords`]
//! * vertical lines: [`TypeSettings::vert_lines`]
//! * the outline: [`TypeSettings::default_outline`]
//!
//! # Which type a file has
//!
//! `GetDocumentTypeOfPath`: the types are tried in order from type 0; the first whose extension
//! list (`szTypeExts`) matches the file name wins. A file that matches no type is type 0, "基本".
//! "テキスト" (`txt`) comes before the others, so `.log` is テキスト even though TeX lists it too.

use super::builtin_types::{BUILTIN_TYPE_COUNT, added_type_name, builtin_types};
use super::extra_types::{ExtraSet, extra_types};
use super::keywords::{KeywordSet, KeywordSets};
use super::types::{TypeSettings, types_count};
use crate::profile::Profile;

/// Separators of the extension list (`m_typeExtSeps`).
const EXT_SEPARATORS: [char; 3] = [' ', ';', ','];
/// Wildcards (`m_typeExtWildcards`).
const WILDCARDS: [char; 2] = ['*', '?'];

/// `IsFileNameMatch`: does `file_name` match the extension list `type_exts` (`"c cpp;h"`,
/// `"*.txt"`, `"Makefile"`)?
///
/// A token without wildcards matches the whole file name, or the text after the last `.`; a token
/// with wildcards is a pattern for the whole file name (`PathMatchSpec`). Case does not matter.
pub fn is_file_name_match(type_exts: &str, file_name: &str) -> bool {
    let name = lower(file_name);
    let ext = name.rfind('.').map(|i| &name[i + 1..]);
    type_exts
        .split(EXT_SEPARATORS)
        .filter(|t| !t.is_empty())
        .any(|token| {
            let token = lower(token);
            if token.contains(WILDCARDS) {
                glob_match(&token, &name)
            } else {
                token == name || ext == Some(token.as_str())
            }
        })
}

fn lower(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

/// `*` (any run of characters, also none) and `?` (one character) against the whole of `text`.
fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0, 0);
    // Where the last `*` was in the pattern, and the text position it matched up to.
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        match p.get(pi) {
            Some('*') => {
                star = Some((pi, ti));
                pi += 1;
            }
            Some(&c) if c == '?' || c == t[ti] => {
                pi += 1;
                ti += 1;
            }
            _ => match star {
                Some((sp, st)) => {
                    star = Some((sp, st + 1));
                    pi = sp + 1;
                    ti = st + 1;
                }
                None => return false,
            },
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

/// Does the settings file leave the type number `index` unused? That is the case when it has no
/// `Types(index)` section, or the section is what Sakura writes for a number it does not use: the name
/// `設定N` (or none) and no extensions.
fn is_unused_slot(profile: &Profile, index: i32) -> bool {
    let section = TypeSettings::section_name(index);
    if profile.entries(&section).next().is_none() {
        return true;
    }
    let no_exts = profile
        .get(&section, "szTypeExts")
        .is_none_or(|v| v.trim().is_empty());
    let placeholder_name = profile
        .get(&section, "szTypeName")
        .is_none_or(|v| v == added_type_name(index));
    no_exts && placeholder_name
}

/// The index of the first type whose extension list matches `file_name`, else 0
/// (`GetDocumentTypeOfPath`). `type_exts` has the extension list of each type, in the order of
/// the types.
pub fn type_index_for<'a>(type_exts: impl IntoIterator<Item = &'a str>, file_name: &str) -> usize {
    type_exts
        .into_iter()
        .position(|exts| is_file_name_match(exts, file_name))
        .unwrap_or(0)
}

/// All document types and keyword sets of the settings.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentTypes {
    types: Vec<TypeSettings>,
    keywords: KeywordSets,
}

impl DocumentTypes {
    /// What the editor has without a settings file: Sakura's 17 types and keyword sets, UME Editor's own types
    /// and sets after them (ADR-0017), and copies of the basic type up to 30 types (as for a file without
    /// `nTypesCount`).
    pub fn builtin() -> Self {
        Self::from_profile(&Profile::new())
    }

    /// The types of a settings file. Like Sakura (`ShareData_IO_Types`), the number of types is
    /// `nTypesCount` of `[Other]` (30 when the file has none, at most 60); type `N` starts as
    /// the built-in type `N` if there is one, else as a copy of the basic type called `設定N`;
    /// then the keys of `Types(N)` are read over it. A key the file does not have keeps the
    /// built-in value, so a file can override one setting of one type, or add types.
    ///
    /// ADR-0017: the numbers from 17 on that the file does not use for a type of its own (no `Types(N)`
    /// section, or the placeholder Sakura writes: the name `設定N` and no extensions) get UME Editor's own
    /// types ([`ExtraSet`] has their keyword sets), in order, as long as there are numbers; such a type is
    /// not read over, because a Sakura file writes an empty section there. A number the file uses is that
    /// type, as in Sakura.
    pub fn from_profile(profile: &Profile) -> Self {
        let count = usize::try_from(types_count(profile)).unwrap_or(1);
        let keywords = KeywordSets::from_profile(profile);
        let builtin = builtin_types();
        let mut extras = extra_types(&keywords).into_iter();
        let types = (0..count)
            .map(|i| {
                let index = i32::try_from(i).unwrap_or(0);
                if i >= BUILTIN_TYPE_COUNT
                    && is_unused_slot(profile, index)
                    && let Some(mut t) = extras.next()
                {
                    t.idx = index;
                    t.id = index;
                    return t;
                }
                let mut t = match builtin.get(i) {
                    Some(t) => {
                        let mut t = t.clone();
                        // A built-in type may use one of UME Editor's sets: the file may have put it elsewhere.
                        for slot in &mut t.keyword_set_idx {
                            if let Some(set) = ExtraSet::from_default_index(*slot) {
                                *slot = keywords.extra_index(set);
                            }
                        }
                        t
                    }
                    None => TypeSettings {
                        idx: index,
                        id: index,
                        type_name: added_type_name(index),
                        ..builtin[0].clone()
                    },
                };
                t.read_over(profile, index);
                // ShareData_IO_Types: the index in the file is the index in the list.
                t.idx = index;
                t
            })
            .collect();
        Self { types, keywords }
    }

    /// How many types there are.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// The type with this index.
    pub fn get(&self, index: usize) -> Option<&TypeSettings> {
        self.types.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &TypeSettings> {
        self.types.iter()
    }

    /// The index of the type called `name` (the first, if two have the same name).
    pub fn index_of_name(&self, name: &str) -> Option<usize> {
        self.types.iter().position(|t| t.type_name == name)
    }

    /// `GetDocumentTypeOfPath`: the type of a file called `file_name` (the name only, no folder).
    pub fn type_index_for_file_name(&self, file_name: &str) -> usize {
        type_index_for(self.types.iter().map(|t| t.type_exts.as_str()), file_name)
    }

    /// `GetDocumentTypeOfExt`: the type of the files with extension `ext`. Sakura gives the
    /// extension to the same function as a file name, so `cpp` and `.cpp` both work (a token
    /// without wildcards matches the whole text, or what follows the last dot).
    pub fn type_index_for_ext(&self, ext: &str) -> usize {
        self.type_index_for_file_name(ext)
    }

    /// All keyword sets.
    pub fn keywords(&self) -> &KeywordSets {
        &self.keywords
    }

    /// The keyword sets type `index` uses, with the slot (0 to 9; the color is `KW1` to `KWA`) of
    /// each. A slot that is `-1` or points at a set that does not exist is left out.
    pub fn keyword_sets_of(&self, index: usize) -> Vec<(usize, &KeywordSet)> {
        let Some(t) = self.get(index) else {
            return Vec::new();
        };
        t.keyword_set_idx
            .iter()
            .enumerate()
            .filter_map(|(slot, &i)| self.keywords.get(i).map(|set| (slot, set)))
            .collect()
    }

    /// The number of Sakura's built-in types (UME Editor's own types and the types of the file come after them).
    pub const BUILTIN_COUNT: usize = BUILTIN_TYPE_COUNT;
}

impl Default for DocumentTypes {
    fn default() -> Self {
        Self::builtin()
    }
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
    fn a_plain_token_matches_the_extension_or_the_whole_name() {
        assert!(is_file_name_match("txt", "memo.txt"));
        assert!(is_file_name_match("txt", "MEMO.TXT"));
        assert!(is_file_name_match("Makefile", "makefile"));
        assert!(!is_file_name_match("txt", "memo.txt.bak"));
        assert!(!is_file_name_match("txt", "txt.bak"));
        // `szTypeExts` is not "a dot and an extension": ".txt" does not match "memo.txt".
        assert!(!is_file_name_match(".txt", "memo.txt"));
    }

    #[test]
    fn the_separators_are_space_semicolon_and_comma() {
        for exts in ["c cpp h", "c;cpp;h", "c,cpp,h", "c ;, cpp"] {
            assert!(is_file_name_match(exts, "x.cpp"), "{exts}");
            assert!(!is_file_name_match(exts, "x.rs"), "{exts}");
        }
        assert!(!is_file_name_match("", "x.cpp"));
    }

    #[test]
    fn a_double_extension_matches_its_last_part() {
        // Sakura compares with the text after the last dot only.
        assert!(is_file_name_match("gz", "a.tar.gz"));
        assert!(!is_file_name_match("tar", "a.tar.gz"));
    }

    #[test]
    fn wildcards_match_the_whole_name() {
        assert!(is_file_name_match("*.txt", "memo.txt"));
        assert!(is_file_name_match("memo?.txt", "memo1.txt"));
        assert!(!is_file_name_match("memo?.txt", "memo.txt"));
        assert!(is_file_name_match("*", "anything"));
        assert!(is_file_name_match("a*b*c", "aXXbYYc"));
        assert!(!is_file_name_match("a*b*c", "aXXbYY"));
        assert!(is_file_name_match("README*", "readme"));
        assert!(!is_file_name_match("*.txt", "memo.txt.bak"));
    }

    #[test]
    fn the_first_matching_type_wins_and_type_zero_is_the_fallback() {
        let types = ["", "txt doc", "c cpp h", "*.h"];
        assert_eq!(type_index_for(types, "a.txt"), 1);
        assert_eq!(type_index_for(types, "a.h"), 2);
        assert_eq!(type_index_for(types, "a.rs"), 0);
        assert_eq!(type_index_for([], "a.rs"), 0);
        // Type 0 is tried too.
        assert_eq!(type_index_for(["rs", "txt"], "a.rs"), 0);
    }

    #[test]
    fn the_builtin_types_are_chosen_by_extension() {
        let types = DocumentTypes::builtin();
        assert_eq!(types.len(), 30);
        let name_of = |file: &str| {
            types
                .get(types.type_index_for_file_name(file))
                .unwrap()
                .type_name
                .clone()
        };
        assert_eq!(name_of("memo.txt"), "テキスト");
        assert_eq!(name_of("a.cpp"), "C/C++");
        assert_eq!(name_of("a.h"), "C/C++");
        assert_eq!(name_of("index.html"), "HTML");
        assert_eq!(name_of("a.py"), "Python");
        assert_eq!(name_of("run.bat"), "MS-DOSバッチファイル");
        assert_eq!(name_of("setup.ini"), "設定ファイル");
        // `.log` is in テキスト and in TeX: the first wins.
        assert_eq!(name_of("build.log"), "テキスト");
        assert_eq!(name_of("paper.aux"), "TeX");
    }

    #[test]
    fn case_does_not_matter_and_unknown_extensions_are_the_basic_type() {
        let types = DocumentTypes::builtin();
        assert_eq!(types.type_index_for_file_name("MAIN.CPP"), 2);
        assert_eq!(types.type_index_for_file_name("Main.Java"), 6);
        assert_eq!(types.type_index_for_file_name("data.xyz"), 0);
        assert_eq!(types.type_index_for_file_name("noextension"), 0);
        assert_eq!(types.type_index_for_file_name(""), 0);
        // The ext form of the macro function: with or without the dot.
        assert_eq!(types.type_index_for_ext("cpp"), 2);
        assert_eq!(types.type_index_for_ext(".cpp"), 2);
    }

    #[test]
    fn a_file_without_types_has_the_builtin_ones_umes_own_and_added_copies_up_to_30() {
        // An old file with no `nTypesCount`: 30 types.
        let types = DocumentTypes::from_profile(&profile("[Common]\r\nbDispSTATUSBAR=1\r\n"));
        assert_eq!(types.len(), 30);
        assert_eq!(types.get(2).unwrap().type_name, "C/C++");
        // 17 to 26 are UME Editor's own types (ADR-0017), the rest copies of the basic type.
        assert_eq!(types.get(17).unwrap().type_name, "JavaScript");
        assert_eq!(types.get(17).unwrap().idx, 17);
        assert_eq!(types.get(27).unwrap().type_name, "設定27");
        assert_eq!(types.get(27).unwrap().type_exts, "");
        assert_eq!(types.get(27).unwrap().idx, 27);
        assert_eq!(types.type_index_for_file_name("a.cpp"), 2);
    }

    #[test]
    fn the_file_overrides_one_setting_of_a_builtin_type() {
        let types = DocumentTypes::from_profile(&profile(
            "[Other]\r\nnTypesCount=17\r\n[Types(2)]\r\nszTypeExts=c,h,inc\r\nszLineComment=#\r\n\
             nLineCommentColumn=-1\r\n",
        ));
        let cpp = types.get(2).unwrap();
        assert_eq!(cpp.type_exts, "c,h,inc");
        assert_eq!(cpp.line_comments[0].delimiter, "#");
        // What the file does not say stays as the built-in type has it.
        assert_eq!(cpp.block_comments[1].from, "#if 0");
        assert_eq!(cpp.keyword_set_idx[0], 0);
        assert_eq!(types.type_index_for_file_name("x.inc"), 2);
        // `cpp` is no longer in the list.
        assert_eq!(types.type_index_for_file_name("x.cpp"), 0);
    }

    #[test]
    fn the_file_adds_a_type() {
        let types = DocumentTypes::from_profile(&profile(
            "[Other]\r\nnTypesCount=18\r\n[Types(17)]\r\nszTypeName=Rust\r\nszTypeExts=rs\r\n\
             szLineComment=//\r\nnLineCommentColumn=-1\r\nnInts=17,10240,0,4,-1,-1,0,1,1,0,0,0\r\n",
        ));
        assert_eq!(types.len(), 18);
        let rust = types.get(17).unwrap();
        assert_eq!(rust.type_name, "Rust");
        assert_eq!(
            rust.line_comments[0],
            super::super::LineComment::new("//", -1)
        );
        assert_eq!(types.type_index_for_file_name("lib.rs"), 17);
        assert_eq!(types.index_of_name("Rust"), Some(17));
        assert_eq!(types.index_of_name("nope"), None);
    }

    #[test]
    fn the_keyword_sets_of_a_type_come_with_their_slot() {
        let types = DocumentTypes::builtin();
        let tex = types.keyword_sets_of(11);
        assert_eq!(tex.len(), 2);
        assert_eq!(tex[0].0, 0);
        assert_eq!(tex[0].1.name, "TeX");
        assert_eq!(tex[1].0, 1);
        assert_eq!(tex[1].1.name, "TeX2");
        assert!(types.keyword_sets_of(0).is_empty());
        assert!(types.keyword_sets_of(99).is_empty());
    }

    #[test]
    fn the_keyword_sets_of_the_file_replace_the_builtin_ones_for_all_types() {
        let types = DocumentTypes::from_profile(&profile(
            "[Other]\r\nnTypesCount=17\r\n[KeyWords]\r\nnKeyWordSetNum=1\r\nszSN[00]=Mine\r\n\
             nCASE[00]=1\r\nnKWN[00]=1\r\nszKW[00]=foo\t\r\n",
        ));
        // Type 2 (C/C++) uses set 0, now the file's set.
        let sets = types.keyword_sets_of(2);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].1.name, "Mine");
        assert!(sets[0].1.contains("foo"));
        // Type 11 (TeX) uses 9 and 10, which do not exist any more.
        assert!(types.keyword_sets_of(11).is_empty());
    }
}
