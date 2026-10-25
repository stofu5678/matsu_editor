//! The widgets of the page 「プラグイン」 that the dialog has no control for: the list of the 40 slots
//! with the description of the selected plugin, and what the buttons do.
//!
//! The messages are Sakura's (`STR_PLGMGR_*` and `STR_PROPCOMPLG_*` of `sakura_core/sakura_rc.rc`).

use gpui::{
    AnyElement, App, ClickEvent, Context, InteractiveElement as _, IntoElement as _,
    ParentElement as _, PromptLevel, SharedString, StatefulInteractiveElement as _, Styled as _,
    Window, div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_component::{h_flex, v_flex};
use matsu_config::plugins::{InstallError, RESERVED_CHARS, SearchDirs};

use super::super::super::dialog::CommonSettingsDialog;
use super::super::super::model::Values;
use super::rows;

pub(super) const ENABLE: &str = "enable_plugin";
pub(super) const OPEN_FOLDER: &str = "plugin_open_folder";
pub(super) const SEARCH_NEW: &str = "plugin_search_new";
pub(super) const REMOVE: &str = "plugin_remove";
pub(super) const URL: &str = "plugin_url";
pub(super) const LIST: &str = "plugin_list";
pub(super) const SELECTED: &str = "plugin_selected";

const SELECTED_BG: u32 = 0x2f6fd0;
const BORDER: u32 = 0xaaaaaa;
const HEADING_BG: u32 = 0xe6e6e6;
const ROW_HEIGHT: f32 = 20.0;
const LABEL_WIDTH: f32 = 80.0;

/// The row that is selected, if any.
fn selected_row(values: &Values) -> Option<&rows::Row> {
    let index = usize::try_from(values.int(SELECTED)).ok()?;
    values.rows(LIST).get(index)
}

/// 新規プラグインを追加 can be used while the plugins are switched on (`EnablePluginPropInput`).
pub(super) fn enabled(values: &Values) -> bool {
    values.bool(ENABLE)
}

/// 削除 can be used on a slot that has a plugin (`IDC_PLUGIN_Remove`).
pub(super) fn can_remove(values: &Values) -> bool {
    selected_row(values).is_some_and(rows::has_plugin)
}

/// 配布元 can be used on a plugin that has a web address. Only `http` and `https` are opened: a
/// `plugin.def` is a file from somewhere else, and other kinds of address start programs.
pub(super) fn can_open_url(values: &Values) -> bool {
    selected_row(values)
        .and_then(|row| row.get(rows::URL))
        .is_some_and(|url| web_address(url))
}

pub(super) fn web_address(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// The list (`IDC_PLUGINLIST`) with the description, the author and the version of the plugin that
/// is selected (`IDC_LABEL_PLUGIN_Description` and the like).
pub(super) fn draw(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let spec = rows::spec();
    let values = dialog.values();
    let selected = usize::try_from(values.int(SELECTED)).ok();
    let width = |percent: u32| gpui::relative(percent as f32 / 100.);
    let heading = h_flex()
        .bg(rgb(HEADING_BG))
        .border_b_1()
        .border_color(rgb(BORDER))
        .children(
            spec.columns
                .iter()
                .filter(|c| c.width > 0)
                .map(|c| div().w(width(c.width)).px_1().child(c.title)),
        );
    let body: Vec<AnyElement> = values
        .rows(LIST)
        .iter()
        .enumerate()
        .map(|(index, row)| {
            h_flex()
                .id(SharedString::from(format!("plugin-row-{index}")))
                .h(px(ROW_HEIGHT))
                .items_center()
                .cursor_pointer()
                .when(selected == Some(index), |d| {
                    d.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                })
                .children(
                    spec.columns
                        .iter()
                        .zip(row)
                        .filter(|(c, _)| c.width > 0)
                        .map(|(c, text)| {
                            div()
                                .w(width(c.width))
                                .px_1()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(text.clone())
                        }),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    select(this, index, cx);
                }))
                .into_any_element()
        })
        .collect();
    let detail = |label: &'static str, column: usize| {
        let text = selected_row(values)
            .and_then(|row| row.get(column))
            .cloned()
            .unwrap_or_default();
        h_flex()
            .h(px(ROW_HEIGHT))
            .child(div().w(px(LABEL_WIDTH)).child(label))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(text),
            )
    };
    v_flex()
        .gap_2()
        .child(
            v_flex()
                .id("plugin-list")
                .h(px(180.))
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(BORDER))
                .child(heading)
                .child(
                    v_flex()
                        .id("plugin-list-rows")
                        .flex_1()
                        .overflow_y_scroll()
                        .children(body),
                ),
        )
        .child(
            v_flex()
                .child(detail("説明", rows::DESCRIPTION))
                .child(detail("作者", rows::AUTHOR))
                .child(detail("バージョン", rows::VERSION)),
        )
        .into_any_element()
}

