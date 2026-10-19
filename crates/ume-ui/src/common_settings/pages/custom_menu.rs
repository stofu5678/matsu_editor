//! 「カスタムメニュー」: `IDD_PROP_CUSTMENU` (`sakura_core/sakura_rc.rc`), `CPropCustmenu`
//! (`sakura_core/prop/CPropComCustmenu.cpp`).
//!
//! The page edits the 25 menus (the right-click menu, menus 1 to 23 and the tab menu 24; see
//! `ume_config::settings::custom_menu`): the menu is chosen in a drop-down list (選択), its name is
//! set with 設定, 「サブメニューとして表示」 is its flag, and the buttons between the lists (削除,
//! ---, →, >>, ↑, ↓) change its items ([`edit`]). The access key of an item is typed in its own box
//! and set with 設定 (Sakura asks for it in a small dialog after a double click on the item).
//! インポート and エクスポート read and write a `.mnu` file ([`transfer`]).
//!
//! OK writes `[CustMenu]` (`CShareData_IO::IO_CustMenu`) under the lock of the settings file and
//! the menus change at once in every window: the popups, the tab menu, `F_CUSTMENU_n` and the main
//! menu's list all read the settings of the application, which OK replaces.
//!
//! The 25 menus are one [`Kind::Numbers`] control ([`data`]); the page draws the lists itself
//! ([`Item::Custom`]). The name box shows the name of the menu that is chosen: for menu 0, which is
//! chosen when the window opens, it is loaded from `szCMN[00]` (the control is `packed`, so that OK
//! never writes the box itself).
//!
//! Differences from Sakura: no item is selected when the window opens or a menu is chosen
//! (Sakura selects the first item when the window opens); the access key is set with a box and a
//! button; `.mnu` files are chosen with the dialog of the OS without a file type filter.
//!
//! Not built: plugin commands (the kind プラグイン has none), the registered macros (the kind
//! 外部マクロ is empty until macros can be registered).

mod data;
mod draw;
mod edit;
mod transfer;

use gpui::{Context, Window};
use ume_config::settings::CustomMenus;
use ume_config::settings::custom_menu::MAX_CUSTOM_MENU;

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::func_lookup::{KINDS, codes_of_kind};
use super::super::model::{Control, Item, Value, Values};
use crate::custom_menu::model::menu_index;
use edit::Edit;

const SECTION: &str = "CustMenu";
const DATA: &str = "custmenu_data";
const MENU: &str = "custmenu_menu";
const KIND: &str = "custmenu_kind";
const FUNC_SEL: &str = "custmenu_func_sel";
const RES_SEL: &str = "custmenu_res_sel";
const NAME: &str = "custmenu_name";
const KEY: &str = "custmenu_key";

/// `STR_SPECIAL_FUNC`: the kind that `CPropCustmenu::SetData` adds after the kinds of `CFuncLookup`.
const SPECIAL_KIND: i32 = 18;
/// The kinds the page offers: those of `CFuncLookup` and 特別機能.
static PAGE_KINDS: [(&str, i32); 19] = {
    let mut kinds = [("", 0); 19];
    let mut i = 0;
    while i < KINDS.len() {
        kinds[i] = KINDS[i];
        i += 1;
    }
    kinds[SPECIAL_KIND as usize] = ("特別機能", SPECIAL_KIND);
    kinds
};
/// The special function list of `sakura_core/func/Funccode.cpp` (`F_WINDOW_LIST` to `F_PLUGIN_LIST`).
const SPECIAL_CODES: [u16; 6] = [29001, 29002, 29003, 29004, 29005, 29006];
/// `STR_PROPCOMCUSTMENU_SEP`.
const SEPARATOR_TEXT: &str = "─────────────";
/// `F_DISABLE`'s string, which `CFuncLookup::Funccode2Name` gives a code with no name.
const UNKNOWN_NAME: &str = "-- 不明 --";

/// The functions of kind `kind` in the order of the list (`CFuncLookup::Pos2FuncCode`, and
/// the special function list for 特別機能).
fn functions_of_kind(kind: i32) -> Vec<u16> {
    if kind == SPECIAL_KIND {
        SPECIAL_CODES.to_vec()
    } else {
        codes_of_kind(kind)
    }
}

/// `CFuncLookup::Funccode2Name`: the name of function `code`; a custom menu has its own name.
fn function_name(code: u16, menus: &CustomMenus) -> String {
    if let Some(name) = menu_index(code).and_then(|index| menus.name(index)) {
        return name;
    }
    Some(crate::bars::function_name(code))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| UNKNOWN_NAME.to_owned())
}

/// The text of a row of the menu's list (`SetDataMenuList`): the name, then the access key in
/// parentheses; a separator is a line.
fn row_text(item: &ume_config::settings::CustomMenuItem, menus: &CustomMenus) -> String {
    let name = if item.is_separator() {
        SEPARATOR_TEXT.to_owned()
    } else {
        function_name(item.function, menus)
    };
    match item.key {
        Some(key) => format!("{name}({key})"),
        None => name,
    }
}

// ---- the state of the page, from the values ----

