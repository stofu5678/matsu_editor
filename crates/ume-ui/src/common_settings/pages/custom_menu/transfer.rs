//! インポート and エクスポート: `CPropCustmenu::Import` and `Export`, `CImpExpCustMenu`
//! (`sakura_core/typeprop/CImpExpManager.cpp`), the `.mnu` file.
//!
//! The file is a profile with the version in `[Info]` and the 25 menus in `[CustMenu]`, the
//! functions as macro names where they have one. Importing reads the file over the menus of the
//! page (what the file does not have stays); nothing is written to the settings file until OK.

use std::path::{Path, PathBuf};

use gpui::{Context, PromptLevel, Window};
use ume_config::Profile;
use ume_config::settings::custom_menu::MNU_VERSION;

use super::super::super::dialog::CommonSettingsDialog;
use super::{current_menu, data, menus_of};
use crate::file_dialog::{self, Ask};

/// `STR_IMPEXP_ERR_FILEOPEN`.
fn cannot_open(path: &Path) -> String {
    format!("ファイルを開けませんでした。\n\n{}", path.display())
}

/// `STR_IMPEXP_CUSTMENU_FORMAT`.
fn wrong_format(path: &Path) -> String {
    format!(
        "カスタムメニュー設定ファイルの形式が違います。\n\n{}",
        path.display()
    )
}

/// `STR_IMPEXP_ERR_EXPORT`.
fn cannot_export(path: &Path) -> String {
    format!("エクスポート出来ませんでした。\n\n{}", path.display())
}

/// `STR_IMPEXP_OK_EXPORT`.
fn exported(path: &Path) -> String {
    format!("ファイルをエクスポートしました。\n\n{}", path.display())
}

fn say(message: String, window: &mut Window, cx: &mut Context<CommonSettingsDialog>) {
    // Nobody waits for this answer.
    drop(window.prompt(PromptLevel::Info, &message, None, &["OK"], cx));
}

impl CommonSettingsDialog {
    /// 「インポート」: asks for a file and reads it over the menus.
    pub(super) fn import_menus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = file_dialog::ask(
            window,
            cx,
            Ask::open(false).titled("カスタムメニュー設定のインポート", "インポート"),
        );
        cx.spawn_in(window, async move |this, cx| {
            let chosen = receiver.await.ok().flatten();
            if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
                this.update_in(cx, |this, window, cx| {
                    this.import_menu_file(&path, window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// `CImpExpCustMenu::Import` of `path`. A file that cannot be read or is not a menu file is
    /// said so and changes nothing. Returns whether the menus were read.
    pub(super) fn import_menu_file(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Ok(profile) = Profile::load(path) else {
            say(cannot_open(path), window, cx);
            return false;
        };
        let mut menus = menus_of(self.values());
        if !menus.import_profile(&profile, &data::code_of_name) {
            say(wrong_format(path), window, cx);
            return false;
        }
        self.put_menus(&menus, cx);
        // The list shows the menu that is chosen again (`CBN_SELCHANGE` of `IDC_COMBO_MENU`).
        let index = current_menu(self.values());
        self.select_menu(index, window, cx);
        true
    }

    /// 「エクスポート」: asks for a name and writes the menus.
    pub(super) fn export_menus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = file_dialog::ask(window, cx, Ask::save().name(Some("menu.mnu".to_owned())));
        cx.spawn_in(window, async move |this, cx| {
            let chosen: Option<PathBuf> = receiver
                .await
                .ok()
                .flatten()
                .and_then(|paths| paths.into_iter().next());
            if let Some(path) = chosen {
                this.update_in(cx, |this, window, cx| {
                    this.export_menu_file(&path, window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// `CImpExpCustMenu::Export` to `path`. Returns whether the file was written.
    pub(super) fn export_menu_file(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let written = menus_of(self.values())
            .export_profile(&data::name_of_code)
            .ok()
            .is_some_and(|profile| profile.save_sakura_style(path, Some(MNU_VERSION)).is_ok());
        say(
            if written {
                exported(path)
            } else {
                cannot_export(path)
            },
            window,
            cx,
        );
        written
    }
}
