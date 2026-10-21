// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/env/CSakuraEnvironment.cpp (GetDlgInitialDir)
//   sakura_core/env/CFileNameManager.cpp (ExpandMetaToFolder)
//   sakura_core/env/CommonSetting.h (EOpenDialogDir)
//   sakura_core/env/CShareData_IO.cpp (eOpenDialogDir, szOpenDialogSelDir)
//   sakura_core/cmd/CViewCommander_File.cpp (Command_FILEOPEN, the default name)
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2002, Moca, genta
// Copyright (C) 2007, ryoji
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (the registry,
// `GetCurrentDirectory`, `GetFileAttributes` and fixed buffers) was rewritten in Rust behind the
// traits of this file, so that the same code runs on every OS, and the tests are new.
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

//! Where the file dialogs start: Sakura's 「ファイルダイアログの初期位置」 (`eOpenDialogDir`).
//!
//! The setting has three values: the current folder (カレントフォルダー, the default), the
//! recently used folder (最近使ったフォルダー) and a given folder (指定フォルダー,
//! `szOpenDialogSelDir`, `%Personal%\` by default), both in the section `[Common]`. A document
//! that has a file always starts its dialogs in the folder of the file
//! (`CSakuraEnvironment::GetDlgInitialDir`).
//!
//! The given folder may hold meta folders such as `%MYDOC%` (`CFileNameManager::ExpandMetaToFolder`).
//! [`expand_meta`] expands them; where a name comes from is up to a [`MetaSource`], because Sakura
//! reads the Windows registry for most of them.
//!
//! Sakura's `bControlProcess` rule (the tray process, which has no document, uses the recent folder
//! instead of the current one) has no counterpart here: every window of UME Editor is an editor
//! window. Nor does UME Editor move the current folder of the process as Sakura does for an untitled
//! document (`CEditDoc::SetCurDirNotitle`); the recent folder and the given folder are chosen
//! directly.

use std::path::{Path, PathBuf};

use crate::Profile;

/// The section and the keys of the setting (`ShareData_IO_Common`).
const SECTION: &str = "Common";
const KEY_DIR: &str = "eOpenDialogDir";
const KEY_SELECTED: &str = "szOpenDialogSelDir";

/// `m_OpenDialogSelDir` before the user changes it (`CShareData::InitCommon`).
pub const DEFAULT_SELECTED: &str = "%Personal%\\";

/// `EOpenDialogDir`: where a document without a file starts its dialogs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OpenDialogDir {
    /// `OPENDIALOGDIR_CUR`, カレントフォルダー.
    #[default]
    Current,
    /// `OPENDIALOGDIR_MRU`, 最近使ったフォルダー.
    Recent,
    /// `OPENDIALOGDIR_SEL`, 指定フォルダー.
    Selected,
}

impl OpenDialogDir {
    /// The setting from its number in the settings file. Sakura throws on any other number; here
    /// it is `None`, and the default is used.
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Current),
            1 => Some(Self::Recent),
            2 => Some(Self::Selected),
            _ => None,
        }
    }

    /// The number in the settings file.
    pub fn code(self) -> i32 {
        match self {
            Self::Current => 0,
            Self::Recent => 1,
            Self::Selected => 2,
        }
    }
}

/// What the settings say about the start folder of the file dialogs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogDirSetting {
    pub mode: OpenDialogDir,
    /// `szOpenDialogSelDir`, with its meta folders not expanded.
    pub selected: String,
}

impl Default for DialogDirSetting {
    fn default() -> Self {
        Self {
            mode: OpenDialogDir::default(),
            selected: DEFAULT_SELECTED.to_owned(),
        }
    }
}

impl DialogDirSetting {
    /// Reads `[Common]` `eOpenDialogDir` and `szOpenDialogSelDir`; what is missing or not valid
    /// is Sakura's default.
    pub fn from_profile(profile: &Profile) -> Self {
        let default = Self::default();
        Self {
            mode: profile
                .get_i32(SECTION, KEY_DIR)
                .and_then(OpenDialogDir::from_code)
                .unwrap_or(default.mode),
            selected: profile
                .get_string(SECTION, KEY_SELECTED)
                .unwrap_or(default.selected),
        }
    }
}

/// Where the meta folders of [`expand_meta`] come from.
pub trait MetaSource {
    /// The folder of the program (`%SAKURA%`).
    fn program_dir(&self) -> Option<String>;
    /// The folder of the settings file (`%SAKURADATA%`).
    fn settings_dir(&self) -> Option<String>;
    /// A folder of the user by the name Windows gives it in the registry key `Shell Folders`
    /// (`Personal`, `Desktop`, `My Pictures`, `Common Desktop` ...). `name` is after the aliases
    /// of Sakura (`MYDOC` is `Personal`).
    fn shell_folder(&self, name: &str) -> Option<String>;
    /// An environment variable.
    fn env_var(&self, name: &str) -> Option<String>;
}

