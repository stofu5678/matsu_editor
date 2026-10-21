//! The right click on a tab: custom menu 24 (the tab menu) for that tab.
//!
//! Sakura: `CTabWnd::OnTabRButtonUp` and `ExecTabCommand` (`sakura_core/window/CTabWnd.cpp`); the
//! default items of the menu are in `CShareData::InitPopupMenu` (`sakura_core/env/CShareData.cpp`).

use gpui::{AnyWindowHandle, Modifiers, MouseButton, TestAppContext, VisualTestContext};
use ume_config::Profile;
use ume_core::buffer::Pos;

use super::tests::{
    active_view, code_of, run, tab_mode, tabs, type_text, visual, window, workspace,
};
use crate::settings::UserSettings;

/// Custom menu 24 with one item, `SelectAll`.
const SELECT_ALL_MENU: &str = "[CustMenu]\nnCMIN[24]=1\nnCMIF[24][00]=30401\nnCMIK[24][00]=A\n";

fn user_settings(cx: &mut TestAppContext, ini: &str) {
    let user = UserSettings::from_profile(Profile::from_bytes(ini.as_bytes()), None);
    cx.update(|app| app.set_global(user));
}

fn tab_center(cx: &mut VisualTestContext, index: usize) -> gpui::Point<gpui::Pixels> {
    cx.debug_bounds(Box::leak(format!("tab-{index}").into_boxed_str()))
        .unwrap_or_else(|| panic!("tab {index} is painted"))
        .center()
}

fn right_click(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>) {
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    cx.run_until_parked();
}

fn menu_is_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    active_view(cx, handle).read_with(cx, |v, _| v.custom_menu.is_open())
}

fn active_tab(cx: &mut TestAppContext, handle: AnyWindowHandle) -> usize {
    workspace(cx, handle).read_with(cx, |ws, _| ws.active_tab())
}

/// The selection of the document of tab `index`.
fn selection_of(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    index: usize,
) -> Option<(Pos, Pos)> {
    workspace(cx, handle).read_with(cx, |ws, cx| {
        ws.tab_editors()[index].read(cx).cursor.selection()
    })
}

/// Three tabs holding "a", "b" and "c"; the last one is active.
fn three_tabs(cx: &mut TestAppContext, ini: &str) -> AnyWindowHandle {
    cx.update(gpui_component::init);
    user_settings(cx, ini);
    let handle = tabs(cx, 3);
    for (i, text) in ["a", "b", "c"].iter().enumerate() {
        workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
            ws.activate_tab(i, window, cx)
        });
        type_text(cx, handle, text);
    }
    handle
}

#[gpui::test]
fn the_menu_opens_for_the_tab_under_the_pointer(cx: &mut TestAppContext) {
    let handle = three_tabs(cx, SELECT_ALL_MENU);
    assert_eq!(active_tab(cx, handle), 2);
    let vcx = visual(cx, handle);
    let at = tab_center(vcx, 0);
    right_click(vcx, at);
    // the tab is made active first (`ExecTabCommand`), and the menu starts 4 pixels above the pointer
    assert_eq!(active_tab(cx, handle), 0);
    assert!(menu_is_open(cx, handle));
    let opened = active_view(cx, handle).read_with(cx, |v, _| v.custom_menu.open_at().unwrap());
    assert_eq!(opened, gpui::point(at.x, at.y - gpui::px(4.)));
}

#[gpui::test]
fn a_function_of_the_menu_runs_on_that_tab(cx: &mut TestAppContext) {
    let handle = three_tabs(cx, SELECT_ALL_MENU);
    let vcx = visual(cx, handle);
    let at = tab_center(vcx, 1);
    right_click(vcx, at);
    vcx.simulate_keystrokes("down enter");
    vcx.run_until_parked();
    assert_eq!(
        selection_of(cx, handle, 1),
        Some((Pos::new(0, 0), Pos::new(0, 1))),
        "the document of the tab that was clicked"
    );
    assert_eq!(selection_of(cx, handle, 0), None);
    assert_eq!(selection_of(cx, handle, 2), None);
}

