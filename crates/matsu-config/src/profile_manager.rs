// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/dlg/CDlgProfileMgr.cpp, sakura_core/dlg/CDlgProfileMgr.h
//   src/test/cpp/tests1/test-cdlgprofilemgr.cpp (test cases)
//
// Original copyright notices of those files:
//   Copyright (C) 2013, Moca
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

//! The profile manager's data: the file `matsu_prof.ini`, the list it holds and the rules for its names.
//!
//! Sakura keeps the list of the profiles in `<ini name without the extension>_prof.ini` next to its ini file
//! (`GetProfileMgrFileName`, `CDlgProfileMgr.cpp`): `sakura.ini` has `sakura_prof.ini`, and MATSU Editor's
//! `matsu.ini` has `matsu_prof.ini`. A process that runs with `-PROF=name` has its own ini file in a sub folder
//! (the folder `name`), and the file of the manager is in the folder above it, so that every profile
//! reads the same list. The section is `[Profile]`:
//!
//! ```text
//! [Profile]
//! nCount=2
//! P[1]=work
//! P[2]=memo
//! nDefaultIndex=1
//! szDllLanguage=
//! bDefaultSelect=0
//! ```
//!
//! * `nCount` and `P[1]` ... `P[nCount]` (counted from 1) are the names of the profiles. The list that the
//!   dialog shows has one more row at the top, 「(default)」, the profile without a name (the folders of the
//!   settings file itself); it is not in the file.
//! * `nDefaultIndex` is the row (0 is 「(default)」, 1 is `P[1]`) that starts when `-PROF` is not given, or -1
//!   for none. A value above `nCount` or below -1 is read as -1 (`IOProfSettings`).
//! * `szDllLanguage` is the language file Sakura loads. MATSU Editor has no language files; the value is kept
//!   as it is, so that the file stays one that Sakura's format allows.
//! * `bDefaultSelect` is the check box 「デフォルト設定にして起動」.
//!
//! The file is read and written in the format of [`Profile`], so a line this program does not know stays.

use std::io;
use std::path::{Path, PathBuf};

use crate::location::{Environment, Os};
use crate::profile::{Profile, ProfileError};

/// The name of the manager's file (`GetProfileMgrFileName`: the ini name without its extension, and
/// `_prof.ini`).
pub const FILE_NAME: &str = "matsu_prof.ini";

/// The first comment of a new file (`CDlgProfileMgr.cpp`, `WriteProfile(.., L"Sakura Profile ini")`).
const HEADER: &str = "MATSU Profile ini";

const SECTION: &str = "Profile";

/// What the first row of the list shows: the profile without a name.
pub const DEFAULT_ROW: &str = "(default)";

/// The characters that a profile name must not have (`szReservedChars` of `CreateProf` and `RenameProf`).
const RESERVED: &str = "/\\*?<>&|:\"'\t";

/// The longest name the input takes, in UTF-16 code units (`_MAX_PATH` - 1, `max_size - 1` of `CreateProf`).
pub const MAX_NAME_LEN: usize = 259;

/// The most names read from a file. Sakura loops over `nCount` as it is, so a file that says two billion
/// profiles keeps it busy; no one has more than this.
const MAX_PROFILES: usize = 10_000;

/// What is in `matsu_prof.ini` (`SProfileSettings`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSettings {
    /// `szDllLanguage`.
    pub dll_language: String,
    /// `nDefaultIndex`: the row that starts without `-PROF`; 0 is 「(default)」, -1 is none.
    pub default_index: i32,
    /// `P[1]` ...: the profiles, without 「(default)」.
    pub profiles: Vec<String>,
    /// `bDefaultSelect`.
    pub default_select: bool,
}

impl Default for ProfileSettings {
    /// What `ReadProfSettings` starts from: the profile without a name is the default one.
    fn default() -> Self {
        Self {
            dll_language: String::new(),
            default_index: 0,
            profiles: Vec::new(),
            default_select: false,
        }
    }
}

/// Why a name is refused (`CreateProf`, `RenameProf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    /// Nothing was typed. Sakura does nothing and says nothing.
    Empty,
    /// A reserved character, or `..` (`STR_DLGPROFILE_ERR_INVALID_CHAR`).
    InvalidChar,
    /// A profile of this name is in the list already, or 「(default)」 (`STR_DLGPROFILE_ERR_ALREADY`).
    Duplicate,
}

/// `wmemicmp`: the names are compared without regard to case, because they are folder names.
fn same_name(a: &str, b: &str) -> bool {
    a.chars()
        .flat_map(char::to_lowercase)
        .eq(b.chars().flat_map(char::to_lowercase))
}

