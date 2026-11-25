//! The tab stops of the question of the import (`IDD_TYPE_ASCERTAIN`) after 読込先: 色選択 as a
//! closed drop-down list (`IDC_COMBO_COLORS`, `CBS_DROPDOWNLIST | WS_VSCROLL | WS_TABSTOP`) and the
//! push buttons OK, キャンセル(&X) and ヘルプ(&H). The files are `.ini` and `.col` files in the
//! test's own folder; no test depends on a path or a line ending of the system.

use gpui::{Entity, TestAppContext, VisualTestContext};

use super::list::TypeList;
use super::transfer_group_tests::{
    adding, color, colors_focused, list_focused, question, question_with_color_files,
};
use crate::radio_group::Ring;
use crate::radio_group::testing::{at, tab_to};

/// Where the focus is in the question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    Group,
    Colors,
    Ok,
    Cancel,
    Help,
    Elsewhere,
}

fn stop(list: &Entity<TypeList>, ring: &Ring, cx: &mut VisualTestContext) -> Stop {
    let (ok, cancel, help) = list.read_with(cx, |l, _| {
        let q = l.ascertain.as_ref().expect("the question");
        (
            q.ok_focus.clone(),
            q.cancel_focus.clone(),
            q.help_focus.clone(),
        )
    });
    if colors_focused(list, cx) {
        return Stop::Colors;
    }
    if at(cx, &[ring]).is_some() {
        return Stop::Group;
    }
    cx.update(|window, _| {
        if ok.is_focused(window) {
            Stop::Ok
        } else if cancel.is_focused(window) {
            Stop::Cancel
        } else if help.is_focused(window) {
            Stop::Help
        } else {
            Stop::Elsewhere
        }
    })
}

fn is_open(list: &Entity<TypeList>, cx: &mut VisualTestContext) -> bool {
    list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().colors.is_open())
}

fn question_is_open(list: &Entity<TypeList>, cx: &mut VisualTestContext) -> bool {
    list.read_with(cx, |l, _| l.ascertain.is_some())
}

/// Tabs to the colors box.
fn to_colors(list: &Entity<TypeList>, ring: &Ring, cx: &mut VisualTestContext) {
    tab_to(cx, &[ring], 0, 12);
    cx.simulate_keystrokes("tab");
    assert_eq!(stop(list, ring, cx), Stop::Colors);
}

#[gpui::test]
fn tab_goes_from_the_group_to_the_colors_ok_cancel_help_and_round(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    tab_to(cx, &[&ring], 0, 12);
    let mut forward = Vec::new();
    for _ in 0..5 {
        cx.simulate_keystrokes("tab");
        forward.push(stop(&list, &ring, cx));
    }
    assert_eq!(
        forward,
        [
            Stop::Colors,
            Stop::Ok,
            Stop::Cancel,
            Stop::Help,
            Stop::Group
        ]
    );
    // and the other way
    let mut back = Vec::new();
    for _ in 0..5 {
        cx.simulate_keystrokes("shift-tab");
        back.push(stop(&list, &ring, cx));
    }
    assert_eq!(
        back,
        [
            Stop::Help,
            Stop::Cancel,
            Stop::Ok,
            Stop::Colors,
            Stop::Group
        ]
    );
    assert!(question_is_open(&list, cx));
}

#[gpui::test]
fn the_closed_box_chooses_with_the_arrows_home_end_page_keys_without_going_round(
    cx: &mut TestAppContext,
) {
    // 基本 and the other types of the list make dozens of rows
    let (mut cx, list, ring) = question_with_color_files(cx, 3);
    let cx = &mut cx;
    let last = list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().choices().len() - 1);
    assert!(last > 3);
    to_colors(&list, &ring, cx);
    assert_eq!(color(&list, cx), 0);
    cx.simulate_keystrokes("up");
    assert_eq!(color(&list, cx), 0, "no wrapping at the top");
    cx.simulate_keystrokes("down down");
    assert_eq!(color(&list, cx), 2);
    cx.simulate_keystrokes("end");
    assert_eq!(color(&list, cx), last);
    cx.simulate_keystrokes("down");
    assert_eq!(color(&list, cx), last, "no wrapping at the bottom");
    cx.simulate_keystrokes("home");
    assert_eq!(color(&list, cx), 0);
    cx.simulate_keystrokes("pagedown");
    assert_eq!(color(&list, cx), last);
    cx.simulate_keystrokes("pageup");
    assert_eq!(color(&list, cx), 0);
    assert!(!is_open(&list, cx), "the keys do not open the list");
    assert!(adding(&list, cx), "and do not touch 読込先");
}

