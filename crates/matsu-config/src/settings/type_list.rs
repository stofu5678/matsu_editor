// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/typeprop/CDlgTypeList.cpp (AddType, CopyType, DelType, UpType, DownType, InitializeType)
//   sakura_core/_main/CControlTray.cpp (OnAddTypeSetting, OnDelTypeSetting)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, genta
//   Copyright (C) 2001, Stonee
//   Copyright (C) 2002, MIK
//   Copyright (C) 2006, ryoji
//   Copyright (C) 2010, Uchi, Beta.Ito, syat
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

//! The operations of the type list (タイプ別設定一覧): 追加, 複製, 削除, 上へ, 下へ and 初期化, done on the
//! `Types(N)` sections of a settings file.
//!
//! Sakura keeps the types in shared memory and writes all of them when it ends. MATSU Editor keeps only what
//! differs from the built-in types in the file ([`DocumentTypes::from_profile`]), so an operation that changes
//! the number of a type first reads the types as the file means them, changes the list, and writes the
//! sections from the first changed number on in full. Keys that [`TypeSettings`] does not know stay with
//! their type, and the new-file keys (`eDefaultCodetype`, `eDefaultEoltype`, `bDefaultBom`) that the file
//! does not set stay unset (the line ending follows the OS then). Nothing new is added to the file: the
//! number of types is `nTypesCount` of `[Other]`, and every type is `Types(N)`.
//!
//! The type with number 0 (基本) never moves and is never deleted (the buttons are disabled for it:
//! `CDlgTypeList::DispatchEvent`, `LBN_SELCHANGE`).

use std::time::{SystemTime, UNIX_EPOCH};

use super::builtin_types::builtin_types;
use super::doc_types::DocumentTypes;
use super::types::{MAX_TYPES, SAKURA_TYPES_COUNT, TypeSettings};
use crate::profile::{Profile, ProfileError};

/// The first number a type of MATSU Editor's own may be implied at ([`DocumentTypes::from_profile`]). A type
/// that is added is written together with the ones from here on, because its extensions change which of them
/// are implied.
pub(crate) const FIRST_IMPLIED: usize = super::builtin_types::BUILTIN_TYPE_COUNT;

/// `m_szTypeName` holds 64 UTF-16 units, the last is the end of the string.
const NAME_UNITS: usize = 63;

/// The new-file keys. They are kept as the file has them, so that "not set" stays "not set".
pub(crate) const NEW_FILE_KEYS: [&str; 3] = ["eDefaultCodetype", "eDefaultEoltype", "bDefaultBom"];

/// Why an operation was not done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListError {
    /// There is no type with that number.
    NoSuchType,
    /// The list has `MAX_TYPES` types (the buttons 追加 and 複製 are disabled then).
    Full,
    /// The operation is not for 基本 (or not for that position).
    NotAllowed,
    /// The list would have exactly 30 types. A file that says `nTypesCount=30` is read as 36 types, the number
    /// MATSU Editor starts with (see [`types_count`](super::types_count)), so 30 cannot be written.
    ThirtyTypes,
    /// The file cannot hold a value.
    Profile(ProfileError),
}

impl std::fmt::Display for ListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListError::NoSuchType => write!(f, "no such type"),
            ListError::Full => write!(f, "the list has {MAX_TYPES} types"),
            ListError::NotAllowed => write!(f, "not for this type"),
            ListError::ThirtyTypes => write!(f, "the list cannot have {SAKURA_TYPES_COUNT} types"),
            ListError::Profile(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ListError {}

impl From<ProfileError> for ListError {
    fn from(e: ProfileError) -> Self {
        ListError::Profile(e)
    }
}

/// One type of the list with the keys of its section that [`TypeSettings`] does not hold.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub settings: TypeSettings,
    /// The entries of the section as the file has them (empty for a type the file has no section for).
    pub raw: Vec<(String, String)>,
}

/// The types as the file means them, with the keys of their sections.
pub(crate) fn load(profile: &Profile) -> Vec<Entry> {
    DocumentTypes::from_profile(profile)
        .iter()
        .enumerate()
        .map(|(i, t)| Entry {
            settings: t.clone(),
            raw: profile
                .entries(&section(i))
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        })
        .collect()
}

fn section(index: usize) -> String {
    TypeSettings::section_name(i32::try_from(index).unwrap_or(i32::MAX))
}

/// Writes `entries` from number `from` on as the sections `Types(N)`, removes the sections of the numbers
/// from `entries.len()` to `old_len`, and writes `nTypesCount`.
pub(crate) fn store(
    profile: &mut Profile,
    from: usize,
    entries: &mut [Entry],
    old_len: usize,
) -> Result<(), ListError> {
    for (i, entry) in entries.iter_mut().enumerate().skip(from) {
        write_entry(profile, i, entry)?;
    }
    for i in entries.len()..old_len {
        profile.remove_section(&section(i));
    }
    profile.set_i32(
        "Other",
        "nTypesCount",
        i32::try_from(entries.len()).unwrap_or(i32::MAX),
    )?;
    Ok(())
}