/// The menus the page holds now.
fn menus_of(values: &Values) -> CustomMenus {
    data::menus_of(values.numbers(DATA))
}

/// The menu that is chosen (`IDC_COMBO_MENU`).
fn current_menu(values: &Values) -> usize {
    usize::try_from(values.int(MENU))
        .unwrap_or(0)
        .min(MAX_CUSTOM_MENU - 1)
}

/// The state of the lists, from the values (`SetDlgItemsEnableState`).
fn edit_of(values: &Values) -> Edit {
    let menus = menus_of(values);
    let items = menus
        .get(current_menu(values))
        .map(|m| m.items.clone())
        .unwrap_or_default();
    let sel = usize::try_from(values.int(RES_SEL))
        .ok()
        .filter(|&i| i < items.len());
    let functions = functions_of_kind(values.int(KIND));
    let func_sel = usize::try_from(values.int(FUNC_SEL))
        .ok()
        .filter(|&i| i < functions.len());
    Edit {
        items,
        sel,
        func_sel,
        func_count: functions.len(),
    }
}

fn has_selection(values: &Values) -> bool {
    edit_of(values).sel.is_some()
}

fn can_move_up(values: &Values) -> bool {
    edit_of(values).sel.is_some_and(|i| i > 0)
}

fn can_move_down(values: &Values) -> bool {
    let edit = edit_of(values);
    edit.sel.is_some_and(|i| i + 1 < edit.items.len())
}

fn can_insert_separator(values: &Values) -> bool {
    !edit_of(values).is_full()
}

fn can_insert(values: &Values) -> bool {
    let edit = edit_of(values);
    edit.sel.is_some() && edit.func_sel.is_some() && !edit.is_full()
}

fn can_add(values: &Values) -> bool {
    let edit = edit_of(values);
    edit.func_sel.is_some() && !edit.is_full()
}

fn can_set_key(values: &Values) -> bool {
    let edit = edit_of(values);
    edit.sel
        .and_then(|i| edit.items.get(i))
        .is_some_and(|item| !item.is_separator())
}

fn clear_function_choice(values: &mut Values, _: &'static str) {
    values.set(FUNC_SEL, Value::Int(-1));
}

pub(in crate::common_settings) fn items() -> Option<Vec<Item>> {
    Some(vec![
        Item::Custom(draw::choosers),
        Item::Row(vec![
            // `packed` makes OK skip the box: the menus are written from `custmenu_data`. It still
            // loads the name of menu 0, which is the one shown first.
            Control::text(NAME, "メニュー名(&N)", "szCMN[00]", "")
                .in_section(SECTION)
                .packed(1, 0)
                .into(),
            Control::button("custmenu_set_name", "設定(&R)", |d, window, cx| {
                d.set_menu_name(window, cx)
            })
            .into(),
        ]),
        Item::Custom(draw::lists),
        Item::Row(vec![
            Control::text(KEY, "アクセスキー(&Y)", "", "").into(),
            Control::button("custmenu_set_key", "設定(&K)", |d, window, cx| {
                d.set_item_key(window, cx)
            })
            .when(can_set_key)
            .into(),
        ]),
        Item::Custom(draw::submenu),
        Item::Row(vec![
            Control::button(
                "custmenu_import",
                "インポート(&I)...",
                |d, window, cx| d.import_menus(window, cx),
            )
            .into(),
            Control::button(
                "custmenu_export",
                "エクスポート(&X)...",
                |d, window, cx| d.export_menus(window, cx),
            )
            .into(),
        ]),
        Item::Hidden(vec![
            Control::choice(MENU, "", "", 0, &[]),
            // Alt+C: the next menu (a drop-down list goes to its next choice).
            Control::button("custmenu_next_menu", "選択(&C)", |d, window, cx| {
                let next = (current_menu(d.values()) + 1) % MAX_CUSTOM_MENU;
                d.select_menu(next, window, cx)
            }),
            Control::choice(KIND, "種別(&K)", "", 0, &PAGE_KINDS).on_change(clear_function_choice),
            Control::choice(FUNC_SEL, "", "", -1, &[]),
            Control::choice(RES_SEL, "", "", -1, &[]),
            Control::numbers(DATA, data::load, data::store),
            Control::button("custmenu_delete", "削除(&D)", |d, window, cx| {
                d.edit_menu(window, cx, Edit::delete)
            })
            .when(has_selection),
            Control::button("custmenu_separator", "---(&S)", |d, window, cx| {
                d.edit_menu(window, cx, Edit::insert_separator)
            })
            .when(can_insert_separator),
            Control::button("custmenu_insert", "→(&A)", |d, window, cx| {
                d.edit_menu_function(window, cx, Edit::insert_function)
            })
            .when(can_insert),
            Control::button("custmenu_add", ">>(&B)", |d, window, cx| {
                d.edit_menu_function(window, cx, Edit::add_function)
            })
            .when(can_add),
            Control::button("custmenu_up", "↑(&U)", |d, window, cx| {
                d.edit_menu(window, cx, Edit::up)
            })
            .when(can_move_up),
            Control::button("custmenu_down", "↓(&O)", |d, window, cx| {
                d.edit_menu(window, cx, Edit::down)
            })
            .when(can_move_down),
            Control::button(
                "custmenu_submenu",
                "サブメニューとして表示(&S)",
                |d, _, cx| {
                    let on = !menus_of(d.values())
                        .get(current_menu(d.values()))
                        .is_some_and(|m| m.popup);
                    d.set_submenu(on, cx);
                },
            ),
        ]),
    ])
}

