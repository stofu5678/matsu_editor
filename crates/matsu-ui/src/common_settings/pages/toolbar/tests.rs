use gpui::{Modifiers, TestAppContext, VisualTestContext};
use matsu_config::Profile;

use super::*;
use crate::common_settings::model::{Kind, controls};
use crate::common_settings::tests::{ok, setup, with_dialog};
use crate::common_settings::{self, Page};

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn edit(list: &[i32], sel: Option<usize>, func_sel: Option<usize>) -> Edit {
    Edit {
        list: list.to_vec(),
        sel,
        func_sel,
        func_count: 5,
    }
}

#[test]
fn only_functions_with_a_button_are_listed_with_their_button_numbers() {
    let files = functions_of_kind(0);
    // F_FILENEW is the first button of the table; F_FILESAVE the third.
    assert_eq!(files[0], (1, 30101));
    assert!(files.contains(&(3, 30103)));
    assert!(files.iter().all(|&(n, c)| n >= 1 && c > 10));
    // The right-click menu and the 24 custom menus; macros and plugins are empty here.
    let menus = functions_of_kind(16);
    assert!(menus.iter().any(|&(_, c)| c == 31580));
    assert!(menus.iter().any(|&(_, c)| c == 31524));
    assert!(functions_of_kind(17).is_empty());
    assert!(functions_of_kind(99).is_empty() && functions_of_kind(-1).is_empty());
}

#[test]
fn the_kind_external_macro_lists_the_registered_macros_with_their_names_and_icons() {
    let macros = functions_of_kind(15);
    // The 50 macros have a button each (`MAX_CUSTMACRO_ICO`), in the order of the table.
    assert_eq!(macros.len(), 50);
    assert!(
        macros
            .iter()
            .enumerate()
            .all(|(i, &(_, c))| c == 31600 + i as u16)
    );
    let mut table = RegisteredMacros::default();
    table.macros[1].file = "b.mac".into();
    table.macros[2].name = "名前".into();
    table.macros[2].file = "c.mac".into();
    let (number, _) = macros[2];
    let (name, icon) = row_of(number, &table);
    assert_eq!(name, "名前");
    assert_eq!(icon, bars::icon_of_function(31602));
    assert!(icon.is_some());
    assert_eq!(row_of(macros[1].0, &table).0, "b.mac");
    assert_eq!(row_of(macros[0].0, &table).0, "マクロ 0 (未登録)");
}

#[test]
fn a_row_shows_the_name_and_icon_of_the_button_or_the_text_of_the_other_kinds() {
    let (name, icon) = row_of(3, &RegisteredMacros::default());
    assert_eq!((name.as_str(), icon), ("上書き保存", Some(2)));
    assert_eq!(
        row_of(0, &RegisteredMacros::default()),
        (ITEM_SEPARATOR.to_owned(), None)
    );
    assert_eq!(
        row_of(WRAP, &RegisteredMacros::default()),
        (ITEM_WRAP.to_owned(), None)
    );
    // A free place of the table, a number past it and -1 (a plugin command) are unknown.
    for number in [481, 9999, -1] {
        assert_eq!(
            row_of(number, &RegisteredMacros::default()),
            (ITEM_UNKNOWN.to_owned(), None),
            "{number}"
        );
    }
}

#[test]
fn the_buttons_between_the_lists_edit_as_sakuras_handlers_do() {
    // 「---」 goes before the selected row, or at the top; it is selected.
    let mut e = edit(&[3, 33], Some(1), None);
    e.insert_separator();
    assert_eq!((e.list.as_slice(), e.sel), (&[3, 0, 33][..], Some(1)));
    let mut e = edit(&[3, 33], None, None);
    e.insert_wrap();
    assert_eq!((e.list.as_slice(), e.sel), (&[WRAP, 3, 33][..], Some(0)));
    // 「削除」 selects the row that took its place, or the last one.
    let mut e = edit(&[3, 0, 33], Some(1), None);
    e.delete();
    assert_eq!((e.list.as_slice(), e.sel), (&[3, 33][..], Some(1)));
    e.delete();
    assert_eq!((e.list.as_slice(), e.sel), (&[3][..], Some(0)));
    e.delete();
    assert_eq!((e.list.as_slice(), e.sel), (&[][..], None));
    e.delete();
    assert!(e.list.is_empty());
    // 「→」: before the selected row; the old row stays selected and the function list moves on.
    let mut e = edit(&[3, 33], Some(1), Some(2));
    e.insert_function(7);
    assert_eq!(e.list, [3, 7, 33]);
    assert_eq!((e.sel, e.func_sel), (Some(2), Some(3)));
    // 「>>」: at the end, selected; the function list stops at its end.
    let mut e = edit(&[3], Some(0), Some(4));
    e.add_function(7);
    assert_eq!(e.list, [3, 7]);
    assert_eq!((e.sel, e.func_sel), (Some(1), None));
    // Without a chosen function nothing is added.
    let mut e = edit(&[3], Some(0), None);
    e.add_function(7);
    e.insert_function(7);
    assert_eq!(e.list, [3]);
    // 「↑」 and 「↓」 stop at the ends.
    let mut e = edit(&[1, 2, 3], Some(1), None);
    e.up();
    assert_eq!((e.list.as_slice(), e.sel), (&[2, 1, 3][..], Some(0)));
    e.up();
    assert_eq!(e.sel, Some(0));
    e.down();
    e.down();
    e.down();
    assert_eq!((e.list.as_slice(), e.sel), (&[1, 3, 2][..], Some(2)));
}