/// Replaces the section `Types(index)` with the entry.
pub(crate) fn write_entry(
    profile: &mut Profile,
    index: usize,
    entry: &mut Entry,
) -> Result<(), ListError> {
    let number = i32::try_from(index).unwrap_or(i32::MAX);
    entry.settings.idx = number;
    let name = section(index);
    profile.remove_section(&name);
    for (key, value) in &entry.raw {
        profile.set(&name, key, value)?;
    }
    entry.settings.write(profile, number)?;
    end_regex_list(profile, &name, &entry.settings)?;
    // `write` always writes the new-file keys. The ones the file did not set stay unset.
    for key in NEW_FILE_KEYS {
        match entry.raw.iter().find(|(k, _)| k == key) {
            Some((_, value)) => profile.set(&name, key, value)?,
            None => {
                profile.remove(&name, key);
            }
        }
    }
    Ok(())
}

/// A list of regex keywords shorter than the limit ends with a blank `RxKey[NNN]=`: without it a type that
/// starts from a built-in one (the first types of a file) would get the keywords of that type back, and so
/// would a type imported over the basic type.
pub(crate) fn end_regex_list(
    profile: &mut Profile,
    section: &str,
    settings: &TypeSettings,
) -> Result<(), ProfileError> {
    let defined = settings
        .regex_keywords
        .iter()
        .filter(|k| !k.pattern.is_empty())
        .count()
        .min(super::type_rules::MAX_REGEX_KEYWORDS);
    if defined < super::type_rules::MAX_REGEX_KEYWORDS {
        let key = format!("RxKey[{defined:03}]");
        if profile.get(section, &key).is_none() {
            profile.set(section, &key, "")?;
        }
    }
    Ok(())
}

pub(crate) fn check_count(count: usize) -> Result<(), ListError> {
    if count > usize::try_from(MAX_TYPES).unwrap_or(usize::MAX) {
        Err(ListError::Full)
    } else if count == usize::try_from(SAKURA_TYPES_COUNT).unwrap_or(0) {
        Err(ListError::ThirtyTypes)
    } else {
        Ok(())
    }
}

/// `(GetTickCount() & 0x3fffffff) + index * 0x10000`, and not an id that another type has.
pub(crate) fn new_id(index: usize, entries: &[Entry]) -> i32 {
    let ticks = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let ticks = i32::try_from(ticks & 0x3fff_ffff).unwrap_or(0);
    let mut id = ticks.saturating_add(i32::try_from(index).unwrap_or(0).saturating_mul(0x10000));
    while entries.iter().any(|e| e.settings.id == id) {
        id += 1;
    }
    id
}

/// `STR_TRAY_TYPE_NAME` ("設定%d").
fn numbered_name(n: usize) -> String {
    format!("設定{n}")
}

/// The name of a type added at `insert` (`OnAddTypeSetting`): `設定(insert + 1)`, and the next number while a
/// type from number 1 on has the name.
pub fn name_for_added(names: &[&str], insert: usize) -> String {
    let mut n = insert + 1;
    let mut name = numbered_name(n);
    let mut k = 1;
    while k < names.len() {
        if names[k] == name {
            n += 1;
            name = numbered_name(n);
            k = 0;
        }
        k += 1;
    }
    name
}

/// The name of the copy of the type called `source` (`CopyType`): a number is added to the name, or the number
/// at its end is made one larger, until no type has the name.
pub fn name_for_copy(source: &str, names: &[&str]) -> String {
    let mut name = source.to_owned();
    let mut n: u64 = 1;
    let mut update = true;
    let mut i = 0;
    while i < names.len() {
        if update {
            // The digits at the end of the name (`WCODE::Is09`).
            let digits = name.chars().rev().take_while(char::is_ascii_digit).count();
            if digits > 0 {
                let start = name.len() - digits;
                n = name[start..]
                    .parse::<u64>()
                    .unwrap_or(u64::MAX)
                    .saturating_add(1);
                name.truncate(start);
            } else {
                n += 1;
            }
            let number = n.to_string();
            // `LimitStringLengthW`: the name must fit with the number.
            let room = NAME_UNITS.saturating_sub(number.len());
            let mut units = 0;
            let mut limited = String::new();
            for c in name.chars() {
                units += c.len_utf16();
                if units > room {
                    break;
                }
                limited.push(c);
            }
            name = limited + &number;
            update = false;
        }
        if names[i] == name {
            i = 0;
            update = true;
        } else {
            i += 1;
        }
    }
    name
}

