//! The kind of window of 大きさ as a drop-down list (`IDC_COMBO_WINTYPE`, `CBS_DROPDOWNLIST`,
//! `WS_GROUP | WS_TABSTOP`, `IDD_WINPOSSIZE`): one Tab stop, Up and Down change the entry while the
//! list is closed (no wrapping), and it can be used only with 直接指定.

use gpui::TestAppContext;

use super::tests::{START, keys, open_dialog, with_dialog};
use super::{
    MODE_DEFAULT, MODE_SET, SIZE_MAXIMIZED, SIZE_MINIMIZED, SIZE_RESTORED, WinSize, current,
};

const SET: WinSize = WinSize {
    size_mode: MODE_SET,
    ..START
};

fn combo_has_focus(cx: &mut TestAppContext) -> bool {
    with_dialog(cx, |d, window, dcx| {
        d.kind_focus.contains_focused(window, dcx)
    })
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
    assert!(with_dialog(cx, |d, _, _| d.kind_open));
    keys(cx, "down");
    keys(cx, "down");
    // the row is only under the keys until Enter takes it
    assert_eq!(kind(cx), SIZE_RESTORED);
    keys(cx, "enter");
    assert!(!with_dialog(cx, |d, _, _| d.kind_open));
    assert_eq!(kind(cx), SIZE_MINIMIZED);
    assert!(
        cx.update(|app| current(app)).is_some(),
        "Enter took the row"
    );
    // Escape closes the list and not the dialog
    keys(cx, "f4");
    keys(cx, "up");
    keys(cx, "escape");
    assert!(!with_dialog(cx, |d, _, _| d.kind_open));
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
