// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for UME Editor.
//
// Original files:
//   sakura_core/env/CommonSetting.h (EFileTreeItemType, SFileTreeItem, SFileTree)
//   sakura_core/env/CShareData.cpp (InitFileTree)
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_FileTree)
//   sakura_core/env/CDataProfile.cpp (IOProfileData<SFileTreeItem>)
//   sakura_core/typeprop/CImpExpManager.cpp (CImpExpFileTree::IO_FileTreeIni)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2012, the authors named in CShareData.cpp (jepro, genta, MIK, asa-o, YAZAKI,
//     hor, ai, Moca, KK, aroka, wmlhq, sui, novice, isearch, ryoji, maru, Uchi, nasukoji and others)
//   Copyright (C) 2008, kobake
//   Copyright (C) 2010, Uchi, Moca
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (the shared memory
// structures with fixed arrays and the `CDataProfile` templates) was rewritten in Rust on
// `Profile`, which keeps every line of the file that is not touched, and the tests are new.
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

//! The settings of Sakura's file tree (ファイルツリー): the items of the tree and where they come from.
//!
//! The tree is made of up to [`MAX_ITEMS`] items, kept in `[Common]` (for all types) and in each
//! `[Types(N)]` section:
//!
//! ```text
//! bFileTreeProject=1
//! szFileTreeProjectIni=
//! nFileTreeItemCount=2
//! FileTree(0).eItemType=0
//! FileTree(0).szTargetPath=.
//! FileTree(0).nDepth=0
//! FileTree(0).szTargetFile=*.*
//! FileTree(0).bIgnoreHidden=1
//! FileTree(0).bIgnoreReadOny=0
//! FileTree(0).bIgnoreSystem=0
//! FileTree(1).eItemType=2
//! FileTree(1).szLabelName=Docs
//! FileTree(1).nDepth=0
//! ```
//!
//! (`bIgnoreReadOny` is Sakura's spelling, kept so that the files are the same.) The same items can
//! be kept in a project file (`_sakurafiletree.ini` by default), in the section `[FileTree]`.
//! Everything else in a file is left as it is when the items are written.

use crate::profile::{Profile, ProfileError};

/// The section of the common settings.
pub const COMMON_SECTION: &str = "Common";
/// The section of a project file (`CImpExpFileTree::IO_FileTreeIni`).
pub const PROJECT_SECTION: &str = "FileTree";
/// `sizeof(SFileTree::m_aItems) / sizeof(SFileTreeItem)`: how many items the settings hold.
pub const MAX_ITEMS: usize = 20;
/// How many items a project file may have. Sakura has no limit there; a file that claims more than
/// this is cut so that a wrong number cannot make the program allocate without end.
pub const MAX_PROJECT_ITEMS: usize = 10_000;
/// `m_sFileTreeDefIniName` of a new `CommonSetting` (`CShareData::InitShareData`): the name of the
/// project file that is looked for in the folders above the current one.
pub const DEFAULT_PROJECT_INI_NAME: &str = "_sakurafiletree.ini";

const KEY_PROJECT: &str = "bFileTreeProject";
const KEY_PROJECT_INI: &str = "szFileTreeProjectIni";
const KEY_COUNT: &str = "nFileTreeItemCount";
const KEY_DEFAULT_INI_NAME: &str = "szFileTreeDefIniName";

/// `EFileTreeItemType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemType {
    /// `EFileTreeItemType_Grep` (0): a folder whose files are listed by patterns (ファイル指定).
    Grep,
    /// `EFileTreeItemType_File` (1): one file.
    File,
    /// `EFileTreeItemType_Folder` (2): a node that groups the items below it (フォルダー).
    Folder,
    /// A number that Sakura does not define. The tree skips such an item.
    Other(i32),
}

impl ItemType {
    /// The type for the number in the file.
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Grep,
            1 => Self::File,
            2 => Self::Folder,
            other => Self::Other(other),
        }
    }

    /// The number in the file.
    pub fn code(self) -> i32 {
        match self {
            Self::Grep => 0,
            Self::File => 1,
            Self::Folder => 2,
            Self::Other(code) => code,
        }
    }
}

