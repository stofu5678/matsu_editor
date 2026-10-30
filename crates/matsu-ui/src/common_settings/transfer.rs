//! What the インポート and エクスポート buttons of the settings pages share (`CImpExpManager::ImportUI` and
//! `ExportUI`, `sakura_core/typeprop/CImpExpManager.cpp`): the dialog with the file types of the kind of
//! file and the import folder, the import folder kept after the choice, and the messages.

use std::path::{Path, PathBuf};

use gpui::{Context, PromptLevel, Window};

use super::dialog::CommonSettingsDialog;
use crate::file_dialog::{self, Ask, import_folder};

/// `STR_IMPEXP_ERR_FILEOPEN`.
pub(crate) fn cannot_open(path: &Path) -> String {
    format!("ファイルを開けませんでした。\n\n{}", path.display())
}

/// `STR_IMPEXP_ERR_EXPORT`.
pub(crate) fn cannot_export(path: &Path) -> String {
    format!("エクスポート出来ませんでした。\n\n{}", path.display())
}

/// `STR_IMPEXP_OK_EXPORT`.
pub(crate) fn exported(path: &Path) -> String {
    format!("ファイルをエクスポートしました。\n\n{}", path.display())
}

/// Tells the user something (`InfoMessage`). Nobody waits for the answer.
pub(crate) fn say(message: &str, window: &mut Window, cx: &mut Context<CommonSettingsDialog>) {
    crate::boxes::tell(window, PromptLevel::Info, message, None, cx);
}

/// What to do with the file that was chosen.
pub(crate) type Chosen =
    fn(&mut CommonSettingsDialog, &Path, &mut Window, &mut Context<CommonSettingsDialog>);

impl CommonSettingsDialog {
    /// Shows `ask` for a file of the kind `wildcard` names (`*.key`) in the import folder, and when a
    /// file is chosen keeps its folder as the import folder (`SetImportFolder`) and calls `chosen`.
    pub(crate) fn pick_settings_file(
        &mut self,
        ask: Ask,
        wildcard: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
        chosen: Chosen,
    ) {
        let ask = import_folder::ask_for(cx, ask, wildcard);
        let receiver = file_dialog::ask(window, cx, ask);
        cx.spawn_in(window, async move |this, cx| {
            let path: Option<PathBuf> = receiver
                .await
                .ok()
                .flatten()
                .and_then(|paths| paths.into_iter().next());
            if let Some(path) = path {
                this.update_in(cx, |this, window, cx| {
                    import_folder::remember(cx, &path);
                    chosen(this, &path, window, cx);
                })
                .ok();
            }
        })
        .detach();
    }
}
