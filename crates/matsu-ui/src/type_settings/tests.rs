//! Tests of the type settings: the pages, the values, and OK and キャンセル with a real settings
//! file in a private temporary folder, in GPUI's headless test windows.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AnyWindowHandle, Context, Entity, TestAppContext, VisualTestContext, Window};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::Profile;
use matsu_config::history::HistoryStore;
use matsu_config::settings::{ColorSet, DocumentTypes, TypeSettings};

use super::ids::*;
use super::pages::color::{COLOR_ITEMS, STRING_TYPES};
use super::pages::pages;
use super::values::{format_vert_lines, load, parse_vert_lines, store};
use super::{PAGE_TITLES, current_dialog, dialog_window, open, sheet};
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Kind, Value, controls};
use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;
use crate::history::{HistoryOptions, HistoryState};
use crate::settings::UserSettings;
use crate::view::EditorView;

/// A folder that is removed at the end of the test.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("matsu-ui-type-settings-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The settings file of a test.
struct Ini {
    _dir: TempDir,
    path: PathBuf,
}

impl Ini {
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(&self.path).unwrap()
    }

    fn profile(&self) -> Profile {
        Profile::load(&self.path).unwrap()
    }

    fn get(&self, section: &str, key: &str) -> Option<String> {
        self.profile().get(section, key).map(str::to_owned)
    }

    /// The entries of every section but the `Types(N)` ones, to see that OK left them alone.
    fn other_sections(&self) -> Vec<(String, Vec<(String, String)>)> {
        let profile = self.profile();
        profile
            .section_names()
            .filter(|name| !name.starts_with("Types("))
            .map(|name| {
                (
                    name.to_owned(),
                    profile
                        .entries(name)
                        .map(|(k, v)| (k.to_owned(), v.to_owned()))
                        .collect(),
                )
            })
            .collect()
    }
}

fn with_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// A settings file with `text`, read as `app::run` reads it, and one editor window.
fn setup(cx: &mut TestAppContext, text: &str) -> (Ini, AnyWindowHandle) {
    cx.update(gpui_component::init);
    let dir = TempDir::new();
    let path = dir.0.join("matsu.ini");
    std::fs::write(&path, with_bom(text)).unwrap();
    cx.update(|cx| {
        let user = UserSettings::load_from(std::slice::from_ref(&path));
        cx.set_global(HistoryState::on_disk(
            HistoryStore::new(path.clone(), false),
            HistoryOptions::default(),
        ));
        crate::tabs::settings::install(cx, &user);
        cx.set_global(user);
    });
    let handle = open_window(cx);
    (Ini { _dir: dir, path }, handle)
}

fn open_window(cx: &mut TestAppContext) -> AnyWindowHandle {
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    handle
}

fn view_of(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    cx.update(|app| crate::file_ops::windows::workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.editor().clone())
}

