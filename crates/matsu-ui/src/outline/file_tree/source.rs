// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files:
//   sakura_core/outline/CDlgFuncList.cpp (CDlgFuncList::LoadFileTreeSetting)
//   sakura_core/dlg/CDlgTagJumpList.cpp (CalcMaxUpDirectory, DirUp)
//   sakura_core/util/file.cpp (GetInidirOrExedir)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2010, the authors named in the original files (genta, Stonee, JEPRO, hor,
//     MIK, aroka, YAZAKI, Moca, frozen, zenryaku, naoh, little YOSHI, novice, ryoji, SUI and others)
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a dialog, fixed
// buffers, backslash paths and the current folder of the process) was rewritten in Rust on
// `Path` and `Profile`, the folder the project file is looked for from is given by the caller,
// and the tests are new.
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

//! Which items the file tree shows: the common settings or the settings of the type, or a project
//! file (`LoadFileTreeSetting`).
//!
//! 1. The settings are `[Common]` when 「アウトライン解析のドッキング位置の設定」 (`nOutlineDockSet`)
//!    is 0, else the `[Types(N)]` section of the document's type.
//! 2. With `bFileTreeProject`, the folder of the document and the folders above it are searched
//!    for the project file (`_sakurafiletree.ini`, the name is `szFileTreeDefIniName`); the nearest
//!    one gives the items, and its folder is `<iniroot>`.
//! 3. Else, if `szFileTreeProjectIni` names a file that can be read (a relative name starts at the
//!    folder of the settings file, then at the folder of the program), that file gives the items.
//! 4. Else the items of the settings.
//!
//! The current folder of Sakura's process is the folder of the document it opened (Sakura moves
//! it there); here it is [`Inputs::base_dir`].

use std::path::{Path, PathBuf};

use matsu_config::Profile;
use matsu_config::dialog_dir::MetaSource;
use matsu_config::settings::file_tree::{
    COMMON_SECTION, FileTree, FileTreeItem, default_project_ini_name, read_project_items,
};
use matsu_config::settings::types::TypeSettings;

use crate::grep::keys::is_relative;

/// How many folders above the current one are searched (`_MAX_PATH / 2`, the most that
/// `CalcMaxUpDirectory` gives).
const MAX_UP: usize = 130;

/// What the lookup needs, collected before it runs in the background.
pub(crate) struct Inputs {
    /// The settings file as it is.
    pub profile: Profile,
    /// The index of the document type (`[Types(N)]`).
    pub type_index: usize,
    /// The current folder: where `.` is, and where the search for the project file starts.
    pub base_dir: PathBuf,
    /// The folders that the meta folders and the relative project file name need.
    pub meta: Box<dyn MetaSource + Send>,
}

/// What the lookup found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Loaded {
    pub items: Vec<FileTreeItem>,
    /// The project file the items came from (`m_szLoadProjectIni`).
    pub project_ini: Option<PathBuf>,
    /// The folder of the project file that was found by the search (`IniDirPath`). It is not set for
    /// the project file of the settings, so `<iniroot>` there is empty, as in Sakura.
    pub ini_root: Option<PathBuf>,
}

/// `LoadFileTreeSetting`.
pub(crate) fn load_setting(inputs: &Inputs) -> Loaded {
    load_dialog_setting(inputs).loaded
}

/// Where the settings of the tree are kept: `EFileTreeSettingFrom`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingFrom {
    /// `EFileTreeSettingFrom_Common`: `[Common]`.
    Common,
    /// `EFileTreeSettingFrom_Type`: the `[Types(N)]` of the document's type.
    Type,
    /// `EFileTreeSettingFrom_File`: a project file found above the document.
    File,
}

/// What `LoadFileTreeSetting` fills in for `CDlgFileTree` (`CFileTreeSetting`): besides the items,
/// where they are kept and what the settings say about project files.
#[derive(Clone, Debug)]
pub(crate) struct DialogSetting {
    /// `m_eFileTreeSettingLoadType`: where the items came from.
    pub loaded_from: SettingFrom,
    /// The section of the settings file that holds the settings, where OK writes them
    /// (`m_eFileTreeSettingOrgType`: `[Common]` or `[Types(N)]`).
    pub section: String,
    /// `m_bProject`.
    pub project: bool,
    /// `m_szDefaultProjectIni`.
    pub default_project_ini: String,
    /// The items, the project file they came from and its folder.
    pub loaded: Loaded,
}

/// `LoadFileTreeSetting`, with everything the dialog needs.
pub(crate) fn load_dialog_setting(inputs: &Inputs) -> DialogSetting {
    let profile = &inputs.profile;
    let by_type = profile
        .get_i32(COMMON_SECTION, "nOutlineDockSet")
        .is_some_and(|set| set != 0);
    let section = if by_type {
        TypeSettings::section_name(i32::try_from(inputs.type_index).unwrap_or(0))
    } else {
        COMMON_SECTION.to_owned()
    };
    let origin = if by_type {
        SettingFrom::Type
    } else {
        SettingFrom::Common
    };
    let tree = FileTree::from_profile(profile, &section);
    let setting = |loaded_from: SettingFrom, loaded: Loaded| DialogSetting {
        loaded_from,
        section: section.clone(),
        project: tree.project,
        default_project_ini: tree.project_ini.clone(),
        loaded,
    };
    if tree.project
        && let Some(found) = find_project_file(&inputs.base_dir, &default_project_ini_name(profile))
    {
        return setting(SettingFrom::File, found);
    }
    if !tree.project_ini.is_empty()
        && let Some(loaded) = read_default_project_file(inputs, &tree.project_ini)
    {
        return setting(origin, loaded);
    }
    let loaded = Loaded {
        items: tree.items.clone(),
        ..Loaded::default()
    };
    setting(origin, loaded)
}

/// The project file that `szFileTreeProjectIni` names (a relative name starts at the folder of the
/// settings file), when it can be read.
pub(crate) fn read_default_project_file(inputs: &Inputs, name: &str) -> Option<Loaded> {
    // the settings are written with Windows paths; a backslash separates folders on every OS
    let on_this_os = if cfg!(windows) {
        name.to_owned()
    } else {
        name.replace('\\', "/")
    };
    let path = if is_relative(name) {
        inidir_or_exedir(inputs.meta.as_ref(), &on_this_os)?
    } else {
        PathBuf::from(&on_this_os)
    };
    let project = Profile::load(&path).ok()?;
    Some(Loaded {
        items: read_project_items(&project),
        project_ini: Some(path),
        ini_root: None,
    })
}

/// The project file called `name` in `start` or the nearest folder above it.
pub(crate) fn find_project_file(start: &Path, name: &str) -> Option<Loaded> {
    for dir in start.ancestors().take(MAX_UP + 1) {
        let path = dir.join(name);
        if let Ok(project) = Profile::load(&path) {
            return Some(Loaded {
                items: read_project_items(&project),
                project_ini: Some(path),
                ini_root: Some(dir.to_owned()),
            });
        }
    }
    None
}

/// `GetInidirOrExedir`: the file in the folder of the settings file if it is there, else the one
/// in the folder of the program if that is there, else the first one (which is not there).
fn inidir_or_exedir(meta: &dyn MetaSource, file: &str) -> Option<PathBuf> {
    let in_settings = meta.settings_dir().map(|dir| Path::new(&dir).join(file));
    if let Some(path) = &in_settings
        && path.exists()
    {
        return in_settings;
    }
    let in_program = meta.program_dir().map(|dir| Path::new(&dir).join(file));
    if let Some(path) = &in_program
        && path.exists()
    {
        return in_program;
    }
    in_settings
}
