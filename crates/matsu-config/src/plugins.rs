// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/plugin/CPluginManager.cpp, sakura_core/plugin/CPlugin.cpp,
//   sakura_core/plugin/CPlugin.h, sakura_core/plugin/CJackManager.cpp,
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_Plugin), sakura_core/env/CDataProfile.cpp
//   (IOProfileData<PluginRec>), sakura_core/env/CShareData.cpp (InitShareData),
//   sakura_core/config/maxdata.h
//
// Original copyright notices of those files:
//   Copyright (C) 2009, syat
//   Copyright (C) 2010, Uchi, Moca
//   Copyright (C) 2011, syat
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

//! The plugins that are registered, and the folders they are in (ADR-0026).
//!
//! Sakura keeps up to 40 plugins in a table (`CommonSetting_Plugin::m_PluginTable`, `MAX_PLUGIN`)
//! and the switch 「プラグインを使用する」. They are the section `[Plugin]` of the settings file
//! (`ShareData_IO_Plugin`):
//!
//! ```text
//! [Plugin]
//! EnablePlugin=1
//! P[00].Name=Sample      <- the name of the folder of the plugin
//! P[00].Id=jp.example.sample
//! P[00].CmdNum=2
//! ```
//!
//! The plugin itself is a folder with a definition file ([`PluginDef`], `plugin.def`), in
//! `plugins\` next to the settings file or next to the executable ([`SearchDirs`]). Its options
//! are in `plugins\<folder>.ini` ([`options`]).
//!
//! Nothing here runs a plugin: it reads, registers and removes them.

pub mod def;
pub mod options;

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use crate::profile::{Profile, ProfileError};
pub use def::{
    CommandDef, FILE_NAME as DEF_FILE_NAME, Jack, OptionDef, OptionType, Plug, PluginDef,
    PluginKind, Unsupported,
};

/// The section of the settings file.
pub const SECTION: &str = "Plugin";
/// `MAX_PLUGIN`: how many plugins can be registered.
pub const MAX_PLUGIN: usize = 40;
/// `MAX_PLUGIN_ID` and `MAX_PLUGIN_NAME` are 63 characters and the end of the string.
pub const ID_MAX_CHARS: usize = 63;
/// The folder of the plugins, in the folder of the settings file or of the executable.
pub const FOLDER_NAME: &str = "plugins";
/// The folder that is never taken for a plugin (`SearchNewPluginDir`: `unuse`).
const UNUSED_FOLDER: &str = "unuse";
/// The language of the texts of the language file when nothing else is known
/// (`LoadAllPlugin`: the language DLL is empty, which is Japanese).
pub const DEFAULT_LANGUAGE: &str = "ja_JP";
/// The characters that a plugin ID cannot have (`InstallPlugin`: `szReservedChars`). The space
/// and the tab are in the list too.
pub const RESERVED_CHARS: &str = "/\\,[]*?<>&|;:=\" \t";

/// What the table says of a slot (`EPluginState`). It is not in the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PluginState {
    /// Nobody is in the slot (`PLS_NONE`).
    #[default]
    None,
    /// Added now (`PLS_INSTALLED`).
    Installed,
    /// Added again over a registered one (`PLS_UPDATED`).
    Updated,
    /// Registered; read from the file (`PLS_STOPPED`).
    Stopped,
    /// Registered and read (`PLS_LOADED`). Nothing in this crate sets it: the plugins are not
    /// run yet.
    Loaded,
    /// Removed now; the slot keeps its ID until the file is written (`PLS_DELETED`).
    Deleted,
}

/// A slot of the table (`PluginRec`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginRec {
    /// The ID of the plugin (`Plugin.Id`).
    pub id: String,
    /// The name of the folder of the plugin (`m_szName`).
    pub name: String,
    /// The number of commands the plugin has (`m_nCmdNum`).
    pub cmd_num: usize,
    pub state: PluginState,
}

impl PluginRec {
    /// Whether a plugin is in the slot, or was until it was removed in this session.
    pub fn is_used(&self) -> bool {
        self.state != PluginState::None
    }
}

