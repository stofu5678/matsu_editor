//! Tests of the custom menus: the model of a menu, and the popup in GPUI's headless test window.
//!
//! The test platform has no fonts, so sizes are the fallbacks (as in `view::tests`).

use gpui::{AppContext as _, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};
use ume_commands::keymap::{DisplayStyle, KeyMap, KeyMapOptions, Platform};
use ume_config::Profile;
use ume_config::settings::CustomMenus;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use super::model::{
    self, Context, CustomMenuList, F_CUSTMENU_BASE, F_MENU_RBUTTON, entries, menu_code, menu_index,
};
use crate::document::SharedDocument;
use crate::menu::{Entry, Item, Lists, MenuAction, NoLists};
use crate::settings::UserSettings;
use crate::view::{EditorView, RunFunction};

// ---- the model ----

fn keymap() -> KeyMap {
    KeyMap::new(KeyMapOptions::for_platform(Platform::Other))
}

/// The menus of a settings file (the macro names `Undo` and `SelectAll` are known).
fn menus(text: &str) -> CustomMenus {
    UserSettings::from_profile(Profile::from_bytes(text.as_bytes()), None).custom_menus()
}

fn build(menus: &CustomMenus, index: usize, available: &dyn crate::menu::Available) -> Vec<Entry> {
    build_with(menus, index, available, &NoLists)
}

fn build_with(
    menus: &CustomMenus,
    index: usize,
    available: &dyn crate::menu::Available,
    lists: &dyn Lists,
) -> Vec<Entry> {
    entries(
        index,
        &Context {
            menus,
            macros: &Default::default(),
            keymap: &keymap(),
            available,
            lists,
            parentheses: true,
        },
    )
}

fn all(_: u16) -> bool {
    true
}

fn items(entries: &[Entry]) -> Vec<&Item> {
    entries
        .iter()
        .filter_map(|e| match e {
            Entry::Item(i) => Some(i),
            _ => None,
        })
        .collect()
}

