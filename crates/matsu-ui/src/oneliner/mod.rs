//! Running a registered one-liner on the text of the window (ADR-0029): a shell command line that gets the selection,
//! or the whole document, on its standard input, and whose output replaces that text, goes on the next line, or goes
//! to the output window.
//!
//! The process and the rules for the text on both sides of it are `matsu_oneliner` and `matsu_core::oneliner`. This
//! is what the window does with them:
//!
//! * [`flow`]: reads the text and the settings, starts the process, shows the box 「コマンド実行中・・・」 of 外部コマンド実行
//!   (`crate::exec::running`) while it runs, and puts the result in the document (one undo step, the new text
//!   selected), in the output window, or nowhere.
//! * [`dialog`]: 「ワンライナーを入力して実行」 with its history and 「登録...」.
//! * [`OneLinerList`]: the one-liners as the items of a menu (`F_ONELINER_LIST`).
//! * [`store`]: what the dialog changes in `[OneLiner]` of the settings file: the history and a new registration.
//!   The settings page (another PR) changes the rest.
//!
//! The functions are `ui_commands/oneliner/`: `F_ONELINER_0` to `F_ONELINER_49` and `F_ONELINER_DIALOG`.

use gpui::App;
use matsu_config::settings::oneliner::OneLiners;

use crate::settings::UserSettings;

pub(crate) mod dialog;
pub(crate) mod flow;
mod list;
pub(crate) mod store;
#[cfg(test)]
mod tests;
mod workdir;

pub(crate) use list::OneLinerList;

/// The name of the submenu in the right-click menu and the main menu.
pub(crate) const SUBMENU_NAME: &str = "ワンライナー";

/// The registered one-liners and the history now.
pub(crate) fn table(cx: &App) -> OneLiners {
    cx.try_global::<UserSettings>()
        .map(UserSettings::oneliners)
        .unwrap_or_default()
}