/// Runs a function of Sakura in the window, as a key does.
fn run(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str, args: &[Arg]) {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    let view = view_of(cx, handle);
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, args, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn dialog(cx: &mut TestAppContext) -> Entity<CommonSettingsDialog> {
    cx.update(|app| current_dialog(app))
        .expect("no sheet is open")
}

fn is_open(cx: &mut TestAppContext) -> bool {
    cx.update(|app| current_dialog(app)).is_some()
}

/// Does something to the sheet and its window, as a click or a key does.
fn with_dialog<R>(
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut CommonSettingsDialog, &mut Window, &mut Context<CommonSettingsDialog>) -> R,
) -> R {
    let dialog = dialog(cx);
    let handle = cx.update(|app| dialog_window(app)).unwrap();
    let result = handle
        .update(cx, |_, window, app| {
            dialog.update(app, |dialog, dcx| f(dialog, window, dcx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

fn ok(cx: &mut TestAppContext) {
    with_dialog(cx, |d, window, dcx| d.ok(window, dcx));
    assert!(!is_open(cx), "OK closes the sheet");
}

fn cancel(cx: &mut TestAppContext) {
    with_dialog(cx, |d, window, dcx| d.cancel(window, dcx));
    assert!(!is_open(cx), "キャンセル closes the sheet");
}

fn check(cx: &mut TestAppContext, id: &'static str, on: bool) {
    with_dialog(cx, |d, _, dcx| d.set_bool(id, on, dcx));
}

fn choose(cx: &mut TestAppContext, id: &'static str, value: i32) {
    with_dialog(cx, |d, _, dcx| d.set_int(id, value, dcx));
}

fn color(cx: &mut TestAppContext, id: &'static str, colorref: i32) {
    with_dialog(cx, |d, _, dcx| d.set_color(id, colorref, dcx));
}

fn type_in(cx: &mut TestAppContext, id: &'static str, text: &str) {
    with_dialog(cx, |d, window, dcx| d.set_input(id, text, window, dcx));
}

fn value(cx: &mut TestAppContext, id: &'static str) -> Value {
    with_dialog(cx, |d, _, _| d.values().get(id).cloned().unwrap())
}

fn tab_width(cx: &mut TestAppContext, view: &Entity<EditorView>) -> u32 {
    view.read_with(cx, |v, _| v.view_settings().tab_width)
}

fn wrap_columns(cx: &mut TestAppContext, view: &Entity<EditorView>) -> u32 {
    view.read_with(cx, |v, _| v.view_settings().wrap_columns)
}

fn type_of(cx: &mut TestAppContext, view: &Entity<EditorView>) -> usize {
    view.read_with(cx, |v, _| v.type_index())
}

/// The numbers of `nInts` of a type (`idx,maxLineKetas,columnSpace,tabSpace,...`).
fn ints(ini: &Ini, index: usize) -> Vec<i32> {
    ini.get(&format!("Types({index})"), "nInts")
        .unwrap()
        .split(',')
        .map(|n| n.parse().unwrap())
        .collect()
}

// ---- the pages ----

#[test]
fn the_pages_are_in_sakuras_order() {
    // `TypePropSheetInfoList` in `CPropTypes.cpp`.
    assert_eq!(
        PAGE_TITLES,
        [
            "スクリーン",
            "カラー",
            "ウィンドウ",
            "支援",
            "正規表現キーワード",
            "キーワードヘルプ"
        ]
    );
    let list = pages(0);
    let titles: Vec<&str> = list.iter().map(|p| p.title).collect();
    assert_eq!(titles, PAGE_TITLES);
    let filled: Vec<bool> = list.iter().map(|p| p.items.is_some()).collect();
    assert_eq!(filled, [true; 6]);
}

#[test]
fn every_control_has_its_own_name_and_a_value() {
    let sheet = sheet(&Profile::new(), 0).unwrap();
    let mut seen = HashSet::new();
    for control in &sheet.controls {
        if !matches!(control.kind, Kind::Radio { .. }) {
            assert!(seen.insert(control.id), "{} is there twice", control.id);
        }
        if !matches!(control.kind, Kind::Button { .. }) {
            assert!(
                sheet.values.get(control.id).is_some(),
                "{} has no value",
                control.id
            );
        }
        // The values come from the type, not from `[Common]`.
        assert!(control.key.is_empty(), "{}", control.id);
    }
}

#[test]
fn the_pages_have_every_control_that_sakura_has() {
    // The check boxes, radio buttons, edit boxes, drop-down lists, list boxes, color buttons and
    // push buttons of `IDD_PROP_SCREEN`, `IDD_PROP_COLOR` and `IDD_PROP_WINDOW`; the spin
    // buttons and the label of the font belong to their neighbours, and the color button of
    // Sakura is a color picker here.
    let list = pages(0);
    let count = |page: usize| controls(list[page].items.as_ref().unwrap()).len();
    // (the font of the type, which the button and the label use, is one of them)
    assert_eq!(count(0), 34);
    // (the list of the first keyword set is drawn by the page, not a control)
    assert_eq!(count(1), 29);
    assert_eq!(count(2), 25);
    // 支援: the word file with its button and the plugin list, the three check boxes of the
    // input completion, the two help files with their buttons and the viewer box, the warning on
    // saving, the three options of the C/C++ indent and the detection of the indentation style.
    // 正規表現キーワード: the check box, the list (with its edit boxes and buttons, one control),
    // インポート and エクスポート. キーワードヘルプ: the check box, the list, the three options of
    // the search, the right-click menu, インポート and エクスポート.
    assert_eq!(count(3), 16);
    assert_eq!(count(4), 4);
    assert_eq!(count(5), 8);
}

#[test]
fn the_color_list_has_every_item_in_sakuras_order() {
    assert_eq!(COLOR_ITEMS.len(), ColorSet::all_names().count());
    for (position, (_, number)) in COLOR_ITEMS.iter().enumerate() {
        assert_eq!(usize::try_from(*number).unwrap(), position);
    }
    let names: Vec<&str> = ColorSet::all_names().collect();
    let label = |name: &str| COLOR_ITEMS[names.iter().position(|n| *n == name).unwrap()].0;
    assert_eq!(label("TXT"), "テキスト");
    assert_eq!(label("CVL"), "カーソル位置縦線");
    assert_eq!(label("TAB"), "TAB記号");
    assert_eq!(label("VER"), "指定桁縦線");
    assert_eq!(label("EOF"), "EOF記号");
    assert_eq!(label("KW1"), "強調キーワード1");
    assert_eq!(label("KWA"), "強調キーワード10");
    assert_eq!(label("RK1"), "正規表現キーワード1");
    assert_eq!(label("DFC"), "DIFF差分表示(変更)");
    assert_eq!(label("PGV"), "表示範囲(ミニマップ)");
    assert_eq!(STRING_TYPES.len(), 5);
}

#[test]
fn the_columns_of_vertical_lines_read_and_write_as_sakura_does() {
    let mut columns = [0; 200];
    columns[..6].copy_from_slice(&[80, -10, 50, 5, 120, 0]);
    // 80, then a line every 5 columns from 10 to 50, then 120.
    let text = format_vert_lines(&columns);
    assert_eq!(text, "80,5(10,50),120");
    let again = parse_vert_lines(&text);
    assert_eq!(&again[..6], &[80, -10, 50, 5, 120, 0]);
    // A step without an end goes to the widest column; a text that stops making sense ends the
    // list where it stands.
    assert_eq!(&parse_vert_lines("3(2)")[..4], &[-2, 10240, 3, 0]);
    assert_eq!(&parse_vert_lines("10,x,20")[..2], &[10, 0]);
    assert_eq!(parse_vert_lines("")[0], 0);
    assert_eq!(&parse_vert_lines("7,9,")[..3], &[7, 9, 0]);
    // A range that runs backwards is left out of the text.
    columns[..4].copy_from_slice(&[-50, 10, 5, 0]);
    assert_eq!(format_vert_lines(&columns), "");
}

#[test]
fn nothing_changes_when_the_values_are_not_touched() {
    // `GetData` of the pages gives back what `SetData` showed, for every type Sakura has and
    // every type of MATSU Editor's.
    let profile = Profile::new();
    let types = DocumentTypes::from_profile(&profile);
    for (index, t) in types.iter().enumerate() {
        let sheet = sheet(&profile, index).unwrap();
        let values = load(&sheet.controls, t, &profile, index, &[]);
        assert_eq!(&store(&values, t), t, "{} ({index})", t.type_name);
    }
}

// ---- opening ----

#[gpui::test]
fn the_command_opens_the_type_of_the_document(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_TYPE", &[]);
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int(TYPE_INDEX)),
        0,
        "a new document has the basic type"
    );
    assert_eq!(with_dialog(cx, |d, _, _| d.page_index()), 0);
    cancel(cx);
    // 一時適用 of a type, then its settings.
    run(cx, handle, "F_CHANGETYPE", &[Arg::Int(3)]);
    run(cx, handle, "F_OPTION_TYPE", &[]);
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(TYPE_INDEX)), 2);
    assert_eq!(
        value(cx, TYPE_NAME),
        Value::Text("C/C++".to_owned()),
        "the name of the type"
    );
    cancel(cx);
}

#[gpui::test]
fn the_list_opens_the_settings_of_the_selected_type(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_TYPE_LIST", &[]);
    let cx2 = &mut VisualTestContext::from_window(handle, cx);
    // The current type is selected; Enter is 設定変更 (`IDOK`).
    cx2.simulate_keystrokes("down down enter");
    cx2.run_until_parked();
    assert!(is_open(cx), "the sheet is open");
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(TYPE_INDEX)), 2);
    let view = view_of(cx, handle);
    assert!(
        view.read_with(cx, |v, _| v.doc_type.list.is_none()),
        "the list is closed"
    );
    assert_eq!(type_of(cx, &view), 0, "the document keeps its type");
    cancel(cx);
}