/// 「プラグインを使用する」 and the table of the plugins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginSettings {
    /// `EnablePlugin`. A new settings file does not use plugins (`InitShareData`).
    pub enabled: bool,
    /// `MAX_PLUGIN` slots.
    pub table: Vec<PluginRec>,
}

impl Default for PluginSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            table: vec![PluginRec::default(); MAX_PLUGIN],
        }
    }
}

/// `P[%02d]`: the prefix of the keys of slot `index`.
fn prefix(index: usize) -> String {
    format!("P[{index:02}]")
}

/// Cuts `text` to the characters a slot holds (`StaticString<MAX_PLUGIN_ID>`).
fn clip(text: &str) -> String {
    text.chars().take(ID_MAX_CHARS).collect()
}

impl PluginSettings {
    /// `ShareData_IO_Plugin` (reading). A slot is read when it has all of `.Name`, `.Id` and
    /// `.CmdNum` (`IOProfileData<PluginRec>` takes nothing when one is missing). A slot with an ID
    /// is registered.
    pub fn from_profile(profile: &Profile) -> Self {
        let mut settings = PluginSettings {
            enabled: profile.get_bool(SECTION, "EnablePlugin").unwrap_or(false),
            ..PluginSettings::default()
        };
        for (index, rec) in settings.table.iter_mut().enumerate() {
            let key = |field: &str| format!("{}.{field}", prefix(index));
            let (Some(name), Some(id), Some(cmd_num)) = (
                profile.get_string(SECTION, &key("Name")),
                profile.get_string(SECTION, &key("Id")),
                profile.get_i32(SECTION, &key("CmdNum")),
            ) else {
                continue;
            };
            rec.name = clip(&name);
            rec.id = clip(&id);
            rec.cmd_num = usize::try_from(cmd_num).unwrap_or(0);
            if !rec.id.is_empty() {
                rec.state = PluginState::Stopped;
            }
        }
        settings
    }

    /// `ShareData_IO_Plugin` (writing). A slot removed in this session is written empty
    /// (`PLS_DELETED`). Returns whether the file changed. A slot that is empty and has no keys in
    /// the file is not written (reading a missing slot gives the same empty slot).
    pub fn store(&self, profile: &mut Profile) -> Result<bool, ProfileError> {
        let before = profile.to_bytes();
        profile.set_bool(SECTION, "EnablePlugin", self.enabled)?;
        for (index, rec) in self.table.iter().enumerate() {
            let key = |field: &str| format!("{}.{field}", prefix(index));
            let (name, id, cmd_num) = match rec.state {
                PluginState::None | PluginState::Deleted => (String::new(), String::new(), 0),
                _ => (rec.name.clone(), rec.id.clone(), rec.cmd_num),
            };
            let in_file = profile.get(SECTION, &key("Id")).is_some();
            if !in_file && id.is_empty() {
                continue;
            }
            profile.set_string(SECTION, &key("Name"), &name)?;
            profile.set_string(SECTION, &key("Id"), &id)?;
            profile.set_i32(SECTION, &key("CmdNum"), i32::try_from(cmd_num).unwrap_or(0))?;
        }
        Ok(profile.to_bytes() != before)
    }

    /// The slot of the folder `name` (the comparison ignores case, `wmemicmp`).
    pub fn find_folder(&self, name: &str) -> Option<usize> {
        self.table
            .iter()
            .position(|rec| rec.is_used() && same_name(&rec.name, name))
    }

    /// `UninstallPlugin`: the slot is emptied except for the ID, which stays to the next write so
    /// that the same plugin added again gets its slot back (and its function codes).
    pub fn uninstall(&mut self, index: usize) {
        if let Some(rec) = self.table.get_mut(index) {
            rec.name.clear();
            rec.state = PluginState::Deleted;
            rec.cmd_num = 0;
        }
    }

