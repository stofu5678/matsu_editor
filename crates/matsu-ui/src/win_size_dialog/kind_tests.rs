//! The kind of window of 大きさ as a drop-down list (`IDC_COMBO_WINTYPE`, `CBS_DROPDOWNLIST`,
//! `WS_GROUP | WS_TABSTOP`, `IDD_WINPOSSIZE`): one Tab stop, Up and Down change the entry while the
//! list is closed (no wrapping), and it can be used only with 直接指定.

use gpui::TestAppContext;

use super::tests::{START, keys, open_dialog, visual, with_dialog};
use super::{
    MODE_DEFAULT, MODE_SET, SIZE_MAXIMIZED, SIZE_MINIMIZED, SIZE_RESTORED, WinSize, current,
};

const SET: WinSize = WinSize {
    size_mode: MODE_SET,
    ..START
};

fn combo_has_focus(cx: &mut TestAppContext) -> bool {
    with_dialog(cx, |d, window, dcx| d.kind.has_focus(window, dcx))
}

fn kind(cx: &mut TestAppContext) -> i32 {
    with_dialog(cx, |d, _, dcx| d.values(dcx).size_type)
}

#[gpui::test]
fn the_combo_box_is_one_tab_stop_that_can_be_left(cx: &mut TestAppContext) {
    let _got = open_dialog(cx, SET);
    for _ in 0..24 {
        if combo_has_focus(cx) {
            break;
        }
        keys(cx, "tab");
    }
    assert!(combo_has_focus(cx), "Tab reached the combo box");
    keys(cx, "tab");
    assert!(!combo_has_focus(cx), "the next Tab leaves it");
    keys(cx, "shift-tab");
    assert!(combo_has_focus(cx), "Shift+Tab comes back to it");
}

#[gpui::test]
fn up_and_down_change_the_kind_while_the_list_is_closed_and_stop_at_the_ends(
    cx: &mut TestAppContext,
) {
    let _got = open_dialog(cx, SET);
    for _ in 0..24 {
        if combo_has_focus(cx) {
            break;
        }
        keys(cx, "tab");
    }
    assert!(combo_has_focus(cx), "Tab reached the combo box");
    // the entries are 普通, 最大化 and (最小化)
    assert_eq!(kind(cx), SIZE_RESTORED);
    keys(cx, "up");
    assert_eq!(kind(cx), SIZE_RESTORED, "no wrapping at the top");
    keys(cx, "down");
    assert_eq!(kind(cx), SIZE_MAXIMIZED);
    keys(cx, "down");
    assert_eq!(kind(cx), SIZE_MINIMIZED);
    keys(cx, "down");
    assert_eq!(kind(cx), SIZE_MINIMIZED, "no wrapping at the bottom");
    keys(cx, "up");
    assert_eq!(kind(cx), SIZE_MAXIMIZED);
    keys(cx, "home");
    assert_eq!(kind(cx), SIZE_RESTORED);
    keys(cx, "end");
    assert_eq!(kind(cx), SIZE_MINIMIZED);
    assert!(
        combo_has_focus(cx),
        "the keys keep the focus on the combo box"
    );
    assert!(
        cx.update(|app| current(app)).is_some(),
        "the dialog is still open"
    );
}

#[gpui::test]
fn the_open_list_takes_the_row_with_enter_and_closes_with_escape(cx: &mut TestAppContext) {
    let got = open_dialog(cx, SET);
    for _ in 0..24 {
        if combo_has_focus(cx) {
            break;
        }
        keys(cx, "tab");
    }
    keys(cx, "f4");
    assert!(with_dialog(cx, |d, _, _| d.kind.is_open()));
    keys(cx, "down");
    keys(cx, "down");
    // the row is only under the keys until Enter takes it
    assert_eq!(kind(cx), SIZE_RESTORED);
    keys(cx, "enter");
    assert!(!with_dialog(cx, |d, _, _| d.kind.is_open()));
    assert_eq!(kind(cx), SIZE_MINIMIZED);
    assert!(
        cx.update(|app| current(app)).is_some(),
        "Enter took the row"
    );
    // Escape closes the list and not the dialog
    keys(cx, "f4");
    keys(cx, "up");
    keys(cx, "escape");
    assert!(!with_dialog(cx, |d, _, _| d.kind.is_open()));
    assert_eq!(kind(cx), SIZE_MINIMIZED);
    assert!(
        cx.update(|app| current(app)).is_some(),
        "Escape closed the list only"
    );
    assert!(got.borrow().is_empty());
}

#[gpui::test]
fn the_combo_box_cannot_be_used_without_a_direct_size(cx: &mut TestAppContext) {
    let _got = open_dialog(
        cx,
        WinSize {
            size_mode: MODE_DEFAULT,
            ..START
        },
    );
    for _ in 0..24 {
        keys(cx, "tab");
        assert!(!combo_has_focus(cx), "Tab skips the greyed combo box");
    }
    // the kind that was set is kept while it cannot be used
    assert_eq!(kind(cx), SIZE_RESTORED);
}