#[gpui::test]
fn the_sheet_of_another_type_takes_the_place_of_the_open_one(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(TYPE_INDEX)), 0);
    cx.update(|app| open(app, 2, None, None)).unwrap();
    cx.run_until_parked();
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(TYPE_INDEX)), 2);
    cancel(cx);
}

#[gpui::test]
fn the_page_used_last_opens_again(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(2, dcx));
    cancel(cx);
    run(cx, handle, "F_OPTION_TYPE", &[]);
    assert_eq!(with_dialog(cx, |d, _, _| d.page_index()), 2);
    cancel(cx);
}

#[gpui::test]
fn a_second_command_brings_the_open_sheet_to_the_front(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_TYPE", &[]);
    let first = dialog(cx);
    run(cx, handle, "F_OPTION_TYPE", &[]);
    assert_eq!(first.entity_id(), dialog(cx).entity_id());
    cancel(cx);
}

// ---- OK and キャンセル ----

#[gpui::test]
fn ok_writes_sakuras_keys_and_the_windows_of_the_type_take_them(cx: &mut TestAppContext) {
    let ini_text =
        "[Common]\r\nbDispSTATUSBAR=1\r\nnMRUArrNum_MAX=7\r\n[Other]\r\nnTypesCount=30\r\n";
    let (ini, a) = setup(cx, ini_text);
    // Window B has the C/C++ type; window C has the basic type, like A.
    let b = open_window(cx);
    run(cx, b, "F_CHANGETYPE", &[Arg::Int(3)]);
    let c = open_window(cx);
    let (view_a, view_b, view_c) = (view_of(cx, a), view_of(cx, b), view_of(cx, c));
    assert_eq!(type_of(cx, &view_b), 2);
    let b_tab = tab_width(cx, &view_b);
    let b_wrap = wrap_columns(cx, &view_b);
    let before = ini.other_sections();

    run(cx, a, "F_OPTION_TYPE", &[]);
    type_in(cx, TAB_SPACE, "8");
    type_in(cx, MAX_LINE_KETAS, "60");
    choose(cx, WRAP_METHOD, 1);
    // The color of the TAB mark: choose the item, then the color (the picker gives a COLORREF).
    let tab_item = ColorSet::all_names().position(|n| n == "TAB").unwrap();
    choose(cx, COLOR_SEL, i32::try_from(tab_item).unwrap());
    color(cx, COLOR_TEXT, 0x0000ff);
    ok(cx);

    // The file has Sakura's keys of that type.
    let numbers = ints(&ini, 0);
    assert_eq!((numbers[0], numbers[1], numbers[3]), (0, 60, 8));
    assert_eq!(ini.get("Types(0)", "nTextWrapMethod").as_deref(), Some("1"));
    assert_eq!(
        ini.get("Types(0)", "C[TAB]").as_deref(),
        Some("1,0,0000ff,f0fbff,0"),
        "disp,bold,text,back,underline in Sakura's form"
    );
    assert_eq!(ini.get("Types(0)", "szTypeName").as_deref(), Some("基本"));
    // The other sections are as they were.
    assert_eq!(ini.other_sections(), before);
    // A key the user did not touch and the file did not set stays unset: the line ending of a
    // new document still follows the OS.
    assert_eq!(ini.get("Types(0)", "eDefaultEoltype"), None);
    assert_eq!(ini.get("Types(0)", "eDefaultCodetype"), None);
    assert_eq!(ini.get("Types(0)", "bDefaultBom"), None);

    // The windows of that type changed at once; the window of another type did not.
    assert_eq!(tab_width(cx, &view_a), 8);
    assert_eq!(wrap_columns(cx, &view_a), 60);
    assert_eq!(tab_width(cx, &view_c), 8);
    assert_eq!(tab_width(cx, &view_b), b_tab);
    assert_eq!(wrap_columns(cx, &view_b), b_wrap);
    // The application's settings are the new ones: a window made now has them.
    let d = open_window(cx);
    let view_d = view_of(cx, d);
    assert_eq!(tab_width(cx, &view_d), 8);
}

#[gpui::test]
fn the_temporary_settings_of_a_window_of_the_type_stay(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    let view = view_of(cx, a);
    // 一時設定: the wrap at the window's width.
    run(cx, a, "F_TMPWRAPWINDOW", &[]);
    run(cx, a, "F_OPTION_TYPE", &[]);
    choose(cx, WRAP_METHOD, 1);
    type_in(cx, MAX_LINE_KETAS, "40");
    ok(cx);
    // The type is the same, so the temporary wrap stays (`CEditDoc::OnChangeSetting`); see
    // `doc_types::keep_tests` for when it ends.
    let (method, columns) = view.read_with(cx, |v, _| (v.prefs.wrap_method, v.prefs.wrap_columns));
    assert_eq!(method, crate::settings::WrapMethod::WindowWidth);
    assert_ne!(columns, 0);
}

#[gpui::test]
fn cancel_writes_nothing_and_changes_no_window(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "[Common]\r\nbDispSTATUSBAR=1\r\n");
    let view = view_of(cx, a);
    let before = ini.bytes();
    let tab = tab_width(cx, &view);
    run(cx, a, "F_OPTION_TYPE", &[]);
    type_in(cx, TAB_SPACE, "8");
    check(cx, WORD_WRAP, true);
    cancel(cx);
    assert_eq!(ini.bytes(), before);
    assert_eq!(tab_width(cx, &view), tab);
}

#[gpui::test]
fn ok_without_a_change_writes_nothing(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "[Common]\r\nbDispSTATUSBAR=1\r\n");
    let before = ini.bytes();
    run(cx, a, "F_OPTION_TYPE", &[]);
    ok(cx);
    assert_eq!(ini.bytes(), before);
}

