//! Tests of the wait of a macro in tab mode: the macro belongs to the tab where it started
//! (Sakura: its `CEditView`), wherever the tab is shown.

use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;

use super::{F_EXECEXTMACRO, write};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window, workspace_of};
use crate::tabs::settings::TabSettings;
use crate::view::EditorView;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// A window in tab mode (the bar shown, grouped) with one untitled document.
fn window(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(gpui_component::init);
    cx.update(|cx| {
        cx.set_global(TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        })
    });
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    handle
}

fn workspace(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<Workspace> {
    cx.update(|app| workspace_of(app, handle)).unwrap()
}

fn active_view(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.editor().clone())
}

fn tab_count(cx: &mut TestAppContext, handle: AnyWindowHandle) -> usize {
    workspace(cx, handle).read_with(cx, |ws, _| ws.tab_count())
}

/// Runs a function on the active tab of the window, the way a key does.
fn run(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str, args: &[Arg]) {
    let code = code_of(function);
    let view = active_view(cx, handle);
    run_on(cx, handle, &view, code, args);
}

fn run_on(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    view: &Entity<EditorView>,
    code: u16,
    args: &[Arg],
) {
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, args, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn step(cx: &mut TestAppContext) {
    for _ in 0..4 {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
    }
}

fn text(cx: &mut TestAppContext, view: &Entity<EditorView>) -> String {
    view.read_with(cx, |v, app| {
        v.document()
            .read(app)
            .doc()
            .line(0)
            .ok()
            .flatten()
            .map(|l| l.text)
            .unwrap_or_default()
    })
}

fn start_macro(cx: &mut TestAppContext, handle: AnyWindowHandle, mac: &str) -> Entity<EditorView> {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "w.mac", mac);
    let view = active_view(cx, handle);
    run_on(
        cx,
        handle,
        &view,
        F_EXECEXTMACRO,
        &[Arg::Text(path.to_string_lossy().into_owned())],
    );
    // the macro file is read when it starts, so the folder may go
    drop(dir);
    view
}

fn waiting(cx: &mut TestAppContext, view: &Entity<EditorView>) -> bool {
    view.read_with(cx, |v, _| v.macros.waiting.is_some())
}

fn close_type_list(cx: &mut TestAppContext, view: &Entity<EditorView>) {
    view.update(cx, |v, cx| {
        v.doc_type.list = None;
        cx.notify();
    });
}

fn type_list_open(cx: &mut TestAppContext, view: &Entity<EditorView>) -> bool {
    view.read_with(cx, |v, _| v.doc_type.list.is_some())
}

#[gpui::test]
fn a_key_macro_waits_for_a_dialog_in_tab_mode(cx: &mut TestAppContext) {
    let handle = window(cx);
    run(cx, handle, "F_FILENEW", &[]);
    assert_eq!(tab_count(cx, handle), 2);
    let view = start_macro(cx, handle, "TypeList();\nChar(66);\n");
    assert!(type_list_open(cx, &view) && waiting(cx, &view));
    step(cx);
    assert_eq!(text(cx, &view), "");
    close_type_list(cx, &view);
    step(cx);
    assert_eq!(text(cx, &view), "B");
    assert!(!waiting(cx, &view));
}

#[gpui::test]
fn a_file_loaded_into_a_new_tab_is_waited_for(cx: &mut TestAppContext) {
    let handle = window(cx);
    let dir = tempfile::tempdir().unwrap();
    let opened = write(dir.path(), "opened.txt", "from file");
    let mac = format!(
        "Char(65);\nFileOpen('{}');\nChar(66);\n",
        opened.to_string_lossy().replace('\\', "\\\\")
    );
    let view = start_macro(cx, handle, &mac);
    step(cx);
    // the first call modified this tab, so the file is a new tab; the call after FileOpen
    // still goes to the tab where the macro started
    assert_eq!(text(cx, &view), "AB");
    assert_eq!(tab_count(cx, handle), 2);
    let loaded = cx.update(|app| {
        let all = editors(app);
        let (_, other) = all.iter().find(|(_, v)| *v != view).unwrap().clone();
        other.read(app).document().read(app).doc().line(0)
    });
    assert_eq!(loaded.unwrap().unwrap().text, "from file");
    assert!(!waiting(cx, &view));
}

#[gpui::test]
fn the_wait_follows_the_tab_to_another_window(cx: &mut TestAppContext) {
    let first = window(cx);
    run(cx, first, "F_FILENEW_NEWWINDOW", &[]);
    let second = cx.windows()[1];
    let view = start_macro(cx, first, "TypeList();\nChar(66);\n");
    assert!(waiting(cx, &view));
    // The tab leaves the window where the macro started, and that window closes.
    run_on(cx, first, &view, code_of("F_TAB_JOINTNEXT"), &[]);
    assert_eq!(cx.windows().len(), 1, "the first window is gone");
    assert_eq!(tab_count(cx, second), 2);
    assert!(waiting(cx, &view), "still waiting for the dialog");
    close_type_list(cx, &view);
    step(cx);
    assert_eq!(text(cx, &view), "B");
    assert!(!waiting(cx, &view));
}

#[gpui::test]
fn closing_the_tab_ends_the_wait(cx: &mut TestAppContext) {
    let handle = window(cx);
    run(cx, handle, "F_FILENEW", &[]);
    let view = start_macro(cx, handle, "TypeList();\nChar(66);\n");
    assert!(waiting(cx, &view));
    run_on(cx, handle, &view, code_of("F_WINCLOSE"), &[]);
    assert_eq!(tab_count(cx, handle), 1);
    drop(view);
    // nothing is left to run, and the remaining tab takes the next macro
    step(cx);
    let view = start_macro(cx, handle, "Char(67);\n");
    step(cx);
    assert_eq!(text(cx, &view), "C");
}
