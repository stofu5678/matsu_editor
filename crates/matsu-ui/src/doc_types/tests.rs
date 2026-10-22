//! The document type of the view: chosen by the file name when a file is opened or saved under
//! another name, and changed by `F_CHANGETYPE` and the list of `F_TYPE_LIST`. The window is
//! GPUI's headless test window.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::Profile;

use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window};
use crate::settings::UserSettings;
use crate::view::EditorView;

fn temp_dir() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("matsu-ui-doctype-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A settings file (UTF-8 with a BOM, as Sakura writes it).
pub(super) fn user(text: &str) -> UserSettings {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    UserSettings::from_profile(Profile::from_bytes(&bytes), None)
}

/// Opens an untitled window, with `settings` read at startup if given.
pub(super) fn window(cx: &mut TestAppContext, settings: Option<UserSettings>) -> AnyWindowHandle {
    cx.update(gpui_component::init);
    if let Some(settings) = settings {
        cx.update(|cx| cx.set_global(settings));
    }
    cx.update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap()
}

pub(super) fn view_of(cx: &mut VisualTestContext) -> Entity<EditorView> {
    cx.update(|_, app| editors(app).last().unwrap().1.clone())
}

pub(super) fn run(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
    function: &str,
    args: &[Arg],
) {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, args, window, cx)
    });
    cx.run_until_parked();
}

pub(super) fn type_index(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> usize {
    view.read_with(cx, |v, _| v.view_settings().type_index)
}

pub(super) fn tab_width(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> u32 {
    view.read_with(cx, |v, _| v.view_settings().tab_width)
}

pub(super) fn locked(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> bool {
    view.read_with(cx, |v, cx| v.type_is_locked(cx))
}

/// Saves the untitled document under `path` (the Save As options come after the path).
fn save_as(view: &Entity<EditorView>, cx: &mut VisualTestContext, path: &std::path::Path) {
    run(view, cx, "F_FILESAVE", &[]);
    let target = path.to_owned();
    cx.simulate_new_path_selection(move |_| Some(target));
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
}

/// Types 1 and 2 with their own tab widths, over the built-in types.
pub(super) const TABS: &str = "[Other]\r\nnTypesCount=17\r\n\
    [Types(1)]\r\nnInts=1,120,0,8,-1,-1,0,1,1,0,0,0\r\n\
    [Types(2)]\r\nnInts=2,10240,0,2,0,-1,0,1,1,0,0,0\r\n";

#[gpui::test]
fn a_file_takes_the_builtin_type_of_its_extension(cx: &mut TestAppContext) {
    let dir = temp_dir();
    // Without a settings file the types are Sakura's built-in ones.
    for (name, expected) in [
        ("a.txt", 1),
        ("A.CPP", 2),
        ("a.py", 13),
        ("a.xyz", 0),
        ("Makefile", 0),
    ] {
        // A window that has a file opens the next one in a new window: one window for each.
        let handle = window(cx, None);
        let cx = &mut VisualTestContext::from_window(handle, cx);
        let view = view_of(cx);
        assert_eq!(type_index(&view, cx), 0);
        let path = dir.join(name);
        std::fs::write(&path, b"x").unwrap();
        run(
            &view,
            cx,
            "F_FILEOPEN",
            &[Arg::Text(path.display().to_string())],
        );
        assert_eq!(type_index(&view, cx), expected, "{name}");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn saving_under_a_new_name_changes_the_type_and_the_tab_width(cx: &mut TestAppContext) {
    let dir = temp_dir();
    let handle = window(cx, Some(user(TABS)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    assert_eq!((type_index(&view, cx), tab_width(&view, cx)), (0, 4));
    cx.simulate_input("a");
    save_as(&view, cx, &dir.join("new.txt"));
    // テキスト: tab width 8 in this settings file.
    assert_eq!((type_index(&view, cx), tab_width(&view, cx)), (1, 8));
    assert!(!locked(&view, cx));
    // Saving again under the same name does not apply the settings again: the zoom stays.
    run(&view, cx, "F_SETFONTSIZEUP", &[Arg::Int(1)]);
    let px = view.read_with(cx, |v, _| v.font_px());
    run(&view, cx, "F_FILESAVE", &[]);
    assert_eq!(view.read_with(cx, |v, _| v.font_px()), px);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn the_type_chosen_by_the_user_stays_when_the_name_changes(cx: &mut TestAppContext) {
    let dir = temp_dir();
    let handle = window(cx, Some(user(TABS)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    // ChangeType(3) is the third type, C/C++ (the argument is the index plus one).
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(3)]);
    assert_eq!((type_index(&view, cx), tab_width(&view, cx)), (2, 2));
    assert!(locked(&view, cx));
    cx.simulate_input("a");
    save_as(&view, cx, &dir.join("new.txt"));
    assert_eq!(type_index(&view, cx), 2, "locked: .txt does not change it");
    // Loading the file again ends the lock (`SetDocumentType` with `force`): .txt is type 1.
    run(&view, cx, "F_FILE_REOPEN", &[]);
    assert_eq!(type_index(&view, cx), 1);
    assert!(!locked(&view, cx));
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn change_type_takes_a_number_zero_or_a_name(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user(TABS)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    // 0 keeps the type and locks it.
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(0)]);
    assert_eq!(type_index(&view, cx), 0);
    assert!(locked(&view, cx));
    // The name of the type (an MATSU Editor addition).
    run(&view, cx, "F_CHANGETYPE", &[Arg::Text("Python".into())]);
    assert_eq!(type_index(&view, cx), 13);
    // No such type, or such a number: nothing happens.
    run(&view, cx, "F_CHANGETYPE", &[Arg::Text("Nope".into())]);
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(99)]);
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(-5)]);
    assert_eq!(type_index(&view, cx), 13);
    // Without an argument: 0.
    run(&view, cx, "F_CHANGETYPE", &[]);
    assert_eq!(type_index(&view, cx), 13);
    // The first type is 1.
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(1)]);
    assert_eq!(type_index(&view, cx), 0);
}

#[gpui::test]
fn a_change_of_type_drops_the_temporary_settings(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user(TABS)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(&view, cx, "F_SETFONTSIZEUP", &[Arg::Int(1)]);
    let zoomed = view.read_with(cx, |v, _| v.font_px());
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(2)]);
    assert!(view.read_with(cx, |v, _| v.font_px()) < zoomed);
}

