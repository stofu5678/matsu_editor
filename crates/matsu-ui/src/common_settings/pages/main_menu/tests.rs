use gpui::TestAppContext;
use matsu_commands::keymap::KeyMap;
use matsu_config::Profile;
use matsu_config::settings::main_menu::{F_OPTION, F_SEPARATOR};
use matsu_config::settings::{MainMenu, MenuRow, RowKind};

use super::tree::{NEW_NODE_NAME, is_special};
use super::*;
use crate::common_settings::model::{Kind, controls};
use crate::common_settings::tests::{Ini, cancel, ok, setup, with_bom, with_dialog};
use crate::common_settings::{self, Page};
use crate::settings::UserSettings;

/// The names of the functions with no custom menus and no registered macros.
fn table_names(code: u16) -> String {
    crate::func_name::funccode_to_name(code, &Default::default(), &Default::default())
}

fn profile(text: &str) -> Profile {
    Profile::from_bytes(&with_bom(text))
}

/// A tree from `(level, function)`: function 0 is a node with its own name (`n<index>`), 1 a
/// separator, a function of the special lists a list, and any other number a function.
fn tree(spec: &[(usize, u16)]) -> Tree {
    let rows = spec
        .iter()
        .enumerate()
        .map(|(i, &(level, function))| {
            let mut row = MenuRow::leaf(level, function);
            match function {
                0 => {
                    row.kind = RowKind::Node;
                    row.name = format!("n{i}");
                }
                1 => row.kind = RowKind::Separator,
                f if is_special(f) => row.kind = RowKind::Special,
                _ => {}
            }
            PageRow {
                row,
                duplicate: false,
                collapsed: false,
            }
        })
        .collect();
    Tree {
        rows,
        sel: None,
        func_sel: None,
        func_count: 5,
    }
}

fn shape(tree: &Tree) -> Vec<(usize, u16)> {
    tree.rows
        .iter()
        .map(|r| (r.row.level, r.row.function))
        .collect()
}

fn selected(spec: &[(usize, u16)], sel: usize) -> Tree {
    Tree {
        sel: Some(sel),
        ..tree(spec)
    }
}

/// Two menus: `n0` with two functions, and `n3` with one.
const TWO_MENUS: [(usize, u16); 5] = [(0, 0), (1, 100), (1, 101), (0, 0), (1, 102)];

// ---- the rules of the buttons (`CPropMainMenu::DispatchEvent`) ----

#[test]
fn a_new_node_goes_to_the_end_of_the_top_level_into_a_node_or_after_a_function() {
    let mut t = tree(&TWO_MENUS);
    t.insert_node();
    assert_eq!(t.sel, Some(5));
    assert_eq!(shape(&t)[5], (0, 0));
    assert_eq!(t.rows[5].row.name, NEW_NODE_NAME);
    // Into the selected node, as its first row (the node opens).
    let mut t = selected(&TWO_MENUS, 3);
    t.rows[3].collapsed = true;
    t.insert_node();
    assert_eq!((t.sel, shape(&t)[4]), (Some(4), (1, 0)));
    assert!(!t.rows[3].collapsed);
    // After a function, in its menu.
    let mut t = selected(&TWO_MENUS, 1);
    t.insert_node();
    assert_eq!((t.sel, shape(&t)[2]), (Some(2), (1, 0)));
}

#[test]
fn a_separator_and_the_function_above_go_before_the_selected_row() {
    let mut t = selected(&TWO_MENUS, 2);
    t.insert_separator();
    assert_eq!(t.sel, Some(2));
    assert_eq!(shape(&t)[2], (1, F_SEPARATOR));
    assert_eq!(t.rows[2].row.kind, RowKind::Separator);
    // Nothing selected: the end of the top level.
    let mut t = tree(&TWO_MENUS);
    t.insert_separator();
    assert_eq!(shape(&t)[5], (0, F_SEPARATOR));
    // A row of the top level has no parent: the new row goes to the very top (`TVI_FIRST`).
    let mut t = selected(&TWO_MENUS, 3);
    t.func_sel = Some(1);
    t.insert_function(Place::Above, 200);
    assert_eq!((t.sel, shape(&t)[0]), (Some(0), (0, 200)));
}

#[test]
fn a_function_below_goes_after_the_selected_function_or_into_the_selected_node() {
    let mut t = selected(&TWO_MENUS, 1);
    t.func_sel = Some(0);
    t.insert_function(Place::Below, 200);
    assert_eq!((t.sel, shape(&t)[2]), (Some(2), (1, 200)));
    // The function list moves on.
    assert_eq!(t.func_sel, Some(1));
    let mut t = selected(&TWO_MENUS, 0);
    t.func_sel = Some(0);
    t.insert_function(Place::Below, 200);
    assert_eq!((t.sel, shape(&t)[1]), (Some(1), (1, 200)));
}