#[gpui::test]
fn f4_alt_down_space_and_a_click_open_the_list_and_enter_takes_a_row(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    to_colors(&list, &ring, cx);
    for opener in ["f4", "alt-down"] {
        cx.simulate_keystrokes(opener);
        assert!(is_open(&list, cx), "{opener} opens the list");
        // the list closes with the same key, and Alt+Up
        cx.simulate_keystrokes(opener);
        assert!(!is_open(&list, cx), "{opener} closes it");
    }
    cx.simulate_keystrokes("f4 down down");
    assert!(is_open(&list, cx));
    assert_eq!(color(&list, cx), 0, "the arrows only move in the open list");
    cx.simulate_keystrokes("enter");
    assert!(!is_open(&list, cx));
    assert_eq!(color(&list, cx), 2, "Enter takes the row");
    assert!(question_is_open(&list, cx), "Enter did not press OK");
    // Alt+Up closes the list without taking a row
    cx.simulate_keystrokes("f4 down alt-up");
    assert!(!is_open(&list, cx));
    assert_eq!(color(&list, cx), 2);
}

#[gpui::test]
fn escape_closes_the_open_list_and_not_the_question_and_f4_does_not_take_a_row(
    cx: &mut TestAppContext,
) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    to_colors(&list, &ring, cx);
    cx.simulate_keystrokes("down f4 down escape");
    assert!(!is_open(&list, cx));
    assert!(question_is_open(&list, cx), "Escape closed the list only");
    assert_eq!(color(&list, cx), 1, "the row of the list was not taken");
    cx.simulate_keystrokes("f4 down f4");
    assert!(!is_open(&list, cx));
    assert_eq!(color(&list, cx), 1);
    // with the list closed Escape is キャンセル
    cx.simulate_keystrokes("escape");
    assert!(!question_is_open(&list, cx));
}

#[gpui::test]
fn tab_and_shift_tab_close_the_open_list_and_move_on(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    to_colors(&list, &ring, cx);
    cx.simulate_keystrokes("f4 down tab");
    assert!(!is_open(&list, cx));
    assert_eq!(stop(&list, &ring, cx), Stop::Ok);
    assert_eq!(color(&list, cx), 0, "Tab takes no row");
    cx.simulate_keystrokes("shift-tab f4 shift-tab");
    assert!(!is_open(&list, cx));
    assert_eq!(stop(&list, &ring, cx), Stop::Group);
}