/// The result of [`expand_meta`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expanded {
    /// The text as far as it could be expanded.
    pub text: String,
    /// False when a meta folder is empty or unknown to the source and has no environment variable
    /// either. Sakura leaves the buffer as far as it got in that case; so does this.
    pub complete: bool,
}

/// The short names Sakura allows for the folders of the registry (`AliasList`).
const ALIASES: [(&str, &str); 9] = [
    ("COMDESKTOP", "Common Desktop"),
    ("COMMUSIC", "CommonMusic"),
    ("COMVIDEO", "CommonVideo"),
    ("MYMUSIC", "My Music"),
    ("MYVIDEO", "Video"),
    ("COMPICT", "CommonPictures"),
    ("MYPICT", "My Pictures"),
    ("COMDOC", "Common Documents"),
    ("MYDOC", "Personal"),
];

/// `ExpandMetaToFolder`: `%%` is `%`; `%SAKURA%` and `%SAKURADATA%` are the folder of the program
/// and of the settings; `%name%` is a folder of the user (an alias such as `%MYDOC%` or the name
/// of the registry), else an environment variable, else it stays as typed. The last backslash of
/// the folder is cut. A `%` without a closing `%` is a `%`.
///
/// The names are compared without regard to case.
pub fn expand_meta(src: &str, source: &dyn MetaSource) -> Expanded {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '%' {
            out.push(c);
            i += 1;
            continue;
        }
        // `%%`
        if chars.get(i + 1) == Some(&'%') {
            out.push('%');
            i += 2;
            continue;
        }
        // a `%` at the end
        if i + 1 == chars.len() {
            out.push('%');
            i += 1;
            continue;
        }
        let rest = &chars[i + 1..];
        let (path, length) = if starts_with_ignore_case(rest, "SAKURA%") {
            (source.program_dir(), "SAKURA".len())
        } else if starts_with_ignore_case(rest, "SAKURADATA%") {
            (source.settings_dir(), "SAKURADATA".len())
        } else if let Some(end) = rest.iter().position(|&c| c == '%') {
            let meta: String = rest[..end].iter().collect();
            let name = ALIASES
                .iter()
                .find(|(alias, _)| alias.eq_ignore_ascii_case(&meta))
                .map_or(meta.as_str(), |(_, original)| original);
            let found = source
                .shell_folder(name)
                .filter(|path| !path.is_empty())
                .or_else(|| source.env_var(name));
            match found {
                Some(path) => (Some(path), end),
                None => {
                    // not defined: the text as typed
                    out.push('%');
                    out.push_str(&meta);
                    out.push('%');
                    i += end + 2;
                    continue;
                }
            }
        } else {
            // no closing `%`: only this one is copied
            out.push('%');
            i += 1;
            continue;
        };
        let mut path = path.unwrap_or_default();
        // the last backslash goes, even from a root such as `A:\`
        if path.ends_with('\\') {
            path.pop();
        }
        if path.is_empty() {
            return Expanded {
                text: out,
                complete: false,
            };
        }
        out.push_str(&path);
        i += length + 2;
    }
    Expanded {
        text: out,
        complete: true,
    }
}

fn starts_with_ignore_case(chars: &[char], prefix: &str) -> bool {
    let mut it = chars.iter();
    prefix
        .chars()
        .all(|p| it.next().is_some_and(|c| c.eq_ignore_ascii_case(&p)))
}

/// What [`DialogDirSetting::initial_dir`] needs to know of the window and the machine.
pub struct Surroundings<'a> {
    /// The file of the document, if it has one.
    pub document: Option<&'a Path>,
    /// The folders of the history that the menu shows, newest first (`CMRUFolder::GetPathList`).
    pub recent_folders: &'a [String],
    /// The current folder of the process.
    pub current_dir: Option<PathBuf>,
    /// The meta folders.
    pub meta: &'a dyn MetaSource,
    /// Whether a folder exists.
    pub is_dir: &'a dyn Fn(&Path) -> bool,
}

