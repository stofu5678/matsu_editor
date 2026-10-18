use gpui::{AnyWindowHandle, TestAppContext};
use ume_config::Profile;
use ume_config::settings::CustomMenus;
use ume_config::settings::custom_menu::{CustomMenu, CustomMenuItem};

use super::*;
use crate::common_settings::model::{Kind, controls};
use crate::common_settings::tests::{Ini, ok, setup, with_bom, with_dialog};
use crate::common_settings::{self, Page};
use crate::settings::UserSettings;

fn profile(text: &str) -> Profile {
    Profile::from_bytes(&with_bom(text))
}

fn item(function: u16, key: Option<char>) -> CustomMenuItem {
    CustomMenuItem { function, key }
}

fn edit(items: &[CustomMenuItem], sel: Option<usize>, func_sel: Option<usize>) -> Edit {
    Edit {
        items: items.to_vec(),
        sel,
        func_sel,
        func_count: 5,
    }
}

fn codes(edit: &Edit) -> Vec<u16> {
    edit.items.iter().map(|i| i.function).collect()
}

// ---- the rules of the buttons (`CPropCustmenu::DispatchEvent`) ----

#[test]
fn a_separator_goes_before_the_selected_row_or_at_the_top_and_is_selected() {
    let rows = [item(10, None), item(20, Some('A'))];
    let mut e = edit(&rows, Some(1), None);
    e.insert_separator();
    assert_eq!((codes(&e), e.sel), (vec![10, 0, 20], Some(1)));
    assert_eq!(e.items[2].key, Some('A'), "the key stays with its item");
    let mut e = edit(&rows, None, None);
    e.insert_separator();
    assert_eq!((codes(&e), e.sel), (vec![0, 10, 20], Some(0)));
    let mut e = edit(&[], None, None);
    e.insert_separator();
    assert_eq!((codes(&e), e.sel), (vec![0], Some(0)));
}

#[test]
fn delete_selects_the_row_that_takes_its_place_or_the_last_one() {
    let rows = [item(10, None), item(0, None), item(20, None)];
    let mut e = edit(&rows, Some(1), None);
    e.delete();
    assert_eq!((codes(&e), e.sel), (vec![10, 20], Some(1)));
    e.delete();
    assert_eq!((codes(&e), e.sel), (vec![10], Some(0)));
    e.delete();
    assert_eq!((codes(&e), e.sel), (vec![], None));
    e.delete();
    assert!(e.items.is_empty());
    // Nothing selected: nothing is removed.
    let mut e = edit(&rows, None, None);
    e.delete();
    assert_eq!(e.items.len(), 3);
}

#[test]
fn insert_puts_the_function_before_the_selected_row_and_selects_it() {
    let rows = [item(10, Some('X')), item(20, None)];
    let mut e = edit(&rows, Some(1), Some(2));
    e.insert_function(7);
    assert_eq!(codes(&e), [10, 7, 20]);
    assert_eq!(e.items[1].key, None, "a new item has no key");
    // The new item is the selected one (unlike the toolbar); the function list moves on.
    assert_eq!((e.sel, e.func_sel), (Some(1), Some(3)));
    // With no row selected it goes to the top.
    let mut e = edit(&rows, None, Some(0));
    e.insert_function(7);
    assert_eq!((codes(&e), e.sel), (vec![7, 10, 20], Some(0)));
    // The function list stops at its end.
    let mut e = edit(&rows, Some(0), Some(4));
    e.insert_function(7);
    assert_eq!(e.func_sel, None);
    // Without a chosen function nothing is put in.
    let mut e = edit(&rows, Some(0), None);
    e.insert_function(7);
    assert_eq!(e.items.len(), 2);
}

#[test]
fn add_puts_the_function_at_the_end_and_selects_it() {
    let rows = [item(10, None), item(20, None)];
    let mut e = edit(&rows, Some(0), Some(1));
    e.add_function(7);
    assert_eq!(codes(&e), [10, 20, 7]);
    assert_eq!((e.sel, e.func_sel), (Some(2), Some(2)));
    let mut e = edit(&rows, Some(0), None);
    e.add_function(7);
    assert_eq!(e.items.len(), 2);
}

