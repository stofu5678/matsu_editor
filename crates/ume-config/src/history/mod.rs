// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/recent/CMRUFile.cpp, CMRUFolder.cpp, CRecentFile.cpp, CRecentFolder.cpp
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_Mru)
//   sakura_core/env/CDataProfile.cpp (IOProfileData for EditInfo)
//   sakura_core/cmd/CViewCommander_Insert.cpp (Command_INS_FILE_USED_RECENTLY and the folder one)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2001, MIK, YAZAKI
//   Copyright (C) 2002, YAZAKI, Moca, genta
//   Copyright (C) 2003, MIK
//   Copyright (C) 2006, genta
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

//! The recent files and folders (最近使ったファイル / フォルダー) and what is remembered of each file.
//!
//! Sakura keeps them in the section `[MRU]` of `sakura.ini` (`ShareData_IO_Mru`):
//!
//! ```text
//! [MRU]
//! _MRU_Counts=2
//! MRU[00].nViewTopLine=0        the row at the top of the view
//! MRU[00].nViewLeftCol=0        the column at the left of the view
//! MRU[00].nX=5                  the caret: column (UTF-16 units) and line (0 based)
//! MRU[00].nY=12
//! MRU[00].nCharCode=4           the encoding (ECodeType)
//! MRU[00].szPath=C:\memo.txt
//! MRU[00].szMark2=:0a           the bookmarks ([`marks`])
//! MRU[00].nType=-1              the id of the document type
//! MRU[00].bFavorite=0
//! _MRUFOLDER_Counts=1
//! MRUFOLDER[00]=C:\
//! MRUFOLDER[00].bFavorite=0
//! _ExceptMRU_Counts=0           paths that are never added
//! ```
//!
//! * [`list`]: the list with its rules (the front, the limits, favorites).
//! * [`marks`]: the text of the bookmarks.
//! * [`store`]: the file, read and written so that several windows (or processes) can add at once.
//!
//! [`History`] is plain data. Where the paths come from is not its business, so Windows paths
//! (`C:\...`, `\\server\share\...`) are as good as Unix ones: they are text.

pub mod list;
pub mod marks;
pub mod store;

pub use list::{Recent, RecentList};
pub use store::HistoryStore;

use crate::profile::{Profile, ProfileError};

/// `MAX_MRU`: how many files the history can keep.
pub const MAX_MRU: usize = 36;
/// `MAX_OPENFOLDER`: how many folders the history can keep.
pub const MAX_OPENFOLDER: usize = 36;
/// The size of Sakura's exclusion list (`CRecentExceptMRU` uses `MAX_MRU`).
pub const MAX_EXCEPT_MRU: usize = 36;
/// `nMRUArrNum_MAX` and `nOPENFOLDERArrNum_MAX` when the file has none (`CShareData.cpp`).
pub const DEFAULT_VIEW_COUNT: usize = 15;
/// `CODE_AUTODETECT`: "no encoding was remembered".
pub const CODE_AUTODETECT: i32 = 99;

const SECTION: &str = "MRU";

/// A file in the history, with what is restored when it is opened again (Sakura's `EditInfo`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    /// Sakura's `ECodeType` number of the encoding the file was opened in ([`CODE_AUTODETECT`]:
    /// not known).
    pub encoding_code: i32,
    /// The caret: the column in UTF-16 units and the line (0 based); negative: not known.
    pub caret_x: i32,
    pub caret_y: i32,
    /// The row at the top of the view and the column at the left; negative: not known.
    pub view_top: i32,
    pub view_left: i32,
    /// The bookmarks as [`marks`] writes them.
    pub marks: String,
    /// The id of the document type (`STypeConfig::m_id`); -1: not known.
    pub type_id: i32,
    pub favorite: bool,
}

impl FileEntry {
    /// An entry that only has a path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            encoding_code: CODE_AUTODETECT,
            caret_x: -1,
            caret_y: -1,
            view_top: -1,
            view_left: -1,
            marks: String::new(),
            type_id: -1,
            favorite: false,
        }
    }
}

impl Recent for FileEntry {
    fn text(&self) -> &str {
        &self.path
    }
    fn is_favorite(&self) -> bool {
        self.favorite
    }
    fn set_favorite_flag(&mut self, favorite: bool) {
        self.favorite = favorite;
    }
}

/// A folder in the history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderEntry {
    pub path: String,
    pub favorite: bool,
}

impl FolderEntry {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            favorite: false,
        }
    }
}

impl Recent for FolderEntry {
    fn text(&self) -> &str {
        &self.path
    }
    fn is_favorite(&self) -> bool {
        self.favorite
    }
    fn set_favorite_flag(&mut self, favorite: bool) {
        self.favorite = favorite;
    }
}

