// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_Macro), sakura_core/env/CDataProfile.cpp
//   (IOProfileData<MacroRec>), sakura_core/func/CFuncLookup.h (MacroRec), sakura_core/env/CShareData.cpp
//   (InitShareData), sakura_core/util/string_ex2.cpp (AddLastChar)
//
// Original copyright notices of those files:
//   Copyright (C) 2001, genta
//   Copyright (C) 2002, YAZAKI
//   Copyright (C) 2003, Moca
//   Copyright (C) 2005, D.S.Koba
//   Copyright (C) 2006, ryoji
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

//! The registered macros (登録マクロ): the section `[Macro]`.
//!
//! Sakura keeps 50 macros in the shared data (`m_Common.m_sMacro.m_MacroTable`, `MAX_CUSTMACRO`): a name, a
//! file and whether the file is read again each time the macro runs. A key, a menu item or a toolbar button
//! calls macro `n` as the function `F_USERMACRO_0 + n`. Three of them can run by themselves: after a file is
//! opened, after the type of the document changed and before a file is saved (`nMacroOnOpened` and the like).
//!
//! [`RegisteredMacros::from_profile`] reads the section the way `ShareData_IO_Macro` does and
//! [`RegisteredMacros::store`] writes it. The folder of the macros is `szMACROFOLDER` in `[Folders]`
//! (`ShareData_IO_Folders`), see [`macro_folder`].

use crate::profile::{Profile, ProfileError};

/// The section of the file.
pub const SECTION: &str = "Macro";
/// The section that holds the folder of the macros (`ShareData_IO_Folders`).
pub const FOLDERS_SECTION: &str = "Folders";
/// `MAX_CUSTMACRO` (`sakura_core/config/maxdata.h`).
pub const MAX_CUSTMACRO: usize = 50;
/// `MACRONAME_MAX`, with the end of the string: a name has at most 63 characters.
pub const MACRONAME_MAX: usize = 64;
/// `_MAX_PATH`: a file has at most 260 characters.
pub const MAX_PATH: usize = 260;
/// `F_USERMACRO_0` (`sakura_core/Funccode_x.hsrc`): the function of macro 0.
pub const F_USERMACRO_0: u16 = 31600;
/// `nMacroCancelTimer` of a new settings file (`CShareData::InitShareData`), in seconds.
pub const DEFAULT_CANCEL_TIMER: i32 = 10;

/// The macro that the function `code` stands for (`F_USERMACRO_0 <= code < F_USERMACRO_0 + MAX_CUSTMACRO`).
pub fn index_of_function(code: u16) -> Option<usize> {
    let n = usize::from(code.checked_sub(F_USERMACRO_0)?);
    (n < MAX_CUSTMACRO).then_some(n)
}

/// The function that calls macro `index`.
pub fn function_of_index(index: usize) -> Option<u16> {
    let n = u16::try_from(index)
        .ok()
        .filter(|_| index < MAX_CUSTMACRO)?;
    Some(F_USERMACRO_0 + n)
}

/// One registered macro (`MacroRec`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MacroRec {
    /// The name shown in menus and on the toolbar (`m_szName`).
    pub name: String,
    /// The file, with no folder when it is in the macro folder (`m_szFile`).
    pub file: String,
    /// Read the file again each time the macro runs (`m_bReloadWhenExecute`).
    pub reload: bool,
}

impl MacroRec {
    /// A macro with a file can run (`MacroRec::IsEnabled`).
    pub fn is_enabled(&self) -> bool {
        !self.file.is_empty()
    }

    /// The name, or the file when the macro has no name (`MacroRec::GetTitle`).
    pub fn title(&self) -> &str {
        if self.name.is_empty() {
            &self.file
        } else {
            &self.name
        }
    }
}

/// The 50 registered macros and the three that run by themselves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredMacros {
    pub macros: Vec<MacroRec>,
    /// `nMacroOnOpened`: the macro that runs after a file is opened.
    pub on_opened: Option<usize>,
    /// `nMacroOnTypeChanged`: after the type of the document changed.
    pub on_type_changed: Option<usize>,
    /// `nMacroOnSave`: before a file is saved.
    pub on_save: Option<usize>,
    /// `nMacroCancelTimer`: the seconds before the dialog that stops a macro.
    pub cancel_timer: i32,
}

impl Default for RegisteredMacros {
    fn default() -> Self {
        Self {
            macros: vec![MacroRec::default(); MAX_CUSTMACRO],
            on_opened: None,
            on_type_changed: None,
            on_save: None,
            cancel_timer: DEFAULT_CANCEL_TIMER,
        }
    }
}

/// `Name[003]`, `File[003]` or `ReloadWhenExecute[003]`: the key of macro `index`
/// (`strprintf(L"[%03d]", i)` is the suffix).
fn key(prefix: &str, index: usize) -> String {
    format!("{prefix}[{index:03}]")
}