#[test]
fn up_and_down_move_the_row_with_its_key_and_stop_at_the_ends() {
    let rows = [item(1, Some('a')), item(2, Some('b')), item(3, Some('c'))];
    let mut e = edit(&rows, Some(1), None);
    e.up();
    assert_eq!((codes(&e), e.sel), (vec![2, 1, 3], Some(0)));
    assert_eq!(e.items[0].key, Some('b'));
    e.up();
    assert_eq!(e.sel, Some(0));
    e.down();
    e.down();
    e.down();
    assert_eq!((codes(&e), e.sel), (vec![1, 3, 2], Some(2)));
    assert_eq!(e.items[2].key, Some('b'));
    let mut e = edit(&rows, None, None);
    e.up();
    e.down();
    assert_eq!(codes(&e), [1, 2, 3]);
}

#[test]
fn a_menu_stops_at_48_items() {
    let full = vec![item(5, None); 48];
    let mut e = edit(&full, Some(0), Some(0));
    assert!(e.is_full());
    e.insert_separator();
    e.insert_function(7);
    e.add_function(7);
    assert_eq!(e.items.len(), 48);
    e.delete();
    assert_eq!(e.items.len(), 47);
    assert!(!e.is_full());
}

#[test]
fn the_access_key_is_set_on_a_function_row_not_on_a_separator() {
    let rows = [item(10, None), item(0, None)];
    let mut e = edit(&rows, Some(0), None);
    e.set_key(Some('Q'));
    assert_eq!(e.items[0].key, Some('Q'));
    e.set_key(None);
    assert_eq!(e.items[0].key, None);
    let mut e = edit(&rows, Some(1), None);
    e.set_key(Some('Q'));
    assert_eq!(e.items[1].key, None, "a separator has no key");
}

// ---- the lists ----

#[test]
fn the_kinds_are_the_function_lookups_and_the_special_functions() {
    assert_eq!(PAGE_KINDS.len(), 19);
    assert_eq!(PAGE_KINDS[16].0, "カスタムメニュー");
    assert_eq!(PAGE_KINDS[18], ("特別機能", 18));
    assert_eq!(functions_of_kind(18), SPECIAL_CODES);
    assert_eq!(functions_of_kind(16).len(), 25);
    assert!(functions_of_kind(99).is_empty());
    // Their names are the labels of the table.
    let menus = CustomMenus::default();
    assert_eq!(function_name(29001, &menus), "ウィンドウリスト");
    assert_eq!(function_name(29004, &menus), "カスタムメニューリスト");
}

#[test]
fn a_custom_menu_function_has_the_name_of_its_menu() {
    let mut named = CustomMenus::default().menus().to_vec();
    named[3].name = "My menu".to_owned();
    let menus = CustomMenus::from_menus(named);
    assert_eq!(function_name(31503, &menus), "My menu");
    assert_eq!(function_name(31580, &menus), "右クリックメニュー");
    assert_eq!(function_name(31524, &menus), "タブメニュー");
    assert_eq!(function_name(31504, &menus), "メニュー4");
    assert_eq!(function_name(9999, &menus), "-- 不明 --");
}

#[test]
fn a_row_shows_the_name_the_key_in_parentheses_or_a_line() {
    let menus = CustomMenus::default();
    assert_eq!(row_text(&item(30102, Some('O')), &menus), "開く...(O)");
    assert_eq!(row_text(&item(30102, None), &menus), "開く...");
    assert_eq!(row_text(&item(0, None), &menus), SEPARATOR_TEXT);
}

// ---- the file ----

#[test]
fn the_menus_survive_the_list_of_numbers() {
    let mut all = CustomMenus::default().menus().to_vec();
    all[2] = CustomMenu {
        name: "日本語の名前😀".to_owned(),
        items: vec![item(30102, Some('O')), item(0, None)],
        popup: false,
    };
    let menus = CustomMenus::from_menus(all);
    assert_eq!(data::menus_of(&data::data_of(&menus)), menus);
    // A list that is cut short gives empty menus for the rest.
    let short = data::data_of(&menus)[..10].to_vec();
    assert_eq!(data::decode(&short).len(), 1);
    assert_eq!(data::menus_of(&short).menus().len(), 25);
    assert!(data::decode(&[]).is_empty());
}