#[gpui::test]
fn an_empty_untitled_document_takes_the_line_ending_of_the_type(cx: &mut TestAppContext) {
    // CR is not the line ending of any OS default.
    let ini = "[Other]\r\nnTypesCount=17\r\n[Types(2)]\r\neDefaultEoltype=3\r\n";
    let handle = window(cx, Some(user(ini)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(3)]);
    assert_eq!(
        view.read_with(cx, |v, app| v.status(app).file_format()),
        "UTF-8  CR"
    );
    // A document with text keeps its line ending when the type changes.
    cx.simulate_input("x");
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(1)]);
    assert_eq!(
        view.read_with(cx, |v, app| v.status(app).file_format()),
        "UTF-8  CR"
    );
}

#[gpui::test]
fn the_list_applies_the_selected_type_with_the_button_and_a_double_click(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user(TABS)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(&view, cx, "F_TYPE_LIST", &[]);
    assert!(view.read_with(cx, |v, _| v.doc_type.list.is_some()));
    // The current type (0) is selected; the list has a row for each of the types.
    let (labels, current) = view.read_with(cx, |v, app| v.type_labels(app));
    assert_eq!(labels.len(), 17);
    assert_eq!(current, 0);
    assert_eq!(
        labels[2],
        "C/C++ ( c,cpp,cxx,cc,cp,c++,h,hpp,hxx,hh,hp,h++,rc,hm )"
    );
    cx.simulate_keystrokes("down down");
    // 一時適用 (the double click does the same).
    let list = view.read_with(cx, |v, _| v.doc_type.list.clone().unwrap());
    list.update_in(cx, |list, window, cx| list.apply(window, cx));
    cx.run_until_parked();
    assert!(view.read_with(cx, |v, _| v.doc_type.list.is_none()));
    assert_eq!((type_index(&view, cx), tab_width(&view, cx)), (2, 2));
    assert!(locked(&view, cx));
}

#[gpui::test]
fn escape_closes_the_list_without_a_change(cx: &mut TestAppContext) {
    let handle = window(cx, None);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(&view, cx, "F_TYPE_LIST", &[]);
    cx.simulate_keystrokes("down escape");
    cx.run_until_parked();
    assert!(view.read_with(cx, |v, _| v.doc_type.list.is_none()));
    assert_eq!(type_index(&view, cx), 0);
    assert!(!locked(&view, cx));
    // The list opens again, and the same command closes it.
    run(&view, cx, "F_TYPE_LIST", &[]);
    run(&view, cx, "F_TYPE_LIST", &[]);
    assert!(view.read_with(cx, |v, _| v.doc_type.list.is_none()));
}
