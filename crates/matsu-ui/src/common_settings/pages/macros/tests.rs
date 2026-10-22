use gpui::TestAppContext;
use matsu_config::Profile;
use matsu_config::settings::macros::RegisteredMacros;

use super::list::{
    FILE, ID, LIST, NAME, ON_OPENED, ON_SAVE, ON_TYPE_CHANGED, RELOAD, ROWS, SELECTED, select,
};
use super::rows::{Fields, fields_of, index_of, register, rows_of, table_of};
use crate::common_settings::model::{Kind, controls};
use crate::common_settings::tests::{cancel, check, ok, run, setup, type_in, with_dialog};
use crate::common_settings::{Page, all_controls};

const INI: &str = "[Other]\r\nx=1\r\n\
[Macro]\r\nName[001]=整形\r\nFile[001]=fmt.mac\r\nReloadWhenExecute[001]=1\r\n\
Name[002]=\r\nFile[002]=b.mjs\r\nReloadWhenExecute[002]=0\r\n\
nMacroOnOpened=2\r\nnMacroOnTypeChanged=-1\r\nnMacroOnSave=1\r\nnMacroCancelTimer=7\r\n\
nMacroPythonDirectory=py\\\r\n\
[Folders]\r\nszMACROFOLDER=C:\\m\r\n\
[Common]\r\nbDispSTATUSBAR=1\r\n";

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn fields(name: &str, file: &str) -> Fields {
    Fields {
        name: name.into(),
        file: file.into(),
        ..Fields::default()
    }
}

// ---- the rows ----

#[test]
fn the_rows_show_the_macros_as_sakura_does() {
    let table = RegisteredMacros::from_profile(&profile(INI));
    let rows = rows_of(&table);
    assert_eq!(rows.len(), 50);
    assert_eq!(rows[0], ["0", "", "", "off", ""]);
    assert_eq!(rows[1], ["1", "整形", "fmt.mac", "on", "S"]);
    assert_eq!(rows[2], ["2", "", "b.mjs", "off", "O"]);
    assert_eq!(rows[49][0], "49");
    // Read back, the rows are the macros.
    assert_eq!(table_of(&rows, 7), table);
}

#[test]
fn one_row_can_be_all_three_automatic_macros_and_the_last_row_with_a_letter_wins() {
    let mut rows = rows_of(&RegisteredMacros::default());
    rows[3][4] = "OTS".into();
    rows[5][4] = "T".into();
    let table = table_of(&rows, 10);
    assert_eq!(table.on_opened, Some(3));
    assert_eq!(table.on_save, Some(3));
    assert_eq!(table.on_type_changed, Some(5));
    // Other letters mean nothing.
    rows[3][4] = "xO".into();
    assert_eq!(table_of(&rows, 10).on_opened, Some(3));
    assert_eq!(table_of(&rows, 10).on_save, None);
}

#[test]
fn the_fields_of_a_row_are_its_texts() {
    let rows = rows_of(&RegisteredMacros::from_profile(&profile(INI)));
    assert_eq!(
        fields_of(&rows[1]),
        Fields {
            name: "整形".into(),
            file: "fmt.mac".into(),
            reload: true,
            opened: false,
            type_changed: false,
            save: true,
        }
    );
    // Only the text `on` is on.
    let mut row = rows[1].clone();
    row[3] = "ON".into();
    assert!(!fields_of(&row).reload);
}

#[test]
fn set_puts_the_fields_into_the_row_and_an_automatic_macro_moves_to_it() {
    let mut rows = rows_of(&RegisteredMacros::from_profile(&profile(INI)));
    // Row 4 takes オープン後: row 2 loses it.
    let mut f = fields("新規", "new.mac");
    f.opened = true;
    f.reload = true;
    register(&mut rows, 4, &f);
    assert_eq!(rows[4], ["4", "新規", "new.mac", "on", "O"]);
    assert_eq!(rows[2][4], "");
    // The other rows are as they were.
    assert_eq!(rows[1], ["1", "整形", "fmt.mac", "on", "S"]);
    // Row 1 with 保存前 off frees it; row 4 with it off does not free the macro of row 1.
    register(&mut rows, 4, &fields("新規", "new.mac"));
    assert_eq!(rows[1][4], "S");
    assert_eq!(rows[4][4], "");
    register(&mut rows, 1, &fields("整形", "fmt.mac"));
    assert_eq!(rows[1][4], "");
    assert_eq!(table_of(&rows, 10).on_save, None);
}

#[test]
fn set_cuts_the_name_to_63_characters_and_the_file_to_260() {
    let mut rows = rows_of(&RegisteredMacros::default());
    let long_name = "名".repeat(80);
    let long_file = "f".repeat(300);
    register(&mut rows, 0, &fields(&long_name, &long_file));
    assert_eq!(rows[0][1].chars().count(), 63);
    assert_eq!(rows[0][2].chars().count(), 260);
    // A row past the list changes nothing.
    let before = rows.clone();
    register(&mut rows, 50, &fields("x", "y"));
    assert_eq!(rows, before);
}