#[test]
fn the_defaults_are_loaded_and_writing_them_again_changes_nothing() {
    let mut p = profile("[Common]\r\nbA=1\r\n");
    let defaults = data::load(&p);
    assert_eq!(data::menus_of(&defaults), CustomMenus::default());
    assert!(!data::store(&defaults, &mut p).unwrap());
    assert_eq!(p.get("CustMenu", "nCMIN[00]"), None);
}

#[test]
fn the_menus_are_written_to_cust_menu_and_the_rest_of_the_file_stays() {
    let mut p = profile(
        "[Other]\r\nx=1\r\n[CustMenu]\r\nszCMN[02]=Old\r\nnCMIN[02]=3\r\nnCMIF[02][00]=Undo\r\n\
         nCMIF[02][01]=30102\r\nnCMIF[02][02]=30103\r\nnCMIK[02][02]=S\r\n[Common]\r\nbA=1\r\n",
    );
    let mut menus = data::menus_of(&data::load(&p)).menus().to_vec();
    assert_eq!(menus[2].items.len(), 3);
    menus[2].name = "New".to_owned();
    menus[2].popup = false;
    menus[2].items = vec![item(30210, Some('U'))];
    assert!(data::store(&data::encode(&menus), &mut p).unwrap());
    assert_eq!(p.get("CustMenu", "szCMN[02]"), Some("New"));
    assert_eq!(p.get("CustMenu", "bCMPOP[02]"), Some("0"));
    assert_eq!(p.get("CustMenu", "nCMIN[02]"), Some("1"));
    assert_eq!(p.get("CustMenu", "nCMIF[02][00]"), Some("30210"));
    assert_eq!(p.get("CustMenu", "nCMIK[02][00]"), Some("U"));
    // The items that are gone have no keys.
    assert_eq!(p.get("CustMenu", "nCMIF[02][01]"), None);
    assert_eq!(p.get("CustMenu", "nCMIF[02][02]"), None);
    assert_eq!(p.get("CustMenu", "nCMIK[02][02]"), None);
    assert_eq!(p.get("Other", "x"), Some("1"));
    assert_eq!(p.get("Common", "bA"), Some("1"));
    // The right-click menu and the tab menu are written too.
    assert_eq!(p.get("CustMenu", "nCMIN[00]"), Some("30"));
    assert_eq!(p.get("CustMenu", "nCMIN[24]"), Some("27"));
    // Reading it again gives the menus.
    assert_eq!(data::menus_of(&data::load(&p)).menus(), &menus[..]);
    assert!(!data::store(&data::encode(&menus), &mut p).unwrap());
}

#[test]
fn the_page_keeps_the_menus_in_a_numbers_control_and_all_its_buttons_are_controls() {
    let items = items().expect("the page is built");
    let list = controls(&items);
    assert!(
        list.iter()
            .any(|c| matches!(c.kind, Kind::Numbers { .. }) && c.id == DATA)
    );
    for id in [
        "custmenu_delete",
        "custmenu_separator",
        "custmenu_insert",
        "custmenu_add",
        "custmenu_up",
        "custmenu_down",
        "custmenu_set_name",
        "custmenu_set_key",
        "custmenu_submenu",
        "custmenu_import",
        "custmenu_export",
    ] {
        assert!(list.iter().any(|c| c.id == id), "{id}");
    }
    // The state of the page is not in `[Common]`: no control with a key writes there.
    for control in &list {
        if !control.key.is_empty() {
            assert_eq!(control.section, SECTION, "{}", control.id);
            assert_ne!(control.bit, 0, "{} would be written", control.id);
        }
    }
}

// ---- the page in the dialog ----

fn press(cx: &mut TestAppContext, id: &'static str) {
    with_dialog(cx, |d, window, dcx| {
        let control = d
            .page_controls()
            .into_iter()
            .find(|c| c.id == id)
            .expect("the button");
        assert!(d.is_usable(&control), "{id} is usable");
        let Kind::Button { run } = control.kind else {
            panic!("{id} is not a button");
        };
        run(d, window, dcx);
    });
}

fn button_is_usable(cx: &mut TestAppContext, id: &'static str) -> bool {
    with_dialog(cx, |d, _, _| {
        let control = d.page_controls().into_iter().find(|c| c.id == id).unwrap();
        d.is_usable(&control)
    })
}