#[test]
fn add_goes_to_the_end_of_the_selected_node_or_of_the_menu_of_the_selected_function() {
    let mut t = selected(&TWO_MENUS, 0);
    t.func_sel = Some(4);
    t.insert_function(Place::Add, 200);
    assert_eq!((t.sel, shape(&t)[3]), (Some(3), (1, 200)));
    // The last function of the list: nothing is chosen after it.
    assert_eq!(t.func_sel, None);
    let mut t = selected(&TWO_MENUS, 1);
    t.func_sel = Some(0);
    t.insert_function(Place::Add, 200);
    assert_eq!((t.sel, shape(&t)[3]), (Some(3), (1, 200)));
    // A function on the top level: at the end of the top level.
    let mut t = selected(&[(0, 100)], 0);
    t.func_sel = Some(0);
    t.insert_function(Place::Add, 200);
    assert_eq!(shape(&t), [(0, 100), (0, 200)]);
    // Without a function chosen nothing is added.
    let mut t = selected(&TWO_MENUS, 0);
    t.insert_function(Place::Add, 200);
    assert_eq!(t.rows.len(), 5);
}

#[test]
fn a_list_is_put_in_as_a_special_row() {
    let mut t = tree(&[(0, 0)]);
    t.func_sel = Some(0);
    t.insert_function(Place::Add, 29002);
    assert_eq!(t.rows[1].row.kind, RowKind::Special);
}

#[test]
fn delete_takes_the_rows_under_the_row_and_selects_the_next_one_the_one_before_or_the_parent() {
    // The next row of the level takes its place.
    let mut t = selected(&TWO_MENUS, 1);
    t.delete();
    assert_eq!((shape(&t).len(), t.sel), (4, Some(1)));
    // The last row: the one before it.
    let mut t = selected(&TWO_MENUS, 2);
    t.delete();
    assert_eq!(t.sel, Some(1));
    // The only row of a menu: the menu.
    let mut t = selected(&TWO_MENUS, 4);
    t.delete();
    assert_eq!((shape(&t).len(), t.sel), (4, Some(3)));
    // A node goes with its rows; the next menu is selected.
    let mut t = selected(&TWO_MENUS, 0);
    t.delete();
    assert_eq!(shape(&t), [(0, 0), (1, 102)]);
    assert_eq!(t.sel, Some(0));
    // The last row of the tree.
    let mut t = selected(&[(0, 100)], 0);
    t.delete();
    assert_eq!((t.rows.len(), t.sel), (0, None));
    // Nothing selected: nothing is removed.
    let mut t = tree(&TWO_MENUS);
    t.delete();
    assert_eq!(t.rows.len(), 5);
}

#[test]
fn up_and_down_move_a_row_with_its_rows_among_its_siblings() {
    let mut t = selected(&TWO_MENUS, 3);
    t.up();
    assert_eq!(shape(&t), [(0, 0), (1, 102), (0, 0), (1, 100), (1, 101)]);
    assert_eq!(t.sel, Some(0));
    t.up();
    assert_eq!(t.sel, Some(0), "the first row cannot go up");
    t.down();
    assert_eq!(shape(&t), TWO_MENUS);
    assert_eq!(t.sel, Some(3), "the moved row stays selected");
    t.down();
    assert_eq!(t.sel, Some(3));
    // A function moves inside its menu only.
    let mut t = selected(&TWO_MENUS, 2);
    t.down();
    assert_eq!(shape(&t), TWO_MENUS);
    t.up();
    assert_eq!(shape(&t)[1..3], [(1, 101), (1, 100)]);
}

#[test]
fn right_puts_a_row_into_the_node_before_it_and_left_takes_it_out_after_its_parent() {
    // 「→」: the second menu goes into the first.
    let mut t = selected(&TWO_MENUS, 3);
    t.rows[0].collapsed = true;
    t.right();
    assert_eq!(shape(&t), [(0, 0), (1, 100), (1, 101), (1, 0), (2, 102)]);
    assert_eq!(t.sel, Some(3));
    assert!(!t.rows[0].collapsed, "the node opens");
    // The row before is a function: nothing happens.
    let mut t = selected(&TWO_MENUS, 2);
    t.right();
    assert_eq!(shape(&t), TWO_MENUS);
    // 「←」: only that row leaves; the rows after it stay in the parent.
    let mut t = selected(&TWO_MENUS, 1);
    t.left();
    assert_eq!(shape(&t), [(0, 0), (1, 101), (0, 100), (0, 0), (1, 102)]);
    assert_eq!(t.sel, Some(2));
    // A row of the top level has no parent.
    let mut t = selected(&TWO_MENUS, 0);
    t.left();
    assert_eq!(shape(&t), TWO_MENUS);
}

#[test]
fn the_access_key_is_not_for_a_separator_and_clears_the_mark_of_a_duplicate() {
    let mut t = selected(&[(0, 0), (1, 100), (1, 1)], 1);
    t.rows[1].duplicate = true;
    t.set_key(Some('A'));
    assert_eq!(t.rows[1].row.key, Some('A'));
    assert!(!t.rows[1].duplicate);
    t.set_key(None);
    assert_eq!(t.rows[1].row.key, None);
    t.sel = Some(2);
    t.set_key(Some('B'));
    assert_eq!(t.rows[2].row.key, None);
}