#[gpui::test]
fn shift_tab_closes_the_open_list(cx: &mut TestAppContext) {
    let _got = open_dialog(cx, SET);
    for _ in 0..24 {
        if combo_has_focus(cx) {
            break;
        }
        keys(cx, "tab");
    }
    keys(cx, "f4");
    assert!(with_dialog(cx, |d, _, _| d.kind.is_open()));
    keys(cx, "shift-tab");
    assert!(!combo_has_focus(cx));
    assert!(
        !with_dialog(cx, |d, _, _| d.kind.is_open()),
        "the focus left the box"
    );
}

#[gpui::test]
fn the_keys_change_nothing_once_the_size_leaves_a_direct_size(cx: &mut TestAppContext) {
    let _got = open_dialog(cx, SET);
    for _ in 0..24 {
        if combo_has_focus(cx) {
            break;
        }
        keys(cx, "tab");
    }
    keys(cx, "f4");
    assert!(with_dialog(cx, |d, _, _| d.kind.is_open()));
    with_dialog(cx, |d, _, dcx| d.set_size_mode(MODE_DEFAULT, dcx));
    assert!(
        !with_dialog(cx, |d, _, _| d.kind.is_open()),
        "the list is closed"
    );
    // the focus may still be in the box: the keys do not reach the choice
    keys(cx, "down");
    keys(cx, "f4");
    assert_eq!(kind(cx), SIZE_RESTORED);
    assert!(!with_dialog(cx, |d, _, _| d.kind.is_open()));
}

#[gpui::test]
fn a_kind_that_is_none_of_the_three_is_shown_and_saved_as_the_first(cx: &mut TestAppContext) {
    // `CDlgWinSize::SetData` shows entry 0 and `GetData` writes what entry 0 says
    let _got = open_dialog(
        cx,
        WinSize {
            size_type: 7,
            ..SET
        },
    );
    assert_eq!(kind(cx), SIZE_RESTORED);
}

#[gpui::test]
fn a_click_on_the_box_opens_the_list_while_the_focus_is_elsewhere(cx: &mut TestAppContext) {
    let _got = open_dialog(cx, SET);
    assert!(!combo_has_focus(cx));
    let bounds = visual(cx)
        .debug_bounds("win-kind-box")
        .expect("the combo box");
    visual(cx).simulate_click(bounds.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(
        with_dialog(cx, |d, _, _| d.kind.is_open()),
        "the list is open"
    );
    assert!(combo_has_focus(cx), "the focus is in the box");
    // a second click closes it
    visual(cx).simulate_click(bounds.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(!with_dialog(cx, |d, _, _| d.kind.is_open()));
}

#[gpui::test]
fn space_opens_the_list_again_after_a_click_opened_it_and_escape_closed_it(
    cx: &mut TestAppContext,
) {
    let _got = open_dialog(cx, SET);
    let bounds = visual(cx)
        .debug_bounds("win-kind-box")
        .expect("the combo box");
    visual(cx).simulate_click(bounds.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(with_dialog(cx, |d, _, _| d.kind.is_open()));
    keys(cx, "escape");
    assert!(!with_dialog(cx, |d, _, _| d.kind.is_open()));
    assert!(
        cx.update(|app| current(app)).is_some(),
        "Escape closed the list only"
    );
    keys(cx, "space");
    assert!(
        with_dialog(cx, |d, _, _| d.kind.is_open()),
        "Space opens the list"
    );
    // in the open list Space takes the row, and Alt+Up closes the list without choosing
    keys(cx, "down");
    keys(cx, "space");
    assert!(!with_dialog(cx, |d, _, _| d.kind.is_open()));
    assert_eq!(kind(cx), SIZE_MAXIMIZED);
    keys(cx, "alt-down");
    assert!(with_dialog(cx, |d, _, _| d.kind.is_open()));
    keys(cx, "alt-up");
    assert!(!with_dialog(cx, |d, _, _| d.kind.is_open()));
    assert_eq!(kind(cx), SIZE_MAXIMIZED);
}

#[gpui::test]
fn the_open_list_is_drawn_over_the_dialog_and_moves_nothing(cx: &mut TestAppContext) {
    let _got = open_dialog(cx, SET);
    let closed = visual(cx)
        .debug_bounds("win-kind-box")
        .expect("the combo box");
    // the height box is under the combo box, and the close button at the bottom
    let spin_closed = visual(cx)
        .debug_bounds("win-size-close")
        .expect("the close button");
    visual(cx).simulate_click(closed.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(with_dialog(cx, |d, _, _| d.kind.is_open()));
    assert_eq!(visual(cx).debug_bounds("win-kind-box"), Some(closed));
    assert_eq!(visual(cx).debug_bounds("win-size-close"), Some(spin_closed));
    // a click on a row takes it, while the focus stays in the box
    let row = visual(cx)
        .debug_bounds("win-kind-row-1")
        .expect("the second row");
    assert!(row.top() >= closed.bottom(), "the list is under the box");
    visual(cx).simulate_click(row.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(kind(cx), SIZE_MAXIMIZED);
    assert!(!with_dialog(cx, |d, _, _| d.kind.is_open()));
}