fn submenu<'a>(entries: &'a [Entry], label: &str) -> &'a [Entry] {
    entries
        .iter()
        .find_map(|e| match e {
            Entry::Submenu { label: l, entries } if l == label => Some(entries.as_slice()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no submenu {label:?} in {entries:?}"))
}

#[test]
fn function_codes_and_menu_numbers_agree() {
    assert_eq!(menu_index(F_MENU_RBUTTON), Some(0));
    assert_eq!(menu_index(F_CUSTMENU_BASE + 1), Some(1));
    assert_eq!(menu_index(F_CUSTMENU_BASE + 24), Some(24));
    // F_CUSTMENU_BASE itself and the code after F_CUSTMENU_24 are not menus
    assert_eq!(menu_index(F_CUSTMENU_BASE), None);
    assert_eq!(menu_index(F_CUSTMENU_BASE + 25), None);
    assert_eq!(menu_index(30210), None);
    for index in 0..25 {
        assert_eq!(menu_index(menu_code(index)), Some(index));
    }
}

// The default right-click menu is `InitPopupMenu`'s: Undo first, with its access key and shortcut.
#[test]
fn the_default_right_click_menu_is_sakuras() {
    let rows = build(&CustomMenus::default(), 0, &all);
    assert_eq!(rows.len(), 30);
    let Entry::Item(undo) = &rows[0] else {
        panic!("{:?}", rows[0]);
    };
    assert_eq!(undo.label, "元に戻す(U)");
    assert_eq!(undo.action, MenuAction::Function(30210));
    assert_eq!(
        undo.shortcut.unwrap().display(DisplayStyle::Sakura),
        "Ctrl+Z"
    );
    assert!(undo.enabled);
    // Three rows of the cut, copy and paste group are separated from Undo and Redo.
    assert_eq!(rows[2], Entry::Separator);
    assert_eq!(rows.iter().filter(|e| **e == Entry::Separator).count(), 7);
}

#[test]
fn a_function_without_a_command_is_grayed_out() {
    let rows = build(&CustomMenus::default(), 0, &|code| code == 30210);
    let shown = items(&rows);
    assert!(shown[0].enabled);
    assert!(shown[1..].iter().all(|i| !i.enabled));
}

// `CKeyBind::MakeMenuLabel` reads `bKeyParentheses` for every menu, the custom menus too: without it a
// name that has its key as a letter shows no `(K)`.
#[test]
fn the_parentheses_setting_applies_to_custom_menus() {
    let all_menus = menus(
        "[CustMenu]\nszCMN[03]=Mine\nnCMIN[03]=1\nnCMIF[03][00]=SelectAll\nnCMIK[03][00]=A\n\
         nCMIN[02]=1\nnCMIF[02][00]=31503\nnCMIK[02][00]=M\n",
    );
    let build = |parentheses| {
        entries(
            2,
            &Context {
                menus: &all_menus,
                macros: &Default::default(),
                keymap: &keymap(),
                available: &all,
                lists: &NoLists,
                parentheses,
            },
        )
    };
    let with = build(true);
    assert_eq!(items(submenu(&with, "Mine(M)"))[0].label, "すべて選択(A)");
    let without = build(false);
    assert_eq!(items(submenu(&without, "Mine"))[0].label, "すべて選択(A)");
}

#[test]
fn a_menu_with_the_popup_flag_is_a_submenu_of_another() {
    let all_menus = menus(
        "[CustMenu]\nszCMN[03]=Mine\nnCMIN[03]=1\nnCMIF[03][00]=SelectAll\nnCMIK[03][00]=A\n\
         nCMIN[02]=2\nnCMIF[02][00]=31503\nnCMIK[02][00]=M\nnCMIF[02][01]=31504\nnCMIK[02][01]=\n",
    );
    let rows = build(&all_menus, 2, &all);
    // The row of custom menu 3 is its name with the row's access key, and holds its items.
    let inside = submenu(&rows, "Mine(M)");
    assert_eq!(items(inside)[0].label, "すべて選択(A)");
    // Custom menu 4 has no items: Sakura makes a submenu that opens nothing; here a grayed item.
    let Entry::Item(empty) = &rows[1] else {
        panic!("{:?}", rows[1]);
    };
    assert_eq!(empty.label, "メニュー4");
    assert!(!empty.enabled);
}

#[test]
fn a_menu_without_the_popup_flag_is_an_item_that_opens_it() {
    let all_menus = menus(
        "[CustMenu]\nbCMPOP[03]=0\nszCMN[03]=Mine\nnCMIN[03]=1\nnCMIF[03][00]=30401\n\
         nCMIN[02]=1\nnCMIF[02][00]=31503\nnCMIK[02][00]=M\n",
    );
    let rows = build(&all_menus, 2, &all);
    let Entry::Item(item) = &rows[0] else {
        panic!("{:?}", rows[0]);
    };
    assert_eq!(item.label, "Mine(M)");
    assert_eq!(item.action, MenuAction::Function(31503));
    assert!(item.enabled);
    // ... unless that menu has no items (`InitMenu_Function` grays it).
    let all_menus = menus("[CustMenu]\nbCMPOP[03]=0\nnCMIN[02]=1\nnCMIF[02][00]=31503\n");
    let rows = build(&all_menus, 2, &all);
    assert!(!items(&rows)[0].enabled);
}

// A menu that holds itself, or a menu above it, is not opened inside again (`pParentMenus`).
#[test]
fn a_menu_is_not_shown_inside_itself() {
    let all_menus = menus(
        "[CustMenu]\nnCMIN[02]=2\nnCMIF[02][00]=31503\nnCMIF[02][01]=31502\n\
         nCMIN[03]=2\nnCMIF[03][00]=31502\nnCMIF[03][01]=30401\n",
    );
    let rows = build(&all_menus, 2, &all);
    // menu 3 inside menu 2; the row of menu 2 in menu 3 is a plain item; the row of menu 2 in menu 2 too
    let inside = submenu(&rows, "メニュー3");
    let Entry::Item(back) = &inside[0] else {
        panic!("{:?}", inside[0]);
    };
    assert_eq!(back.action, MenuAction::Function(31502));
    let Entry::Item(itself) = &rows[1] else {
        panic!("{:?}", rows[1]);
    };
    assert_eq!(itself.action, MenuAction::Function(31502));
    // Opened as the menu itself, the same holds.
    let rows = build(&all_menus, 3, &all);
    let inside = submenu(&rows, "メニュー2");
    assert_eq!(items(inside)[1].action, MenuAction::Function(31502));
}

#[test]
fn the_right_click_menu_can_be_a_submenu_too() {
    let all_menus = menus("[CustMenu]\nnCMIN[02]=1\nnCMIF[02][00]=RMenu\nnCMIK[02][00]=R\n");
    let rows = build(&all_menus, 2, &all);
    let inside = submenu(&rows, "右クリックメニュー(R)");
    assert_eq!(inside.len(), 30);
}

#[test]
fn an_unnamed_menu_has_the_default_name_and_a_named_one_its_name() {
    let all_menus = menus(
        "[CustMenu]\nszCMN[07]=Named\nnCMIN[02]=3\nnCMIF[02][00]=31507\nnCMIF[02][01]=31508\n\
         nCMIF[02][02]=31509\nnCMIN[07]=1\nnCMIF[07][00]=30401\nnCMIN[08]=1\nnCMIF[08][00]=30401\n\
         nCMIN[09]=1\nnCMIF[09][00]=30401\nbCMPOP[09]=0\n",
    );
    let rows = build(&all_menus, 2, &all);
    submenu(&rows, "Named");
    submenu(&rows, "メニュー8");
    // the third is a plain item (no popup flag), named in the same way
    assert_eq!(items(&rows)[0].label, "メニュー9");
}

#[test]
fn a_menu_out_of_range_or_empty_has_no_rows() {
    assert!(build(&CustomMenus::default(), 25, &all).is_empty());
    assert!(build(&CustomMenus::default(), 5, &all).is_empty());
}

#[test]
fn the_lists_of_sakura_are_filled_in_or_say_none() {
    // F_FILE_USED_RECENTLY (29002) and F_CUSTMENU_LIST (29004) are lists.
    let all_menus = menus("[CustMenu]\nnCMIN[02]=2\nnCMIF[02][00]=29002\nnCMIF[02][01]=29004\n");
    let rows = build(&all_menus, 2, &all);
    let none = items(&rows)[0];
    assert_eq!((none.label.as_str(), none.enabled), ("(なし)", false));
    let custom = CustomMenuList(&all_menus);
    let rows = build_with(&all_menus, 2, &all, &custom);
    let names: Vec<_> = items(&rows)
        .iter()
        .skip(1)
        .map(|i| (i.label.clone(), i.action.clone()))
        .collect();
    // the menus that have items, the right-click menu first
    assert_eq!(
        names,
        [
            ("右クリックメニュー".to_owned(), MenuAction::Function(31580)),
            ("メニュー1".to_owned(), MenuAction::Function(31501)),
            ("メニュー2".to_owned(), MenuAction::Function(31502)),
            ("タブメニュー".to_owned(), MenuAction::Function(31524)),
        ]
    );
}

// The main menu's row for the custom menus lists the menus that have items (`InitMenu_Special`).
#[test]
fn the_main_menu_lists_the_custom_menus() {
    fn labels(entries: &[Entry], out: &mut Vec<String>) {
        for e in entries {
            match e {
                Entry::Item(i) => out.push(i.label.clone()),
                Entry::Submenu { entries, .. } => labels(entries, out),
                Entry::Separator => {}
            }
        }
    }
    let defaults = CustomMenus::default();
    let lists = CustomMenuList(&defaults);
    let mut found = Vec::new();
    for top in crate::menu::main_menu_with(&keymap(), &all, &lists) {
        labels(&top.entries, &mut found);
    }
    for name in ["右クリックメニュー", "メニュー1", "タブメニュー"] {
        assert!(
            found.iter().any(|l| l == name),
            "{name} is not in the menus"
        );
    }
    assert!(!found.iter().any(|l| l == "メニュー2"));
}

// The names the default menus use are functions of Sakura's table, with the labels Sakura shows.
#[test]
fn every_default_function_is_in_sakuras_table() {
    use ume_commands::sakura::FUNCTIONS;
    let defaults = CustomMenus::default();
    for index in [0, 1, 24] {
        for item in &defaults.get(index).unwrap().items {
            if !item.is_separator() {
                let f = FUNCTIONS.iter().find(|f| f.code == item.function);
                assert!(
                    f.is_some_and(|f| f.label.is_some()),
                    "{} has no label",
                    item.function
                );
            }
        }
    }
}

// ---- the popup ----

struct Fixture {
    document: Entity<SharedDocument>,
    view: Entity<EditorView>,
}

fn setup<'a>(
    cx: &'a mut TestAppContext,
    ini: &str,
    text: &str,
) -> (Fixture, &'a mut VisualTestContext) {
    let user = UserSettings::from_profile(Profile::from_bytes(ini.as_bytes()), None);
    cx.update(|cx| {
        gpui_component::init(cx);
        cx.set_global(user);
    });
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let shared = document.clone();
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    // The menu is drawn in the next frame.
    vcx.run_until_parked();
    (Fixture { document, view }, vcx)
}

impl Fixture {
    fn is_open(&self, cx: &mut VisualTestContext) -> bool {
        self.view.read_with(cx, |v, _| v.custom_menu.is_open())
    }

    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view.read_with(cx, |v, _| v.cursor.caret())
    }

    fn selection(&self, cx: &mut VisualTestContext) -> Option<(Pos, Pos)> {
        self.view.read_with(cx, |v, _| v.cursor.selection())
    }

    fn lines(&self, cx: &mut VisualTestContext) -> Vec<String> {
        self.document.read_with(cx, |shared, _| {
            let doc = shared.doc();
            (0..doc.line_count())
                .map(|i| doc.line(i).unwrap().unwrap().text)
                .collect()
        })
    }
}