/// The number of an automatic macro: `-1` and anything outside the table is none.
fn auto_index(profile: &Profile, name: &str) -> Option<usize> {
    let n = profile.get_i32(SECTION, name)?;
    usize::try_from(n).ok().filter(|&n| n < MAX_CUSTMACRO)
}

fn auto_number(index: Option<usize>) -> i32 {
    index.and_then(|n| i32::try_from(n).ok()).unwrap_or(-1)
}

impl RegisteredMacros {
    /// `ShareData_IO_Macro` in reading mode. A macro whose `Name` or `File` key is missing is left
    /// empty (`IOProfileData<MacroRec>` gives up at the first key it does not find); a missing
    /// `ReloadWhenExecute` is off.
    pub fn from_profile(profile: &Profile) -> Self {
        let mut table = Self::default();
        for (i, rec) in table.macros.iter_mut().enumerate() {
            let (Some(name), Some(file)) = (
                profile.get_string(SECTION, &key("Name", i)),
                profile.get_string(SECTION, &key("File", i)),
            ) else {
                continue;
            };
            rec.name = name;
            rec.file = file;
            rec.reload = profile
                .get_bool(SECTION, &key("ReloadWhenExecute", i))
                .unwrap_or(false);
        }
        table.on_opened = auto_index(profile, "nMacroOnOpened");
        table.on_type_changed = auto_index(profile, "nMacroOnTypeChanged");
        table.on_save = auto_index(profile, "nMacroOnSave");
        table.cancel_timer = profile
            .get_i32(SECTION, "nMacroCancelTimer")
            .unwrap_or(DEFAULT_CANCEL_TIMER);
        table
    }

    /// The macro `index`, if it is in the table.
    pub fn get(&self, index: usize) -> Option<&MacroRec> {
        self.macros.get(index)
    }

    /// The macro that the function `code` calls.
    pub fn of_function(&self, code: u16) -> Option<&MacroRec> {
        self.get(index_of_function(code)?)
    }

    /// `ShareData_IO_Macro` in writing mode. A macro with no name and no file is not written (and
    /// its keys are removed); the numbers of the automatic macros and the timer always are.
    /// Returns whether the file changed; a value the file already has is left as written.
    pub fn store(&self, profile: &mut Profile) -> Result<bool, ProfileError> {
        let mut changed = false;
        let mut gone = Vec::new();
        for (i, rec) in self.macros.iter().enumerate() {
            if rec.name.is_empty() && rec.file.is_empty() {
                gone.extend(["Name", "File", "ReloadWhenExecute"].map(|p| key(p, i)));
                continue;
            }
            for (name, value) in [("Name", &rec.name), ("File", &rec.file)] {
                let name = key(name, i);
                if profile.get(SECTION, &name) != Some(value.as_str()) {
                    profile.set(SECTION, &name, value)?;
                    changed = true;
                }
            }
            let name = key("ReloadWhenExecute", i);
            if profile.get_bool(SECTION, &name) != Some(rec.reload) {
                profile.set_bool(SECTION, &name, rec.reload)?;
                changed = true;
            }
        }
        for (name, value) in [
            ("nMacroOnOpened", auto_number(self.on_opened)),
            ("nMacroOnTypeChanged", auto_number(self.on_type_changed)),
            ("nMacroOnSave", auto_number(self.on_save)),
            ("nMacroCancelTimer", self.cancel_timer),
        ] {
            if profile.get_i32(SECTION, name) != Some(value) {
                profile.set_i32(SECTION, name, value)?;
                changed = true;
            }
        }
        for name in gone {
            changed |= profile.remove(SECTION, &name);
        }
        Ok(changed)
    }
}

/// `szMACROFOLDER` of `[Folders]` as written (`CShareData_IO::ShareData_IO_Folders`); `None` when the
/// file does not have it.
pub fn macro_folder(profile: &Profile) -> Option<String> {
    profile.get_string(FOLDERS_SECTION, "szMACROFOLDER")
}

/// `AddLastChar(folder, L'\\')` of `CPropMacro::GetData`: the folder ends with a backslash. A folder that
/// is empty stays empty here (Sakura makes it `\`, which a macro file is then looked for in the folder of
/// the settings file; an empty folder means the same without the root of a Unix file system in it).
pub fn with_last_separator(folder: &str) -> String {
    if folder.is_empty() || folder.ends_with('\\') {
        folder.to_owned()
    } else {
        format!("{folder}\\")
    }
}

/// Writes the folder of the macros into `[Folders]`; returns whether the file changed.
pub fn store_macro_folder(profile: &mut Profile, folder: &str) -> Result<bool, ProfileError> {
    if profile.get(FOLDERS_SECTION, "szMACROFOLDER") == Some(folder) {
        return Ok(false);
    }
    profile.set(FOLDERS_SECTION, "szMACROFOLDER", folder)?;
    Ok(true)
}

#[cfg(test)]
mod tests;