/// `SFileTreeItem`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTreeItem {
    /// `m_eFileTreeItemType`.
    pub item_type: ItemType,
    /// `m_szTargetPath`: the folder (Grep) or the file (File); it may hold meta folders such as
    /// `%MYDOC%` and, in a project file, `<iniroot>`.
    pub target_path: String,
    /// `m_szLabelName`: what the tree shows; empty means the path.
    pub label: String,
    /// `m_nDepth`: how deep the item is under the folders (Folder items) above it.
    pub depth: i32,
    /// `m_szTargetFile` (Grep): the file patterns, as Grep's ファイル box has them.
    pub target_file: String,
    /// `m_bIgnoreHidden` (Grep): leave hidden files out.
    pub ignore_hidden: bool,
    /// `m_bIgnoreReadOnly` (Grep): leave read-only files out.
    pub ignore_read_only: bool,
    /// `m_bIgnoreSystem` (Grep): leave system files out.
    pub ignore_system: bool,
}

impl Default for FileTreeItem {
    /// `SFileTreeItem::SFileTreeItem()`.
    fn default() -> Self {
        Self {
            item_type: ItemType::Grep,
            target_path: String::new(),
            label: String::new(),
            depth: 0,
            target_file: String::new(),
            ignore_hidden: true,
            ignore_read_only: false,
            ignore_system: false,
        }
    }
}

/// A value to write.
enum Value<'a> {
    Str(&'a str),
    Int(i32),
    Bool(bool),
}

/// The key of a field of item `index` (`FileTree(0).szTargetPath`).
fn item_key(index: usize, field: &str) -> String {
    format!("FileTree({index}).{field}")
}

impl FileTreeItem {
    /// `IOProfileData<SFileTreeItem>`, reading: a field that the file lacks keeps its value.
    fn read(&mut self, profile: &Profile, section: &str, index: usize) {
        let key = |field: &str| item_key(index, field);
        if let Some(code) = profile.get_i32(section, &key("eItemType")) {
            self.item_type = ItemType::from_code(code);
        }
        if let Some(v) = profile.get_string(section, &key("szTargetPath")) {
            self.target_path = v;
        }
        if let Some(v) = profile.get_string(section, &key("szLabelName")) {
            self.label = v;
        }
        if let Some(v) = profile.get_i32(section, &key("nDepth")) {
            self.depth = v;
        }
        if let Some(v) = profile.get_string(section, &key("szTargetFile")) {
            self.target_file = v;
        }
        if let Some(v) = profile.get_bool(section, &key("bIgnoreHidden")) {
            self.ignore_hidden = v;
        }
        if let Some(v) = profile.get_bool(section, &key("bIgnoreReadOny")) {
            self.ignore_read_only = v;
        }
        if let Some(v) = profile.get_bool(section, &key("bIgnoreSystem")) {
            self.ignore_system = v;
        }
    }

    /// `IOProfileData<SFileTreeItem>`, writing: the path is written for Grep and File items, the
    /// label for Folder items and for the others that have one, the patterns and the three
    /// options for Grep items. Returns the keys written.
    fn write(
        &self,
        profile: &mut Profile,
        section: &str,
        index: usize,
    ) -> Result<Vec<String>, ProfileError> {
        let mut fields: Vec<(&str, Value)> = vec![("eItemType", Value::Int(self.item_type.code()))];
        if matches!(self.item_type, ItemType::Grep | ItemType::File) {
            fields.push(("szTargetPath", Value::Str(&self.target_path)));
        }
        if self.item_type == ItemType::Folder || !self.label.is_empty() {
            fields.push(("szLabelName", Value::Str(&self.label)));
        }
        fields.push(("nDepth", Value::Int(self.depth)));
        if self.item_type == ItemType::Grep {
            fields.push(("szTargetFile", Value::Str(&self.target_file)));
            fields.push(("bIgnoreHidden", Value::Bool(self.ignore_hidden)));
            fields.push(("bIgnoreReadOny", Value::Bool(self.ignore_read_only)));
            fields.push(("bIgnoreSystem", Value::Bool(self.ignore_system)));
        }
        let mut keys = Vec::with_capacity(fields.len());
        for (field, value) in fields {
            let key = item_key(index, field);
            match value {
                Value::Str(v) => profile.set_string(section, &key, v)?,
                Value::Int(v) => profile.set_i32(section, &key, v)?,
                Value::Bool(v) => profile.set_bool(section, &key, v)?,
            }
            keys.push(key);
        }
        Ok(keys)
    }
}

/// `SFileTree`: the items of the tree and where else to look for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTree {
    /// `m_bProject`: look for a project file in the current folder and the folders above it.
    pub project: bool,
    /// `m_szProjectIni`: the project file to use when none is found (a relative path starts at the
    /// folder of the settings file). Empty means none.
    pub project_ini: String,
    /// `m_aItems[0..m_nItemCount]`.
    pub items: Vec<FileTreeItem>,
}