/// The window position of the cell at `row` and `col`.
fn point_of(
    f: &Fixture,
    cx: &mut VisualTestContext,
    row: usize,
    col: usize,
) -> gpui::Point<gpui::Pixels> {
    f.view.read_with(cx, |v, _| {
        let paint = v.paint.expect("the window was painted");
        let m = v.metrics;
        gpui::point(
            gpui::px(paint.origin_x + (col as f32 + 0.1) * m.cell_width),
            gpui::px(paint.origin_y + (row as f32 + 0.5) * m.line_height - v.scroll_y),
        )
    })
}

fn right_click(cx: &mut VisualTestContext, p: gpui::Point<gpui::Pixels>) {
    cx.simulate_mouse_down(p, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(p, MouseButton::Right, Modifiers::default());
}

/// A menu of one item that selects everything, as custom menu 2.
const SELECT_ALL_MENU: &str = "[CustMenu]\nnCMIN[02]=1\nnCMIF[02][00]=30401\nnCMIK[02][00]=A\n";

#[gpui::test]
fn the_functions_of_the_menus_are_ui_commands(cx: &mut TestAppContext) {
    let _ = cx;
    let registry = crate::ui_command::UiRegistry::global();
    assert_eq!(registry.get_by_code(31580).unwrap().id, "menu_rbutton");
    for n in 1..=24u16 {
        let reg = registry.get_by_code(31500 + n).unwrap();
        assert_eq!(reg.id, format!("custmenu_{n}"));
        assert_eq!(reg.sakura, format!("F_CUSTMENU_{n}"));
    }
}

#[gpui::test]
fn the_right_click_menu_opens_at_the_pointer(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", "abc\ndef\nghi");
    assert!(!f.is_open(cx));
    let p = point_of(&f, cx, 1, 1);
    right_click(cx, p);
    assert!(f.is_open(cx));
    // the caret went to the click (OnRBUTTONDOWN), and the popup starts 4 pixels above the pointer
    assert_eq!(f.caret(cx), Pos::new(1, 1));
    let at = f
        .view
        .read_with(cx, |v, _| v.custom_menu.open_at().unwrap());
    assert_eq!(at, gpui::point(p.x, p.y - gpui::px(4.)));
}

#[gpui::test]
fn a_right_click_in_the_selection_keeps_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", "abcdef\nghi");
    cx.simulate_keystrokes("shift-right shift-right shift-right");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
    // inside: the cell of 'b' (the click's half of the cell does not matter, unlike a left click)
    let p = f.view.read_with(cx, |v, _| {
        let paint = v.paint.unwrap();
        gpui::point(
            gpui::px(paint.origin_x + 1.9 * v.metrics.cell_width),
            gpui::px(paint.origin_y + 0.5 * v.metrics.line_height),
        )
    });
    right_click(cx, p);
    assert!(f.is_open(cx));
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
}