#[gpui::test]
fn a_click_opens_the_list_while_the_focus_is_elsewhere_and_a_click_on_a_row_takes_it(
    cx: &mut TestAppContext,
) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    assert_eq!(stop(&list, &ring, cx), Stop::Elsewhere);
    let closed = cx
        .debug_bounds("type-ascertain-color-box")
        .expect("the box");
    let below = cx.debug_bounds("type-ascertain-ok").expect("OK");
    cx.simulate_click(closed.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(is_open(&list, cx), "the list is open");
    assert_eq!(stop(&list, &ring, cx), Stop::Colors, "the focus is in it");
    // drawn over the dialog: nothing moves
    assert_eq!(cx.debug_bounds("type-ascertain-color-box"), Some(closed));
    assert_eq!(cx.debug_bounds("type-ascertain-ok"), Some(below));
    let row = cx
        .debug_bounds("type-ascertain-color-row-1")
        .expect("the second row");
    assert!(row.top() >= closed.bottom(), "the list is under the box");
    cx.simulate_click(row.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(color(&list, cx), 1);
    assert!(!is_open(&list, cx));
    assert!(question_is_open(&list, cx));
    // F4 opens the list again, and Space takes the row
    cx.simulate_keystrokes("f4 down space");
    assert_eq!(color(&list, cx), 2);
    assert!(!is_open(&list, cx));
}

#[gpui::test]
fn after_a_mouse_open_the_box_is_still_one_tab_stop(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    let closed = cx
        .debug_bounds("type-ascertain-color-box")
        .expect("the box");
    cx.simulate_click(closed.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(is_open(&list, cx));
    cx.simulate_keystrokes("escape");
    cx.simulate_keystrokes("tab");
    assert_eq!(stop(&list, &ring, cx), Stop::Ok, "Tab goes straight on");
    cx.simulate_keystrokes("shift-tab shift-tab");
    assert_eq!(stop(&list, &ring, cx), Stop::Group);
    // a row taken with the mouse leaves the same
    cx.simulate_click(closed.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    let row = cx
        .debug_bounds("type-ascertain-color-row-1")
        .expect("the second row");
    cx.simulate_click(row.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(color(&list, cx), 1);
    cx.simulate_keystrokes("tab");
    assert_eq!(stop(&list, &ring, cx), Stop::Ok);
    cx.simulate_keystrokes("shift-tab shift-tab");
    assert_eq!(stop(&list, &ring, cx), Stop::Group);
}

#[gpui::test]
fn the_open_list_closes_when_the_focus_leaves_the_box(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    to_colors(&list, &ring, cx);
    cx.simulate_keystrokes("f4");
    assert!(is_open(&list, cx));
    // the focus moves without a key of the box (a click on a button of the question)
    let focus = list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().help_focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    assert_eq!(stop(&list, &ring, cx), Stop::Help);
    assert!(!is_open(&list, cx), "the list closed with the focus");
}

#[gpui::test]
fn many_colors_make_a_list_that_scrolls_and_does_not_grow_with_them(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question_with_color_files(cx, 40);
    let cx = &mut cx;
    to_colors(&list, &ring, cx);
    cx.simulate_keystrokes("f4");
    let shown = cx
        .debug_bounds("type-ascertain-color-list")
        .expect("the list");
    assert!(shown.size.height <= gpui::px(180.), "{:?}", shown.size);
    assert!(
        list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().choices().len()
            > 40)
    );
    // the keys go to the last row of the list
    cx.simulate_keystrokes("end enter");
    let last = list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().choices().len() - 1);
    assert_eq!(color(&list, cx), last);
    let name = list.read_with(cx, |l, _| {
        l.ascertain.as_ref().unwrap().choices()[last].1.clone()
    });
    assert!(name.starts_with("File -- "), "{name}");
}

#[gpui::test]
fn alt_c_moves_the_focus_to_the_colors(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    assert_eq!(stop(&list, &ring, cx), Stop::Elsewhere);
    cx.simulate_keystrokes("alt-c");
    assert_eq!(stop(&list, &ring, cx), Stop::Colors);
    assert!(!is_open(&list, cx), "Alt+C only moves the focus");
    assert!(question_is_open(&list, cx));
    // the box is one Tab stop: the next one is OK
    cx.simulate_keystrokes("tab");
    assert_eq!(stop(&list, &ring, cx), Stop::Ok);
    // from the group too
    cx.simulate_keystrokes("shift-tab shift-tab");
    assert_eq!(stop(&list, &ring, cx), Stop::Group);
    cx.simulate_keystrokes("alt-c");
    assert_eq!(stop(&list, &ring, cx), Stop::Colors);
}

#[gpui::test]
fn space_and_enter_press_the_button_that_has_the_focus(cx: &mut TestAppContext) {
    // キャンセル
    for key in ["space", "enter"] {
        let (mut cx, list, ring) = question(cx);
        let cx = &mut cx;
        to_colors(&list, &ring, cx);
        cx.simulate_keystrokes("tab tab");
        assert_eq!(stop(&list, &ring, cx), Stop::Cancel);
        cx.simulate_keystrokes(key);
        assert!(!question_is_open(&list, cx), "{key} on キャンセル");
        assert!(list_focused(&list, cx), "the type list has its keys again");
    }
    // ヘルプ: the help page opens and the question stays
    for key in ["space", "enter"] {
        use crate::support::help::testing::{clear, page_of_function, take};
        let (mut cx, list, ring) = question(cx);
        let cx = &mut cx;
        to_colors(&list, &ring, cx);
        cx.simulate_keystrokes("tab tab tab");
        assert_eq!(stop(&list, &ring, cx), Stop::Help);
        clear();
        cx.simulate_keystrokes(key);
        assert_eq!(take(), [page_of_function("F_TYPE_LIST")], "{key} on ヘルプ");
        assert!(question_is_open(&list, cx));
    }
    // OK: Space and Enter, and Enter anywhere else, take the question and give the focus back
    for (tabs, key) in [(1, "space"), (1, "enter"), (0, "enter"), (2, "enter")] {
        let (mut cx, list, ring) = question(cx);
        let cx = &mut cx;
        to_colors(&list, &ring, cx);
        for _ in 0..tabs {
            cx.simulate_keystrokes("tab");
        }
        cx.simulate_keystrokes(key);
        cx.run_until_parked();
        assert!(!question_is_open(&list, cx), "{key} after {tabs} Tabs");
        if cx.has_pending_prompt() {
            cx.simulate_prompt_answer("OK");
        }
        assert!(list_focused(&list, cx));
    }
}

#[gpui::test]
fn alt_o_presses_ok_and_alt_x_cancels(cx: &mut TestAppContext) {
    let (mut cx, list, _ring) = question(cx);
    let cx = &mut cx;
    cx.simulate_keystrokes("alt-x");
    assert!(!question_is_open(&list, cx), "Alt+X is キャンセル");
    let (mut cx, list, _ring) = question(cx);
    let cx = &mut cx;
    cx.simulate_keystrokes("alt-o");
    cx.run_until_parked();
    assert!(!question_is_open(&list, cx), "Alt+O is OK");
    if cx.has_pending_prompt() {
        cx.simulate_prompt_answer("OK");
    }
}

#[gpui::test]
fn the_buttons_can_be_clicked(cx: &mut TestAppContext) {
    let (mut cx, list, _ring) = question(cx);
    let cx = &mut cx;
    let cancel = cx
        .debug_bounds("type-ascertain-cancel")
        .expect("キャンセル");
    cx.simulate_click(cancel.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(!question_is_open(&list, cx));
}