impl Default for FileTree {
    /// `CShareData::InitFileTree`: one item that lists everything of the current folder.
    fn default() -> Self {
        Self {
            project: true,
            project_ini: String::new(),
            items: vec![FileTreeItem {
                target_path: ".".to_owned(),
                target_file: "*.*".to_owned(),
                ..FileTreeItem::default()
            }],
        }
    }
}

impl FileTree {
    /// `ShareData_IO_FileTree`, reading `section` (`[Common]` or a `[Types(N)]`). What the file
    /// lacks is what `InitFileTree` gave: the first of the 20 places has the item of the default,
    /// the others are empty.
    pub fn from_profile(profile: &Profile, section: &str) -> Self {
        let mut tree = Self::default();
        if let Some(v) = profile.get_bool(section, KEY_PROJECT) {
            tree.project = v;
        }
        if let Some(v) = profile.get_string(section, KEY_PROJECT_INI) {
            tree.project_ini = v;
        }
        let mut places = tree.items.clone();
        places.resize_with(MAX_ITEMS, FileTreeItem::default);
        let count = profile
            .get_i32(section, KEY_COUNT)
            .map_or(1, |n| usize::try_from(n.max(0)).unwrap_or(0))
            .min(MAX_ITEMS);
        places.truncate(count);
        for (index, item) in places.iter_mut().enumerate() {
            item.read(profile, section, index);
        }
        tree.items = places;
        tree
    }

    /// Only the first two lines of `ShareData_IO_FileTree`: `bFileTreeProject` and
    /// `szFileTreeProjectIni`. The settings dialog writes just these when the items it shows come
    /// from a project file (`CDlgFileTree::GetData` leaves `m_nItemCount` and the items alone).
    pub fn write_options_to(
        &self,
        profile: &mut Profile,
        section: &str,
    ) -> Result<(), ProfileError> {
        profile.set_bool(section, KEY_PROJECT, self.project)?;
        profile.set_string(section, KEY_PROJECT_INI, &self.project_ini)
    }

    /// `ShareData_IO_FileTree`, writing `section`. Items past [`MAX_ITEMS`] are not written. The
    /// keys of items that are no longer there are removed.
    pub fn write_to(&self, profile: &mut Profile, section: &str) -> Result<(), ProfileError> {
        self.write_options_to(profile, section)?;
        let items = &self.items[..self.items.len().min(MAX_ITEMS)];
        let count = i32::try_from(items.len()).unwrap_or(i32::MAX);
        profile.set_i32(section, KEY_COUNT, count)?;
        write_items_to(profile, section, items)
    }
}

/// `CImpExpFileTree::IO_FileTreeIni`, reading: the items of a project file. A file without
/// `[FileTree]` or without a count has none.
pub fn read_project_items(profile: &Profile) -> Vec<FileTreeItem> {
    let count = profile
        .get_i32(PROJECT_SECTION, KEY_COUNT)
        .map_or(0, |n| usize::try_from(n.max(0)).unwrap_or(0))
        .min(MAX_PROJECT_ITEMS);
    (0..count)
        .map(|index| {
            let mut item = FileTreeItem::default();
            item.read(profile, PROJECT_SECTION, index);
            item
        })
        .collect()
}

/// `CImpExpFileTree::IO_FileTreeIni`, writing: the count and the items in `[FileTree]`.
pub fn write_project_items(
    profile: &mut Profile,
    items: &[FileTreeItem],
) -> Result<(), ProfileError> {
    let count = i32::try_from(items.len()).unwrap_or(i32::MAX);
    profile.set_i32(PROJECT_SECTION, KEY_COUNT, count)?;
    write_items_to(profile, PROJECT_SECTION, items)
}

/// Writes the items and removes the keys of items that are not (or no longer) written.
fn write_items_to(
    profile: &mut Profile,
    section: &str,
    items: &[FileTreeItem],
) -> Result<(), ProfileError> {
    let mut written: Vec<String> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        written.extend(item.write(profile, section, index)?);
    }
    let stale: Vec<String> = profile
        .entries(section)
        .map(|(key, _)| key)
        .filter(|key| key.starts_with("FileTree(") && !written.iter().any(|w| w == key))
        .map(str::to_owned)
        .collect();
    for key in stale {
        profile.remove(section, &key);
    }
    Ok(())
}

/// `szFileTreeDefIniName` of `[Common]`: the name of the project file that is looked for.
pub fn default_project_ini_name(profile: &Profile) -> String {
    profile
        .get_string(COMMON_SECTION, KEY_DEFAULT_INI_NAME)
        .unwrap_or_else(|| DEFAULT_PROJECT_INI_NAME.to_owned())
}

#[cfg(test)]
mod tests;