#[gpui::test]
fn the_defaults_of_the_tab_menu_open(cx: &mut TestAppContext) {
    // Sakura's first items: 上書き保存, 名前を付けて保存, 閉じる, 閉じて開く, ウィンドウを閉じる, 開き直す
    let defaults = ume_config::settings::CustomMenus::default();
    let functions: Vec<u16> = defaults
        .get(24)
        .unwrap()
        .items
        .iter()
        .filter(|item| !item.is_separator())
        .map(|item| item.function)
        .take(6)
        .collect();
    let expected: Vec<u16> = [
        "F_FILESAVE",
        "F_FILESAVEAS_DIALOG",
        "F_FILECLOSE",
        "F_FILECLOSE_OPEN",
        "F_WINCLOSE",
        "F_FILE_REOPEN",
    ]
    .iter()
    .map(|name| code_of(name))
    .collect();
    assert_eq!(functions, expected);

    let handle = three_tabs(cx, "");
    let vcx = visual(cx, handle);
    let at = tab_center(vcx, 2);
    right_click(vcx, at);
    assert!(menu_is_open(cx, handle));
}

#[gpui::test]
fn a_users_menu_replaces_the_defaults(cx: &mut TestAppContext) {
    let handle = three_tabs(cx, "[CustMenu]\nnCMIN[24]=0\n");
    let vcx = visual(cx, handle);
    let at = tab_center(vcx, 0);
    right_click(vcx, at);
    // a menu with no items shows nothing, but the tab was still made active
    assert_eq!(active_tab(cx, handle), 0);
    assert!(!menu_is_open(cx, handle));
}

#[gpui::test]
fn the_left_button_does_not_open_the_menu(cx: &mut TestAppContext) {
    let handle = three_tabs(cx, SELECT_ALL_MENU);
    let vcx = visual(cx, handle);
    let at = tab_center(vcx, 0);
    vcx.simulate_mouse_down(at, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
    vcx.run_until_parked();
    assert_eq!(active_tab(cx, handle), 0);
    assert!(!menu_is_open(cx, handle));
}

#[gpui::test]
fn the_bar_without_grouping_opens_the_menu_in_the_window_of_the_tab(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    user_settings(cx, SELECT_ALL_MENU);
    let first = window(cx, true);
    tab_mode(cx, true, false);
    run(cx, first, "F_FILENEW");
    let second = cx.windows()[1];
    // the bar of the first window lists both documents; a right click on the second brings
    // that window to the front and opens the menu there
    let vcx = visual(cx, first);
    let at = tab_center(vcx, 1);
    right_click(vcx, at);
    cx.run_until_parked();
    assert!(menu_is_open(cx, second));
    assert!(!menu_is_open(cx, first));
}

#[gpui::test]
fn the_bar_without_grouping_opens_the_menu_for_the_document_of_this_window(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    user_settings(cx, SELECT_ALL_MENU);
    let first = window(cx, true);
    tab_mode(cx, true, false);
    run(cx, first, "F_FILENEW");
    let vcx = visual(cx, first);
    let at = tab_center(vcx, 0);
    right_click(vcx, at);
    assert!(menu_is_open(cx, first));
}

#[gpui::test]
fn the_tabs_have_no_tooltip_while_the_tab_menu_is_open(cx: &mut TestAppContext) {
    let handle = three_tabs(cx, SELECT_ALL_MENU);
    let shown = |cx: &mut TestAppContext| {
        workspace(cx, handle).read_with(cx, |ws, cx| ws.tab_tooltips_shown(cx))
    };
    assert!(shown(cx));
    let vcx = visual(cx, handle);
    let at = tab_center(vcx, 0);
    right_click(vcx, at);
    assert!(menu_is_open(cx, handle));
    assert!(!shown(cx), "the tooltip would cover the first item");
    let vcx = visual(cx, handle);
    vcx.simulate_keystrokes("escape");
    vcx.run_until_parked();
    assert!(!menu_is_open(cx, handle));
    assert!(shown(cx));
}