#[test]
fn the_id_box_is_a_number_of_the_list() {
    assert_eq!(index_of("7"), 7);
    assert_eq!(index_of(" 49 "), 49);
    assert_eq!(index_of("50"), 49);
    assert_eq!(index_of("-1"), 0);
    assert_eq!(index_of("abc"), 0);
}

#[test]
fn the_page_has_the_controls_of_sakura_and_only_the_list_state_is_keyless() {
    let list = controls(&super::items().unwrap());
    let ids: Vec<&str> = list.iter().map(|c| c.id).collect();
    for id in [
        "macro_dir",
        "macro_dir_ref",
        "macro_python_dir",
        "macro_python_ref",
        NAME,
        ID,
        FILE,
        "macro_register",
        RELOAD,
        "macro_cancel_timer",
        ON_OPENED,
        ON_TYPE_CHANGED,
        ON_SAVE,
        LIST,
        SELECTED,
    ] {
        assert!(ids.contains(&id), "{id}");
    }
    // The Python folder and the wait before the stop dialog are shown, not used.
    for id in ["macro_python_dir", "macro_python_ref", "macro_cancel_timer"] {
        assert!(!list.iter().find(|c| c.id == id).unwrap().live, "{id}");
    }
    assert!(list.iter().any(|c| matches!(c.kind, Kind::Table { .. })));
    // Every other control of the page is live.
    assert!(
        list.iter()
            .filter(|c| !c.id.starts_with("macro_python") && c.id != "macro_cancel_timer")
            .all(|c| c.live)
    );
    // The keys are Sakura's.
    let key = |id: &str| list.iter().find(|c| c.id == id).unwrap().key;
    assert_eq!(key("macro_dir"), "szMACROFOLDER");
    assert_eq!(key("macro_python_dir"), "nMacroPythonDirectory");
    assert_eq!(key("macro_cancel_timer"), "nMacroCancelTimer");
    assert!(all_controls().iter().any(|c| c.id == LIST));
}

// ---- the file ----

#[test]
fn the_list_is_stored_in_macro_and_the_folder_ends_with_a_backslash() {
    let mut file = profile(INI);
    let mut rows = rows_of(&RegisteredMacros::from_profile(&file));
    register(&mut rows, 3, &fields("三", "c.js"));
    let spec = super::rows::spec();
    assert!(spec.store(&rows, &mut file).unwrap());
    assert_eq!(file.get("Macro", "Name[003]"), Some("三"));
    assert_eq!(file.get("Macro", "File[003]"), Some("c.js"));
    assert_eq!(file.get("Macro", "ReloadWhenExecute[003]"), Some("0"));
    // What is not the list's stays: the timer, the Python folder, the other sections.
    assert_eq!(file.get("Macro", "nMacroCancelTimer"), Some("7"));
    assert_eq!(file.get("Macro", "nMacroPythonDirectory"), Some("py\\"));
    assert_eq!(file.get("Other", "x"), Some("1"));
    assert_eq!(file.get("Folders", "szMACROFOLDER"), Some("C:\\m\\"));
    // The list is read back from the file.
    assert_eq!(spec.load(&file), rows);
    // Storing what is there changes nothing.
    assert!(!spec.store(&rows, &mut file).unwrap());
}

// ---- the dialog ----

fn open_page(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) {
    run(cx, handle, "F_OPTION");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Macro, dcx));
}

fn list(cx: &mut TestAppContext) -> Vec<Vec<String>> {
    with_dialog(cx, |d, _, _| d.values().rows(LIST).to_vec())
}

fn text(cx: &mut TestAppContext, id: &str) -> String {
    with_dialog(cx, |d, _, dcx| d.input_text(id, dcx).unwrap_or_default())
}

fn press_set(cx: &mut TestAppContext) {
    with_dialog(cx, super::list::register);
}

fn click(cx: &mut TestAppContext, row: usize) {
    with_dialog(cx, |d, window, dcx| select(d, row, window, dcx));
}

#[gpui::test]
fn the_page_shows_the_50_macros_and_the_folder(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    let rows = list(cx);
    assert_eq!(rows.len(), ROWS);
    assert_eq!(rows[1], ["1", "整形", "fmt.mac", "on", "S"]);
    assert_eq!(text(cx, "macro_dir"), "C:\\m");
    // Nothing is selected, and the Id box starts at 0.
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(SELECTED)), -1);
    assert_eq!(text(cx, ID), "0");
    // The unused controls show what the file has.
    assert_eq!(text(cx, "macro_python_dir"), "py\\");
    assert_eq!(text(cx, "macro_cancel_timer"), "7");
}