    /// `InstallPlugin`: registers the plugin in the folder `folder`, which has a `plugin.def`
    /// in one of `dirs`. Returns the slot.
    ///
    /// The slot is the first empty one, or the one that has the same ID. `replace_same_id`
    /// says what to do when a plugin with the same ID is registered: Sakura asks, and a "no"
    /// stops there; here the caller asks and passes the answer (`false` gives
    /// [`InstallError::SameId`], which names the folder that is registered now).
    pub fn install(
        &mut self,
        dirs: &SearchDirs,
        folder: &str,
        replace_same_id: bool,
    ) -> Result<usize, InstallError> {
        let path = dirs.def_file(folder).ok_or(InstallError::NoDefinition)?;
        let bytes = std::fs::read(path).map_err(|_| InstallError::NoDefinition)?;
        let profile = Profile::from_bytes(&bytes);
        let id = profile
            .get("Plugin", "Id")
            .filter(|id| !id.is_empty())
            .ok_or(InstallError::NoId)?
            .to_owned();
        check_id(&id)?;

        let mut empty = None;
        let mut duplicate = false;
        for (index, rec) in self.table.iter().enumerate() {
            if empty.is_none() && rec.state == PluginState::None {
                empty = Some(index);
            }
            if rec.id == id {
                if !replace_same_id && rec.state != PluginState::Deleted {
                    return Err(InstallError::SameId {
                        folder: rec.name.clone(),
                    });
                }
                // A slot that was removed gets its plugin back; no `break` before this point,
                // as in Sakura (an ID further down can be the same).
                empty = Some(index);
                duplicate = rec.state != PluginState::Deleted;
                break;
            }
        }
        let index = empty.ok_or(InstallError::TableFull)?;
        let rec = &mut self.table[index];
        rec.name = clip(folder);
        rec.id = clip(&id);
        rec.state = if duplicate {
            PluginState::Updated
        } else {
            PluginState::Installed
        };
        rec.cmd_num = PluginDef::install_command_count(&profile);
        Ok(index)
    }
}

/// Why a plugin cannot be registered (`InstallPlugin`'s `errorMsg`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallError {
    /// `STR_PLGMGR_INST_DEF`: the folder has no `plugin.def`.
    NoDefinition,
    /// `STR_PLGMGR_INST_ID`: `Plugin.Id` is missing or empty.
    NoId,
    /// `STR_PLGMGR_INST_RESERVE1`: the ID has a character from [`RESERVED_CHARS`].
    ReservedChar(char),
    /// `STR_PLGMGR_INST_IDNUM`: the ID starts with a digit.
    IdStartsWithDigit,
    /// The ID has more than [`ID_MAX_CHARS`] characters. Sakura cuts it silently, and then the
    /// plugin is never found again by its ID; MATSU refuses it (the help says 63 at most).
    IdTooLong,
    /// `STR_PLGMGR_INST_MAX`: no empty slot.
    TableFull,
    /// A plugin with the same ID is registered under the name `folder`
    /// (`STR_PLGMGR_INST_NAME`).
    SameId { folder: String },
}

/// The rules for a plugin ID (`InstallPlugin`). The help also says the ID is made of ASCII letters
/// and digits and `{}._-`; the code only refuses [`RESERVED_CHARS`] and a digit at the start.
pub fn check_id(id: &str) -> Result<(), InstallError> {
    if id.chars().count() > ID_MAX_CHARS {
        return Err(InstallError::IdTooLong);
    }
    if let Some(c) = id.chars().find(|c| RESERVED_CHARS.contains(*c)) {
        return Err(InstallError::ReservedChar(c));
    }
    if id.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(InstallError::IdStartsWithDigit);
    }
    Ok(())
}

/// Whether two folder names are the same (`wmemicmp`: the case is ignored, also of letters that are
/// not ASCII).
pub fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// The two places that hold the folders of the plugins (`CPluginManager::CPluginManager`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchDirs {
    /// `plugins\` in the folder of the settings file (`m_sBaseDir`). The options are kept here.
    pub user: PathBuf,
    /// `plugins\` in the folder of the executable (`m_sExePluginDir`); `None` when it is the same.
    pub exe: Option<PathBuf>,
}

