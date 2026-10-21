//! 「マクロ」: `IDD_PROP_MACRO` (`sakura_core/sakura_rc.rc`), `CPropMacro`
//! (`sakura_core/prop/CPropComMacro.cpp`).
//!
//! The page registers up to 50 macros (a name, a file, and whether the file is read again each
//! time the macro runs) that keys, menus and the toolbar call as `F_USERMACRO_0 + n`, and names
//! the macros that run by themselves after a file is opened, after the type changed and before a
//! file is saved. OK writes `[Macro]` (`ShareData_IO_Macro`) and `szMACROFOLDER` of `[Folders]`
//! (`ShareData_IO_Folders`) under the lock of the settings file; the rest of the file stays.
//!
//! * [`rows`]: the list (a [`Kind::Table`] control that the page draws itself) and what 設定 does
//!   to it.
//! * [`list`]: the widgets: the list, the click on a row, 設定 and the folder button.
//!
//! The runtime side is in [`crate::macros`] (`registered.rs`).
//!
//! Not available here (shown greyed out, never written):
//!
//! * `Pythonフォルダー`: the macro host finds its Python itself (ADR-0007).
//! * `キャンセル確認待ち時間`: Sakura waits that many seconds and then asks whether to stop a
//!   Windows Script Host macro (`CWSH`). A macro here has the button 中止 in its panel from the
//!   start, so there is no wait to set.
//!
//! The file box is an edit box, not a drop-down list of the files of the macro folder.

mod list;
mod rows;

use super::super::Env;
use super::super::model::{Control, Item, Values};

/// `STR_PROPCOMMACR_*` and the dialog `IDD_PROP_MACRO`. The controls with the prefix `macro_` hold
/// only the state of the page (no key in the file).
pub(in crate::common_settings) fn items() -> Option<Vec<Item>> {
    Some(vec![
        Item::Row(vec![
            Control::text("macro_dir", "マクロフォルダー", "szMACROFOLDER", "")
                .in_section("Folders")
                .into(),
            Control::button("macro_dir_ref", "...", list::browse_folder)
                .named("マクロフォルダーを参照")
                .into(),
        ]),
        Item::Row(vec![
            Control::text(
                "macro_python_dir",
                "Pythonフォルダー",
                "nMacroPythonDirectory",
                "",
            )
            .in_section("Macro")
            .unavailable()
            .into(),
            Control::button("macro_python_ref", "...", list::browse_python)
                .named("Pythonフォルダーを参照")
                .unavailable()
                .into(),
        ]),
        Item::Custom(list::draw),
        Item::Row(vec![
            Control::text(list::NAME, "名前(&N)", "", "").into(),
            Control::number(list::ID, "Id:", "", 0, 0, 49).into(),
        ]),
        Item::Row(vec![
            Control::text(list::FILE, "&File", "", "").into(),
            Control::button("macro_register", "設定(&A)", list::register).into(),
        ]),
        Item::Row(vec![
            Control::check(
                list::RELOAD,
                "マクロを実行するたびにファイルを読み込みなおす（&L）",
                "",
                false,
            )
            .into(),
        ]),
        Item::Row(vec![
            Control::number(
                "macro_cancel_timer",
                "キャンセル確認待ち時間",
                "nMacroCancelTimer",
                10,
                0,
                9999,
            )
            .in_section("Macro")
            .unavailable()
            .into(),
            Item::Label("秒"),
        ]),
        Item::Row(vec![
            Item::Label("自動実行:"),
            Control::check(list::ON_OPENED, "新規／開ファイル後(&O)", "", false).into(),
            Control::check(list::ON_TYPE_CHANGED, "タイプ変更後(&T)", "", false).into(),
            Control::check(list::ON_SAVE, "ファイル保存前(&S)", "", false).into(),
        ]),
        // The state of the list: the rows (`IDC_MACROLIST`), kept in the file as `[Macro]`, and
        // the row that is selected (-1: none). The list is last, so that its write also puts the
        // backslash at the end of the macro folder written before it.
        Item::Hidden(vec![
            Control::choice(list::SELECTED, "", "", -1, &[]),
            Control::table(list::LIST, "", rows::spec()).named("登録マクロ"),
        ]),
    ])
}

/// What OK does to the windows. The settings are the new ones when this runs, and a macro is
/// found in them each time it runs, so there is nothing to tell the windows.
pub(in crate::common_settings) fn apply(_env: &mut Env, _after: &Values, _before: Option<&Values>) {
}

#[cfg(test)]
mod tests;