#[gpui::test]
fn a_number_out_of_range_is_put_into_its_range(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    type_in(cx, MAX_LINE_KETAS, "99999");
    type_in(cx, TAB_SPACE, "100");
    ok(cx);
    let numbers = ints(&ini, 0);
    assert_eq!((numbers[1], numbers[3]), (10240, 64));

    run(cx, a, "F_OPTION_TYPE", &[]);
    // `GetDlgItemInt`: text that is no number is 0, which the range lifts to its least.
    type_in(cx, MAX_LINE_KETAS, "abc");
    type_in(cx, TAB_SPACE, "-5");
    ok(cx);
    let numbers = ints(&ini, 0);
    assert_eq!(
        (numbers[1], numbers[3]),
        (10, 2),
        "the reader of the file keeps a TAB width from 2 to the wrap column minus 2"
    );
}

#[gpui::test]
fn the_name_and_the_extensions_of_the_basic_type_cannot_be_changed(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    type_in(cx, TYPE_NAME, "別の名前");
    type_in(cx, TYPE_EXTS, "xyz");
    assert_eq!(
        value(cx, TYPE_NAME),
        Value::Text("基本".to_owned()),
        "Types(0) is greyed out"
    );
    check(cx, WORD_WRAP, true);
    ok(cx);
    assert_eq!(ini.get("Types(0)", "szTypeName").as_deref(), Some("基本"));
}

#[gpui::test]
fn another_type_can_be_renamed_and_the_documents_follow_the_new_extensions(
    cx: &mut TestAppContext,
) {
    let (ini, a) = setup(cx, "");
    // The type "テキスト" (number 1).
    run(cx, a, "F_CHANGETYPE", &[Arg::Int(2)]);
    run(cx, a, "F_OPTION_TYPE", &[]);
    assert_eq!(value(cx, TYPE_NAME), Value::Text("テキスト".to_owned()));
    type_in(cx, TYPE_NAME, "メモ");
    ok(cx);
    assert_eq!(ini.get("Types(1)", "szTypeName").as_deref(), Some("メモ"));
    let view = view_of(cx, a);
    assert_eq!(type_of(cx, &view), 1, "the type the user chose is kept");
    // The list shows the new name.
    let (labels, _) = view.read_with(cx, |v, app| v.type_labels(app));
    assert!(labels[1].starts_with("メモ ("), "{}", labels[1]);
}

#[gpui::test]
fn a_type_of_ours_is_written_in_its_slot_with_all_its_keys(cx: &mut TestAppContext) {
    // ADR-0017: MATSU Editor's types fill the numbers the file does not use.
    let ini_text = "[Other]\r\nnTypesCount=30\r\n[Types(17)]\r\nszTypeName=設定17\r\n";
    let (ini, _a) = setup(cx, ini_text);
    let types = DocumentTypes::from_profile(&ini.profile());
    let index = types.index_of_name("JavaScript").unwrap();
    assert!(index >= 17);
    let names_before: Vec<String> = types.iter().map(|t| t.type_name.clone()).collect();

    cx.update(|app| open(app, index, None, None)).unwrap();
    cx.run_until_parked();
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int(TYPE_INDEX)),
        i32::try_from(index).unwrap()
    );
    type_in(cx, TAB_SPACE, "3");
    ok(cx);

    let section = format!("Types({index})");
    assert_eq!(
        ini.get(&section, "szTypeName").as_deref(),
        Some("JavaScript")
    );
    assert!(
        ini.get(&section, "szTypeExts")
            .is_some_and(|e| e.contains("js"))
    );
    assert_eq!(ints(&ini, index)[3], 3);
    // All the keys, also those the user did not touch.
    for key in [
        "szLineComment",
        "eDefaultCodetype",
        "eDefaultEoltype",
        "bDefaultBom",
        "bAutoIndent",
        "C[CMT]",
        "nDefaultOutline",
        "bUseRegexKeyword",
    ] {
        assert!(ini.get(&section, key).is_some(), "{key}");
    }
    // Read again, it is the same type with the new TAB width, in the same number, and the other
    // types of ours did not move.
    let types = DocumentTypes::from_profile(&ini.profile());
    let t = types.get(index).unwrap();
    assert_eq!(t.type_name, "JavaScript");
    assert_eq!(t.tab_space, 3);
    assert_eq!(t.line_comments[0].delimiter, "//");
    let names_after: Vec<String> = types.iter().map(|t| t.type_name.clone()).collect();
    assert_eq!(names_after, names_before);
}

#[gpui::test]
fn the_new_file_keys_are_written_when_the_user_changes_them(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    // The sheet shows the line ending a new document gets: the one of the OS, as the file does
    // not set it.
    let (os, other, other_label) = if cfg!(windows) {
        (1, 2, "UTF-8  LF")
    } else {
        (2, 1, "UTF-8  CRLF")
    };
    run(cx, a, "F_OPTION_TYPE", &[]);
    assert_eq!(value(cx, DEFAULT_EOL), Value::Int(os));
    choose(cx, DEFAULT_EOL, other);
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "eDefaultEoltype"),
        Some(other.to_string())
    );
    // The ones that did not change stay unset.
    assert_eq!(ini.get("Types(0)", "eDefaultCodetype"), None);
    // A new document has the line ending now.
    let b = open_window(cx);
    let view = view_of(cx, b);
    assert_eq!(
        view.read_with(cx, |v, app| v.status(app).file_format()),
        other_label
    );

    // The key is in the file now, and the line ending of the OS is a choice too.
    run(cx, a, "F_OPTION_TYPE", &[]);
    choose(cx, DEFAULT_EOL, os);
    ok(cx);
    assert_eq!(ini.get("Types(0)", "eDefaultEoltype"), Some(os.to_string()));
}

#[gpui::test]
fn a_tab_width_the_reader_would_change_is_put_into_its_range_at_once(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    let view = view_of(cx, a);
    run(cx, a, "F_OPTION_TYPE", &[]);
    type_in(cx, TAB_SPACE, "1");
    ok(cx);
    assert_eq!(ints(&ini, 0)[3], 2);
    assert_eq!(tab_width(cx, &view), 2);

    // A TAB width of 64 does not fit a wrap column of 10.
    run(cx, a, "F_OPTION_TYPE", &[]);
    type_in(cx, TAB_SPACE, "64");
    type_in(cx, MAX_LINE_KETAS, "10");
    ok(cx);
    assert_eq!(ints(&ini, 0)[3], 8);
    assert_eq!(tab_width(cx, &view), 8);
}

