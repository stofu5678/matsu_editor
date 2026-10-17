// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/recent/CRecentCmd.cpp, CRecentCurDir.cpp, CRecentImp.h (AppendItem),
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_Cmd, and nExecFlgOpt of ShareData_IO_Other),
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

//! What 外部コマンド実行 remembers: the commands that were run, the folders they were run in, and
//! the choices of the dialog.
//!
//! Sakura keeps them in the shared data (`m_sHistory.m_aCommands`, `m_aCurDirs`, `m_nExecFlgOpt`)
//! and writes them to the file:
//!
//! ```text
//! [Cmd]
//! nCmdArrNum=2
//! szCmdArr[00]=sort
//! szCmdArr[01]=dir
//! nCurDirArrNum=1
//! szCurDirArr[00]=C:\work
//! [Other]
//! nExecFlgOpt=1
//! ```
//!
//! Both lists are newest first, at most 32 long, and never keep a string twice (`CRecentImp`).
//! Everything else in the file is left as it is.

use crate::profile::{Profile, ProfileError};
use crate::value::parse_long;

/// The section of the two lists.
pub const SECTION: &str = "Cmd";
/// The section of the choices (`ShareData_IO_Other`).
pub const OTHER_SECTION: &str = "Other";
/// `MAX_CMDARR`: how many commands and how many folders are kept.
pub const MAX_ENTRIES: usize = 32;
/// `MAX_CMDLEN`: a command must be shorter than this many UTF-16 units to be kept.
pub const MAX_COMMAND_UNITS: usize = 1024;
/// `_MAX_PATH`: a folder must be shorter than this many UTF-16 units to be kept.
pub const MAX_DIR_UNITS: usize = 260;

/// A history: newest first, at most [`MAX_ENTRIES`] strings shorter than `max_units`, none twice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recent {
    items: Vec<String>,
    max_units: usize,
}

impl Recent {
    fn new(max_units: usize) -> Self {
        Self {
            items: Vec::new(),
            max_units,
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

    fn fits(&self, text: &str) -> bool {
        !text.is_empty() && text.encode_utf16().count() < self.max_units
    }

    /// `CRecentImp::AppendItem`: puts `text` first; one that is already there moves to the front,
    /// and when the list is full the oldest goes. An empty string, or one that is too long, is
    /// not kept. Returns whether the list changed.
    pub fn add(&mut self, text: &str) -> bool {
        if !self.fits(text) {
            return false;
        }
        if let Some(i) = self.items.iter().position(|s| s == text) {
            if i == 0 {
                return false;
            }
            self.items.remove(i);
        } else if self.items.len() >= MAX_ENTRIES {
            self.items.truncate(MAX_ENTRIES - 1);
        }
        self.items.insert(0, text.to_owned());
        true
    }

    /// Takes in the strings of `other` (the file, as another window or process saved it): they go
    /// on top of ours in their order, and the strings only we have stay below them.
    fn merge_from(&mut self, other: &Recent) {
        for text in other.items.iter().rev() {
            self.add(text);
        }
    }
}

/// The commands, the folders and the choices of the dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecHistory {
    /// `m_aCommands`.
    pub commands: Recent,
    /// `m_aCurDirs`.
    pub cur_dirs: Recent,
    /// `m_nExecFlgOpt`: the sum of bits that the dialog started with the last time. `None` until
    /// the file or the dialog has one (the program starts with 標準出力を得る only).
    pub flags: Option<i32>,
}

impl Default for ExecHistory {
    fn default() -> Self {
        Self {
            commands: Recent::new(MAX_COMMAND_UNITS),
            cur_dirs: Recent::new(MAX_DIR_UNITS),
            flags: None,
        }
    }
}

impl ExecHistory {
    /// `AddToCmdArr`. Returns whether the list changed.
    pub fn add_command(&mut self, command: &str) -> bool {
        self.commands.add(command)
    }

    /// `CRecentCurDir::AppendItem`.
    pub fn add_cur_dir(&mut self, dir: &str) -> bool {
        self.cur_dirs.add(dir)
    }

    /// Takes in what the file has now (the ones of the file go on top of ours); the choices of the file are
    /// older than ours and are left out.
    pub fn merge_from(&mut self, other: &ExecHistory) {
        self.commands.merge_from(&other.commands);
        self.cur_dirs.merge_from(&other.cur_dirs);
        if self.flags.is_none() {
            self.flags = other.flags;
        }
    }

    /// Reads `[Cmd]` and `nExecFlgOpt` of `[Other]`. A missing section or entry means nothing.
    /// The old name of the choice, `bGetStdout`, is read when `nExecFlgOpt` is missing
    /// (`ShareData_IO_Other`).
    pub fn from_profile(profile: &Profile) -> Self {
        let mut history = Self::default();
        let keep = |recent: &Recent, items: Vec<String>| -> Vec<String> {
            items.into_iter().filter(|s| recent.fits(s)).collect()
        };
        let commands = read_set(profile, "szCmdArr", "nCmdArrNum");
        history.commands.items = keep(&history.commands, commands);
        let dirs = read_set(profile, "szCurDirArr", "nCurDirArrNum");
        history.cur_dirs.items = keep(&history.cur_dirs, dirs);
        history.flags = profile
            .get(OTHER_SECTION, "nExecFlgOpt")
            .or_else(|| profile.get(OTHER_SECTION, "bGetStdout"))
            .and_then(parse_long);
        history
    }

    /// Writes `[Cmd]` and `nExecFlgOpt`, leaving the rest of the file alone. A string that the
    /// file format cannot hold (a line break) is skipped.
    pub fn write_to(&self, profile: &mut Profile) -> Result<(), ProfileError> {
        write_set(
            profile,
            "szCmdArr",
            "nCmdArrNum",
            self.commands.items.as_slice(),
        )?;
        write_set(
            profile,
            "szCurDirArr",
            "nCurDirArrNum",
            self.cur_dirs.items.as_slice(),
        )?;
        if let Some(flags) = self.flags {
            profile.set_i32(OTHER_SECTION, "nExecFlgOpt", flags)?;
        }
        Ok(())
    }
}

/// `IOProfileDataSet`, reading, with the name of the count given: if the count is missing all
/// places are read, and the empty ones are dropped by the caller.
fn read_set(profile: &Profile, name: &str, count_name: &str) -> Vec<String> {
    let count = match profile.get(SECTION, count_name).and_then(parse_long) {
        Some(n) => usize::try_from(n.max(0)).unwrap_or(0).min(MAX_ENTRIES),
        None => MAX_ENTRIES,
    };
    (0..count)
        .filter_map(|i| profile.get(SECTION, &format!("{name}[{i:02}]")))
        .map(str::to_owned)
        .collect()
}

/// `IOProfileDataSet`, writing.
fn write_set(
    profile: &mut Profile,
    name: &str,
    count_name: &str,
    items: &[String],
) -> Result<(), ProfileError> {
    let items: Vec<&String> = items.iter().filter(|s| !s.contains(['\r', '\n'])).collect();
    let count = i32::try_from(items.len()).unwrap_or(i32::MAX);
    profile.set_i32(SECTION, count_name, count)?;
    for (i, item) in items.iter().enumerate() {
        profile.set(SECTION, &format!("{name}[{i:02}]"), item)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