#[gpui::test]
fn a_right_click_outside_the_selection_moves_the_caret(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", "abcdef\nghi");
    cx.simulate_keystrokes("shift-right shift-right shift-right");
    // the end of the range is not inside it
    let p = point_of(&f, cx, 0, 3);
    right_click(cx, p);
    assert!(f.is_open(cx));
    assert_eq!(f.selection(cx), None);
    assert_eq!(f.caret(cx), Pos::new(0, 3));
}

#[gpui::test]
fn the_function_of_the_right_click_is_the_key_maps(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", "abc");
    // Alt+right click has no function in Sakura's table: nothing opens, but the caret still moves
    let p = point_of(&f, cx, 0, 2);
    cx.simulate_mouse_down(p, MouseButton::Right, Modifiers::alt());
    cx.simulate_mouse_up(p, MouseButton::Right, Modifiers::alt());
    assert!(!f.is_open(cx));
    assert_eq!(f.caret(cx), Pos::new(0, 2));
}

// Sakura shows the menu at `::GetCursorPos` for a key too (`CEditView::CreatePopUpMenuSub`).
#[gpui::test]
fn the_menu_key_opens_the_right_click_menu_at_the_pointer(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", "abc\ndef");
    let p = point_of(&f, cx, 1, 2);
    cx.simulate_mouse_move(p, None, Modifiers::default());
    cx.simulate_keystrokes("menu");
    assert!(f.is_open(cx));
    let at = f
        .view
        .read_with(cx, |v, _| v.custom_menu.open_at().unwrap());
    assert_eq!(at, gpui::point(p.x, p.y - gpui::px(4.)));
    // the caret did not move
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn choosing_an_item_runs_its_function(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, SELECT_ALL_MENU, "abc\ndef");
    cx.dispatch_action(RunFunction { code: 31502 });
    assert!(f.is_open(cx));
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(1, 3))));
    assert!(!f.is_open(cx), "the popup closes");
}