fn open_page(cx: &mut TestAppContext, text: &str) -> (Ini, AnyWindowHandle) {
    let (ini, handle) = setup(cx, text);
    cx.update(|app| common_settings::open(app, Some(Page::CustomMenu), Some(handle)));
    cx.run_until_parked();
    (ini, handle)
}

fn items_now(cx: &mut TestAppContext) -> Vec<CustomMenuItem> {
    with_dialog(cx, |d, _, _| edit_of(d.values()).items)
}

fn choose_menu(cx: &mut TestAppContext, index: usize) {
    with_dialog(cx, |d, window, dcx| d.select_menu(index, window, dcx));
}

fn select_row(cx: &mut TestAppContext, row: usize) {
    with_dialog(cx, |d, window, dcx| d.select_item(row, window, dcx));
}

fn choose_function(cx: &mut TestAppContext, kind: i32, row: i32) {
    with_dialog(cx, |d, _, dcx| {
        d.set_int(KIND, kind, dcx);
        d.set_int(FUNC_SEL, row, dcx);
    });
}

fn custom_menus(cx: &mut TestAppContext) -> CustomMenus {
    cx.update(|app| app.global::<UserSettings>().custom_menus())
}

#[gpui::test]
fn the_page_shows_menu_0_with_its_name_and_the_buttons_follow_the_rules(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (_ini, _) = open_page(cx, "[CustMenu]\r\nszCMN[00]=Mine\r\n");
    // Menu 0 is chosen; its name is in the box and its default items are in the list.
    let (name, key) = with_dialog(cx, |d, _, dcx| {
        (
            d.input_text(NAME, dcx).unwrap(),
            d.input_text(KEY, dcx).unwrap(),
        )
    });
    assert_eq!((name.as_str(), key.as_str()), ("Mine", ""));
    assert_eq!(items_now(cx).len(), 30);
    // Nothing is selected, nothing is chosen in the function list.
    for id in [
        "custmenu_delete",
        "custmenu_up",
        "custmenu_down",
        "custmenu_insert",
        "custmenu_add",
        "custmenu_set_key",
    ] {
        assert!(!button_is_usable(cx, id), "{id}");
    }
    assert!(button_is_usable(cx, "custmenu_separator"));
    // Choosing a row enables 削除 and ↓; the first row cannot go up.
    select_row(cx, 0);
    assert!(button_is_usable(cx, "custmenu_delete"));
    assert!(button_is_usable(cx, "custmenu_down"));
    assert!(!button_is_usable(cx, "custmenu_up"));
    assert!(button_is_usable(cx, "custmenu_set_key"));
    select_row(cx, 2);
    assert!(
        !button_is_usable(cx, "custmenu_set_key"),
        "the third row of the right-click menu is a separator"
    );
    // A function chosen: 「→」 needs a row too.
    choose_function(cx, 0, 1);
    assert!(button_is_usable(cx, "custmenu_insert"));
    assert!(button_is_usable(cx, "custmenu_add"));
    // The last row cannot go down.
    select_row(cx, 29);
    assert!(!button_is_usable(cx, "custmenu_down"));
}

#[gpui::test]
fn the_buttons_change_the_items_of_the_chosen_menu_only(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (_ini, _) = open_page(cx, "");
    choose_menu(cx, 5);
    assert!(items_now(cx).is_empty(), "menu 5 has no items by default");
    // 「>>」 adds the chosen function and selects it; the function list moves on.
    choose_function(cx, 0, 2);
    press(cx, "custmenu_add");
    let first = items_now(cx);
    assert_eq!(first.len(), 1);
    let code = functions_of_kind(0)[2];
    assert_eq!(first[0], item(code, None));
    let (sel, func) = with_dialog(cx, |d, _, _| {
        (d.values().int(RES_SEL), d.values().int(FUNC_SEL))
    });
    assert_eq!((sel, func), (0, 3));
    // Again: the next function is added after it.
    press(cx, "custmenu_add");
    assert_eq!(items_now(cx)[1].function, functions_of_kind(0)[3]);
    // 「---」 goes before the selected row and is selected; 「→」 goes before it.
    press(cx, "custmenu_separator");
    let list = items_now(cx);
    assert!(list[1].is_separator() && list.len() == 3);
    press(cx, "custmenu_insert");
    let list = items_now(cx);
    assert_eq!(list.len(), 4);
    assert_eq!(list[1].function, functions_of_kind(0)[4]);
    // 「↑」 and 「↓」 move the selected row.
    press(cx, "custmenu_up");
    assert_eq!(items_now(cx)[0].function, functions_of_kind(0)[4]);
    press(cx, "custmenu_down");
    press(cx, "custmenu_delete");
    assert_eq!(items_now(cx).len(), 3);
    // The other menus are as they were.
    choose_menu(cx, 1);
    assert_eq!(items_now(cx).len(), 7);
    choose_menu(cx, 5);
    assert_eq!(items_now(cx).len(), 3);
}

