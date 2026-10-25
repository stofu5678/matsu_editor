// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/typeprop/CImpExpManager.cpp (CImpExpType::ImportAscertain, Import, Export; CImpExpColors)
//   sakura_core/typeprop/CDlgTypeList.cpp (Import, Export)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, genta
//   Copyright (C) 2010, Uchi, Moca
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

//! The files of the type list: the settings of one type (`.ini`) and the colors of a type (`.col`), as
//! Sakura writes and reads them (`CImpExpType`, `CImpExpColors`), so that the files can be exchanged.
//!
//! # The type file
//!
//! A settings file with the first line `; タイプ別設定 Ver1` and the sections
//! * `[Info]`: `szVersion` (the version of the editor that wrote it) and `vStructureVersion` (the version of
//!   the settings structure; the importer asks when it differs from its own),
//! * `[Types]`: the same keys as a `Types(N)` section,
//! * `[TypeEx]`: for each of the keyword sets of the type (`n` = 1 to 10) the name `szKeyword[n]`, the file
//!   `szKeywordFile[n]` (a keyword definition file, `.kwd`, next to the type file) and the case rule
//!   `szKeywordCase[n]`. The type refers to the sets by name, because the numbers differ between the
//!   settings of two users.
//!
//! Not built: the outline and the smart indent of a plugin (`szPluginOutlineId`, `szPluginSmartIndentId`);
//! MATSU Editor has no plugin table to find them in.
//!
//! # The color file
//!
//! A settings file with the first line `; テキストエディタ色設定 Ver3` and the section `[SakuraColor]` with
//! the keys `C[NAME]` of a type's `Types(N)` section.

use super::color::ColorSet;
use super::keyword_file;
use super::keywords::{
    KeywordSets, MAX_KEYWORD_LEN, MAX_KEYWORD_SETS, MAX_SET_NAME_LEN, SetDraft, store_sets,
};
use super::type_list::{Entry, ListError, check_count, load, new_id, store, write_entry};
use super::types::TypeSettings;
use crate::profile::{Profile, ProfileError};

/// `vStructureVersion` of the files MATSU Editor writes: `N_SHAREDATA_VERSION` of the Sakura whose type files
/// MATSU Editor reads and writes (`sakura_core/config/system_constants.h`).
pub const STRUCTURE_VERSION: i32 = 183;

/// `WSTR_TYPE_HEAD`
const TYPE_HEADER: &str = " タイプ別設定 Ver1";
/// `WSTR_COLORDATA_HEAD3`
const COLOR_HEADER: &str = " テキストエディタ色設定 Ver3";
/// `szSecColor`
const COLOR_SECTION: &str = "SakuraColor";
/// `STR_TYPE_NAME_BASIS`
const BASIC_NAME: &str = "基本";
const INFO: &str = "Info";
const TYPES: &str = "Types";
const TYPE_EX: &str = "TypeEx";
/// The most keyword sets a type has.
const KEYWORD_SLOTS: usize = 10;
/// `Types(0)` of a temporary profile.
const TEMP: &str = "Types(0)";

/// `GetDefaultExtension` of `CImpExpType`.
pub const TYPE_WILDCARD: &str = "*.ini";
/// `GetDefaultExtension` of `CImpExpColors`.
pub const COLOR_WILDCARD: &str = "*.col";

/// A keyword definition file that goes with an exported type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordExport {
    /// The name of the file (`MakeExportFileName` of the name of the set).
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// What [`export_type`] makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedType {
    /// The type file.
    pub bytes: Vec<u8>,
    /// The keyword definition files, to be written next to the type file.
    pub keyword_files: Vec<KeywordExport>,
}

/// The name of the file the export offers for a type (`SetBaseName`).
pub fn export_file_name(type_name: &str) -> String {
    keyword_file::export_file_name_with(type_name, "ini")
}

/// `CImpExpType::Export`: the files of type `index`. `product_version` is written as `szVersion`.
pub fn export_type(
    profile: &Profile,
    index: usize,
    product_version: &str,
) -> Result<ExportedType, ListError> {
    let entries = load(profile);
    let entry = entries.get(index).ok_or(ListError::NoSuchType)?;
    let settings = &entry.settings;
    let mut temp = Profile::new();
    settings.write(&mut temp, 0)?;
    let mut out = Profile::new();
    for (key, value) in temp.entries(TEMP) {
        // The new-file keys the settings file does not set are not the type's own choice.
        if super::type_list::NEW_FILE_KEYS.contains(&key)
            && !entry.raw.iter().any(|(k, _)| k == key)
        {
            continue;
        }
        out.set(TYPES, key, value)?;
    }
    let sets = KeywordSets::from_profile(profile);
    let mut keyword_files = Vec::new();
    for (i, &set_index) in settings
        .keyword_set_idx
        .iter()
        .enumerate()
        .take(KEYWORD_SLOTS)
    {
        let Some(set) = sets.get(set_index) else {
            continue;
        };
        let n = i + 1;
        out.set(TYPE_EX, &format!("szKeyword[{n}]"), &set.name)?;
        let file_name = keyword_file::export_file_name(&set.name);
        let words: Vec<String> = set.words().to_vec();
        keyword_files.push(KeywordExport {
            bytes: keyword_file::render(&set.name, set.case_sensitive, &words),
            file_name: file_name.clone(),
        });
        out.set(TYPE_EX, &format!("szKeywordFile[{n}]"), &file_name)?;
        out.set_bool(TYPE_EX, &format!("szKeywordCase[{n}]"), set.case_sensitive)?;
    }
    out.set(INFO, "szVersion", product_version)?;
    out.set_i32(INFO, "vStructureVersion", STRUCTURE_VERSION)?;
    Ok(ExportedType {
        bytes: out.to_sakura_bytes(Some(TYPE_HEADER)),
        keyword_files,
    })
}