#[gpui::test]
fn new_file_keys_the_user_did_not_change_stay_as_the_file_has_them(cx: &mut TestAppContext) {
    // `eDefaultEoltype=0` is "no conversion": the type has the line ending of the basic type.
    // `eDefaultCodetype=936` is a code page, which the list of the sheet does not have.
    let ini_text = "[Types(0)]\r\neDefaultEoltype=2\r\n[Types(2)]\r\neDefaultEoltype=0\r\neDefaultCodetype=936\r\n";
    let (ini, a) = setup(cx, ini_text);
    run(cx, a, "F_CHANGETYPE", &[Arg::Int(3)]);
    run(cx, a, "F_OPTION_TYPE", &[]);
    type_in(cx, TAB_SPACE, "7");
    ok(cx);
    assert_eq!(ints(&ini, 2)[3], 7);
    assert_eq!(ini.get("Types(2)", "eDefaultEoltype").as_deref(), Some("0"));
    assert_eq!(
        ini.get("Types(2)", "eDefaultCodetype").as_deref(),
        Some("936")
    );
    assert_eq!(ini.get("Types(2)", "bDefaultBom"), None);
}

#[gpui::test]
fn a_bom_the_user_chose_is_written_when_the_encoding_changes_its_default(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    // UTF-16 has a BOM by default; the user does not want it.
    choose(cx, DEFAULT_CODE, 3);
    check(cx, DEFAULT_BOM, false);
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "eDefaultCodetype").as_deref(),
        Some("3")
    );
    assert_eq!(ini.get("Types(0)", "bDefaultBom").as_deref(), Some("0"));
}

#[gpui::test]
fn the_values_of_a_color_item_are_merged_one_by_one(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(1, dcx));
    let item = ColorSet::all_names().position(|n| n == "CMT").unwrap();
    choose(cx, COLOR_SEL, i32::try_from(item).unwrap());
    color(cx, COLOR_TEXT, 0x0000ff);
    // Another process makes the same item bold and underlined meanwhile.
    let mut profile = ini.profile();
    profile
        .set("Types(0)", "C[CMT]", "1,1,008000,f0fbff,1")
        .unwrap();
    profile.save(&ini.path).unwrap();
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "C[CMT]").as_deref(),
        Some("1,1,0000ff,f0fbff,1"),
        "the new text color of the user, the bold and underline of the other"
    );
}

#[gpui::test]
fn what_another_process_changed_meanwhile_stays(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "[Types(0)]\r\nbAutoIndent=1\r\nnLineSpace=3\r\n");
    run(cx, a, "F_OPTION_TYPE", &[]);
    type_in(cx, TAB_SPACE, "6");
    // Another process changes the type while the sheet is open.
    let mut profile = ini.profile();
    profile.set("Types(0)", "bAutoIndent", "0").unwrap();
    profile.set("Types(0)", "nLineSpace", "9").unwrap();
    profile.save(&ini.path).unwrap();
    ok(cx);
    assert_eq!(ints(&ini, 0)[3], 6, "the change of the user");
    assert_eq!(
        ini.get("Types(0)", "bAutoIndent").as_deref(),
        Some("0"),
        "what the sheet does not show as changed is as the file has it"
    );
    assert_eq!(ini.get("Types(0)", "nLineSpace").as_deref(), Some("9"));
}

#[gpui::test]
fn the_bom_follows_the_encoding(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    // UTF-16 has a BOM by default; Shift_JIS has none and the box is greyed out.
    choose(cx, DEFAULT_CODE, 3);
    assert_eq!(value(cx, DEFAULT_BOM), Value::Bool(true));
    choose(cx, DEFAULT_CODE, 0);
    assert_eq!(value(cx, DEFAULT_BOM), Value::Bool(false));
    check(cx, DEFAULT_BOM, true);
    assert_eq!(
        value(cx, DEFAULT_BOM),
        Value::Bool(false),
        "the box of an encoding without a BOM is greyed out"
    );
    choose(cx, DEFAULT_CODE, 4);
    check(cx, DEFAULT_BOM, true);
    ok(cx);
    // UTF-8 is what the type had, so only the BOM is new.
    assert_eq!(ini.get("Types(0)", "eDefaultCodetype"), None);
    assert_eq!(ini.get("Types(0)", "bDefaultBom").as_deref(), Some("1"));

    run(cx, a, "F_OPTION_TYPE", &[]);
    choose(cx, DEFAULT_CODE, 0);
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "eDefaultCodetype").as_deref(),
        Some("0")
    );
    assert_eq!(ini.get("Types(0)", "bDefaultBom").as_deref(), Some("0"));
}

// ---- the controls ----

#[gpui::test]
fn the_controls_an_item_does_not_have_are_greyed_out(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(1, dcx));
    let usable = |cx: &mut TestAppContext, id: &str| {
        with_dialog(cx, |d, _, _| {
            let control = d.page_controls().into_iter().find(|c| c.id == id).unwrap();
            d.is_usable(&control)
        })
    };
    // テキスト is always shown and has no bold or underline; ルーラー has no effects but may be
    // switched off; カーソル has no background.
    assert!(!usable(cx, COLOR_DISP));
    assert!(!usable(cx, COLOR_BOLD));
    assert!(!usable(cx, COLOR_UNDERLINE));
    assert!(usable(cx, COLOR_TEXT) && usable(cx, COLOR_BACK));
    choose(cx, COLOR_SEL, 2);
    assert!(usable(cx, COLOR_TEXT) && !usable(cx, COLOR_BACK));
    // カーソル位置縦線 may be bold, not underlined.
    choose(cx, COLOR_SEL, 6);
    assert!(usable(cx, COLOR_BOLD) && !usable(cx, COLOR_UNDERLINE));
    // 表示範囲 has no text color.
    choose(cx, COLOR_SEL, 56);
    assert!(!usable(cx, COLOR_TEXT) && usable(cx, COLOR_BACK));
    // The dialogs of the page can be used.
    assert!(usable(cx, KEYWORD_SELECT) && usable(cx, KEYWORD_EDIT));
    cancel(cx);
}