/// The name of an initialized type (`InitializeType`): `設定(index + 1)`, and the next number while another type
/// from number 1 on has the name. `names` are the names of all types, the one at `index` is not compared.
pub fn name_for_initialized(names: &[&str], index: usize) -> String {
    let mut n = index + 1;
    let mut update = true;
    let mut name = String::new();
    let mut i = 1;
    while i < names.len() {
        if update {
            name = numbered_name(n);
            n += 1;
            update = false;
        }
        if i != index && names[i] == name {
            i = 0;
            update = true;
        }
        i += 1;
    }
    name
}

fn names(entries: &[Entry]) -> Vec<&str> {
    entries
        .iter()
        .map(|e| e.settings.type_name.as_str())
        .collect()
}

/// 追加 (`AddType`, `OnAddTypeSetting`): a copy of 基本 at the end of the list, with the name `設定N` and no
/// extensions. Returns its number.
pub fn add_type(profile: &mut Profile) -> Result<usize, ListError> {
    let mut entries = load(profile);
    let old_len = entries.len();
    check_count(old_len + 1)?;
    let mut entry = entries[0].clone();
    entry.settings.id = new_id(old_len, &entries);
    entry.settings.type_name = name_for_added(&names(&entries), old_len);
    entry.settings.type_exts.clear();
    entries.push(entry);
    store(profile, FIRST_IMPLIED.min(old_len), &mut entries, old_len)?;
    Ok(old_len)
}

/// 複製 (`CopyType`): a copy of the type at the end of the list with a numbered name. The extensions are copied
/// too. Returns the number of the copy.
pub fn copy_type(profile: &mut Profile, index: usize) -> Result<usize, ListError> {
    let mut entries = load(profile);
    let old_len = entries.len();
    let source = entries.get(index).ok_or(ListError::NoSuchType)?;
    check_count(old_len + 1)?;
    let mut entry = source.clone();
    entry.settings.type_name = name_for_copy(&entry.settings.type_name, &names(&entries));
    entry.settings.id = new_id(old_len, &entries);
    entries.push(entry);
    store(profile, FIRST_IMPLIED.min(old_len), &mut entries, old_len)?;
    Ok(old_len)
}

/// 削除 (`DelType`, `OnDelTypeSetting`): the types after it move up by one. 基本 cannot be deleted.
pub fn delete_type(profile: &mut Profile, index: usize) -> Result<(), ListError> {
    let mut entries = load(profile);
    let old_len = entries.len();
    if index == 0 {
        return Err(ListError::NotAllowed);
    }
    if index >= old_len {
        return Err(ListError::NoSuchType);
    }
    check_count(old_len - 1)?;
    entries.remove(index);
    store(profile, index, &mut entries, old_len)
}

/// 上へ and 下へ (`UpType`, `DownType`): swaps the type with the one before it (`up`) or after it. 基本 does not
/// move, and nothing moves before 基本 (the button is disabled for number 1). Returns the new number.
pub fn move_type(profile: &mut Profile, index: usize, up: bool) -> Result<usize, ListError> {
    let mut entries = load(profile);
    let old_len = entries.len();
    if index >= old_len {
        return Err(ListError::NoSuchType);
    }
    let other = if up {
        if index <= 1 {
            return Err(ListError::NotAllowed);
        }
        index - 1
    } else {
        if index == 0 || index + 1 >= old_len {
            return Err(ListError::NotAllowed);
        }
        index + 1
    };
    entries.swap(index, other);
    store(profile, index.min(other), &mut entries, old_len)?;
    Ok(other)
}

/// 初期化 (`InitializeType`). 基本 goes back to the built-in settings. Any other type becomes a copy of 基本
/// with the name `設定N` and no extensions, and a new id. Returns the new name.
pub fn reset_type(profile: &mut Profile, index: usize) -> Result<String, ListError> {
    let mut entries = load(profile);
    let old_len = entries.len();
    if index >= old_len {
        return Err(ListError::NoSuchType);
    }
    let entry = if index == 0 {
        // `CType_Basis::InitTypeConfig`
        Entry {
            settings: builtin_types().swap_remove(0),
            raw: Vec::new(),
        }
    } else {
        let mut entry = entries[0].clone();
        entry.settings.type_name = name_for_initialized(&names(&entries), index);
        entry.settings.type_exts.clear();
        entry.settings.id = new_id(index, &entries);
        entry
    };
    let name = entry.settings.type_name.clone();
    entries[index] = entry;
    // The numbers after it are written too: a type that is only implied (MATSU Editor's own) must not move
    // to another number when this one stops being a free number.
    store(profile, index, &mut entries, old_len)?;
    Ok(name)
}

#[cfg(test)]
mod tests;