#[test]
fn the_list_stops_at_the_most_a_toolbar_can_hold() {
    let mut e = edit(&vec![3; MAX_BUTTONS], Some(0), Some(0));
    e.insert_separator();
    e.add_function(7);
    assert_eq!(e.list.len(), MAX_BUTTONS);
}

#[test]
fn the_list_is_written_to_toolbar_and_the_rest_of_the_file_stays() {
    let mut p = profile(
        "[Other]\r\nx=1\r\n[Toolbar]\r\nbToolBarIsFlat=1\r\nnToolBarButtonNum=3\r\nnTBB[000]=3\r\n\
         nTBB[001]=0\r\nnTBB[002]=33\r\n[Common]\r\nbA=1\r\n",
    );
    assert!(store(&[33, 0, 34, 1], &mut p).unwrap());
    assert_eq!(p.get("Toolbar", "nToolBarButtonNum"), Some("4"));
    assert_eq!(p.get("Toolbar", "nTBB[000]"), Some("33"));
    assert_eq!(p.get("Toolbar", "nTBB[003]"), Some("1"));
    assert_eq!(p.get("Toolbar", "bToolBarIsFlat"), Some("1"));
    assert_eq!(p.get("Other", "x"), Some("1"));
    assert_eq!(p.get("Common", "bA"), Some("1"));
    assert_eq!(load(&p), [33, 0, 34, 1]);
    // The same list again writes nothing; a shorter one removes the entries left over.
    assert!(!store(&[33, 0, 34, 1], &mut p).unwrap());
    assert!(store(&[33], &mut p).unwrap());
    assert_eq!(p.get("Toolbar", "nToolBarButtonNum"), Some("1"));
    assert_eq!(p.get("Toolbar", "nTBB[001]"), None);
    assert_eq!(p.get("Toolbar", "nTBB[003]"), None);
    // An empty toolbar is a list of 0 buttons (not the default).
    assert!(store(&[], &mut p).unwrap());
    assert_eq!(load(&p), Vec::<i32>::new());
}

#[test]
fn a_file_without_a_list_that_gets_the_default_list_is_left_alone() {
    let mut p = profile("[Common]\r\nbA=1\r\n");
    let default = load(&p);
    assert!(!store(&default, &mut p).unwrap());
    assert_eq!(p.get("Toolbar", "nToolBarButtonNum"), None);
}

#[test]
fn the_page_has_a_numbers_control_and_its_hidden_state_is_not_drawn_or_written() {
    let list = controls(&items());
    assert!(
        list.iter()
            .any(|c| matches!(c.kind, Kind::Numbers { .. }) && c.id == "toolbar_buttons")
    );
    // Every button the page draws is a control, so its Alt key works.
    for id in [
        "toolbar_delete",
        "toolbar_separator",
        "toolbar_insert",
        "toolbar_add",
        "toolbar_up",
        "toolbar_down",
        "toolbar_wrap",
    ] {
        assert!(list.iter().any(|c| c.id == id), "{id}");
    }
}

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