fn invalid(error: ProfileError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

impl ProfileSettings {
    /// The rows of the list: 「(default)」 and the profiles.
    pub fn rows(&self) -> usize {
        self.profiles.len() + 1
    }

    /// What a row shows, without the mark of the default one.
    pub fn row_label(&self, row: usize) -> &str {
        match row {
            0 => DEFAULT_ROW,
            _ => self.profiles.get(row - 1).map_or("", String::as_str),
        }
    }

    /// The profile a row stands for: the empty name for 「(default)」 (`GetData`).
    pub fn profile_of(&self, row: usize) -> &str {
        match row {
            0 => "",
            _ => self.profiles.get(row - 1).map_or("", String::as_str),
        }
    }

    /// Whether the row is the one that starts without `-PROF` (the one with the star).
    pub fn is_default_row(&self, row: usize) -> bool {
        usize::try_from(self.default_index) == Ok(row)
    }

    /// `SetDefaultProf`: the row becomes the default one.
    pub fn set_default(&mut self, row: usize) {
        self.default_index = i32::try_from(row).unwrap_or(-1);
    }

    /// `ClearDefaultProf`: no row is the default one.
    pub fn clear_default(&mut self) {
        self.default_index = -1;
    }

    /// Whether `name` could be a profile: the checks of `CreateProf` and `RenameProf`, in their order.
    /// `skip` is the row that is being renamed (it may keep its own name in another case).
    pub fn check_name(&self, name: &str, skip: Option<usize>) -> Result<(), NameError> {
        if name.is_empty() {
            return Err(NameError::Empty);
        }
        // Sakura's list, and what `-PROF=` cannot select (`.`, control characters): the launcher would
        // silently start the profile without a name for such a name
        if name.chars().any(|c| RESERVED.contains(c)) || name == ".." || !valid_folder_name(name) {
            return Err(NameError::InvalidChar);
        }
        let taken = (0..self.rows())
            .filter(|row| Some(*row) != skip)
            .any(|row| same_name(name, self.row_label(row)));
        if taken {
            return Err(NameError::Duplicate);
        }
        Ok(())
    }

    /// `CreateProf`: adds the profile at the end of the list and returns its row. The name was checked.
    pub fn add(&mut self, name: &str) -> usize {
        self.profiles.push(name.to_owned());
        self.profiles.len()
    }

    /// `RenameProf`: the row (not 「(default)」) gets the name and keeps its star. The name was checked.
    pub fn rename(&mut self, row: usize, name: &str) {
        if let Some(profile) = row.checked_sub(1).and_then(|i| self.profiles.get_mut(i)) {
            name.clone_into(profile);
        }
    }

    /// `DeleteProf`: takes the row (not 「(default)」) out of the list. The star goes with the row it was on; the
    /// rows below move up. Returns the row to choose next: the same place, or the last row when the last
    /// one went (`DeleteProf`).
    pub fn remove(&mut self, row: usize) -> usize {
        if row == 0 || row > self.profiles.len() {
            return row.min(self.profiles.len());
        }
        self.profiles.remove(row - 1);
        match usize::try_from(self.default_index) {
            Ok(default) if default == row => self.default_index = -1,
            Ok(default) if default > row => self.default_index -= 1,
            _ => {}
        }
        row.min(self.profiles.len())
    }

    /// `IOProfSettings` (reading): the values of the file, with what Sakura keeps when a key is missing.
    pub fn from_profile(file: &Profile) -> Self {
        let mut settings = Self::default();
        let count = file.get_i32(SECTION, "nCount").unwrap_or(0);
        let wanted = usize::try_from(count).unwrap_or(0).min(MAX_PROFILES);
        for i in 1..=wanted {
            // a name that is missing is an empty one, as in Sakura (it reads into an empty string)
            let name = file
                .get_string(SECTION, &format!("P[{i}]"))
                .unwrap_or_default();
            settings.profiles.push(name);
        }
        if let Some(index) = file.get_i32(SECTION, "nDefaultIndex") {
            settings.default_index = index;
        }
        if count < settings.default_index || settings.default_index < -1 {
            settings.default_index = -1;
        }
        if let Some(language) = file.get_string(SECTION, "szDllLanguage") {
            settings.dll_language = language;
        }
        if let Some(select) = file.get_bool(SECTION, "bDefaultSelect") {
            settings.default_select = select;
        }
        settings
    }

    /// `IOProfSettings` (writing): puts the values in `file`. Names that an earlier, longer list left behind
    /// are taken out: Sakura writes a new file, and a `P[n]` above `nCount` would never be read anyway.
    pub fn write_to(&self, file: &mut Profile) -> io::Result<()> {
        let old_count = file.get_i32(SECTION, "nCount").unwrap_or(0);
        let count = i32::try_from(self.profiles.len()).unwrap_or(i32::MAX);
        file.set_i32(SECTION, "nCount", count).map_err(invalid)?;
        for (i, name) in self.profiles.iter().enumerate() {
            file.set(SECTION, &format!("P[{}]", i + 1), name)
                .map_err(invalid)?;
        }
        let old = usize::try_from(old_count).unwrap_or(0).min(MAX_PROFILES);
        for i in (self.profiles.len() + 1)..=old {
            file.remove(SECTION, &format!("P[{i}]"));
        }
        file.set_i32(SECTION, "nDefaultIndex", self.default_index)
            .map_err(invalid)?;
        file.set(SECTION, "szDllLanguage", &self.dll_language)
            .map_err(invalid)?;
        file.set_i32(SECTION, "bDefaultSelect", i32::from(self.default_select))
            .map_err(invalid)
    }
}

/// `ReadProfSettings`: the settings of the file, or `None` when the file cannot be read (it does not exist).
pub fn read(path: &Path) -> Option<ProfileSettings> {
    Profile::load(path)
        .ok()
        .map(|file| ProfileSettings::from_profile(&file))
}

/// `WriteProfSettings`: writes the settings. A file that is there keeps the lines this program does not know.
pub fn write(path: &Path, settings: &ProfileSettings) -> io::Result<()> {
    match Profile::load(path) {
        Ok(mut file) => {
            settings.write_to(&mut file)?;
            file.save(path)
        }
        Err(_) => {
            let mut file = Profile::new();
            settings.write_to(&mut file)?;
            file.save_sakura_style(path, Some(HEADER))
        }
    }
}

/// The folder of the manager's file and of the profiles' folders: the folder of the settings file that the
/// process would use without `-PROF` (`GetProfileMgrFileName` goes up one folder when a profile is on), by
/// the same rule as [`profile_dir`] (the old product name's file counts when it is the only one).
pub fn base_dir(os: Os, env: &Environment) -> Option<PathBuf> {
    let without_profile = Environment {
        profile_name: None,
        profile_dir: None,
        ..env.clone()
    };
    crate::migrate_old_name::settings_file(os, &without_profile)?
        .parent()
        .map(Path::to_path_buf)
}

/// Where `matsu_prof.ini` is.
pub fn settings_path(os: Os, env: &Environment) -> Option<PathBuf> {
    base_dir(os, env).map(|dir| dir.join(FILE_NAME))
}

/// `GetProfileDirectory`: the folder where the settings of a profile are, or will be after the migration
/// from the old product name. It is the folder of the settings file that a process started with
/// `-PROF=name` uses (`migrate_old_name::settings_file`, the same choice as `UserSettings` after
/// the migration). On Windows with a `matsu.ini` next to the program that is the folder next to the program
/// only if `name\matsu.ini` (or its old file) is there already, else the user's own folder.
pub fn profile_dir(os: Os, env: &Environment, name: &str) -> Option<PathBuf> {
    // another profile than the one of this process: its own places, not the folder that was settled
    let with_profile = Environment {
        profile_name: Some(name.to_owned()),
        profile_dir: None,
        ..env.clone()
    };
    // One rule with the migration from the old product name (`migrate_old_name::settings_file`): the current
    // settings file wins in any place, and a profile that has only the old file is settled where that file
    // is, because the migration then writes `matsu.ini` there.
    crate::migrate_old_name::settings_file(os, &with_profile)?
        .parent()
        .map(Path::to_path_buf)
}

/// Whether `name` can be the name of a profile folder, for `-PROF=` and for the manager (one rule for
/// both): not empty, not `.` or `..`, shorter than 260 characters, no control character, and none of
/// `/ \ : * ? " < > |`.
pub fn valid_folder_name(name: &str) -> bool {
    !(name.is_empty()
        || name == "."
        || name == ".."
        || name.chars().count() >= 260
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c)))
}