#[test]
fn the_name_of_a_node_is_cut_empty_means_the_new_name_and_a_changed_name_is_its_own() {
    let mut t = selected(&TWO_MENUS, 0);
    t.set_name("File", "n0");
    assert_eq!(t.rows[0].row.name, "File");
    t.set_name("", "File");
    assert_eq!(t.rows[0].row.name, NEW_NODE_NAME);
    t.set_name(&"あ".repeat(60), NEW_NODE_NAME);
    assert_eq!(t.rows[0].row.name.chars().count(), 40);
    // A node that takes its name from its function keeps it while the name shown stays.
    let mut t = selected(&TWO_MENUS, 0);
    t.rows[0].row.function = 34052;
    t.rows[0].row.name.clear();
    t.set_name("ファイル", "ファイル");
    assert_eq!(t.rows[0].row.function, 34052);
    t.set_name("Mine", "ファイル");
    assert_eq!(
        (t.rows[0].row.function, t.rows[0].row.name.as_str()),
        (0, "Mine")
    );
    // A function has no name to set.
    let mut t = selected(&TWO_MENUS, 1);
    t.set_name("x", "y");
    assert_eq!(t.rows[1].row.function, 100);
}

// ---- 検査 (`Check_MainMenu`) ----

#[test]
fn the_check_finds_the_missing_common_settings_the_duplicate_and_the_unset_keys() {
    let mut t = tree(&[(0, 0), (1, 100), (1, 101), (1, 1), (0, 0), (1, 102)]);
    t.rows[0].row.key = Some('F');
    t.rows[1].row.key = Some('A');
    t.rows[2].row.key = Some('A');
    t.rows[4].row.key = Some('G');
    let (message, first) = t.check();
    // No 共通設定, a duplicate key in the first menu, and the last function has no key.
    assert_eq!(
        message,
        "『共通設定』が有りません。\n重複したアクセスキーがあります。\n未設定のアクセスキーがあります。\n"
    );
    // The first problem found is the first of the two rows with the same key.
    assert_eq!(first, Some(1));
    assert!(t.rows[1].duplicate && t.rows[2].duplicate);
    assert!(!t.rows[0].duplicate && !t.rows[4].duplicate);
    // A row that is a separator needs no key.
    assert!(!t.rows[3].duplicate);
    // Fix the key: the mark goes at the next check.
    t.rows[2].row.key = Some('B');
    t.rows[2].duplicate = false;
    let (_, _) = t.check();
    assert!(!t.rows[1].duplicate, "the key is unique now");
}

#[test]
fn a_menu_without_a_problem_has_an_empty_message() {
    let mut t = tree(&[(0, 0), (1, F_OPTION), (1, 1), (1, 29002)]);
    t.rows[0].row.key = Some('S');
    t.rows[1].row.key = Some('O');
    assert_eq!(t.check(), (String::new(), None));
}

#[test]
fn the_check_counts_the_menus_of_the_bar_and_the_rows() {
    let mut spec = vec![(0, F_OPTION)];
    spec.extend((0..21).map(|_| (0, 100)));
    let mut t = tree(&spec);
    for (i, row) in t.rows.iter_mut().enumerate() {
        row.row.key = char::from_u32(0x41 + i as u32);
    }
    let (message, _) = t.check();
    assert_eq!(message, "トップレベルの項目数が多すぎます。\n");
    let mut spec = vec![(0, 0), (1, F_OPTION)];
    spec.extend((0..500).map(|_| (1, 1)));
    let (message, _) = tree(&spec).check();
    assert!(message.contains("登録項目数が多すぎます。\n"), "{message}");
}

// ---- the data of the page ----

#[test]
fn the_state_is_kept_as_numbers_and_comes_back() {
    let mut t = tree(&TWO_MENUS);
    t.rows[0].row.key = Some('F');
    t.rows[0].row.name = "A & B あ".to_owned();
    t.rows[1].duplicate = true;
    t.rows[3].collapsed = true;
    let state = State {
        rows: t.rows,
        key_parentheses: true,
    };
    assert_eq!(State::decode(&state.encode()), state);
    // A list that is cut short ends at the last whole row.
    let data = state.encode();
    assert_eq!(State::decode(&data[..data.len() - 3]).rows.len(), 4);
    assert!(State::decode(&[]).rows.is_empty());
}

#[test]
fn the_default_is_loaded_and_writing_it_again_changes_nothing() {
    let mut p = profile("[Common]\r\nbA=1\r\n");
    let data = data::load(&p);
    let state = State::decode(&data);
    assert_eq!(state.menu(), *crate::menu::default_main_menu());
    assert!(state.key_parentheses);
    // The nodes start closed: only the menus of the bar show.
    let tree = Tree {
        rows: state.rows.clone(),
        sel: None,
        func_sel: None,
        func_count: 0,
    };
    let tops = state.rows.iter().filter(|r| r.row.level == 0).count();
    assert_eq!(tree.visible().len(), tops);
    assert!(!data::store(&data, &mut p).unwrap());
    assert_eq!(p.get("MainMenu", "nMainMenuNum"), None);
}