#[gpui::test]
fn a_change_of_the_controls_belongs_to_the_item_that_is_chosen(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(1, dcx));
    let item =
        |name: &str| i32::try_from(ColorSet::all_names().position(|n| n == name).unwrap()).unwrap();
    // コメント: switch it off, make it bold and give it a background.
    choose(cx, COLOR_SEL, item("CMT"));
    check(cx, COLOR_DISP, false);
    check(cx, COLOR_BOLD, true);
    color(cx, COLOR_BACK, 0x00ff_8000);
    // The next item shows its own values, and the first shows what was set when we come back.
    choose(cx, COLOR_SEL, item("URL"));
    assert_eq!(value(cx, COLOR_DISP), Value::Bool(true));
    assert_eq!(value(cx, COLOR_UNDERLINE), Value::Bool(true));
    choose(cx, COLOR_SEL, item("CMT"));
    assert_eq!(value(cx, COLOR_DISP), Value::Bool(false));
    assert_eq!(value(cx, COLOR_BOLD), Value::Bool(true));
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "C[CMT]").as_deref(),
        Some("0,1,008000,ff8000,0"),
        "the text color is as it was; a color is written as the COLORREF 0xBBGGRR"
    );
    // The URL item is as it was (Sakura's default).
    assert_eq!(
        ini.get("Types(0)", "C[URL]").as_deref(),
        Some("1,0,ff0000,f0fbff,1")
    );
}

#[gpui::test]
fn the_color_pickers_show_the_colors_of_the_chosen_item(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(1, dcx));
    let handle = cx.update(|app| dialog_window(app)).unwrap();
    let shown = |cx: &mut TestAppContext, id: &str| {
        let id = id.to_owned();
        with_dialog(cx, move |d, _, dcx| d.picker_color(&id, dcx))
    };
    // Draw the window, so that the pickers are made to show the values.
    VisualTestContext::from_window(handle, cx).run_until_parked();
    // テキスト: black on Sakura's cream.
    assert_eq!(shown(cx, COLOR_TEXT), Some(0x000000));
    assert_eq!(shown(cx, COLOR_BACK), Some(0xf0fbff));
    // TAB記号: grey (128, 128, 128).
    choose(cx, COLOR_SEL, 11);
    VisualTestContext::from_window(handle, cx).run_until_parked();
    assert_eq!(shown(cx, COLOR_TEXT), Some(0x808080));
    // A color from the picker (here a call, as the picker's event does) is the color of the item.
    color(cx, COLOR_TEXT, 0x123456);
    choose(cx, COLOR_SEL, 0);
    choose(cx, COLOR_SEL, 11);
    VisualTestContext::from_window(handle, cx).run_until_parked();
    assert_eq!(shown(cx, COLOR_TEXT), Some(0x123456));
    cancel(cx);
}

#[gpui::test]
fn the_rules_of_the_coloring_are_changed_on_the_color_page(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(1, dcx));
    type_in(cx, BLOCK1_FROM, "{-");
    type_in(cx, BLOCK1_TO, "-}");
    type_in(cx, LINE_COMMENT[0], "--");
    check(cx, LINE_COMMENT_POS_ON[0], true);
    type_in(cx, LINE_COMMENT_POS[0], "3");
    type_in(cx, LINE_COMMENT[1], "#");
    choose(cx, STRING_TYPE, 1);
    check(cx, STRING_LINE_ONLY, true);
    check(cx, STRING_END_LINE, true);
    type_in(cx, VERT_LINES, "80,5(10,50)");
    ok(cx);
    let section = "Types(0)";
    assert_eq!(
        ini.get(section, "szBlockCommentFrom").as_deref(),
        Some("{-")
    );
    assert_eq!(ini.get(section, "szBlockCommentTo").as_deref(), Some("-}"));
    assert_eq!(ini.get(section, "szLineComment").as_deref(), Some("--"));
    // The column is counted from 1 on the page and from 0 in the file.
    assert_eq!(ini.get(section, "nLineCommentColumn").as_deref(), Some("2"));
    assert_eq!(ini.get(section, "szLineComment2").as_deref(), Some("#"));
    assert_eq!(
        ini.get(section, "nLineCommentColumn2").as_deref(),
        Some("-1")
    );
    assert_eq!(ini.get(section, "bStringLineOnly").as_deref(), Some("1"));
    assert_eq!(ini.get(section, "bStringEndLine").as_deref(), Some("1"));
    assert_eq!(ints(&ini, 0)[6], 1, "the escape of strings is in nInts");
    assert_eq!(ini.get(section, "nVertLineIdx1").as_deref(), Some("80"));
    assert_eq!(ini.get(section, "nVertLineIdx2").as_deref(), Some("-10"));
    assert_eq!(ini.get(section, "nVertLineIdx3").as_deref(), Some("50"));
    assert_eq!(ini.get(section, "nVertLineIdx4").as_deref(), Some("5"));
    let view = view_of(cx, a);
    assert_eq!(
        view.read_with(cx, |v, _| v.view_settings().vert_lines.clone()),
        [80, -10, 50, 5]
    );
}

#[gpui::test]
fn a_control_that_is_not_available_is_never_written(cx: &mut TestAppContext) {
    // The file has an IME state and a background image for the type; the page shows them
    // greyed out, and OK leaves them as they are.
    let ini_text =
        "[Types(0)]\r\nnImeState=9\r\nbgImgPath=C:\\a.png\r\nbgImgOpacity=100\r\nnLineSpace=7\r\n";
    let (ini, a) = setup(cx, ini_text);
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(2, dcx));
    assert_eq!(value(cx, IME_SWITCH), Value::Int(1));
    assert_eq!(value(cx, IME_STATE), Value::Int(2));
    assert_eq!(value(cx, BG_PATH), Value::Text("C:\\a.png".to_owned()));
    assert_eq!(value(cx, BG_TRANSPARENCY), Value::Int(155));
    // Choosing it does nothing.
    choose(cx, IME_SWITCH, 2);
    assert_eq!(value(cx, IME_SWITCH), Value::Int(1));
    type_in(cx, BG_PATH, "other");
    check(cx, WORD_WRAP, true);
    with_dialog(cx, |d, _, dcx| d.select_index(0, dcx));
    check(cx, WORD_WRAP, true);
    ok(cx);
    assert_eq!(ini.get("Types(0)", "nImeState").as_deref(), Some("9"));
    assert_eq!(
        ini.get("Types(0)", "bgImgPath").as_deref(),
        Some("C:\\a.png")
    );
    assert_eq!(ini.get("Types(0)", "nLineSpace").as_deref(), Some("7"));
    assert_eq!(ints(&ini, 0)[9], 1, "the English word wrap is in nInts");
}

