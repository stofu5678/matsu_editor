//! The toolbar row on the tabs that are not editors (a preview, a shell, the AI log): it has the height of
//! the toolbar, so the top of the window does not move between kinds of tab, and its button closes the tab.

use gpui::{AnyWindowHandle, Bounds, Pixels, TestAppContext};

use super::terminal_tests::{open as open_shell, staying, terminal_window, wait_for_text};
use super::tests::{answer, run, tab_count, visual, window, workspace};
use crate::ai_log;

fn bar(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<Bounds<Pixels>> {
    visual(cx, handle).debug_bounds("tool-bar")
}

fn close_button(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Bounds<Pixels> {
    let number = crate::bars::toolbar::own_tab_close_button().number;
    let selector: &'static str = Box::leak(format!("tool-button-{number}").into_boxed_str());
    visual(cx, handle)
        .debug_bounds(selector)
        .expect("the close button is drawn")
}

fn click_close(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    let at = close_button(cx, handle).center();
    let vcx = visual(cx, handle);
    vcx.simulate_click(at, gpui::Modifiers::none());
    vcx.run_until_parked();
}

fn open_log(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ai_log::open(ws, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn the_row_has_the_height_and_place_of_the_toolbar_on_every_kind_of_tab(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let editor = bar(cx, handle).expect("the toolbar of the document");

    run(cx, handle, "F_PREVIEW_TAB");
    let preview = bar(cx, handle).expect("the row on the preview");
    assert_eq!(preview, editor);

    let _shell = open_shell(cx, handle, staying());
    let shell = bar(cx, handle).expect("the row on the shell");
    assert_eq!(shell, editor);

    open_log(cx, handle);
    let log = bar(cx, handle).expect("the row on the AI log");
    assert_eq!(log, editor);
}

#[gpui::test]
fn the_row_follows_the_toggle_of_the_toolbar(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    run(cx, handle, "F_PREVIEW_TAB");
    assert!(bar(cx, handle).is_some());
    cx.update(|cx| crate::bars::update(cx, |s| s.toolbar = false));
    cx.run_until_parked();
    assert!(bar(cx, handle).is_none());
}

#[gpui::test]
fn the_button_closes_a_preview(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    run(cx, handle, "F_PREVIEW_TAB");
    assert_eq!(tab_count(cx, handle), 2);
    click_close(cx, handle);
    assert_eq!(tab_count(cx, handle), 1);
    let ws = workspace(cx, handle);
    assert!(ws.read_with(cx, |ws, _| {
        ws.tab_content(ws.active_tab()).unwrap().is_editor()
    }));
}

#[gpui::test]
fn the_button_closes_the_ai_log(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open_log(cx, handle);
    assert_eq!(tab_count(cx, handle), 2);
    // The log opens behind the document: go to it first.
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.activate_tab(1, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    click_close(cx, handle);
    assert_eq!(tab_count(cx, handle), 1);
}

#[gpui::test]
fn the_button_closes_a_shell_after_the_question(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let shell = open_shell(cx, handle, staying());
    // Closing a ConPTY before its client has started can leave the client running (Windows).
    wait_for_text(cx, &shell, "hello");
    assert_eq!(tab_count(cx, handle), 2);
    click_close(cx, handle);
    // A running shell is asked about first, as with the close of the tab.
    answer(cx, handle, "はい(Y)");
    assert_eq!(tab_count(cx, handle), 1);
}

#[gpui::test]
fn a_toolbar_without_buttons_has_the_same_height_on_every_kind_of_tab(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    cx.update(|cx| cx.set_global(crate::bars::toolbar::ToolbarButtons(Vec::new())));
    cx.run_until_parked();
    let editor = bar(cx, handle).expect("the toolbar of the document");
    run(cx, handle, "F_PREVIEW_TAB");
    let preview = bar(cx, handle).expect("the row on the preview");
    assert_eq!(preview, editor);
}
