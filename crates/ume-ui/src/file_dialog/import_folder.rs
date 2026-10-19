//! The folder the import and export dialogs of the settings start in (`m_szIMPORTFOLDER`,
//! `[Folders]` `szIMPORTFOLDER`), and the file types they list.
//!
//! Sakura: `CImpExpManager::ImportUI` and `ExportUI` (`sakura_core/typeprop/CImpExpManager.cpp`) make
//! the dialog with the wildcard of the kind of file (`GetDefaultExtension`: `*.key`, `*.mnu`, `*.kwd`,
//! `*.rkw` ...) and the import folder, and `SetImportFolder` keeps the folder of the file that was
//! chosen. The folder is the folder of the settings file (`GetInidir`) until then
//! (`CShareData::InitShareData`).

use std::path::{Path, PathBuf};

use gpui::{App, Global};

use super::{Ask, save_types};
use crate::settings::UserSettings;

const SECTION: &str = "Folders";
const KEY: &str = "szIMPORTFOLDER";

/// The folder that was chosen last in this session. The settings file has it too, but the settings
/// the windows hold are the ones read at the start.
struct Remembered(PathBuf);

impl Global for Remembered {}

/// The dialog for a file of the kind that `wildcard` names (`*.key`): the types ユーザー指定 (the
/// wildcard), テキストファイル and すべてのファイル, and the import folder.
pub(crate) fn ask_for(cx: &App, ask: Ask, wildcard: &str) -> Ask {
    ask.save_types(save_types::SaveTypes {
        filters: save_types::filters(wildcard),
        // `SetDefaultExtension(L"txt")` of every save dialog.
        default_extension: Some("txt".to_owned()),
    })
    .start(import_folder(cx))
}

/// The import folder: the one chosen last, else the one of the settings file, else the folder of the
/// settings file. `None` when there is none that exists.
pub(crate) fn import_folder(cx: &App) -> Option<PathBuf> {
    if let Some(Remembered(dir)) = cx.try_global::<Remembered>()
        && dir.is_dir()
    {
        return Some(dir.clone());
    }
    let written = cx
        .try_global::<UserSettings>()
        .and_then(UserSettings::profile)
        .and_then(|profile| profile.get(SECTION, KEY).map(str::to_owned));
    written
        .and_then(|text| super::expanded_folder(cx, &text))
        .or_else(|| super::settings_dir(cx).filter(|dir| dir.is_dir()))
}

/// `SetImportFolder`: the folder of `file` is the import folder from now on, and the settings file
/// says so.
pub(crate) fn remember(cx: &mut App, file: &Path) {
    let Some(dir) = file.parent().filter(|dir| !dir.as_os_str().is_empty()) else {
        return;
    };
    cx.set_global(Remembered(dir.to_owned()));
    // `wcscat_s( m_szIMPORTFOLDER, L"\\" )`
    let mut text = dir.to_string_lossy().into_owned();
    text.push(std::path::MAIN_SEPARATOR);
    crate::history::write_settings(cx, |profile| {
        if profile.get(SECTION, KEY) == Some(text.as_str()) {
            return Ok(false);
        }
        profile.set(SECTION, KEY, &text)?;
        Ok(true)
    });
}