#[gpui::test]
fn every_page_of_the_sheet_has_its_controls(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    for page in 0..PAGE_TITLES.len() {
        with_dialog(cx, |d, _, dcx| d.select_index(page, dcx));
        assert_eq!(with_dialog(cx, |d, _, _| d.page_index()), page);
        assert!(!with_dialog(cx, |d, _, _| d.page_controls().is_empty()));
    }
    cancel(cx);
}

#[gpui::test]
fn the_sheet_is_not_the_common_settings(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    // The common settings do not see the sheet, and the sheet's page does not become theirs.
    assert!(!cx.update(|app| crate::common_settings::current_dialog(app).is_some()));
    cancel(cx);
    run(cx, a, "F_OPTION", &[]);
    assert!(cx.update(|app| crate::common_settings::current_dialog(app).is_some()));
    assert_eq!(
        with_dialog_common(cx, |d| d.page()),
        crate::common_settings::Page::General
    );
}

fn with_dialog_common<R>(cx: &mut TestAppContext, f: impl FnOnce(&CommonSettingsDialog) -> R) -> R {
    let dialog = cx
        .update(|app| crate::common_settings::current_dialog(app))
        .unwrap();
    dialog.read_with(cx, |d, _| f(d))
}

#[test]
fn a_type_that_is_not_there_has_no_sheet() {
    assert!(sheet(&Profile::new(), 9999).is_none());
    let t: TypeSettings = TypeSettings::basic();
    assert_eq!(t.tab_space, 4);
}

mod colors;
mod lists;
mod moved;
mod screen_layout;
mod transfer;

// ---- 支援 ----

/// Whether the control of the page the sheet shows can be used now.
fn usable(cx: &mut TestAppContext, id: &str) -> bool {
    with_dialog(cx, |d, _, _| {
        let control = d.page_controls().into_iter().find(|c| c.id == id).unwrap();
        d.is_usable(&control)
    })
}

#[gpui::test]
fn the_rule_file_of_the_outline_can_be_chosen_and_is_written(cx: &mut TestAppContext) {
    // `IDC_RADIO_OUTLINERULEFILE` and `IDC_EDIT_OUTLINERULEFILE` (`CPropTypesScreen`)
    let (ini, a) = setup(
        cx,
        "[Types(0)]\r\nnDefaultOutline=12\r\nszOutlineRuleFilename=rules.txt\r\n",
    );
    run(cx, a, "F_OPTION_TYPE", &[]);
    assert_eq!(value(cx, OUTLINE_MODE), Value::Int(1));
    assert_eq!(
        value(cx, OUTLINE_RULE_FILE),
        Value::Text("rules.txt".to_owned())
    );
    // the file and its button can be used when the rule file is chosen, the standard rule not
    assert!(usable(cx, OUTLINE_RULE_FILE));
    assert!(usable(cx, OUTLINE_RULE_BROWSE));
    assert!(!usable(cx, OUTLINE));
    with_dialog(cx, |d, _, dcx| d.select_index(0, dcx));
    type_in(cx, OUTLINE_RULE_FILE, "outline/memo.rul");
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "szOutlineRuleFilename").as_deref(),
        Some("outline/memo.rul")
    );
    assert_eq!(
        ini.get("Types(0)", "nDefaultOutline").as_deref(),
        Some("12")
    );
}

#[gpui::test]
fn the_rule_file_is_kept_while_the_standard_rule_is_chosen(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "[Types(0)]\r\nszOutlineRuleFilename=keep.rul\r\n");
    run(cx, a, "F_OPTION_TYPE", &[]);
    assert_eq!(value(cx, OUTLINE_MODE), Value::Int(0));
    assert!(usable(cx, OUTLINE));
    assert!(!usable(cx, OUTLINE_RULE_FILE));
    assert!(!usable(cx, OUTLINE_RULE_BROWSE));
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "szOutlineRuleFilename").as_deref(),
        Some("keep.rul")
    );
}

#[gpui::test]
fn the_support_page_shows_sakuras_defaults_and_what_the_file_says(cx: &mut TestAppContext) {
    let (_ini, a) = setup(
        cx,
        "[Types(0)]\r\nszHokanFile=words.kwd\r\nbHokanLoHiCase=1\r\nbUseHokanByFile=0\r\n\
         szExtHelp=ext.chm\r\nbTypeHtmlHelpIsSingle=0\r\nnHokanType=5\r\n",
    );
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(3, dcx));
    assert_eq!(value(cx, HOKAN_FILE), Value::Text("words.kwd".to_owned()));
    assert_eq!(value(cx, HOKAN_LO_HI_CASE), Value::Bool(true));
    assert_eq!(value(cx, HOKAN_BY_FILE), Value::Bool(false));
    assert_eq!(value(cx, HOKAN_BY_KEYWORD), Value::Bool(false));
    // A plugin the list does not have shows as なし (`SetData` selects the first).
    assert_eq!(value(cx, HOKAN_TYPE), Value::Int(0));
    assert_eq!(value(cx, EXT_HELP), Value::Text("ext.chm".to_owned()));
    assert_eq!(value(cx, HTML_HELP_SINGLE), Value::Bool(false));
    // What the file does not say is Sakura's default (`CType.cpp`).
    assert_eq!(value(cx, CHK_ENTER_AT_END), Value::Bool(true));
    assert_eq!(value(cx, INDENT_CPP_STRING), Value::Bool(true));
    assert_eq!(value(cx, INDENT_CPP_UNDO), Value::Bool(false));
    assert_eq!(value(cx, DETECT_INDENT), Value::Bool(true));
    cancel(cx);
}