impl DialogDirSetting {
    /// `GetDlgInitialDir`: the folder the dialogs start in, or `None` when the dialog is to start
    /// where the OS wants (Sakura passes a folder that does not exist on, and the dialog then
    /// ignores it).
    ///
    /// * A document with a file: the folder of the file.
    /// * Current: the current folder of the process.
    /// * Recent: the first of the recent folders that exists, else the current folder.
    /// * Selected: the given folder with its meta folders expanded.
    pub fn initial_dir(&self, around: &Surroundings<'_>) -> Option<PathBuf> {
        let existing = |path: PathBuf| {
            (!path.as_os_str().is_empty() && (around.is_dir)(&path)).then_some(path)
        };
        if let Some(dir) = around
            .document
            .and_then(Path::parent)
            .filter(|dir| !dir.as_os_str().is_empty())
        {
            return existing(dir.to_owned());
        }
        match self.mode {
            OpenDialogDir::Current => around.current_dir.clone().and_then(existing),
            OpenDialogDir::Recent => around
                .recent_folders
                .iter()
                .map(PathBuf::from)
                .find_map(existing)
                .or_else(|| around.current_dir.clone().and_then(existing)),
            OpenDialogDir::Selected => {
                let expanded = expand_meta(&self.selected, around.meta).text;
                existing(PathBuf::from(native_separators(&expanded)))
            }
        }
    }
}

/// The settings file of Windows holds backslashes (`%Personal%\`); on the other OSes they are
/// separators too.
fn native_separators(text: &str) -> String {
    if cfg!(windows) {
        text.to_owned()
    } else {
        text.replace('\\', "/")
    }
}

/// The folder the macro file dialogs start in: `[Folders]` `szMACROFOLDER` (マクロ用フォルダー), which
/// is the folder of the settings file when the file does not say (`CShareData::InitCommon`).
///
/// A relative folder is taken from the folder of the settings file if that exists, else from the
/// folder of the program, else from the settings file again (`GetInidirOrExedir`,
/// `sakura_core/util/file.cpp`), as `Command_SAVEKEYMACRO`, `Command_LOADKEYMACRO` and
/// `Command_EXECEXTMACRO` do (`sakura_core/cmd/CViewCommander_Macro.cpp`). `None` when the base
/// folder it needs is not known.
pub fn macro_folder(
    setting: Option<&str>,
    settings_dir: Option<&Path>,
    program_dir: Option<&Path>,
    is_dir: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let Some(text) = setting else {
        return settings_dir.map(Path::to_owned);
    };
    let folder = PathBuf::from(native_separators(text));
    if folder.is_absolute() {
        return Some(folder);
    }
    let from_settings = settings_dir.map(|dir| dir.join(&folder));
    if let Some(path) = from_settings.as_ref().filter(|path| is_dir(path)) {
        return Some(path.clone());
    }
    if let Some(path) = program_dir
        .map(|dir| dir.join(&folder))
        .filter(|path| is_dir(path))
    {
        return Some(path);
    }
    from_settings
}

/// The file name of the recorded key macro (`CShareData::GetMacroFilename( -1 )`).
pub const KEY_MACRO_FILE: &str = "RecKey.mac";

/// The file that stopping a key macro recording writes: `RecKey.mac` in the macro folder
/// (`Command_RECKEYMACRO` asks `CShareData::GetMacroFilename( -1 )`, `sakura_core/env/CShareData.cpp`).
/// The folder is found as [`macro_folder`] does.
///
/// Sakura leaves the name bare when the folder is empty, which means the current directory of the
/// process. This editor uses the folder of the settings file instead, the folder that an empty macro
/// folder means everywhere else. `None` when the base folder it needs is not known.
pub fn key_macro_file(
    setting: Option<&str>,
    settings_dir: Option<&Path>,
    program_dir: Option<&Path>,
    is_dir: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    macro_folder(setting, settings_dir, program_dir, is_dir).map(|dir| dir.join(KEY_MACRO_FILE))
}

/// The fourth argument (`defaultName`) of the macro `FileOpen`: the folder the dialog starts in
/// and the file name it offers (`Command_FILEOPEN`, `sakura_core/cmd/CViewCommander_File.cpp`).
///
/// The folder is everything up to the last separator (`_wsplitpath`'s drive and directory). A text
/// that ends with a separator names a folder only. A file name is offered only when its folder
/// exists (`is_dir`), else the dialog starts where the OS wants.
pub fn split_default_name(
    default_name: &str,
    is_dir: &dyn Fn(&Path) -> bool,
) -> (Option<PathBuf>, Option<String>) {
    if default_name.is_empty() {
        return (None, None);
    }
    let cut = default_name.rfind(['\\', '/']).map_or(0, |at| at + 1);
    let (dir, name) = default_name.split_at(cut);
    let dir_path = PathBuf::from(native_separators(dir));
    let exists = !dir.is_empty() && is_dir(&dir_path);
    let start = exists.then_some(dir_path);
    // a name without a folder is offered as it is
    let name = (!name.is_empty() && (dir.is_empty() || exists)).then(|| name.to_owned());
    (start, name)
}

#[cfg(test)]
mod tests;
