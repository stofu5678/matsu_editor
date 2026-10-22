//! Tab and Shift+Tab never leave the in-window dialogs that rely on `keep_tab_inside`
//! (`crate::dialog_focus`). The window of a headless test has no tab stop outside the dialog,
//! so these tests guard the containment (and the focus a dialog starts with), while
//! `dialog_focus::tests` shows with a tab stop outside that the mechanism holds.

use gpui::{AnyWindowHandle, Entity, FocusHandle, Focusable, TestAppContext, VisualTestContext};
use ume_commands::sakura::FUNCTIONS;

use crate::dialog_focus::assert_tab_stays_inside;
use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window};
use crate::view::EditorView;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

fn window(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<EditorView>) {
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    let view = cx.update(|app| {
        editors(app)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, view)| view)
            .unwrap()
    });
    (handle, view)
}

fn focus_of<T: Focusable>(entity: &Option<Entity<T>>, cx: &gpui::App) -> Option<FocusHandle> {
    entity.as_ref().map(|e| e.read(cx).focus_handle(cx))
}

#[gpui::test]
fn tab_stays_in_the_about_dialog(cx: &mut TestAppContext) {
    run(cx, "F_ABOUT", |v, app| focus_of(&v.support.about, app));
}

#[gpui::test]
fn tab_stays_in_the_control_code_dialog(cx: &mut TestAppContext) {
    run(cx, "F_CTRL_CODE_DIALOG", |v, app| {
        focus_of(&v.support.ctrl_code, app)
    });
}

#[gpui::test]
fn tab_stays_in_the_command_list(cx: &mut TestAppContext) {
    run(cx, "F_MENU_ALLFUNC", |v, app| {
        focus_of(&v.support.command_list, app)
    });
}

#[gpui::test]
fn tab_stays_in_the_tags_file_dialog(cx: &mut TestAppContext) {
    run(cx, "F_TAGS_MAKE", |v, app| {
        focus_of(&v.support.tags_make, app)
    });
}

#[gpui::test]
fn tab_stays_in_the_type_list(cx: &mut TestAppContext) {
    run(cx, "F_TYPE_LIST", |v, app| focus_of(&v.doc_type.list, app));
}

#[gpui::test]
fn tab_stays_in_the_window_list(cx: &mut TestAppContext) {
    run(cx, "F_WINLIST", |v, app| focus_of(&v.window_list, app));
}

#[gpui::test]
fn tab_stays_in_the_compare_dialog(cx: &mut TestAppContext) {
    run(cx, "F_COMPARE", |v, app| focus_of(&v.diff.compare, app));
}

#[gpui::test]
fn tab_stays_in_the_print_setting_dialog(cx: &mut TestAppContext) {
    run(cx, "F_PRINT_PAGESETUP", |v, app| {
        focus_of(&v.print.setup_dialog, app)
    });
}

#[gpui::test]
fn tab_stays_in_the_find_dialog(cx: &mut TestAppContext) {
    run(cx, "F_SEARCH_DIALOG", |v, app| {
        v.panel.as_ref().map(|p| p.read(app).scope.clone())
    });
}

#[gpui::test]
fn tab_stays_in_the_replace_dialog(cx: &mut TestAppContext) {
    run(cx, "F_REPLACE_DIALOG", |v, app| {
        v.panel.as_ref().map(|p| p.read(app).scope.clone())
    });
}

#[gpui::test]
fn tab_stays_in_the_grep_dialog(cx: &mut TestAppContext) {
    run(cx, "F_GREP_DIALOG", |v, app| {
        v.grep.panel.as_ref().map(|p| p.read(app).scope.clone())
    });
}

#[gpui::test]
fn tab_stays_in_the_diff_dialog(cx: &mut TestAppContext) {
    run(cx, "F_DIFF_DIALOG", |v, app| {
        v.diff.panel.as_ref().map(|p| p.read(app).scope.clone())
    });
}

/// Opens the dialog with the function and checks that Tab stays in the dialog `scope` finds.
fn run(
    cx: &mut TestAppContext,
    function: &str,
    scope: impl FnOnce(&EditorView, &gpui::App) -> Option<FocusHandle>,
) {
    let (handle, view) = window(cx);
    let code = code_of(function);
    let keys = VisualTestContext::from_window(handle, cx).into_mut();
    view.update_in(keys, |v, window, cx| v.run_function(code, &[], window, cx));
    keys.run_until_parked();
    let focus = view
        .read_with(keys, |v, app| scope(v, app))
        .unwrap_or_else(|| panic!("{function} opens its dialog"));
    assert_tab_stays_inside(keys, &focus);
}