#[gpui::test]
fn a_menu_holds_48_items_at_most(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (_ini, _) = open_page(cx, "[CustMenu]\r\nnCMIN[07]=47\r\n");
    choose_menu(cx, 7);
    assert_eq!(items_now(cx).len(), 47);
    choose_function(cx, 0, 0);
    press(cx, "custmenu_add");
    assert_eq!(items_now(cx).len(), 48);
    // Full: 「---」, 「→」 and 「>>」 cannot be used.
    select_row(cx, 0);
    choose_function(cx, 0, 0);
    for id in ["custmenu_separator", "custmenu_insert", "custmenu_add"] {
        assert!(!button_is_usable(cx, id), "{id}");
    }
    press(cx, "custmenu_delete");
    assert!(button_is_usable(cx, "custmenu_separator"));
}

#[gpui::test]
fn the_name_the_submenu_flag_and_the_access_key_are_set_per_item_and_menu(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (_ini, _) = open_page(cx, "");
    choose_menu(cx, 3);
    // 設定 takes the box as the name, cut to 32 units; an empty name is the default one.
    let long = "あ".repeat(40);
    with_dialog(cx, |d, window, dcx| d.set_input(NAME, &long, window, dcx));
    press(cx, "custmenu_set_name");
    let name = custom_name(cx, 3);
    assert_eq!(name.chars().count(), 32);
    // The menus the window lists show the name (menus are chosen by their names).
    with_dialog(cx, |d, window, dcx| d.set_input(NAME, "Work", window, dcx));
    press(cx, "custmenu_set_name");
    assert_eq!(custom_name(cx, 3), "Work");
    assert_eq!(
        function_name(31503, &with_dialog(cx, |d, _, _| menus_of(d.values()))),
        "Work"
    );
    // Another menu has its own name; coming back shows this one in the box.
    choose_menu(cx, 4);
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text(NAME, dcx)).unwrap(),
        ""
    );
    choose_menu(cx, 3);
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text(NAME, dcx)).unwrap(),
        "Work"
    );
    with_dialog(cx, |d, window, dcx| d.set_input(NAME, "", window, dcx));
    press(cx, "custmenu_set_name");
    assert_eq!(custom_name(cx, 3), "");

    // 「サブメニューとして表示」 is a flag of the menu.
    let popup = |cx: &mut TestAppContext, index| {
        with_dialog(cx, |d, _, _| menus_of(d.values()).get(index).unwrap().popup)
    };
    assert!(popup(cx, 3));
    press(cx, "custmenu_submenu");
    assert!(!popup(cx, 3));
    assert!(popup(cx, 4), "the other menus keep theirs");
    with_dialog(cx, |d, _, dcx| d.set_submenu(true, dcx));
    assert!(popup(cx, 3));

    // The access key: the first row of the right-click menu is 元に戻す(U).
    choose_menu(cx, 0);
    select_row(cx, 0);
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text(KEY, dcx)).unwrap(),
        "U"
    );
    with_dialog(cx, |d, window, dcx| d.set_input(KEY, "Z", window, dcx));
    press(cx, "custmenu_set_key");
    assert_eq!(items_now(cx)[0].key, Some('Z'));
    // A longer text gives its first character; an empty box clears the key.
    with_dialog(cx, |d, window, dcx| d.set_input(KEY, "yx", window, dcx));
    press(cx, "custmenu_set_key");
    assert_eq!(items_now(cx)[0].key, Some('y'));
    // A character that the file cannot hold changes nothing.
    with_dialog(cx, |d, window, dcx| d.set_input(KEY, "あ", window, dcx));
    press(cx, "custmenu_set_key");
    assert_eq!(items_now(cx)[0].key, Some('y'));
    with_dialog(cx, |d, window, dcx| d.set_input(KEY, "", window, dcx));
    press(cx, "custmenu_set_key");
    assert_eq!(items_now(cx)[0].key, None);
    // The list shows the key in parentheses.
    let text = with_dialog(cx, |d, _, _| {
        let menus = menus_of(d.values());
        row_text(&edit_of(d.values()).items[1], &menus)
    });
    assert_eq!(text, "やり直し(R)");
}