#[gpui::test]
fn escape_closes_the_menu_and_changes_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, SELECT_ALL_MENU, "abc");
    cx.dispatch_action(RunFunction { code: 31502 });
    assert!(f.is_open(cx));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!f.is_open(cx));
    assert_eq!(f.selection(cx), None);
    // the keys go to the text again
    cx.simulate_input("x");
    assert_eq!(f.lines(cx), ["xabc"]);
}

// A key macro records the function that was chosen, not the menu that showed it (`F_MENU_RBUTTON` and
// `F_CUSTMENU_n` are not key macro functions in `CanFuncIsKeyMacro`).
// A menu of Sakura is modal, and the popup is sized for the window it opened in: a resize closes it.
#[gpui::test]
fn resizing_the_window_closes_the_menu(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, SELECT_ALL_MENU, "abc");
    cx.dispatch_action(RunFunction { code: 31502 });
    assert!(f.is_open(cx));
    cx.simulate_resize(gpui::size(gpui::px(500.), gpui::px(300.)));
    cx.run_until_parked();
    assert!(!f.is_open(cx));
}

#[gpui::test]
fn a_key_macro_records_the_chosen_function(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, SELECT_ALL_MENU, "abc");
    f.view.update(cx, |v, cx| v.toggle_key_macro_recording(cx));
    cx.dispatch_action(RunFunction { code: 31502 });
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    let dir = tempfile::tempdir().unwrap();
    f.view.update(cx, |v, cx| v.toggle_key_macro_recording(cx));
    let path = dir.path().join("rec.mac");
    f.view
        .read_with(cx, |v, app| v.save_key_macro_to(&path, app))
        .unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("SelectAll"), "{saved}");
    assert!(!saved.contains("CustMenu"), "{saved}");
    assert!(!saved.contains("RMenu"), "{saved}");
}

#[gpui::test]
fn a_custom_menu_with_no_items_shows_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", "abc");
    // custom menus 2 to 23 have no items by default
    cx.dispatch_action(RunFunction { code: 31502 });
    assert!(!f.is_open(cx));
}

#[gpui::test]
fn the_right_click_menu_with_no_items_shows_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "[CustMenu]\nnCMIN[00]=0\n", "abc");
    let p = point_of(&f, cx, 0, 1);
    right_click(cx, p);
    assert!(!f.is_open(cx));
    // the caret still went to the click
    assert_eq!(f.caret(cx), Pos::new(0, 1));
}

#[gpui::test]
fn a_right_click_on_the_scroll_bar_does_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", "abc");
    let p = f.view.read_with(cx, |v, _| {
        let paint = v.paint.unwrap();
        gpui::point(
            gpui::px(paint.scrollbar_left + 2.),
            gpui::px(paint.origin_y + 5.),
        )
    });
    right_click(cx, p);
    assert!(!f.is_open(cx));
}

// `model::F_CUSTMENU_BASE` is Sakura's value (31500): the table of functions has the codes.
#[test]
fn the_codes_are_in_the_function_table() {
    use ume_commands::sakura::FUNCTIONS;
    let name = |code| FUNCTIONS.iter().find(|f| f.code == code).unwrap().name;
    assert_eq!(name(F_MENU_RBUTTON), "F_MENU_RBUTTON");
    assert_eq!(name(F_CUSTMENU_BASE + 1), "F_CUSTMENU_1");
    assert_eq!(name(model::F_CUSTMENU_LAST), "F_CUSTMENU_24");
    assert_eq!(name(model::F_CUSTMENU_LIST), "F_CUSTMENU_LIST");
}