/// A type file that was read (`CImpExpType::ImportAscertain`).
#[derive(Debug, Clone)]
pub struct TypeFile {
    profile: Profile,
}

impl TypeFile {
    /// Reads a type file. A file without `vStructureVersion` is not one (`STR_IMPEXP_ERR_TYPE`).
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let profile = Profile::from_bytes(bytes);
        profile.get_i32(INFO, "vStructureVersion")?;
        Some(Self { profile })
    }

    /// `vStructureVersion`
    pub fn structure_version(&self) -> i32 {
        self.profile
            .get_i32(INFO, "vStructureVersion")
            .unwrap_or_default()
    }

    /// `szVersion`: the editor that wrote the file (`?` when the file does not say).
    pub fn version(&self) -> String {
        self.profile
            .get_string(INFO, "szVersion")
            .unwrap_or_else(|| "?".to_owned())
    }

    /// The name of the type in the file.
    pub fn type_name(&self) -> String {
        self.profile
            .get_string(TYPES, "szTypeName")
            .unwrap_or_default()
    }
}

/// Where an imported type goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A new type at the end of the list.
    Add,
    /// Replaces the type with this number.
    Overwrite(usize),
}

/// Where the colors of an imported type come from (`SAscertainInfo::nColorType`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Colors {
    /// The colors of the file (the first entry of the list, "そのままインポート").
    AsFile,
    /// The colors of the type with this number.
    OfType(usize),
    /// The colors of a color file that was read.
    Given(Box<ColorSet>),
}

/// What [`import_type`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    /// The number of the type.
    pub index: usize,
    /// The keyword definition files named by the type file, and whether each was read (`files` of `Import`).
    pub keyword_files: Vec<(String, bool)>,
}