/// The folder of a file path, with its last separator (`_wsplitpath_s`: drive and directory), as
/// Sakura puts it in the folder history. Both `\` and `/` separate. Empty if the path has no
/// folder part.
pub fn folder_of(path: &str) -> String {
    if let Some(i) = path.rfind(['\\', '/']) {
        return path[..=i].to_owned();
    }
    // `C:memo.txt` has the drive only.
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return path[..2].to_owned();
    }
    String::new()
}

/// The recent files and folders, and the paths that are not added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    pub files: RecentList<FileEntry>,
    pub folders: RecentList<FolderEntry>,
    /// `ExceptMRU`: a path that has any of these in it (any case) is not added, unless it is in
    /// the history already (`CMRUFile::Add`). `%MYDOC%`-style variables are not expanded.
    pub except: Vec<String>,
}

impl History {
    /// An empty history that shows `files` files and `folders` folders.
    pub fn new(files: usize, folders: usize, case_insensitive: bool) -> Self {
        Self {
            files: RecentList::new(MAX_MRU, files, case_insensitive),
            folders: RecentList::new(MAX_OPENFOLDER, folders, case_insensitive),
            except: Vec::new(),
        }
    }

    fn excluded(&self, path: &str) -> bool {
        let lower = path.to_lowercase();
        // An empty text would match everything; Sakura's `wcsistr` does match it, but a line left
        // empty in the list is a mistake, not a wish to keep every file out of the history.
        self.except
            .iter()
            .filter(|e| !e.is_empty())
            .any(|e| lower.contains(&e.to_lowercase()))
    }

    /// Adds a folder at the front (`CMRUFolder::Add`). An empty path and a path that is excluded
    /// (and not in the list yet) are ignored.
    pub fn add_folder(&mut self, folder: &str) {
        if folder.is_empty() {
            return;
        }
        if self.folders.find(folder).is_none() && self.excluded(folder) {
            return;
        }
        self.folders.append(FolderEntry::new(folder));
    }

    /// Adds a file at the front, and its folder to the folders (`CMRUFile::Add`).
    pub fn add_file(&mut self, entry: FileEntry) {
        if entry.path.is_empty() {
            return;
        }
        if self.files.find(&entry.path).is_none() && self.excluded(&entry.path) {
            return;
        }
        self.add_folder(&folder_of(&entry.path));
        self.files.append(entry);
    }

    /// The remembered data of a file (`CMRUFile::GetEditInfo`).
    pub fn file(&self, path: &str) -> Option<&FileEntry> {
        self.files.get(self.files.find(path)?)
    }

    /// The paths of the shown files, newest first (`CMRUFile::GetPathList`).
    pub fn file_paths(&self) -> Vec<&str> {
        self.files
            .visible()
            .iter()
            .map(|f| f.path.as_str())
            .collect()
    }

    /// The paths of the shown folders, newest first.
    pub fn folder_paths(&self) -> Vec<&str> {
        self.folders
            .visible()
            .iter()
            .map(|f| f.path.as_str())
            .collect()
    }

    /// The text `F_INS_FILE_USED_RECENTLY` inserts: each shown path followed by the line ending
    /// `eol` (`Command_INS_FILE_USED_RECENTLY`).
    pub fn files_text(&self, eol: &str) -> String {
        join_lines(&self.file_paths(), eol)
    }

    /// The text `F_INS_FOLDER_USED_RECENTLY` inserts.
    pub fn folders_text(&self, eol: &str) -> String {
        join_lines(&self.folder_paths(), eol)
    }