/// A row is selected.
pub(super) fn select(
    dialog: &mut CommonSettingsDialog,
    index: usize,
    cx: &mut Context<CommonSettingsDialog>,
) {
    if index < dialog.values().rows(LIST).len() {
        dialog.set_int(SELECTED, i32::try_from(index).unwrap_or(-1), cx);
    }
}

/// Says something with one button (`InfoMessage`, `WarningMessage`). Nobody waits for the answer.
fn notice(window: &mut Window, cx: &mut App, message: &str) {
    drop(window.prompt(PromptLevel::Info, message, None, &["OK"], cx));
}

/// `STR_PLGMGR_INST_*`: why a plugin was not registered.
pub(super) fn install_error_text(error: &InstallError) -> String {
    match error {
        InstallError::NoDefinition => "プラグイン定義ファイル（plugin.def）がありません".into(),
        InstallError::NoId => "Plugin.IDがありません".into(),
        InstallError::ReservedChar(_) => {
            let list: String = RESERVED_CHARS.chars().filter(|c| *c != '\t').collect();
            format!("Plugin.IDに\"{list}\"は使用できません")
        }
        InstallError::IdStartsWithDigit => "Plugin.IDの先頭に数字は使用できません".into(),
        InstallError::IdTooLong => "Plugin.IDは63文字までです".into(),
        InstallError::FolderNameTooLong => "フォルダー名は63文字までです".into(),
        InstallError::TableFull => "プラグインをこれ以上登録できません".into(),
        InstallError::SameId { .. } => "ユーザーキャンセル".into(),
    }
}

/// The table as the list now has it.
fn settings_of(dialog: &CommonSettingsDialog) -> matsu_config::plugins::PluginSettings {
    rows::settings_of(dialog.values().rows(LIST), dialog.values().bool(ENABLE))
}

/// Registers the plugin in `folder` and puts it in the list (`InstallPlugin`). `replace` answers the
/// question of a plugin with the same ID; with `false` the error says which folder it is.
pub(super) fn add_folder(
    dialog: &mut CommonSettingsDialog,
    dirs: &SearchDirs,
    folder: &str,
    replace: bool,
    cx: &mut Context<CommonSettingsDialog>,
) -> Result<usize, InstallError> {
    let mut settings = settings_of(dialog);
    let index = settings.install(dirs, folder, replace)?;
    let fresh = rows::rows_of(&settings, Some(dirs));
    dialog.set_rows(LIST, fresh, cx);
    select(dialog, index, cx);
    Ok(index)
}