#[test]
fn the_menu_is_written_to_main_menu_and_the_rest_of_the_file_stays() {
    let mut p = profile(
        "[Other]\r\nx=1\r\n[MainMenu]\r\nnMainMenuVer=2\r\nnMainMenuNum=3\r\nbKeyParentheses=1\r\n\
         MM[000]=0,0,34052,F,\r\nMM[001]=1,1,Undo,U,\r\nMM[002]=1,1,31112,O,\r\n[Common]\r\nbA=1\r\n",
    );
    let mut state = State::decode(&data::load(&p));
    assert_eq!(state.rows.len(), 3);
    state.rows[2].row.key = Some('P');
    state.rows.push(PageRow {
        row: MenuRow::leaf(1, 30102),
        duplicate: false,
        collapsed: false,
    });
    state.key_parentheses = false;
    assert!(data::store(&state.encode(), &mut p).unwrap());
    assert_eq!(p.get("MainMenu", "nMainMenuNum"), Some("4"));
    assert_eq!(p.get("MainMenu", "bKeyParentheses"), Some("0"));
    assert_eq!(p.get("MainMenu", "nMainMenuVer"), Some("2"));
    // The row that did not change keeps the file's text (the macro name).
    assert_eq!(p.get("MainMenu", "MM[001]"), Some("1,1,Undo,U,"));
    assert_eq!(p.get("MainMenu", "MM[002]"), Some("1,1,31112,P,"));
    assert_eq!(p.get("MainMenu", "MM[003]"), Some("1,1,30102,,"));
    assert_eq!(p.get("Other", "x"), Some("1"));
    assert_eq!(p.get("Common", "bA"), Some("1"));
    // Reading it again gives the page back, and writing it again changes nothing.
    let again = State::decode(&data::load(&p));
    assert_eq!(again.menu(), state.menu());
    assert!(!data::store(&data::load(&p), &mut p).unwrap());
}

#[test]
fn the_page_keeps_the_menu_in_a_numbers_control_and_all_its_buttons_are_controls() {
    let items = items().expect("the page is built");
    let list = controls(&items);
    assert!(
        list.iter()
            .any(|c| matches!(c.kind, Kind::Numbers { .. }) && c.id == DATA)
    );
    for id in [
        "mainmenu_delete",
        "mainmenu_node",
        "mainmenu_separator",
        "mainmenu_above",
        "mainmenu_below",
        "mainmenu_add",
        "mainmenu_up",
        "mainmenu_down",
        "mainmenu_right",
        "mainmenu_left",
        "mainmenu_expand",
        "mainmenu_collapse",
        "mainmenu_set_name",
        "mainmenu_set_key",
        "mainmenu_parentheses",
        "mainmenu_import",
        "mainmenu_export",
        "mainmenu_check",
        "mainmenu_clear",
        "mainmenu_initialize",
    ] {
        assert!(list.iter().any(|c| c.id == id), "{id}");
    }
    // Only the numbers are written to the file: no control has a key.
    assert!(list.iter().all(|c| c.key.is_empty()));
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

fn open_page(cx: &mut TestAppContext, text: &str) -> Ini {
    let (ini, handle) = setup(cx, text);
    cx.update(|app| common_settings::open(app, Some(Page::MainMenu), Some(handle)));
    cx.run_until_parked();
    ini
}

fn tree_now(cx: &mut TestAppContext) -> Tree {
    with_dialog(cx, |d, _, _| tree_of(d.values()))
}

fn select_row(cx: &mut TestAppContext, row: usize) {
    with_dialog(cx, |d, window, dcx| d.main_menu_select(row, window, dcx));
}

fn choose_function(cx: &mut TestAppContext, kind: i32, row: i32) {
    with_dialog(cx, |d, _, dcx| {
        d.set_int(KIND, kind, dcx);
        d.set_int(FUNC_SEL, row, dcx);
    });
}

fn answer(cx: &mut TestAppContext, button: &str) {
    cx.simulate_prompt_answer(button);
    cx.run_until_parked();
}

fn user_menu(cx: &mut TestAppContext) -> MainMenu {
    cx.update(|app| app.global::<UserSettings>().main_menu())
}

fn tops(cx: &mut TestAppContext) -> Vec<usize> {
    tree_now(cx)
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.row.level == 0)
        .map(|(i, _)| i)
        .collect()
}

fn index_of(cx: &mut TestAppContext, function: u16) -> usize {
    tree_now(cx)
        .rows
        .iter()
        .position(|r| r.row.function == function)
        .unwrap()
}

