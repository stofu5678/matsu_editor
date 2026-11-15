//! The toolbar row on the tabs that are not editors (a preview, a shell, the AI log): it has the height of
//! the toolbar, so the top of the window does not move between kinds of tab, and its button closes the tab.
//! The function key bar stays too (only its close button works there).

use gpui::{AnyWindowHandle, Bounds, Pixels, TestAppContext};

use super::terminal_tests::{open as open_shell, staying, terminal_window, wait_for_text};
use super::tests::{answer, code_of, run, tab_count, visual, window, workspace};
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

/// The function key bar and the tab strip, as the window draws them now.
fn funckey_rows(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
) -> (Bounds<Pixels>, Bounds<Pixels>) {
    let vcx = visual(cx, handle);
    (
        vcx.debug_bounds("function-key-bar")
            .expect("the function key bar is drawn"),
        vcx.debug_bounds("tab-strip")
            .expect("the tab strip is drawn"),
    )
}

fn show_tab(cx: &mut TestAppContext, handle: AnyWindowHandle, index: usize) {
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.activate_tab(index, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn funckeys_at(cx: &mut TestAppContext, place: crate::bars::FuncKeyPlace) {
    cx.update(|cx| {
        crate::bars::update(cx, |s| {
            s.funckeys = true;
            s.funckey_place = place;
        })
    });
    cx.run_until_parked();
}

/// Puts `function` on F1 (no modifier) in the key map of the application.
fn bind_f1(cx: &mut TestAppContext, function: &str) {
    use matsu_commands::keymap::{Key, KeyMap, KeyMapOptions, Platform, UserRow};
    let mut map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
    let mut functions = [0u16; 8];
    functions[0] = code_of(function);
    map.apply_user_rows(&[UserRow {
        key: Key::function(1).unwrap(),
        functions,
    }]);
    cx.update(|cx| {
        crate::app::set_key_map(cx, map);
        cx.refresh_windows();
    });
    cx.run_until_parked();
}

fn click_funckey(cx: &mut TestAppContext, handle: AnyWindowHandle, index: usize) {
    let selector: &'static str = Box::leak(format!("function-key-{index}").into_boxed_str());
    let at = visual(cx, handle)
        .debug_bounds(selector)
        .expect("the button is drawn")
        .center();
    let vcx = visual(cx, handle);
    vcx.simulate_click(at, gpui::Modifiers::none());
    vcx.run_until_parked();
}

fn funckeys_stay_on_every_kind_of_tab(cx: &mut TestAppContext, place: crate::bars::FuncKeyPlace) {
    let handle = terminal_window(cx);
    funckeys_at(cx, place);
    let editor = funckey_rows(cx, handle);

    run(cx, handle, "F_PREVIEW_TAB");
    assert_eq!(funckey_rows(cx, handle), editor, "preview");

    let _shell = open_shell(cx, handle, staying());
    assert_eq!(funckey_rows(cx, handle), editor, "shell");

    open_log(cx, handle);
    // The log opens behind the document: go to it first.
    let last = tab_count(cx, handle) - 1;
    show_tab(cx, handle, last);
    assert_eq!(funckey_rows(cx, handle), editor, "AI log");

    show_tab(cx, handle, 0);
    assert_eq!(funckey_rows(cx, handle), editor, "back on the editor");
}

#[gpui::test]
fn the_function_keys_stay_at_the_top_on_every_kind_of_tab(cx: &mut TestAppContext) {
    funckeys_stay_on_every_kind_of_tab(cx, crate::bars::FuncKeyPlace::Top);
}

#[gpui::test]
fn the_function_keys_stay_at_the_bottom_on_every_kind_of_tab(cx: &mut TestAppContext) {
    funckeys_stay_on_every_kind_of_tab(cx, crate::bars::FuncKeyPlace::Bottom);
}

#[gpui::test]
fn only_the_close_function_key_works_on_a_preview(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    funckeys_at(cx, crate::bars::FuncKeyPlace::Top);
    run(cx, handle, "F_PREVIEW_TAB");
    assert_eq!(tab_count(cx, handle), 2);

    // A function of the editor is grayed out here and does nothing.
    bind_f1(cx, "F_FILENEW");
    click_funckey(cx, handle, 0);
    assert_eq!(tab_count(cx, handle), 2);

    // F_WINCLOSE closes the preview, as the button of the toolbar row does.
    bind_f1(cx, "F_WINCLOSE");
    click_funckey(cx, handle, 0);
    assert_eq!(tab_count(cx, handle), 1);
}