impl CommonSettingsDialog {
    /// Puts `menus` in the values.
    fn put_menus(&mut self, menus: &CustomMenus, cx: &mut Context<Self>) {
        self.set_numbers(DATA, data::data_of(menus), cx);
    }

    /// The edit boxes show what the chosen item and menu have.
    fn show_item_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let edit = edit_of(self.values());
        let key = edit
            .sel
            .and_then(|i| edit.items.get(i))
            .and_then(|item| item.key)
            .map(String::from)
            .unwrap_or_default();
        self.set_input(KEY, &key, window, cx);
    }

    /// Applies one of the edits of the menu's list to the values.
    fn edit_menu(&mut self, window: &mut Window, cx: &mut Context<Self>, change: fn(&mut Edit)) {
        let mut edit = edit_of(self.values());
        change(&mut edit);
        self.put_menu_edit(edit, window, cx);
    }

    /// The same for the edits that take the selected function.
    fn edit_menu_function(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        change: fn(&mut Edit, u16),
    ) {
        let mut edit = edit_of(self.values());
        let functions = functions_of_kind(self.values().int(KIND));
        let Some(&code) = edit.func_sel.and_then(|i| functions.get(i)) else {
            return;
        };
        change(&mut edit, code);
        self.put_menu_edit(edit, window, cx);
    }

    fn put_menu_edit(&mut self, edit: Edit, window: &mut Window, cx: &mut Context<Self>) {
        let to_int = |sel: Option<usize>| sel.map_or(-1, |i| i as i32);
        let index = current_menu(self.values());
        let mut menus = menus_of(self.values()).menus().to_vec();
        menus[index].items = edit.items;
        self.put_menus(&CustomMenus::from_menus(menus), cx);
        self.set_int(RES_SEL, to_int(edit.sel), cx);
        self.set_int(FUNC_SEL, to_int(edit.func_sel), cx);
        self.show_item_key(window, cx);
    }

    /// A click on a row of the menu's list.
    pub(super) fn select_item(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.set_int(RES_SEL, row as i32, cx);
        self.show_item_key(window, cx);
    }

    /// `CBN_SELCHANGE` of `IDC_COMBO_MENU` (`SetDataMenuList`): the list shows the items of menu
    /// `index`, the box its name.
    pub(super) fn select_menu(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_int(MENU, index.min(MAX_CUSTOM_MENU - 1) as i32, cx);
        self.set_int(RES_SEL, -1, cx);
        let name = menus_of(self.values())
            .get(current_menu(self.values()))
            .map(|m| m.name.clone())
            .unwrap_or_default();
        self.set_input(NAME, &name, window, cx);
        self.set_input(KEY, "", window, cx);
    }

    /// 「設定」 beside the name (`IDC_BUTTON_MENUNAME`): the box becomes the name of the menu, cut
    /// to `MAX_CUSTOM_MENU_NAME_LEN`; an empty name is the default name.
    fn set_menu_name(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.input_text(NAME, cx) else {
            return;
        };
        let index = current_menu(self.values());
        let mut menus = menus_of(self.values()).menus().to_vec();
        menus[index].name = text;
        self.put_menus(&CustomMenus::from_menus(menus), cx);
    }

    /// 「設定」 beside the access key (`LBN_DBLCLK` of `IDC_LIST_RES`): the first character of the
    /// box is the key of the selected item; an empty box clears it. The file holds ASCII keys only
    /// (`TryParse<KEYCODE>`), so another character changes nothing; a space is no key.
    fn set_item_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.input_text(KEY, cx) else {
            return;
        };
        let key = match text.chars().next() {
            None | Some(' ') => None,
            Some(c) if c.is_ascii_graphic() => Some(c),
            Some(_) => return,
        };
        let mut edit = edit_of(self.values());
        edit.set_key(key);
        self.put_menu_edit(edit, window, cx);
    }

    /// 「サブメニューとして表示」 (`IDC_CHECK_SUBMENU`).
    pub(super) fn set_submenu(&mut self, on: bool, cx: &mut Context<Self>) {
        let index = current_menu(self.values());
        let mut menus = menus_of(self.values()).menus().to_vec();
        menus[index].popup = on;
        self.put_menus(&CustomMenus::from_menus(menus), cx);
    }
}

/// What OK does to the windows: nothing of its own. OK writes `[CustMenu]` and replaces the
/// application's settings, which the popups, the tab menu and the main menu read when they open
/// (`commit` makes the menus again).
pub(in crate::common_settings) fn apply(_env: &mut Env, _after: &Values, _before: Option<&Values>) {
}

#[cfg(test)]
mod tests;
