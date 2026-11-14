//! ZIPプラグインを導入 (`IDC_PLUGIN_INST_ZIP`) and the ReadMe button (`IDC_PLUGIN_README`) of the
//! page 「プラグイン」 (ADR-0026, PR 11).
//!
//! Sakura: `CPropPlugin::DispatchEvent` (`sakura_core/prop/CPropComPlugin.cpp`) asks for a `.zip`
//! file and calls `CPluginManager::InstZipPlugin`, which checks the file (`ChkPluginDef`), asks
//! when the plugin is installed already (`STR_PLGMGR_INST_ZIP_ALREADY`), unpacks it into
//! `plugins\` (`Unzip`) and registers it (`InstallPlugin` with the update flag on). The rules for
//! the file are in [`matsu_config::plugins::zip`].
//!
//! The plugin is registered as an added plugin: off, until the user enables it on the
//! confirmation screen (ADR-0026, chapter 5).
//!
//! The ReadMe button is `CPropPlugin::GetReadMeFile` (the file) and `BrowseReadMe` (a new editor
//! window with `-R -CODE=99`, read-only, the encoding detected).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use gpui::{Context, PathPromptOptions, PromptLevel, Window};
use matsu_config::plugins::SearchDirs;
use matsu_config::plugins::zip::ZipPlugin;

use super::super::super::dialog::CommonSettingsDialog;
use super::super::super::model::Values;
use super::list::{self, add_folder, install_error_text, notice};
use super::rows;

pub(super) const INSTALL_ZIP: &str = "plugin_install_zip";
pub(super) const README: &str = "plugin_readme";

/// What the dialog says when a plugin could not be installed (`STR_PLGMGR_INST_ZIP_ERR`).
fn install_failed(zip: &str, reason: &str) -> String {
    format!("プラグイン「{zip}」をインストールできませんでした\n理由：{reason}")
}

/// 「ZIPプラグインを導入」: asks for a file, checks it, asks when the plugin is there already,
/// unpacks it and puts it in the list.
pub(super) fn install_zip(
    _: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let Some(dirs) = super::search_dirs(cx) else {
        notice(window, cx, "プラグインフォルダーがありません");
        return;
    };
    let chosen = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("ZIPファイルを選んでください".into()),
    });
    cx.spawn_in(window, async move |this, cx| {
        let Ok(Ok(Some(paths))) = chosen.await else {
            return;
        };
        let Some(path) = paths.into_iter().next() else {
            return;
        };
        let shown = path.display().to_string();
        // The file is read and checked off the UI thread (it can be 64 MiB).
        let inspected = cx
            .background_executor()
            .spawn({
                let path = path.clone();
                async move { ZipPlugin::inspect(&path) }
            })
            .await;
        let zip = match inspected {
            Ok(zip) => zip,
            Err(error) => {
                let message = install_failed(&shown, &error.text());
                this.update_in(cx, |_, window, cx| notice(window, cx, &message))
                    .ok();
                return;
            }
        };
        let Ok(known) = this.update(cx, |this, _| is_installed(this.values(), &dirs, &zip)) else {
            return;
        };
        if known {
            // `STR_PLGMGR_INST_ZIP_ALREADY`
            let question = format!("「{shown}」は既にインストールされています\n上書きしますか？");
            let Ok(answer) = this.update_in(cx, |_, window, cx| {
                crate::boxes::prompt(
                    window,
                    PromptLevel::Info,
                    &question,
                    None,
                    &["はい", "いいえ"],
                    cx,
                )
            }) else {
                return;
            };
            if !matches!(answer.await, Ok(0)) {
                return;
            }
        }
        let user = dirs.user.clone();
        let zip = std::sync::Arc::new(zip);
        let unpacked = cx
            .background_executor()
            .spawn({
                let zip = zip.clone();
                async move { zip.extract(&user) }
            })
            .await;
        let message = match unpacked {
            // `STR_PLGMGR_INST_ZIP_UNZIP`
            Err(error) => Some(format!(
                "ZIPプラグイン「{shown}」の解凍に失敗しました\n{}",
                error.text()
            )),
            Ok(()) => this
                .update_in(cx, |this, _, cx| {
                    register(this, &dirs, &zip, cx)
                        .err()
                        .map(|reason| install_failed(&shown, &reason))
                })
                .ok()
                .flatten(),
        };
        if let Some(message) = message {
            this.update_in(cx, |_, window, cx| notice(window, cx, &message))
                .ok();
        }
    })
    .detach();
}