#[gpui::test]
fn the_page_shows_the_menu_bar_and_the_buttons_follow_the_rules(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let _ini = open_page(cx, "");
    let t = tree_now(cx);
    assert_eq!(t.rows.len(), crate::menu::default_main_menu().rows.len());
    let tops = t.rows.iter().filter(|r| r.row.level == 0).count();
    assert_eq!(t.visible().len(), tops, "the nodes start closed");
    // Nothing is selected: only the buttons that need no row work.
    for id in [
        "mainmenu_delete",
        "mainmenu_up",
        "mainmenu_down",
        "mainmenu_right",
        "mainmenu_left",
        "mainmenu_above",
        "mainmenu_below",
        "mainmenu_add",
        "mainmenu_set_key",
        "mainmenu_set_name",
    ] {
        assert!(!button_is_usable(cx, id), "{id}");
    }
    for id in ["mainmenu_node", "mainmenu_separator", "mainmenu_expand"] {
        assert!(button_is_usable(cx, id), "{id}");
    }
    // The first menu: it can go down, not up, in, or out; it is a node, so it has a name.
    select_row(cx, 0);
    assert!(button_is_usable(cx, "mainmenu_delete"));
    assert!(button_is_usable(cx, "mainmenu_down"));
    assert!(!button_is_usable(cx, "mainmenu_up"));
    assert!(!button_is_usable(cx, "mainmenu_right"));
    assert!(!button_is_usable(cx, "mainmenu_left"));
    assert!(button_is_usable(cx, "mainmenu_set_name"));
    assert!(button_is_usable(cx, "mainmenu_set_key"));
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text(NAME, dcx)).unwrap(),
        "ファイル"
    );
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text(KEY, dcx)).unwrap(),
        "F"
    );
    // A function chosen: 上⇒ and 下⇒ need a row, >> does not.
    choose_function(cx, 0, 1);
    assert!(button_is_usable(cx, "mainmenu_above"));
    assert!(button_is_usable(cx, "mainmenu_add"));
    // A separator of the file menu has no key and no name; it can go left.
    with_dialog(cx, |d, window, dcx| {
        d.main_menu_edit(window, dcx, |tree| tree.expand_all(true));
    });
    let separator = tree_now(cx)
        .rows
        .iter()
        .position(|r| r.row.kind == RowKind::Separator)
        .unwrap();
    select_row(cx, separator);
    assert!(!button_is_usable(cx, "mainmenu_set_key"));
    assert!(!button_is_usable(cx, "mainmenu_set_name"));
    assert!(button_is_usable(cx, "mainmenu_left"));
}

#[gpui::test]
fn the_buttons_change_the_tree_and_the_boxes_set_names_and_keys(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let _ini = open_page(cx, "");
    let rows_before = tree_now(cx).rows.len();
    // ＋ with nothing selected: a new node at the end, selected, with its name in the box.
    press(cx, "mainmenu_node");
    let t = tree_now(cx);
    assert_eq!(t.rows.len(), rows_before + 1);
    assert_eq!(t.sel, Some(rows_before));
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text(NAME, dcx)).unwrap(),
        NEW_NODE_NAME
    );
    // 設定 takes the box as the name and the key box as the key.
    with_dialog(cx, |d, window, dcx| {
        d.set_input(NAME, "My menu", window, dcx)
    });
    press(cx, "mainmenu_set_name");
    with_dialog(cx, |d, window, dcx| d.set_input(KEY, "M", window, dcx));
    press(cx, "mainmenu_set_key");
    let row = tree_now(cx).rows[rows_before].clone();
    assert_eq!((row.row.name.as_str(), row.row.key), ("My menu", Some('M')));
    assert_eq!(row_text(&row, &table_names), "My menu(M)");
    // >> puts the chosen function in the node and moves the list on.
    choose_function(cx, 0, 0);
    press(cx, "mainmenu_add");
    let code = functions_of_kind(0)[0];
    let t = tree_now(cx);
    assert_eq!(t.rows[rows_before + 1].row.function, code);
    assert_eq!(t.rows[rows_before + 1].row.level, 1);
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(FUNC_SEL)), 1);
    // ← takes the function out; 削除 removes it.
    press(cx, "mainmenu_left");
    assert_eq!(tree_now(cx).rows[rows_before + 1].row.level, 0);
    press(cx, "mainmenu_delete");
    assert_eq!(tree_now(cx).rows.len(), rows_before + 1);
}

#[gpui::test]
fn deleting_a_menu_with_rows_asks_first_and_clear_and_initialize_ask(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let _ini = open_page(cx, "");
    let rows_before = tree_now(cx).rows.len();
    select_row(cx, 0);
    press(cx, "mainmenu_delete");
    let (question, _) = cx.pending_prompt().expect("the question");
    assert_eq!(
        question,
        "選択している項目を下位項目ごと削除します。\nよろしいですか？"
    );
    answer(cx, "キャンセル");
    assert_eq!(tree_now(cx).rows.len(), rows_before);
    press(cx, "mainmenu_delete");
    answer(cx, "OK");
    assert!(tree_now(cx).rows.len() < rows_before);
    // クリア empties the tree after asking.
    press(cx, "mainmenu_clear");
    let (question, _) = cx.pending_prompt().expect("the question");
    assert_eq!(question, "メニューの設定をクリアします。\nよろしいですか？");
    answer(cx, "キャンセル");
    assert!(!tree_now(cx).rows.is_empty());
    press(cx, "mainmenu_clear");
    answer(cx, "OK");
    assert!(tree_now(cx).rows.is_empty());
    // 初期設定 brings Sakura's menu back, with its setting of the access keys.
    with_dialog(cx, |d, _, dcx| d.main_menu_set_parentheses(false, dcx));
    press(cx, "mainmenu_initialize");
    let (question, _) = cx.pending_prompt().expect("the question");
    assert_eq!(
        question,
        "メニューの設定を初期状態に戻します。\nよろしいですか？"
    );
    answer(cx, "OK");
    let state = with_dialog(cx, |d, _, _| state_of(d.values()));
    assert_eq!(state.menu(), *crate::menu::default_main_menu());
    assert!(state.key_parentheses);
    cancel(cx);
}

