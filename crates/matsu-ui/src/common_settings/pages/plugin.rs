//! 「プラグイン」: `IDD_PROP_PLUGIN` (`sakura_core/sakura_rc.rc`), `CPropPlugin`
//! (`sakura_core/prop/CPropComPlugin.cpp`), ADR-0026.
//!
//! The page switches the plugins on (`EnablePlugin`), lists the 40 slots of the plugin table
//! (`[Plugin]` of the settings file: `P[nn].Name`, `.Id`, `.CmdNum`), adds the plugin folders that
//! it finds in `plugins\` (新規プラグインを追加, `CPluginManager::SearchNewPlugin`) and removes a
//! plugin from the table (削除, `UninstallPlugin`). OK writes the section `[Plugin]` under the lock of
//! the settings file; the rest of the file stays.
//!
//! * [`rows`]: the rows of the list, how they are made from the table and the folders, and how
//!   the table is made from them.
//! * [`list`]: the widgets: the list with the description of the selected plugin, and the buttons.
//!
//! Nothing runs a plugin yet (ADR-0026, the PR table): there is no host, no consent screen, no
//! jack. The state 「登録済み」 says that the plugin is in the table and its folder is there, and the
//! states for what MATSU cannot run (「VBScript は使えません」, 「DLL プラグインは使えません」) follow
//! ADR-0026, chapter 1.
//!
//! Not built here (they come in later PRs of ADR-0026): ZIPプラグインを導入, 設定 (the options of a
//! plugin), ReadMe.

mod list;
pub(in crate::common_settings) mod rows;

use gpui::App;
use matsu_config::plugins::{SearchDirs, set_search_dirs};

use super::super::Env;
use super::super::model::{Control, Item, Values};

/// The folders of the plugins for the settings file and the executable of this application.
pub(in crate::common_settings) fn search_dirs(cx: &App) -> Option<SearchDirs> {
    let settings = crate::doc_types::with_user(cx, crate::settings::history_file)?;
    Some(SearchDirs::new(
        &settings,
        crate::support::help::exe_dir().as_deref(),
    ))
}

/// Tells the loader of the page where the plugins are (the loader gets only the settings file).
/// Call it before the values of the page are loaded.
pub(in crate::common_settings) fn refresh_dirs(cx: &App) {
    set_search_dirs(search_dirs(cx));
}

/// `IDD_PROP_PLUGIN`. The controls with the prefix `plugin_` hold only the state of the page.
pub(in crate::common_settings) fn items() -> Option<Vec<Item>> {
    Some(vec![
        Item::Row(vec![
            Control::check(
                list::ENABLE,
                "プラグインを有効にする(&E)",
                "EnablePlugin",
                false,
            )
            .in_section(matsu_config::plugins::SECTION)
            .into(),
            Control::button(list::OPEN_FOLDER, "フォルダーを開く(&F)", list::open_folder).into(),
            Control::button(
                list::SEARCH_NEW,
                "新規プラグインを追加(&I)",
                list::search_new,
            )
            .when(list::enabled)
            .into(),
        ]),
        Item::Label("プラグイン一覧"),
        Item::Custom(list::draw),
        Item::Row(vec![
            Control::button(list::REMOVE, "削除(&R)", list::remove)
                .when(list::can_remove)
                .into(),
            Control::button(list::URL, "配布元(&D)", list::open_url)
                .when(list::can_open_url)
                .into(),
        ]),
        // The state of the list: the slots (kept in the file as `[Plugin]`), and the row that is
        // selected (-1: none).
        Item::Hidden(vec![
            Control::choice(list::SELECTED, "", "", -1, &[]),
            Control::table(list::LIST, "", rows::spec()).named("プラグイン一覧"),
        ]),
    ])
}

/// What OK does to the windows. Nothing runs the plugins yet, so there is nothing to tell them.
pub(in crate::common_settings) fn apply(_env: &mut Env, _after: &Values, _before: Option<&Values>) {
}

#[cfg(test)]
mod tests;
