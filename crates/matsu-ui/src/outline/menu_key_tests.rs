//! The letters in the right-click menu of the outline pane and in its submenu ウィンドウの位置(W)
//! (`menu_bar::popup_keys`).
//!
//! Sakura: `CMenuDrawer::OnMenuChar` (`sakura_core/uiparts/CMenuDrawer.cpp`): the item that has the
//! key runs, a submenu opens.

use gpui::{AnyWindowHandle, Modifiers, MouseButton, TestAppContext};

use super::dock::DockSide;
use super::layout_tests::{current_place, open, rects, visual};
use super::tests::{pane, run};

/// The pane is docked at the left, and its menu is open at the middle of the window, where the
/// submenu has room at the right (the Right key opens it, as in `tabs/popup_key_tests.rs`).
fn open_menu(cx: &mut TestAppContext) -> AnyWindowHandle {
    open_menu_at(cx, "[Common]\r\neOutlineDockSide=1\r\n")
}

fn open_menu_at(cx: &mut TestAppContext, ini: &str) -> AnyWindowHandle {
    cx.update(gpui_component::init);
    let handle = open(cx, ini);
    run(cx, handle, "F_OUTLINE");
    let at = rects(cx, handle).pane.center();
    let vcx = visual(cx, handle);
    vcx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    vcx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    cx.run_until_parked();
    assert!(menu_is_open(cx, handle), "the menu is open");
    handle
}

fn menu_is_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    pane(cx, handle).is_some_and(|p| p.read_with(cx, |p, _| p.menu_is_open()))
}

fn keys(cx: &mut TestAppContext, handle: AnyWindowHandle, keys: &str) {
    let vcx = visual(cx, handle);
    vcx.simulate_keystrokes(keys);
    vcx.run_until_parked();
}

/// The letter of a submenu selects it and sends the Right key a frame later, once the popup has drawn
/// its selection. A test window draws no frames of its own, so the Right key is pressed here (as
/// `tabs/popup_key_tests.rs` does).
fn open_position_menu(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    keys(cx, handle, "w");
    keys(cx, handle, "right");
    assert!(menu_is_open(cx, handle), "the submenu is open");
}

#[gpui::test]
fn a_letter_of_the_first_level_opens_the_submenu_and_a_letter_of_it_runs(cx: &mut TestAppContext) {
    let handle = open_menu(cx);
    // W opens ウィンドウの位置: a letter of the submenu then runs.
    open_position_menu(cx, handle);
    keys(cx, handle, "r");
    assert!(!menu_is_open(cx, handle), "the menu closes");
    assert_eq!(current_place(cx, handle).side, DockSide::Right);
}

#[gpui::test]
fn a_letter_of_the_first_level_runs_its_item(cx: &mut TestAppContext) {
    let handle = open_menu(cx);
    // X is 閉じる.
    keys(cx, handle, "x");
    assert!(pane(cx, handle).is_none());
}

#[gpui::test]
fn a_letter_in_the_submenu_runs_its_item_whatever_the_case(cx: &mut TestAppContext) {
    for (key, side) in [
        ("t", DockSide::Top),
        ("shift-b", DockSide::Bottom),
        ("f", DockSide::Float),
        ("i", DockSide::Undockable),
    ] {
        let handle = open_menu(cx);
        open_position_menu(cx, handle);
        keys(cx, handle, key);
        assert!(!menu_is_open(cx, handle), "{key}: the menu closes");
        assert_eq!(current_place(cx, handle).side, side, "{key}");
    }
}

#[gpui::test]
fn a_letter_that_no_item_of_the_submenu_has_does_nothing(cx: &mut TestAppContext) {
    let handle = open_menu(cx);
    open_position_menu(cx, handle);
    // E is an item of the first level, not of the submenu; Z is nobody's.
    keys(cx, handle, "z");
    assert!(menu_is_open(cx, handle));
    assert_eq!(current_place(cx, handle).side, DockSide::Left);
    // The submenu still has the focus: a letter it has runs.
    keys(cx, handle, "b");
    assert!(!menu_is_open(cx, handle));
    assert_eq!(current_place(cx, handle).side, DockSide::Bottom);
}