impl SearchDirs {
    /// The places for the settings file `settings` and the executable in `exe_dir`.
    pub fn new(settings: &Path, exe_dir: Option<&Path>) -> Self {
        let user = settings
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(FOLDER_NAME);
        let exe = exe_dir.map(|dir| dir.join(FOLDER_NAME)).filter(|p| *p != user);
        SearchDirs { user, exe }
    }

    /// The folders, the user's first.
    pub fn all(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.user.as_path()).chain(self.exe.as_deref())
    }

    /// `plugin.def` of the plugin in `folder`: the user's `plugins\` is looked at first.
    pub fn def_file(&self, folder: &str) -> Option<PathBuf> {
        self.all()
            .map(|dir| dir.join(folder).join(DEF_FILE_NAME))
            .find(|path| path.is_file())
    }

    /// The folder of the plugin `folder`.
    pub fn plugin_dir(&self, folder: &str) -> Option<PathBuf> {
        self.def_file(folder)
            .and_then(|file| file.parent().map(Path::to_owned))
    }

    /// The file of the options of the plugin `folder`: `plugins\<folder>.ini` next to the settings
    /// (`CPlugin::GetOptionPath`).
    pub fn option_file(&self, folder: &str) -> PathBuf {
        self.user.join(format!("{folder}.ini"))
    }

    /// Reads the definition of the plugin in `folder` and its language file
    /// `local\plugin_<language>.def` (`CPluginManager::LoadPlugin`). `None` when there is no
    /// `plugin.def`.
    pub fn load_def(&self, folder: &str, language: &str) -> Option<PluginDef> {
        let file = self.def_file(folder)?;
        let main = std::fs::read(&file).ok()?;
        let dir = file.parent()?;
        let l10n = std::fs::read(
            dir.join(def::L10N_DIR)
                .join(format!("plugin_{language}.def")),
        )
        .ok();
        Some(PluginDef::from_bytes(&main, l10n.as_deref()))
    }

    /// `SearchNewPlugin`: makes the user's `plugins\` if it is not there. Returns whether the folder
    /// is there now.
    pub fn ensure_user_dir(&self) -> bool {
        self.user.is_dir() || std::fs::create_dir_all(&self.user).is_ok()
    }

    /// `SearchNewPluginDir`: the folders that have a `plugin.def` and are not registered yet,
    /// the user's `plugins\` first and then the executable's. A hidden folder and the folder named
    /// `unuse` are not looked at.
    pub fn find_new(&self, settings: &PluginSettings) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for dir in self.all() {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut names: Vec<String> = entries
                .flatten()
                .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
                .filter(|entry| !is_hidden(entry))
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| !same_name(name, UNUSED_FOLDER))
                .collect();
            names.sort_by_key(|name| name.to_lowercase());
            for name in names {
                if settings.find_folder(&name).is_some()
                    || found.iter().any(|f| same_name(f, &name))
                {
                    continue;
                }
                if dir.join(&name).join(DEF_FILE_NAME).is_file() {
                    found.push(name);
                }
            }
        }
        found
    }
}

/// A folder that the file manager hides (`FILE_ATTRIBUTE_HIDDEN`; a name with a dot at the start on
/// the other systems).
fn is_hidden(entry: &std::fs::DirEntry) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        entry
            .metadata()
            .is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
    }
    #[cfg(not(windows))]
    {
        entry.file_name().to_string_lossy().starts_with('.')
    }
}

thread_local! {
    /// Where the plugins are, for the code that has no way to be told (the loader of a page of the
    /// settings gets only the file). The window sets it on its thread before it reads the
    /// settings. A thread of its own keeps the tests from sharing it.
    static SEARCH_DIRS: RefCell<Option<SearchDirs>> = const { RefCell::new(None) };
}

/// Tells this thread where the plugins are (`None`: nowhere).
pub fn set_search_dirs(dirs: Option<SearchDirs>) {
    SEARCH_DIRS.with(|cell| *cell.borrow_mut() = dirs);
}

/// Where the plugins are, as [`set_search_dirs`] set it on this thread.
pub fn search_dirs() -> Option<SearchDirs> {
    SEARCH_DIRS.with(|cell| cell.borrow().clone())
}

#[cfg(test)]
mod tests;