#[gpui::test]
fn the_check_says_what_it_found_and_selects_the_first_row_with_a_problem(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let _ini = open_page(cx, "");
    // Sakura's own menu has keys that two items of a menu share and items with no key.
    press(cx, "mainmenu_check");
    let (message, _) = cx.pending_prompt().expect("the message");
    assert_eq!(
        message,
        "重複したアクセスキーがあります。\n未設定のアクセスキーがあります。\n"
    );
    answer(cx, "OK");
    // A small menu with a key for everything is fine.
    with_dialog(cx, |d, _, dcx| {
        let mut rows = vec![
            PageRow {
                row: MenuRow {
                    kind: RowKind::Node,
                    key: Some('F'),
                    name: "A".to_owned(),
                    ..MenuRow::leaf(0, 0)
                },
                duplicate: false,
                collapsed: false,
            },
            PageRow {
                row: MenuRow {
                    key: Some('O'),
                    ..MenuRow::leaf(1, F_OPTION)
                },
                duplicate: false,
                collapsed: false,
            },
            PageRow {
                row: MenuRow {
                    kind: RowKind::Node,
                    key: Some('E'),
                    name: "B".to_owned(),
                    ..MenuRow::leaf(0, 0)
                },
                duplicate: false,
                collapsed: false,
            },
        ];
        rows[2].row.function = 0;
        d.main_menu_put(
            &State {
                rows,
                key_parentheses: true,
            },
            dcx,
        );
    });
    press(cx, "mainmenu_check");
    let (message, _) = cx.pending_prompt().expect("the message");
    assert_eq!(message, "問題ありませんでした。");
    answer(cx, "OK");
    // Two menus with the same key.
    let second = tops(cx)[1];
    select_row(cx, second);
    with_dialog(cx, |d, window, dcx| d.set_input(KEY, "F", window, dcx));
    press(cx, "mainmenu_set_key");
    press(cx, "mainmenu_check");
    let (message, _) = cx.pending_prompt().expect("the message");
    assert_eq!(message, "重複したアクセスキーがあります。\n");
    answer(cx, "OK");
    let t = tree_now(cx);
    assert!(t.rows[0].duplicate && t.rows[second].duplicate);
    assert_eq!(t.sel, Some(0));
    assert_eq!(row_text(&t.rows[0], &table_names), ">A(F)");
    // Setting the key again takes the mark off.
    select_row(cx, second);
    with_dialog(cx, |d, window, dcx| d.set_input(KEY, "E", window, dcx));
    press(cx, "mainmenu_set_key");
    assert!(!tree_now(cx).rows[second].duplicate);
}

#[gpui::test]
fn ok_writes_the_menu_and_every_window_takes_it_at_once(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let (ini, first) = setup(cx, "[Other]\r\nx=1\r\n[Common]\r\nbA=1\r\n");
    cx.update(|app| {
        app.set_global(crate::app::MenuKeyMap(KeyMap::default()));
    });
    cx.update(|app| common_settings::open(app, Some(Page::MainMenu), Some(first)));
    cx.run_until_parked();
    // A new menu with a function in it, and the access keys without parentheses.
    press(cx, "mainmenu_node");
    with_dialog(cx, |d, window, dcx| d.set_input(NAME, "Mine", window, dcx));
    press(cx, "mainmenu_set_name");
    with_dialog(cx, |d, window, dcx| d.set_input(KEY, "M", window, dcx));
    press(cx, "mainmenu_set_key");
    choose_function(cx, 0, 1);
    press(cx, "mainmenu_add");
    with_dialog(cx, |d, _, dcx| d.main_menu_set_parentheses(false, dcx));
    let rows = tree_now(cx).rows.len();
    ok(cx);

    let p = ini.profile();
    assert_eq!(p.get("MainMenu", "nMainMenuVer"), Some("2"));
    assert_eq!(
        p.get("MainMenu", "nMainMenuNum"),
        Some(rows.to_string().as_str())
    );
    assert_eq!(p.get("MainMenu", "bKeyParentheses"), Some("0"));
    let last = format!("MM[{:03}]", rows - 1);
    assert_eq!(
        p.get("MainMenu", &last),
        Some(format!("1,1,{},,", functions_of_kind(0)[1]).as_str())
    );
    assert_eq!(
        p.get("MainMenu", &format!("MM[{:03}]", rows - 2)),
        Some("0,0,0,M,Mine")
    );
    assert_eq!(p.get("Other", "x"), Some("1"));
    assert_eq!(p.get("Common", "bA"), Some("1"));

    // The settings of the application have it, and the menus were made again from them.
    let menu = user_menu(cx);
    assert_eq!(menu.rows.len(), rows);
    assert!(!menu.key_parentheses);
    let menus = cx.update(|app| app.get_menus().unwrap_or_default());
    let mine = menus
        .iter()
        .find(|m| m.name.contains("Mine"))
        .expect("the menu");
    assert_eq!(mine.items.len(), 1);
    // "SJISで開き直す" shows its key as a letter of the name when the setting is off.
    let file = menus.iter().find(|m| m.name.contains("ファイル")).unwrap();
    let reopen = file
        .items
        .iter()
        .find_map(|i| match i {
            gpui::OwnedMenuItem::Submenu(m) if m.name.starts_with("開き直す") => Some(m),
            _ => None,
        })
        .expect("the reopen submenu");
    assert!(reopen.items.iter().any(
        |i| matches!(i, gpui::OwnedMenuItem::Action { name, .. } if name.starts_with("SJISで開き直す") && !name.contains("(S)"))
    ));
}