#[gpui::test]
fn ok_writes_the_buttons_and_every_window_shows_them(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (ini, first) = setup(
        cx,
        "[Other]\r\nx=1\r\n[Toolbar]\r\nnToolBarButtonNum=2\r\nnTBB[000]=3\r\nnTBB[001]=33\r\n",
    );
    cx.update(|app| common_settings::open(app, Some(Page::Toolbar), Some(first)));
    cx.run_until_parked();
    // Nothing is chosen in the function list yet: 「→」 and 「>>」 cannot be used; the first row
    // of the toolbar is selected: 「削除」 can, 「↑」 cannot.
    assert!(!button_is_usable(cx, "toolbar_add"));
    assert!(!button_is_usable(cx, "toolbar_insert"));
    assert!(button_is_usable(cx, "toolbar_delete"));
    assert!(!button_is_usable(cx, "toolbar_up"));
    assert!(button_is_usable(cx, "toolbar_down"));
    // Choose 新規作成 (button 1) in 「ファイル操作系」 and add it, then a separator in front.
    with_dialog(cx, |d, _, dcx| {
        d.set_int("toolbar_kind", 0, dcx);
        d.set_int("toolbar_func_sel", 0, dcx);
    });
    press(cx, "toolbar_add");
    press(cx, "toolbar_separator");
    let list = with_dialog(cx, |d, _, _| d.values().numbers("toolbar_buttons").to_vec());
    // The new button went to the end and was selected; the separator goes before it.
    assert_eq!(list, [3, 33, 0, 1]);
    // Changing the kind clears the choice in the function list.
    with_dialog(cx, |d, _, dcx| d.set_int("toolbar_kind", 1, dcx));
    assert!(!button_is_usable(cx, "toolbar_add"));
    // A second window: opened before OK, it follows at OK.
    let second = crate::common_settings::tests::open_window(cx);
    ok(cx);
    let profile = ini.profile();
    assert_eq!(profile.get("Toolbar", "nToolBarButtonNum"), Some("4"));
    assert_eq!(profile.get("Toolbar", "nTBB[002]"), Some("0"));
    assert_eq!(profile.get("Toolbar", "nTBB[003]"), Some("1"));
    assert_eq!(profile.get("Other", "x"), Some("1"));
    assert_eq!(
        cx.update(|app| app.global::<ToolbarButtons>().clone()),
        ToolbarButtons(vec![3, 33, 0, 1])
    );
    for handle in [first, second] {
        let cx = &mut VisualTestContext::from_window(handle, cx);
        let toolbar = cx.debug_bounds("tool-bar").expect("the toolbar");
        let new_file = cx.debug_bounds("tool-button-1").expect("the new button");
        assert!(toolbar.contains(&new_file.center()));
        // Sakura's default list has many more buttons; this window has the four.
        assert!(cx.debug_bounds("tool-button-225").is_none());
        let _ = Modifiers::none();
    }
}

#[gpui::test]
fn cancel_changes_nothing(cx: &mut TestAppContext) {
    let (ini, first) = setup(cx, "[Toolbar]\r\nnToolBarButtonNum=1\r\nnTBB[000]=3\r\n");
    let bytes = ini.bytes();
    cx.update(|app| common_settings::open(app, Some(Page::Toolbar), Some(first)));
    cx.run_until_parked();
    press(cx, "toolbar_wrap");
    press(cx, "toolbar_delete");
    crate::common_settings::tests::cancel(cx);
    assert_eq!(ini.bytes(), bytes);
    assert!(cx.update(|app| app.try_global::<ToolbarButtons>().is_none()));
}

fn plugin_command(code: u16, plug: usize) -> matsu_commands::plugin_commands::CommandInfo {
    matsu_commands::plugin_commands::CommandInfo {
        code,
        slot: 0,
        plug,
        plugin_id: "toolbar.test".to_owned(),
        plugin_name: "Toolbar test".to_owned(),
        label: "Do it".to_owned(),
        handler: "do.js".to_owned(),
        icon: String::new(),
        added: false,
    }
}

#[test]
fn a_plugin_button_is_listed_by_its_kind_and_written_by_the_name_of_its_command() {
    use matsu_commands::plugin_commands as pc;
    pc::set_slots(vec!["toolbar.test".to_owned()]);
    pc::add_command(plugin_command(20001, 1));
    assert!(functions_of_kind(17).contains(&(20001, 20001)));
    let (name, icon) = row_of(20001, &RegisteredMacros::default());
    assert_eq!(name, "Do it");
    assert_eq!(icon, crate::bars::toolbar::plugin_icon());
    assert!(icon.is_some());

    let mut p = profile("[Toolbar]\r\nnToolBarButtonNum=1\r\nnTBB[000]=3\r\n");
    assert!(store(&[20001], &mut p).unwrap());
    assert_eq!(p.get("Toolbar", "nTBB[000]"), Some("toolbar.test/01"));
    assert_eq!(load(&p), vec![20001]);
    // The plugin is gone: the name stays unresolved and the button is an undefined one.
    pc::set_commands(Vec::new());
    assert_eq!(load(&p), vec![-1]);
    pc::set_slots(Vec::new());
}