/// `CImpExpType::Import` and `CDlgTypeList::Import`: puts the type of `file` at `target`.
///
/// The type starts as 基本 and the keys of the file are read over it. 基本 itself (number 0) gets the name 基本,
/// no extensions and the id 0; any other number gets a new id. Sakura decides this by the number the file has
/// (`m_nIdx`), so a file of any other type that is imported over 基本 would give 基本 a name and an id, and a
/// file of 基本 that is added would get id 0 twice; here the number the type is stored at decides.
///
/// The keyword sets are found by name in the sets of the user's settings; a set that is not there is added,
/// and `read_keyword_file` (given the name of the file in `szKeywordFile[n]`) tells what the file holds: its
/// words are added to the set, and its case rule is taken (`CImpExpKeyWord::Import`). A slot the file names no
/// set for gets none (Sakura keeps the number of the exporter's list, which means another set here).
pub fn import_type(
    profile: &mut Profile,
    file: &TypeFile,
    target: Target,
    colors: Colors,
    read_keyword_file: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<Imported, ListError> {
    let mut entries = load(profile);
    let old_len = entries.len();
    let index = match target {
        Target::Add => {
            check_count(old_len + 1)?;
            old_len
        }
        Target::Overwrite(i) if i < old_len => i,
        Target::Overwrite(_) => return Err(ListError::NoSuchType),
    };
    // "ベースのデータは基本"
    let basic = entries[0].settings.clone();
    let mut settings = basic.clone();
    let mut temp = Profile::new();
    for (key, value) in file.profile.entries(TYPES) {
        temp.set(TEMP, key, value)?;
    }
    settings.read_over(&temp, 0);
    if index == 0 {
        settings.type_name = BASIC_NAME.to_owned();
        settings.type_exts.clear();
        settings.id = 0;
    } else {
        settings.id = new_id(index, &entries);
    }
    // The colors. The "show" flag is the file's.
    let source = match colors {
        Colors::AsFile => None,
        Colors::OfType(i) => entries.get(i).map(|e| e.settings.colors.clone()),
        Colors::Given(set) => Some(*set),
    };
    if let Some(source) = source {
        for name in ColorSet::all_names() {
            if let (Some(from), Some(to)) = (source.get(name), settings.colors.get_mut(name)) {
                let disp = to.disp;
                *to = *from;
                to.disp = disp;
            }
        }
    }
    // The plugin functions of the exporter mean nothing here.
    if file.profile.get(TYPE_EX, "szPluginOutlineId").is_some() {
        settings.default_outline = basic.default_outline;
    }
    if file.profile.get(TYPE_EX, "szPluginSmartIndentId").is_some() {
        settings.smart_indent = basic.smart_indent;
    }
    let keyword_files = import_keyword_sets(profile, file, &mut settings, read_keyword_file)?;
    // The new-file keys the file sets stay set.
    let raw = super::type_list::NEW_FILE_KEYS
        .iter()
        .filter_map(|key| {
            file.profile
                .get(TYPES, key)
                .map(|value| ((*key).to_owned(), value.to_owned()))
        })
        .collect();
    let mut entry = Entry { settings, raw };
    if index == old_len {
        entries.push(entry);
        store(profile, old_len, &mut entries, old_len)?;
    } else {
        write_entry(profile, index, &mut entry)?;
    }
    Ok(Imported {
        index,
        keyword_files,
    })
}

/// The `[TypeEx]` part of the import: finds or adds the sets and puts their numbers in `settings`.
fn import_keyword_sets(
    profile: &mut Profile,
    file: &TypeFile,
    settings: &mut TypeSettings,
    read_keyword_file: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<Vec<(String, bool)>, ProfileError> {
    let mut drafts = KeywordSets::from_profile(profile).drafts();
    let before = drafts.clone();
    let mut files = Vec::new();
    for i in 0..KEYWORD_SLOTS {
        let n = i + 1;
        let Some(name) = file.profile.get(TYPE_EX, &format!("szKeyword[{n}]")) else {
            settings.keyword_set_idx[i] = -1;
            continue;
        };
        let name = limit_units(name, MAX_SET_NAME_LEN);
        let found = drafts.iter().position(|d| d.name == name);
        let at = match found {
            Some(at) => Some(at),
            None if drafts.len() < MAX_KEYWORD_SETS => {
                drafts.push(SetDraft {
                    name,
                    case_sensitive: false,
                    words: Vec::new(),
                });
                Some(drafts.len() - 1)
            }
            None => None,
        };
        settings.keyword_set_idx[i] = at.and_then(|a| i32::try_from(a).ok()).unwrap_or(-1);
        let (Some(at), Some(file_name)) = (
            at,
            file.profile.get(TYPE_EX, &format!("szKeywordFile[{n}]")),
        ) else {
            continue;
        };
        let case = file
            .profile
            .get_bool(TYPE_EX, &format!("szKeywordCase[{n}]"))
            .unwrap_or(false);
        match read_keyword_file(file_name) {
            Some(bytes) => {
                let words = keyword_file::parse(&bytes);
                let draft = &mut drafts[at];
                for word in words.words {
                    let word = limit_units(&word, MAX_KEYWORD_LEN);
                    if !draft.words.contains(&word) {
                        draft.words.push(word);
                    }
                }
                draft.case_sensitive = words.case_sensitive.unwrap_or(case);
                files.push((file_name.to_owned(), true));
            }
            None => files.push((file_name.to_owned(), false)),
        }
    }
    if drafts != before {
        store_sets(profile, &drafts)?;
    }
    Ok(files)
}

/// Cuts `s` to at most `max` UTF-16 units, at a character boundary.
fn limit_units(s: &str, max: usize) -> String {
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

/// `CImpExpColors::Export`: the bytes of a color file.
pub fn render_colors(colors: &ColorSet) -> Vec<u8> {
    let mut profile = Profile::new();
    // A new profile has nothing to compare with, so every entry is written.
    let _ = colors.write(&mut profile, COLOR_SECTION);
    profile.to_sakura_bytes(Some(COLOR_HEADER))
}

/// `CImpExpColors::Import`: the colors of a color file over `base` (an entry the file does not have keeps
/// the value of `base`). `None` when the first line is not the header of the current format
/// (`STR_IMPEXP_ERR_COLOR_OLD`).
pub fn parse_colors(bytes: &[u8], base: &ColorSet) -> Option<ColorSet> {
    let text = match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => String::from_utf8_lossy(rest).into_owned(),
        None => encoding_rs::SHIFT_JIS
            .decode_without_bom_handling(bytes)
            .0
            .into_owned(),
    };
    let first: Vec<char> = text
        .split(['\r', '\n'])
        .next()
        .unwrap_or_default()
        .chars()
        .collect();
    // The comment mark is one character (`;`) or two (`//`).
    let rest: String = if first.len() >= 2 {
        let skip = if first[0] == ';' { 1 } else { 2 };
        first[skip..].iter().collect()
    } else {
        first.iter().collect()
    };
    if rest != COLOR_HEADER {
        return None;
    }
    let profile = Profile::from_bytes(bytes);
    let mut colors = base.clone();
    colors.read(&profile, COLOR_SECTION);
    Some(colors)
}

#[cfg(test)]
mod tests;