fn custom_name(cx: &mut TestAppContext, index: usize) -> String {
    with_dialog(cx, |d, _, _| {
        menus_of(d.values()).get(index).unwrap().name.clone()
    })
}

#[gpui::test]
fn ok_writes_the_menus_and_every_window_takes_them_at_once(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (ini, first) = setup(cx, "[Other]\r\nx=1\r\n[Common]\r\nbA=1\r\n");
    let second = crate::common_settings::tests::open_window(cx);
    cx.update(|app| common_settings::open(app, Some(Page::CustomMenu), Some(first)));
    cx.run_until_parked();
    // Before: menu 5 is empty, so it shows nothing.
    assert!(!popup_opens(cx, second, 5));
    choose_menu(cx, 5);
    with_dialog(cx, |d, window, dcx| d.set_input(NAME, "Five", window, dcx));
    press(cx, "custmenu_set_name");
    choose_function(cx, 0, 0);
    press(cx, "custmenu_add");
    press(cx, "custmenu_add");
    // The tab menu loses its first item.
    choose_menu(cx, 24);
    select_row(cx, 0);
    press(cx, "custmenu_delete");
    ok(cx);

    let p = ini.profile();
    assert_eq!(p.get("CustMenu", "szCMN[05]"), Some("Five"));
    assert_eq!(p.get("CustMenu", "nCMIN[05]"), Some("2"));
    assert_eq!(p.get("CustMenu", "nCMIN[24]"), Some("26"));
    assert_eq!(
        p.get("CustMenu", "nCMIF[05][00]"),
        Some(functions_of_kind(0)[0].to_string().as_str())
    );
    assert_eq!(p.get("Other", "x"), Some("1"));
    assert_eq!(p.get("Common", "bA"), Some("1"));
    // The box of the name was not written as menu 0's name.
    assert_eq!(p.get("CustMenu", "szCMN[00]"), Some(""));

    let menus = custom_menus(cx);
    assert_eq!(menus.name(5).as_deref(), Some("Five"));
    assert_eq!(menus.get(5).unwrap().items.len(), 2);
    assert_eq!(menus.get(24).unwrap().items.len(), 26);
    assert_eq!(menus.get(24).unwrap().items[0].function, 30104);
    // The popup of `F_CUSTMENU_5` opens in both windows now, and the main menu lists the menu.
    for handle in [first, second] {
        assert!(popup_opens(cx, handle, 5));
    }
    let list = crate::custom_menu::model::CustomMenuList(&menus);
    let names: Vec<String> = crate::menu::Lists::items(&list, 29004)
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert!(names.iter().any(|n| n == "Five"), "{names:?}");
}

fn popup_opens(cx: &mut TestAppContext, handle: AnyWindowHandle, index: usize) -> bool {
    let view = cx
        .update(|app| crate::file_ops::windows::workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.editor().clone());
    let opened = handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.show_custom_menu(index, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    opened
}

#[gpui::test]
fn the_unchanged_page_writes_no_section_and_the_typed_boxes_are_not_the_names(
    cx: &mut TestAppContext,
) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (ini, _) = open_page(cx, "[Other]\r\nx=1\r\n");
    // Look at another menu and type in the boxes without pressing 設定.
    choose_menu(cx, 2);
    with_dialog(cx, |d, window, dcx| d.set_input(NAME, "Typed", window, dcx));
    ok(cx);
    // OK writes the other pages' keys, but no [CustMenu] for menus that are Sakura's defaults.
    assert!(
        !ini.profile().section_names().any(|n| n == "CustMenu"),
        "no [CustMenu] is written for the defaults"
    );
}