#[gpui::test]
fn a_click_on_a_row_fills_the_fields_and_clicking_it_again_keeps_what_was_typed(
    cx: &mut TestAppContext,
) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    click(cx, 1);
    assert_eq!(text(cx, ID), "1");
    assert_eq!(text(cx, NAME), "整形");
    assert_eq!(text(cx, FILE), "fmt.mac");
    let on = |cx: &mut TestAppContext, id| with_dialog(cx, |d, _, _| d.values().bool(id));
    assert!(on(cx, RELOAD) && on(cx, ON_SAVE));
    assert!(!on(cx, ON_OPENED) && !on(cx, ON_TYPE_CHANGED));
    type_in(cx, NAME, "編集中");
    click(cx, 1);
    assert_eq!(text(cx, NAME), "編集中");
    click(cx, 2);
    assert_eq!(text(cx, NAME), "");
    assert_eq!(text(cx, FILE), "b.mjs");
    assert!(on(cx, ON_OPENED) && !on(cx, ON_SAVE) && !on(cx, RELOAD));
}

#[gpui::test]
fn the_button_set_writes_the_fields_to_the_row_of_the_id_box(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    // The row that is selected does not matter: the Id box says where.
    click(cx, 1);
    type_in(cx, ID, "9");
    type_in(cx, NAME, "九番");
    type_in(cx, FILE, "nine.mac");
    check(cx, RELOAD, true);
    check(cx, ON_TYPE_CHANGED, true);
    press_set(cx);
    let rows = list(cx);
    // 保存前 was on in the fields (they came from row 1), so row 9 takes it from row 1.
    assert_eq!(rows[9], ["9", "九番", "nine.mac", "on", "TS"]);
    assert_eq!(rows[1], ["1", "整形", "fmt.mac", "on", ""]);
}

#[gpui::test]
fn ok_writes_the_macros_and_keeps_the_rest_of_the_file(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    type_in(cx, ID, "3");
    type_in(cx, NAME, "三");
    type_in(cx, FILE, "c.js");
    check(cx, ON_OPENED, true);
    press_set(cx);
    // The cancel timer and the Python folder cannot be changed.
    type_in(cx, "macro_cancel_timer", "99");
    type_in(cx, "macro_python_dir", "other");
    ok(cx);
    let file = ini.profile();
    assert_eq!(file.get("Macro", "Name[003]"), Some("三"));
    assert_eq!(file.get("Macro", "File[003]"), Some("c.js"));
    assert_eq!(file.get("Macro", "nMacroOnOpened"), Some("3"));
    assert_eq!(file.get("Macro", "nMacroOnSave"), Some("1"));
    assert_eq!(file.get("Macro", "nMacroCancelTimer"), Some("7"));
    assert_eq!(file.get("Macro", "nMacroPythonDirectory"), Some("py\\"));
    assert_eq!(file.get("Folders", "szMACROFOLDER"), Some("C:\\m\\"));
    assert_eq!(file.get("Other", "x"), Some("1"));
    assert_eq!(file.get("Common", "bDispSTATUSBAR"), Some("1"));
    // The windows use them at once.
    let settings = cx.update(|app| crate::macros::registered_table(app));
    assert_eq!(settings.on_opened, Some(3));
    assert_eq!(settings.get(3).unwrap().file, "c.js");
}

#[gpui::test]
fn the_folder_box_is_written_to_folders(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    type_in(cx, "macro_dir", "D:\\マクロ");
    ok(cx);
    assert_eq!(
        ini.profile().get("Folders", "szMACROFOLDER"),
        Some("D:\\マクロ\\")
    );
}

#[gpui::test]
fn cancel_changes_nothing(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    let before = ini.bytes();
    open_page(cx, handle);
    type_in(cx, ID, "3");
    type_in(cx, FILE, "c.js");
    press_set(cx);
    cancel(cx);
    assert_eq!(ini.bytes(), before);
}

#[gpui::test]
fn ok_with_nothing_changed_leaves_the_macro_sections_as_they_are(cx: &mut TestAppContext) {
    // The folder gets its backslash on OK, as Sakura's `GetData` does; with it in the file, the
    // two sections do not change (the other pages write their own keys).
    let text = INI.replace("C:\\m\r\n", "C:\\m\\\r\n");
    let (ini, handle) = setup(cx, &text);
    let sections = |p: &Profile| -> Vec<(String, String)> {
        ["Macro", "Folders"]
            .into_iter()
            .flat_map(|s| p.entries(s).map(|(k, v)| (k.to_owned(), v.to_owned())))
            .collect()
    };
    let before = sections(&ini.profile());
    open_page(cx, handle);
    ok(cx);
    assert_eq!(sections(&ini.profile()), before);
}
