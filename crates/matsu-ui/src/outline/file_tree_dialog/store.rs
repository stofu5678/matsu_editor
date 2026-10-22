//! What OK writes, and the project files of the dialog (import, export), apart from the window.
//!
//! * The settings go to `matsu.ini` in the keys of Sakura (`ShareData_IO_FileTree`): `[Common]` or
//!   the `[Types(N)]` of the type, through [`crate::history::write_settings`], the lock every
//!   writer of the file takes, so that the other lines and what another window wrote meanwhile
//!   stay.
//! * A project file (`_sakurafiletree.ini`, `[FileTree]`) is read and written with
//!   `CImpExpFileTree::IO_FileTreeIni` (`sakura_core/typeprop/CImpExpManager.cpp`).

use std::io;
use std::path::Path;

use gpui::App;
use matsu_config::settings::file_tree::{
    FileTree, FileTreeItem, PROJECT_SECTION, read_project_items, write_project_items,
};
use matsu_config::{Profile, ProfileError};

use crate::settings::UserSettings;

/// The first line of a file that the export writes (`WSTR_FILETREE_HEAD_V1`).
pub(super) const EXPORT_HEAD: &str = "SakuraEditorFileTree_Ver1";

/// What `CDlgFileTree::GetData` puts into the settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    /// `[Common]` or `[Types(N)]`.
    pub section: String,
    /// 各フォルダーの設定を読み込む (`bFileTreeProject`).
    pub project: bool,
    /// デフォルト設定ファイル名 (`szFileTreeProjectIni`).
    pub project_ini: String,
    /// The items to keep in the settings. `None` when they come from a project file, which gets
    /// them instead: the settings keep their own then.
    pub items: Option<Vec<FileTreeItem>>,
}

/// Puts the outcome into the settings `profile` and says whether it changed.
pub(crate) fn write_outcome(
    profile: &mut Profile,
    outcome: &Outcome,
) -> Result<bool, ProfileError> {
    let before = profile.to_bytes();
    let tree = FileTree {
        project: outcome.project,
        project_ini: outcome.project_ini.clone(),
        items: outcome.items.clone().unwrap_or_default(),
    };
    if outcome.items.is_some() {
        tree.write_to(profile, &outcome.section)?;
    } else {
        tree.write_options_to(profile, &outcome.section)?;
    }
    Ok(profile.to_bytes() != before)
}

/// Writes the outcome to the settings file under its lock, and makes the settings the windows
/// hold the new ones. A file that cannot be written (no place for it, no right to write) leaves
/// the settings for this session, as the other dialogs do, when there is no settings file at all.
/// When there is one that cannot be written, nothing changes and the caller is told (`false`): the
/// pane reads the file first, so it would seem that OK did nothing.
pub(crate) fn commit(cx: &mut App, outcome: &Outcome) -> bool {
    let mut written: Option<Profile> = None;
    crate::history::write_settings(cx, |profile| {
        let changed = write_outcome(profile, outcome)?;
        written = Some(profile.clone());
        Ok(changed)
    });
    // written: the file now says what the outcome says. Otherwise nothing changes for the windows
    // either, so that キャンセル after the message leaves no trace.
    let persisted = match crate::history::settings_path(cx) {
        Some(file) => Profile::load(&file).is_ok_and(|mut on_disk| {
            write_outcome(&mut on_disk, outcome).is_ok_and(|changed| !changed)
        }),
        None => true,
    };
    if !persisted {
        return false;
    }
    let user = cx.try_global::<UserSettings>().cloned();
    let profile = written.unwrap_or_else(|| {
        let mut profile = user
            .as_ref()
            .and_then(|user| user.profile().cloned())
            .unwrap_or_default();
        // The values are valid, so this only fails for a key or value the file cannot hold.
        let _ = write_outcome(&mut profile, outcome);
        profile
    });
    let path = user.as_ref().and_then(crate::settings::history_file);
    cx.set_global(UserSettings::from_profile(profile, path));
    true
}

/// `CImpExpFileTree::Import`: the items of a project file. A file without the count of items
/// leaves the items as they are (`IO_FileTreeIni` reads the count with the number of items as
/// its default).
pub(crate) fn import_items(path: &Path, current: &[FileTreeItem]) -> io::Result<Vec<FileTreeItem>> {
    let profile = Profile::load(path)?;
    if profile.get(PROJECT_SECTION, "nFileTreeItemCount").is_none() {
        return Ok(current.to_vec());
    }
    Ok(read_project_items(&profile))
}

/// `CImpExpFileTree::Export`: a new file with the items in `[FileTree]`, written the way Sakura
/// writes its files.
pub(crate) fn export_items(path: &Path, items: &[FileTreeItem]) -> io::Result<()> {
    let mut profile = Profile::new();
    write_project_items(&mut profile, items).map_err(invalid)?;
    profile.save_sakura_style(path, Some(EXPORT_HEAD))
}

/// The items go back to the project file they came from (`CDlgFileTree::GetData` exports them to
/// `m_szLoadProjectIni`). Sakura writes the file anew; a file that is there keeps its other
/// lines here.
pub(crate) fn write_project_file(path: &Path, items: &[FileTreeItem]) -> io::Result<()> {
    match Profile::load(path) {
        Ok(mut profile) => {
            write_project_items(&mut profile, items).map_err(invalid)?;
            profile.save(path)
        }
        // (a file that is there and cannot be read is not replaced)
        Err(error) if error.kind() == io::ErrorKind::NotFound => export_items(path, items),
        Err(error) => Err(error),
    }
}

fn invalid(error: ProfileError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}