#[gpui::test]
fn the_unchanged_page_writes_no_section_and_the_boxes_are_not_written(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let ini = open_page(cx, "[Other]\r\nx=1\r\n");
    select_row(cx, 2);
    with_dialog(cx, |d, window, dcx| d.set_input(KEY, "Z", window, dcx));
    with_dialog(cx, |d, window, dcx| d.set_input(NAME, "Typed", window, dcx));
    ok(cx);
    assert!(
        !ini.profile().section_names().any(|n| n == "MainMenu"),
        "no [MainMenu] is written for Sakura's menu"
    );
}

#[gpui::test]
fn a_menu_in_the_file_is_what_the_page_starts_from(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let _ini = open_page(
        cx,
        "[MainMenu]\r\nnMainMenuVer=2\r\nnMainMenuNum=3\r\nbKeyParentheses=0\r\n\
         MM[000]=0,0,0,X,Only && menu\r\nMM[001]=1,1,31112,O,\r\nMM[002]=1,2,1,,\r\n",
    );
    let state = with_dialog(cx, |d, _, _| state_of(d.values()));
    assert!(!state.key_parentheses);
    assert_eq!(state.rows.len(), 3);
    assert_eq!(row_name(&state.rows[0], &table_names), "Only & menu");
    // The window has this menu bar.
    let menu = user_menu(cx);
    let menus = crate::menu::main_menu_for(
        &menu,
        &KeyMap::default(),
        &crate::menu::Registered,
        &crate::menu::NoLists,
        &Default::default(),
    );
    assert_eq!(menus.len(), 1);
    assert_eq!(menus[0].label, "Only & menu(X)");
}

#[gpui::test]
fn export_writes_the_menu_file_and_import_reads_it_back(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let ini = open_page(cx, "");
    let file = ini.dir.0.join("MainMenu.ini");
    // A menu of one node and two functions, one of which has a macro name.
    with_dialog(cx, |d, window, dcx| {
        let rows = vec![
            PageRow {
                row: MenuRow {
                    name: "Mine".to_owned(),
                    key: Some('M'),
                    ..MenuRow::leaf(0, 0)
                },
                duplicate: false,
                collapsed: true,
            },
            PageRow {
                row: MenuRow {
                    key: Some('U'),
                    ..MenuRow::leaf(1, 30210)
                },
                duplicate: false,
                collapsed: false,
            },
            PageRow {
                row: MenuRow::leaf(1, F_OPTION),
                duplicate: false,
                collapsed: false,
            },
        ];
        let mut rows = rows;
        rows[0].row.kind = RowKind::Node;
        d.main_menu_put(
            &State {
                rows,
                key_parentheses: false,
            },
            dcx,
        );
        assert!(d.main_menu_export_file(&file, window, dcx));
    });
    answer(cx, "OK");
    let written = Profile::load(&file).unwrap();
    assert_eq!(
        written.get("Info", "MENU_VERSION"),
        Some("SakuraEditorMainMenu Ver1")
    );
    assert_eq!(written.get("MainMenu", "nMainMenuNum"), Some("3"));
    assert_eq!(written.get("MainMenu", "bKeyParentheses"), Some("0"));
    assert_eq!(written.get("MainMenu", "MM[000]"), Some("0,0,0,M,Mine"));
    // A function with a macro name is written by it, one without by its code.
    assert_eq!(written.get("MainMenu", "MM[001]"), Some("1,1,Undo,U,"));
    assert_eq!(
        written.get("MainMenu", "MM[002]"),
        Some("1,1,OptionCommon,,")
    );

    // The page goes back to Sakura's menu, then the file is read over it.
    press(cx, "mainmenu_initialize");
    answer(cx, "OK");
    assert_eq!(
        tree_now(cx).rows.len(),
        crate::menu::default_main_menu().rows.len()
    );
    with_dialog(cx, |d, window, dcx| {
        assert!(d.main_menu_import_file(&file, window, dcx))
    });
    let state = with_dialog(cx, |d, _, _| state_of(d.values()));
    assert_eq!(state.rows.len(), 3);
    assert!(!state.key_parentheses);
    assert_eq!(state.rows[0].row.name, "Mine");
    assert_eq!(state.rows[1].row.function, 30210);
    assert_eq!(state.rows[1].row.key, Some('U'));
    // Nothing is in the settings file until OK.
    assert_eq!(ini.profile().get("MainMenu", "nMainMenuNum"), None);
    ok(cx);
    assert_eq!(ini.profile().get("MainMenu", "nMainMenuNum"), Some("3"));
}