/// What the start of a process does about the profile (`CDlgProfileMgr::TrySelectProfile`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Startup {
    /// The command line, or the file of the manager, settles the profile: this one (the empty name is the
    /// profile without a name).
    Profile(String),
    /// The profile manager is shown, and the user chooses.
    Dialog,
}

/// `TrySelectProfile`. `show` is `-PROFMGR`, `given` the profile of `-PROF=` (the empty name is a profile
/// too), `settings` the file of the manager (`None` when it cannot be read).
///
/// Sakura: `-PROFMGR` shows the dialog; else `-PROF` settles the profile; else a file that is not there
/// settles the profile without a name; else `nDefaultIndex` between 1 and `nCount` settles the profile of
/// that row, and anything else shows the dialog (to ask for a new default).
///
/// One rule differs: a default index of 0 (「(default)」 was made the default with デフォルト設定) settles the
/// profile without a name here. Sakura's test `0 < nDefaultIndex` shows the dialog at every start for that
/// choice, so the choice could not be kept (the dialog's own check box and the button say the row is the
/// one that starts). `-1` and an index above the count show the dialog, as in Sakura
/// (`test-cdlgprofilemgr.cpp`, `TrySelectProfile_005`, `_006`).
pub fn startup(show: bool, given: Option<&str>, settings: Option<&ProfileSettings>) -> Startup {
    if show {
        return Startup::Dialog;
    }
    if let Some(name) = given {
        return Startup::Profile(name.to_owned());
    }
    let Some(settings) = settings else {
        return Startup::Profile(String::new());
    };
    match usize::try_from(settings.default_index) {
        Ok(row) if row < settings.rows() => Startup::Profile(settings.profile_of(row).to_owned()),
        _ => Startup::Dialog,
    }
}

#[cfg(test)]
mod tests;