/// Whether the plugin of `zip` is in the table, or its folder is in `plugins\` already: files
/// would be replaced.
pub(super) fn is_installed(values: &Values, dirs: &SearchDirs, zip: &ZipPlugin) -> bool {
    let settings = rows::settings_of(values.rows(list::LIST), values.bool(list::ENABLE));
    settings.find_folder(&zip.folder).is_some() || dirs.user.join(&zip.folder).exists()
}

/// `InstallPlugin` after the unzip: the plugin in `plugins\<folder>` goes into the list, replacing
/// a plugin with the same ID (Sakura passes the update flag, so it does not ask again).
pub(super) fn register(
    dialog: &mut CommonSettingsDialog,
    dirs: &SearchDirs,
    zip: &ZipPlugin,
    cx: &mut Context<CommonSettingsDialog>,
) -> Result<usize, String> {
    add_folder(dialog, dirs, &zip.folder, true, cx).map_err(|e| install_error_text(&e))
}

/// ReadMe can be used on a plugin that has a ReadMe file (`EnableWindow( IDC_PLUGIN_README, ...
/// !GetReadMeFile(...).empty() )`).
pub(super) fn can_open_readme(values: &Values) -> bool {
    selected_readme(values).is_some()
}

fn selected_readme(values: &Values) -> Option<PathBuf> {
    let index = usize::try_from(values.int(list::SELECTED)).ok()?;
    let row = values.rows(list::LIST).get(index)?;
    if !rows::has_plugin(row) {
        return None;
    }
    matsu_config::plugins::search_dirs()?.readme_file(&row[rows::FOLDER])
}

/// ReadMe: opens the file in a new editor window, read-only, with the encoding detected
/// (`BrowseReadMe`: `-R -CODE=99`).
pub(super) fn open_readme(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    match selected_readme(dialog.values()) {
        Some(file) => {
            if !launch(&readme_arguments(&file)) {
                // `STR_PROPCOMPLG_ERR2`
                notice(window, cx, "ReadMeファイルが開けません");
            }
        }
        // `STR_PROPCOMPLG_ERR3`
        None => notice(window, cx, "ReadMeファイルが見つかりません "),
    }
}

/// The command line that opens a ReadMe, as `BrowseReadMe` builds it.
pub(super) fn readme_arguments(file: &Path) -> Vec<OsString> {
    vec![file.into(), "-R".into(), "-CODE=99".into()]
}

#[cfg(test)]
pub(super) static LAUNCHED: std::sync::Mutex<Vec<Vec<OsString>>> =
    std::sync::Mutex::new(Vec::new());

/// Starts this program with `arguments` (`CreateProcess` of `BrowseReadMe`). A running MATSU Editor
/// takes the launch over and opens the file in a window of its own (ADR-0016).
#[cfg(not(test))]
fn launch(arguments: &[OsString]) -> bool {
    let Ok(program) = std::env::current_exe() else {
        return false;
    };
    match std::process::Command::new(program)
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            // Nobody needs the result; the thread only keeps the finished process from staying
            // in the process table.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            true
        }
        Err(_) => false,
    }
}

/// The tests do not start the test program again; they read what would have been started.
#[cfg(test)]
fn launch(arguments: &[OsString]) -> bool {
    LAUNCHED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(arguments.to_vec());
    true
}
