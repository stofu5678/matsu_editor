//! インポート and エクスポート: `CPropMainMenu::Import` and `Export`, `CImpExpMainMenu`
//! (`sakura_core/typeprop/CImpExpManager.cpp`), the `.ini` file.
//!
//! The file is a profile with the version in `[Info]` and the menu in `[MainMenu]`, the functions as
//! macro names where they have one. Importing reads the file over the menu of the page and shows
//! the new menu; nothing is written to the settings file until OK.

use std::path::{Path, PathBuf};

use gpui::{Context, PromptLevel, Window};
use matsu_config::Profile;
use matsu_config::settings::main_menu::MNU_VERSION;

use super::super::super::dialog::CommonSettingsDialog;
use super::data::{self, State};
use super::{RES_SEL, state_of};
use crate::file_dialog::{self, Ask};

/// `STR_IMPEXP_ERR_FILEOPEN`.
fn cannot_open(path: &Path) -> String {
    format!("ファイルを開けませんでした。\n\n{}", path.display())
}

/// `STR_IMPEXP_MEINMENU`.
fn wrong_format(path: &Path) -> String {
    format!(
        "メインメニュー設定ファイルの形式が違います。\n\n{}",
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
    crate::boxes::tell(window, PromptLevel::Info, &message, None, cx);
}

impl CommonSettingsDialog {
    /// 「インポート」: asks for a file and reads it over the menu.
    pub(super) fn main_menu_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = file_dialog::ask(
            window,
            cx,
            Ask::open(false).titled("メインメニュー設定のインポート", "インポート"),
        );
        cx.spawn_in(window, async move |this, cx| {
            let chosen = receiver.await.ok().flatten();
            if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
                this.update_in(cx, |this, window, cx| {
                    this.main_menu_import_file(&path, window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// `CImpExpMainMenu::Import` of `path`. A file that cannot be read or is not a main menu file is
    /// said so and changes nothing. Returns whether the menu was read.
    pub(super) fn main_menu_import_file(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Ok(profile) = Profile::load(path) else {
            say(cannot_open(path), window, cx);
            return false;
        };
        let before = state_of(self.values()).menu();
        let mut menu = before.clone();
        if !menu.import_profile(&profile, &data::code_of_name) {
            say(wrong_format(path), window, cx);
            return false;
        }
        // A file with no menu in it leaves the page as it is.
        if menu != before {
            self.main_menu_put(&State::of_menu(&menu), cx);
            self.set_int(RES_SEL, -1, cx);
            self.main_menu_show_selected(window, cx);
        }
        true
    }

    /// 「エクスポート」: asks for a name and writes the menu.
    pub(super) fn main_menu_export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = file_dialog::ask(
            window,
            cx,
            Ask::save().name(Some("MainMenu.ini".to_owned())),
        );
        cx.spawn_in(window, async move |this, cx| {
            let chosen: Option<PathBuf> = receiver
                .await
                .ok()
                .flatten()
                .and_then(|paths| paths.into_iter().next());
            if let Some(path) = chosen {
                this.update_in(cx, |this, window, cx| {
                    this.main_menu_export_file(&path, window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// `CImpExpMainMenu::Export` to `path`: the menu as OK would write it. Returns whether the file
    /// was written.
    pub(super) fn main_menu_export_file(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let written = state_of(self.values())
            .menu()
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