/// 新規プラグインを追加 (`CPluginManager::SearchNewPlugin`): looks for folders that have a
/// `plugin.def` and are not registered, and asks about each one.
pub(super) fn search_new(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let Some(dirs) = super::search_dirs(cx) else {
        notice(window, cx, "プラグインフォルダーがありません");
        return;
    };
    if !dirs.ensure_user_dir() {
        // `STR_PLGMGR_FOLDER`
        notice(window, cx, "プラグインフォルダーを作成出来ません");
        return;
    }
    let found = dirs.find_new(&settings_of(dialog));
    if found.is_empty() {
        // `STR_PLGMGR_NEWPLUGIN`
        notice(window, cx, "新しいプラグインは見つかりませんでした");
        return;
    }
    cx.spawn_in(window, async move |this, cx| {
        for folder in found {
            // `STR_PLGMGR_INSTALL` (`Select3Message`: はい, いいえ, キャンセル)
            let question = format!("プラグイン「{folder}」をインストールしますか？");
            let Ok(answer) = this.update_in(cx, |_, window, cx| {
                window.prompt(
                    PromptLevel::Info,
                    &question,
                    None,
                    &["はい", "いいえ", "キャンセル"],
                    cx,
                )
            }) else {
                return;
            };
            match answer.await {
                Ok(0) => {}
                Ok(1) => continue,
                _ => {
                    this.update_in(cx, |_, window, cx| {
                        notice(window, cx, "キャンセルされました");
                    })
                    .ok();
                    return;
                }
            }
            let mut result = this
                .update_in(cx, |this, _, cx| {
                    add_folder(this, &dirs, &folder, false, cx)
                })
                .map_err(|_| ());
            if let Ok(Err(InstallError::SameId { folder: old })) = &result {
                // `STR_PLGMGR_INST_NAME`
                let question = format!(
                    "同じプラグインが別の名前でインストールされています。上書きしますか？\n{}\n{}",
                    format_args!("　はい　→　新しい「{folder}」を使用"),
                    format_args!("　いいえ→　インストール済みの「{old}」を使用"),
                );
                let Ok(answer) = this.update_in(cx, |_, window, cx| {
                    window.prompt(PromptLevel::Info, &question, None, &["はい", "いいえ"], cx)
                }) else {
                    return;
                };
                result = if let Ok(0) = answer.await {
                    this.update_in(cx, |this, _, cx| add_folder(this, &dirs, &folder, true, cx))
                        .map_err(|_| ())
                } else {
                    continue;
                };
            }
            if let Ok(Err(error)) = result {
                // `STR_PLGMGR_INSTALL_ERR`
                let message = format!(
                    "プラグイン「{folder}」をインストールできませんでした\n理由：{}",
                    install_error_text(&error)
                );
                this.update_in(cx, |_, window, cx| notice(window, cx, &message))
                    .ok();
            }
        }
    })
    .detach();
}

/// 削除 (`UninstallPlugin`): the slot is emptied, except for its ID.
pub(super) fn remove_selected(
    dialog: &mut CommonSettingsDialog,
    dirs: Option<&SearchDirs>,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let Some(index) = usize::try_from(dialog.values().int(SELECTED)).ok() else {
        return;
    };
    let mut list = dialog.values().rows(LIST).to_vec();
    if !list.get(index).is_some_and(rows::has_plugin) {
        return;
    }
    rows::uninstall(&mut list, index, dirs);
    dialog.set_rows(LIST, list, cx);
}

/// 削除 asks first (`STR_PROPCOMPLG_DELETE`, with the name of the folder).
pub(super) fn remove(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let Some(row) = selected_row(dialog.values()).filter(|row| rows::has_plugin(row)) else {
        return;
    };
    let question = format!("{} を削除しますか", row[rows::FOLDER]);
    let answer = window.prompt(PromptLevel::Info, &question, None, &["はい", "いいえ"], cx);
    cx.spawn_in(window, async move |this, cx| {
        if let Ok(0) = answer.await {
            this.update_in(cx, |this, _, cx| {
                let dirs = super::search_dirs(cx);
                remove_selected(this, dirs.as_ref(), cx);
            })
            .ok();
        }
    })
    .detach();
}

/// フォルダーを開く (`IDC_PLUGIN_OpenFolder`): the folder `plugins\` next to the settings file.
pub(super) fn open_folder(
    _: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    match super::search_dirs(cx) {
        Some(dirs) if dirs.ensure_user_dir() => cx.open_with_system(&dirs.user),
        _ => notice(window, cx, "プラグインフォルダーを作成出来ません"),
    }
}

/// 配布元 (`IDC_PLUGIN_URL`): the address of the plugin in the browser.
pub(super) fn open_url(
    dialog: &mut CommonSettingsDialog,
    _: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let url = selected_row(dialog.values())
        .and_then(|row| row.get(rows::URL))
        .map(|url| url.trim().to_owned())
        .filter(|url| web_address(url));
    if let Some(url) = url {
        cx.open_url(&url);
    }
}