#[gpui::test]
fn the_completion_and_help_file_settings_are_live_and_the_rest_is_greyed_out(
    cx: &mut TestAppContext,
) {
    let (_ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(3, dcx));
    for id in [
        HOKAN_FILE,
        HOKAN_FILE_BROWSE,
        HOKAN_LO_HI_CASE,
        HOKAN_BY_FILE,
        HOKAN_BY_KEYWORD,
        EXT_HELP,
        EXT_HELP_BROWSE,
        EXT_HTML_HELP,
        EXT_HTML_HELP_BROWSE,
        HTML_HELP_SINGLE,
    ] {
        assert!(usable(cx, id), "{id}");
    }
    for id in [
        HOKAN_TYPE,
        CHK_ENTER_AT_END,
        INDENT_CPP_STRING,
        INDENT_CPP_COMMENT,
        INDENT_CPP_UNDO,
        DETECT_INDENT,
    ] {
        assert!(!usable(cx, id), "{id}");
    }
    cancel(cx);
}

#[gpui::test]
fn ok_writes_the_external_help_files_of_the_type(cx: &mut TestAppContext) {
    // `CPropTypesSupport::GetData`
    let (ini, a) = setup(cx, "[Types(0)]\r\nszExtHelp=old.chm\r\n");
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(3, dcx));
    assert_eq!(value(cx, EXT_HELP), Value::Text("old.chm".to_owned()));
    assert_eq!(value(cx, HTML_HELP_SINGLE), Value::Bool(true));
    type_in(cx, EXT_HELP, "docs/new.hlp");
    type_in(cx, EXT_HTML_HELP, "html.chm");
    check(cx, HTML_HELP_SINGLE, false);
    ok(cx);
    let get = |key: &str| ini.get("Types(0)", key).unwrap_or_default();
    assert_eq!(get("szExtHelp"), "docs/new.hlp");
    assert_eq!(get("szExtHtmlHelp"), "html.chm");
    assert_eq!(get("bTypeHtmlHelpIsSingle"), "0");
}

#[gpui::test]
fn ok_writes_the_completion_settings_and_keeps_what_the_page_cannot_change(
    cx: &mut TestAppContext,
) {
    let (ini, a) = setup(
        cx,
        "[Types(0)]\r\nszExtHelp=keep.chm\r\nbTypeHtmlHelpIsSingle=0\r\nbIndentCppUndoSep=1\r\n",
    );
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(3, dcx));
    type_in(cx, HOKAN_FILE, "words.kwd");
    check(cx, HOKAN_LO_HI_CASE, true);
    check(cx, HOKAN_BY_FILE, false);
    check(cx, HOKAN_BY_KEYWORD, true);
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "szHokanFile").as_deref(),
        Some("words.kwd")
    );
    assert_eq!(ini.get("Types(0)", "bHokanLoHiCase").as_deref(), Some("1"));
    assert_eq!(ini.get("Types(0)", "bUseHokanByFile").as_deref(), Some("0"));
    assert_eq!(
        ini.get("Types(0)", "bUseHokanByKeyword").as_deref(),
        Some("1")
    );
    // What the page cannot change is as the file had it.
    assert_eq!(
        ini.get("Types(0)", "szExtHelp").as_deref(),
        Some("keep.chm")
    );
    assert_eq!(
        ini.get("Types(0)", "bTypeHtmlHelpIsSingle").as_deref(),
        Some("0")
    );
    assert_eq!(
        ini.get("Types(0)", "bIndentCppUndoSep").as_deref(),
        Some("1")
    );
    // The application's settings are the new ones.
    let (lo_hi, by_file, by_keyword) = cx.update(|app| {
        crate::doc_types::with_user(app, |user| {
            let t = user.document_types().get(0).unwrap();
            (
                t.hokan_lo_hi_case,
                t.use_hokan_by_file,
                t.use_hokan_by_keyword,
            )
        })
    });
    assert!(lo_hi && !by_file && by_keyword);
}

#[gpui::test]
fn the_completion_settings_stay_when_the_page_is_not_touched(cx: &mut TestAppContext) {
    let (ini, a) = setup(
        cx,
        "[Types(0)]\r\nszHokanFile=w.kwd\r\nbHokanLoHiCase=1\r\nnInts=0,80,0,4,-1,-1,0,1,1,0,0,0\r\n",
    );
    run(cx, a, "F_OPTION_TYPE", &[]);
    // Another page changes; the completion settings are written back as they were.
    type_in(cx, TAB_SPACE, "8");
    ok(cx);
    assert_eq!(ini.get("Types(0)", "szHokanFile").as_deref(), Some("w.kwd"));
    assert_eq!(ini.get("Types(0)", "bHokanLoHiCase").as_deref(), Some("1"));
    assert_eq!(ini.get("Types(0)", "bUseHokanByFile").as_deref(), Some("1"));
}

#[test]
fn the_word_file_the_dialog_gives_is_relative_to_the_folder_of_the_settings() {
    use super::pages::support::path_for_box;
    use std::path::Path;
    let dir = Path::new("/home/u/.config/matsu");
    assert_eq!(
        path_for_box(Path::new("/home/u/.config/matsu/words/a.kwd"), Some(dir)),
        Path::new("words/a.kwd").to_string_lossy()
    );
    // a file elsewhere keeps its whole path, and so does any file when there is no folder
    assert_eq!(
        path_for_box(Path::new("/tmp/a.kwd"), Some(dir)),
        Path::new("/tmp/a.kwd").to_string_lossy()
    );
    assert_eq!(
        path_for_box(Path::new("/tmp/a.kwd"), None),
        Path::new("/tmp/a.kwd").to_string_lossy()
    );
}

#[gpui::test]
fn a_type_page_taller_than_the_window_has_a_scroll_bar(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_TYPE", &[]);
    let dialog_window = cx.update(|app| dialog_window(app)).unwrap();
    VisualTestContext::from_window(dialog_window, cx)
        .simulate_resize(gpui::size(gpui::px(800.), gpui::px(200.)));
    cx.run_until_parked();
    assert!(
        VisualTestContext::from_window(dialog_window, cx)
            .debug_bounds("cs-scrollbar")
            .is_some(),
        "the scroll bar of the page body"
    );
    let overflow = with_dialog(cx, |d, _, _| d.body_scroll.max_offset().y);
    assert!(overflow > gpui::px(0.), "the page overflows the window");
    cancel(cx);
}

mod dialogs;
mod keyword_numbers;
