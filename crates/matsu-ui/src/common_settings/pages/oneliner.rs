//! 「ワンライナー」: the settings page of the registered one-liners (ADR-0029, decision 7).
//!
//! Sakura has no such page, so it is MATSU Editor's own, after the pages of Sakura (and after 「シェル」,
//! whose shell the one-liners run in). It is made like the page 「マクロ」 (`pages/macros.rs`): a list of 50
//! numbered slots, and below it the fields of the slot the Id box names, which the button 設定 puts into the
//! list. OK writes `[OneLiner]` (`matsu_config::settings::oneliner`) under the lock of the settings file; the
//! history of the input dialog (`Recent`) and the keys this program does not know stay as the file has them.
//! Cancel writes nothing.
//!
//! * [`rows`]: the list (a table control that the page draws itself), the limits and what 設定 does.
//! * [`list`]: the widgets: the list, the click on a row, the note and 設定.
//!
//! A value that the file cannot hold (a line break, a name of 64 UTF-16 units or more, a command of 1024 or
//! more) is refused by 設定 with the reason, and a note in red under the fields says so while it is typed;
//! nothing is cut. A name with no command is refused too (the file drops a slot with no command).
//!
//! The windows read the settings at the time they use them, so what OK wrote is used at once. The other pages
//! of the dialog (the keys, the custom menus) name a one-liner by what the file has, not by what is edited
//! here and not yet written, as they do for the registered macros.

mod list;
mod rows;

use super::super::Env;
use super::super::model::{Control, Item, Values};

/// The controls with the prefix `oneliner_` hold only the state of the page (no key in the file): the list
/// is kept in the file as `[OneLiner]` by its own store.
pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Custom(list::draw),
        Item::Row(vec![
            Control::text(list::NAME, "名前(&N)", "", "").into(),
            Control::number(list::ID, "Id:", "", 0, 0, 49).into(),
        ]),
        Item::Row(vec![
            Control::text(list::COMMAND, "コマンド(&C)", "", "").into(),
            Control::button("oneliner_register", "設定(&A)", list::register).into(),
        ]),
        Item::Row(vec![
            Control::choice(
                list::OUTPUT,
                "結果の置き場所(&O)",
                "",
                0,
                &rows::OUTPUT_OPTIONS,
            )
            .into(),
        ]),
        Item::Custom(list::draw_note),
        Item::Label(
            "選んだ文字列(選んでいないときは文書全体)を標準入力に渡し、出力を結果の置き場所に入れます。\nコマンドは 1 行で書きます。シェルは「シェル」のページのものです。\nコマンドのない番号は、登録されていません。",
        ),
        // The state of the list: the rows, kept in the file as `[OneLiner]`, and the row that is selected
        // (-1: none).
        Item::Hidden(vec![
            Control::choice(list::SELECTED, "", "", -1, &[]),
            Control::table(list::LIST, "", rows::spec()).named("登録ワンライナー"),
        ]),
    ]
}

/// What OK does to the windows: nothing besides the file. The windows read `[OneLiner]` from the settings
/// that OK made the new ones.
pub(in crate::common_settings) fn apply(_: &mut Env, _: &Values, _: Option<&Values>) {}

#[cfg(test)]
mod tests;