#[gpui::test]
fn a_file_that_is_not_a_menu_file_is_refused_and_changes_nothing(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let ini = open_page(cx, "");
    let wrong = ini.dir.0.join("wrong.ini");
    std::fs::write(
        &wrong,
        with_bom(
            "[Info]\r\nMENU_VERSION=SakuraEditorMenu_Ver2\r\n[MainMenu]\r\nnMainMenuNum=1\r\n",
        ),
    )
    .unwrap();
    let missing = ini.dir.0.join("missing.ini");
    let before = with_dialog(cx, |d, _, _| state_of(d.values()));
    with_dialog(cx, |d, window, dcx| {
        assert!(!d.main_menu_import_file(&wrong, window, dcx));
    });
    let (message, _) = cx.pending_prompt().expect("the message");
    assert!(
        message.starts_with("メインメニュー設定ファイルの形式が違います。\n\n"),
        "{message}"
    );
    answer(cx, "OK");
    with_dialog(cx, |d, window, dcx| {
        assert!(!d.main_menu_import_file(&missing, window, dcx));
    });
    let (message, _) = cx.pending_prompt().expect("the message");
    assert!(
        message.starts_with("ファイルを開けませんでした。\n\n"),
        "{message}"
    );
    answer(cx, "OK");
    assert_eq!(with_dialog(cx, |d, _, _| state_of(d.values())), before);
}

#[gpui::test]
fn the_alt_keys_of_the_labels_work(cx: &mut TestAppContext) {
    let _restore = crate::common_settings::tests::RestoreEditOptions::new();
    let _ini = open_page(cx, "");
    // Alt+K is the kind (the next one), Alt+P the check box, Alt+N a new node.
    assert!(with_dialog(cx, |d, window, dcx| d.use_access_key('k', window, dcx)));
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(KIND)), 1);
    assert!(with_dialog(cx, |d, window, dcx| d.use_access_key('p', window, dcx)));
    assert!(!with_dialog(cx, |d, _, _| state_of(d.values()).key_parentheses));
    let rows = tree_now(cx).rows.len();
    assert!(with_dialog(cx, |d, window, dcx| d.use_access_key('n', window, dcx)));
    assert_eq!(tree_now(cx).rows.len(), rows + 1);
    // 検査 (Alt+T) says that there is a problem now: the new node has no key.
    assert!(with_dialog(cx, |d, window, dcx| d.use_access_key('t', window, dcx)));
    let (message, _) = cx.pending_prompt().expect("the message");
    assert!(
        message.ends_with("未設定のアクセスキーがあります。\n"),
        "{message}"
    );
    answer(cx, "OK");
    // The new row is the last; a function of the default menu is still found.
    assert!(index_of(cx, F_OPTION) < rows);
}

const OLD_MENU_FILE: &str = "[MainMenu]\r\nnMainMenuVer=2\r\nnMainMenuNum=3\r\nbKeyParentheses=1\r\n\
     MM[000]=0,0,34052,F,\r\nMM[001]=1,1,30603,L,\r\nMM[002]=1,1,31112,O,\r\n";

fn functions_of(profile: &Profile) -> Vec<u16> {
    State::decode(&data::load(profile))
        .rows
        .iter()
        .map(|r| r.row.function)
        .collect()
}

#[test]
fn an_old_menu_shows_the_matsu_items_and_ok_with_no_change_writes_nothing() {
    let mut p = profile(OLD_MENU_FILE);
    assert_eq!(functions_of(&p), [34052, 30603, 32700, 32701, 31112]);
    let before = p.to_bytes();
    assert!(!data::store(&data::load(&p), &mut p).unwrap());
    assert_eq!(p.to_bytes(), before, "the file keeps its bytes");
    assert_eq!(p.get("MainMenu", "nMatsuMainMenuVer"), None);
}

#[test]
fn a_changed_old_menu_is_written_with_the_matsu_items_and_the_matsu_version() {
    let mut p = profile(OLD_MENU_FILE);
    let mut state = State::decode(&data::load(&p));
    state.key_parentheses = false;
    assert!(data::store(&state.encode(), &mut p).unwrap());
    assert_eq!(p.get("MainMenu", "nMatsuMainMenuVer"), Some("2"));
    assert_eq!(p.get("MainMenu", "nMainMenuNum"), Some("5"));
    assert_eq!(p.get("MainMenu", "MM[002]"), Some("1,1,32700,N,"));
    // The user takes 32700 out and saves: it does not come back.
    let mut state = State::decode(&data::load(&p));
    state.rows.retain(|r| r.row.function != 32700);
    assert!(data::store(&state.encode(), &mut p).unwrap());
    assert_eq!(functions_of(&p), [34052, 30603, 32701, 31112]);
}