#[gpui::test]
fn export_writes_a_mnu_file_and_import_reads_it_back(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (ini, _) = open_page(cx, "");
    let file = ini.dir.0.join("menus.mnu");
    // Menu 5 gets a named menu with a macro function and a separator.
    choose_menu(cx, 5);
    with_dialog(cx, |d, window, dcx| d.set_input(NAME, "Five", window, dcx));
    press(cx, "custmenu_set_name");
    with_dialog(cx, |d, window, dcx| {
        let mut menus = menus_of(d.values()).menus().to_vec();
        menus[5].items = vec![item(30210, Some('U')), item(0, None), item(30102, None)];
        menus[5].popup = false;
        d.put_menus(&CustomMenus::from_menus(menus), dcx);
        assert!(d.export_menu_file(&file, window, dcx));
    });
    let written = Profile::load(&file).unwrap();
    assert_eq!(
        written.get("Info", "MENU_VERSION"),
        Some("SakuraEditorMenu_Ver2")
    );
    assert_eq!(written.get("Info", "MAX_CUSTOM_MENU"), Some("25"));
    assert_eq!(written.get("CustMenu", "szCMN[05]"), Some("Five"));
    assert_eq!(written.get("CustMenu", "bCMPOP[05]"), Some("0"));
    // A function with a macro name is written by it, one without by its code; a separator is 0.
    assert_eq!(written.get("CustMenu", "nCMIF[05][00]"), Some("Undo"));
    assert_eq!(written.get("CustMenu", "nCMIF[05][01]"), Some("0"));
    assert_eq!(written.get("CustMenu", "nCMIF[05][02]"), Some("30102"));
    assert_eq!(written.get("CustMenu", "nCMIK[05][00]"), Some("U"));

    // Another menu changes, then the file is read over: menu 5 comes back, the rest is the file's.
    choose_menu(cx, 5);
    with_dialog(cx, |d, _, dcx| {
        let mut menus = menus_of(d.values()).menus().to_vec();
        menus[5] = CustomMenu {
            name: String::new(),
            items: vec![],
            popup: true,
        };
        d.put_menus(&CustomMenus::from_menus(menus), dcx);
    });
    assert!(items_now(cx).is_empty());
    with_dialog(cx, |d, window, dcx| {
        assert!(d.import_menu_file(&file, window, dcx))
    });
    let (items, name, popup) = with_dialog(cx, |d, _, dcx| {
        let menus = menus_of(d.values());
        (
            edit_of(d.values()).items,
            d.input_text(NAME, dcx).unwrap(),
            menus.get(5).unwrap().popup,
        )
    });
    assert_eq!(
        items,
        vec![item(30210, Some('U')), item(0, None), item(30102, None)]
    );
    assert_eq!(name, "Five", "the page shows the menu again");
    assert!(!popup);
    // Nothing is in the settings file until OK.
    assert_eq!(ini.profile().get("CustMenu", "szCMN[05]"), None);
    ok(cx);
    assert_eq!(ini.profile().get("CustMenu", "szCMN[05]"), Some("Five"));
}

#[gpui::test]
fn a_file_that_is_not_a_menu_file_is_refused_and_changes_nothing(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (ini, _) = open_page(cx, "");
    let wrong = ini.dir.0.join("wrong.mnu");
    std::fs::write(
        &wrong,
        with_bom("[Info]\r\nMENU_VERSION=Other\r\n[CustMenu]\r\nnCMIN[00]=0\r\n"),
    )
    .unwrap();
    let missing = ini.dir.0.join("missing.mnu");
    let before = with_dialog(cx, |d, _, _| menus_of(d.values()));
    with_dialog(cx, |d, window, dcx| {
        assert!(!d.import_menu_file(&wrong, window, dcx));
        assert!(!d.import_menu_file(&missing, window, dcx));
    });
    assert_eq!(with_dialog(cx, |d, _, _| menus_of(d.values())), before);
}

#[gpui::test]
fn changing_the_kind_clears_the_chosen_function(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (_ini, _) = open_page(cx, "");
    choose_function(cx, 0, 2);
    assert!(button_is_usable(cx, "custmenu_add"));
    with_dialog(cx, |d, _, dcx| d.set_int(KIND, 1, dcx));
    assert!(!button_is_usable(cx, "custmenu_add"));
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(FUNC_SEL)), -1);
}