    /// Reads the history from the section `[MRU]` and the view counts from `[Common]`
    /// (`ShareData_IO_Mru`). A count above the size of the list is cut to it; an entry with no
    /// path is dropped.
    pub fn from_profile(profile: &Profile, case_insensitive: bool) -> Self {
        let view = |key: &str| {
            let n = profile
                .get_i32("Common", key)
                .map_or(DEFAULT_VIEW_COUNT, |n| usize::try_from(n).unwrap_or(0));
            n.min(MAX_MRU)
        };
        let mut history = Self::new(
            view("nMRUArrNum_MAX"),
            view("nOPENFOLDERArrNum_MAX"),
            case_insensitive,
        );
        let count = |key: &str, max: usize| {
            profile
                .get_i32(SECTION, key)
                .map_or(0, |n| usize::try_from(n).unwrap_or(0))
                .min(max)
        };
        for i in 0..count("_MRU_Counts", MAX_MRU) {
            let key = |name: &str| format!("MRU[{i:02}]{name}");
            let mut entry = FileEntry::new(profile.get(SECTION, &key(".szPath")).unwrap_or(""));
            if entry.path.is_empty() {
                continue;
            }
            let int =
                |name: &str, default: i32| profile.get_i32(SECTION, &key(name)).unwrap_or(default);
            entry.view_top = int(".nViewTopLine", -1);
            entry.view_left = int(".nViewLeftCol", -1);
            entry.caret_x = int(".nX", -1);
            entry.caret_y = int(".nY", -1);
            entry.encoding_code = int(".nCharCode", CODE_AUTODETECT);
            // `szMark2` (version 2), else the old `szMark`.
            entry.marks = profile
                .get(SECTION, &key(".szMark2"))
                .or_else(|| profile.get(SECTION, &key(".szMark")))
                .unwrap_or("")
                .to_owned();
            entry.type_id = int(".nType", -1);
            entry.favorite = profile
                .get_bool(SECTION, &key(".bFavorite"))
                .unwrap_or(false);
            history.files.push_loaded(entry);
        }
        for i in 0..count("_MRUFOLDER_Counts", MAX_OPENFOLDER) {
            let key = format!("MRUFOLDER[{i:02}]");
            let path = profile.get(SECTION, &key).unwrap_or("");
            if path.is_empty() {
                continue;
            }
            let mut entry = FolderEntry::new(path);
            entry.favorite = profile
                .get_bool(SECTION, &format!("{key}.bFavorite"))
                .unwrap_or(false);
            history.folders.push_loaded(entry);
        }
        for i in 0..count("_ExceptMRU_Counts", MAX_EXCEPT_MRU) {
            if let Some(path) = profile.get(SECTION, &format!("ExceptMRU[{i:02}]")) {
                history.except.push(path.to_owned());
            }
        }
        // A favorite outside the shown part comes in (`CRecentImp::Create` -> `UpdateView`).
        let (files, folders) = (history.files.view_count(), history.folders.view_count());
        history.files.change_view_count(files);
        history.folders.change_view_count(folders);
        history
    }

    /// Writes the history into the section `[MRU]` (`ShareData_IO_Mru`). Entries the history no
    /// longer has are removed; other keys of the section and the rest of the file stay.
    pub fn write_to(&self, profile: &mut Profile) -> Result<(), ProfileError> {
        profile.set_i32(SECTION, "_MRU_Counts", self.files.len() as i32)?;
        for (i, e) in self.files.items().iter().enumerate() {
            let key = |name: &str| format!("MRU[{i:02}]{name}");
            profile.set_i32(SECTION, &key(".nViewTopLine"), e.view_top)?;
            profile.set_i32(SECTION, &key(".nViewLeftCol"), e.view_left)?;
            profile.set_i32(SECTION, &key(".nX"), e.caret_x)?;
            profile.set_i32(SECTION, &key(".nY"), e.caret_y)?;
            profile.set_i32(SECTION, &key(".nCharCode"), e.encoding_code)?;
            profile.set_string(SECTION, &key(".szPath"), &e.path)?;
            profile.set_string(SECTION, &key(".szMark2"), &e.marks)?;
            profile.set_i32(SECTION, &key(".nType"), e.type_id)?;
            profile.set_bool(SECTION, &key(".bFavorite"), e.favorite)?;
        }
        remove_after(profile, "MRU[", self.files.len());
        profile.set_i32(SECTION, "_MRUFOLDER_Counts", self.folders.len() as i32)?;
        for (i, e) in self.folders.items().iter().enumerate() {
            let key = format!("MRUFOLDER[{i:02}]");
            profile.set_string(SECTION, &key, &e.path)?;
            profile.set_bool(SECTION, &format!("{key}.bFavorite"), e.favorite)?;
        }
        remove_after(profile, "MRUFOLDER[", self.folders.len());
        profile.set_i32(SECTION, "_ExceptMRU_Counts", self.except.len() as i32)?;
        for (i, path) in self.except.iter().enumerate() {
            profile.set_string(SECTION, &format!("ExceptMRU[{i:02}]"), path)?;
        }
        remove_after(profile, "ExceptMRU[", self.except.len());
        Ok(())
    }
}

fn join_lines(paths: &[&str], eol: &str) -> String {
    let mut text = String::new();
    for path in paths {
        text.push_str(path);
        text.push_str(eol);
    }
    text
}

/// Removes the keys `<prefix>NN]...` of the section whose number is `count` or more.
fn remove_after(profile: &mut Profile, prefix: &str, count: usize) {
    let stale: Vec<String> = profile
        .entries(SECTION)
        .filter_map(|(key, _)| {
            let rest = key.strip_prefix(prefix)?;
            let number: usize = rest[..rest.find(']')?].parse().ok()?;
            (number >= count).then(|| key.to_owned())
        })
        .collect();
    for key in stale {
        profile.remove(SECTION, &key);
    }
}

#[cfg(test)]
mod tests;
