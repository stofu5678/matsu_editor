//! 読込先 of the question of the import (`IDD_TYPE_ASCERTAIN`: `IDC_RADIO_TYPE_ADD` and
//! `IDC_RADIO_TYPE_TO`) as one group of radio buttons with one Tab stop. The resource file gives
//! neither button `WS_TABSTOP`, so Tab never reaches them in Sakura; here it does, as in the other
//! groups. The files of these tests are `.ini` files in the test's own folder.

use gpui::{TestAppContext, VisualTestContext};

use super::list_tests::{open_list, setup};
use super::tests::view_of;
use crate::radio_group::Ring;
use crate::radio_group::testing::{assert_stops_at_checked, at, tab_to, trail};

/// A type file of one type, written in the folder of the test.
const TYPE_FILE: &str = "; タイプ別設定 Ver1\r\n\r\n\
    [Info]\r\nszVersion=2.4.2.0\r\nvStructureVersion=183\r\n\r\n\
    [Types]\r\nid=7\r\nnInts=9,90,0,2,-1,-1,0,0,0,0,0,0\r\nszTypeName=日本語タイプ\r\nszTypeExts=jp\r\n";

/// Opens the question and gives its ring and the list that shows it.
pub(super) fn question(
    cx: &mut TestAppContext,
) -> (VisualTestContext, gpui::Entity<super::list::TypeList>, Ring) {
    question_with_color_files(cx, 0)
}

/// The same, with `count` color files (`*.col`) next to the type file.
pub(super) fn question_with_color_files(
    cx: &mut TestAppContext,
    count: usize,
) -> (VisualTestContext, gpui::Entity<super::list::TypeList>, Ring) {
    let (ini, handle) = setup(cx, "");
    for n in 0..count {
        std::fs::write(ini.folder().join(format!("c{n:02}.col")), b"").unwrap();
    }
    let mut cx = VisualTestContext::from_window(handle, cx);
    let view = view_of(&mut cx);
    let list = open_list(&view, &mut cx);
    let file = ini.folder().join("jp.ini");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(TYPE_FILE.as_bytes());
    std::fs::write(&file, bytes).unwrap();
    assert!(list.update_in(&mut cx, |l, window, lcx| {
        l.import_chosen(&file, window, lcx)
    }));
    cx.run_until_parked();
    let ring = list.read_with(&cx, |l, _| {
        l.ascertain.as_ref().unwrap().location_ring.clone()
    });
    (cx, list, ring)
}

pub(super) fn adding(
    list: &gpui::Entity<super::list::TypeList>,
    cx: &mut VisualTestContext,
) -> bool {
    list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().add)
}

#[gpui::test]
fn tab_stops_on_the_group_once_at_the_checked_button(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    // 新規追加 is checked at first (there is room)
    assert!(adding(&list, cx));
    let forward = trail(cx, &[&ring], "tab", 12);
    assert_stops_at_checked(&forward, &[0]);
    let back = trail(cx, &[&ring], "shift-tab", 12);
    assert_stops_at_checked(&back, &[0]);
    // Tab stops at the other button once it is checked
    list.update(cx, |l, cx| l.set_location(false, cx));
    cx.run_until_parked();
    let forward = trail(cx, &[&ring], "tab", 12);
    assert_stops_at_checked(&forward, &[1]);
}

#[gpui::test]
fn the_arrow_keys_choose_the_other_button_and_go_round(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    // before the colors have the focus, the arrow keys are not theirs
    cx.simulate_keystrokes("down");
    assert_eq!(
        list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().color),
        0
    );
    assert!(adding(&list, cx));
    assert_eq!(tab_to(cx, &[&ring], 0, 12), 0);
    // two buttons: every arrow key goes to the other one, round the end
    for (key, add, to) in [
        ("down", false, 1),
        ("up", true, 0),
        ("right", false, 1),
        ("left", true, 0),
        ("down", false, 1),
    ] {
        cx.simulate_keystrokes(key);
        assert_eq!(adding(&list, cx), add, "{key}");
        assert_eq!(at(cx, &[&ring]), Some((0, to)), "{key}");
    }
    // with the focus on the group, the arrow keys do not move the colors
    assert_eq!(
        list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().color),
        0
    );
    // Tab goes on to 色選択 (`IDC_COMBO_COLORS` has `WS_TABSTOP`), and Shift+Tab comes back to
    // the button that is chosen now
    cx.simulate_keystrokes("tab");
    assert_eq!(at(cx, &[&ring]), None);
    assert!(colors_focused(&list, cx));
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(at(cx, &[&ring]), Some((0, 1)));
}

pub(super) fn colors_focused(
    list: &gpui::Entity<super::list::TypeList>,
    cx: &mut VisualTestContext,
) -> bool {
    let focus = list.read_with(cx, |l, _| {
        l.ascertain.as_ref().unwrap().colors.focus().clone()
    });
    cx.update(|window, cx| focus.contains_focused(window, cx))
}

pub(super) fn color(
    list: &gpui::Entity<super::list::TypeList>,
    cx: &mut VisualTestContext,
) -> usize {
    list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().color)
}

#[gpui::test]
fn the_colors_are_a_tab_stop_after_the_group_and_the_arrow_keys_move_them(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    assert_eq!(tab_to(cx, &[&ring], 0, 12), 0);
    // Tab from the group reaches the colors, where Up and Down move the color and not the group
    cx.simulate_keystrokes("tab");
    assert!(colors_focused(&list, cx));
    assert_eq!(color(&list, cx), 0);
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("down");
    assert_eq!(color(&list, cx), 2);
    cx.simulate_keystrokes("up");
    assert_eq!(color(&list, cx), 1);
    assert!(adding(&list, cx), "the arrow keys did not touch 読込先");
    assert!(colors_focused(&list, cx));
    // Shift+Tab returns to the group
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(at(cx, &[&ring]), Some((0, 0)));
    assert_eq!(color(&list, cx), 1);
}

#[gpui::test]
fn alt_a_chooses_the_new_type(cx: &mut TestAppContext) {
    let (mut cx, list, _ring) = question(cx);
    let cx = &mut cx;
    list.update(cx, |l, cx| l.set_location(false, cx));
    assert!(!adding(&list, cx));
    cx.simulate_keystrokes("alt-a");
    assert!(adding(&list, cx));
    // the question is still open
    assert!(list.read_with(cx, |l, _| l.ascertain.is_some()));
}

#[gpui::test]
fn alt_a_takes_the_focus_to_the_button_and_the_arrow_keys_go_on_from_it(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    list.update(cx, |l, cx| l.set_location(false, cx));
    // the focus is not in the group
    assert_ne!(at(cx, &[&ring]).map(|stop| stop.0), Some(0));
    cx.simulate_keystrokes("alt-a");
    assert!(adding(&list, cx));
    assert_eq!(at(cx, &[&ring]), Some((0, 0)));
    cx.simulate_keystrokes("down");
    assert!(!adding(&list, cx));
    assert_eq!(at(cx, &[&ring]), Some((0, 1)));
    // the key of the checked button focuses it too
    cx.simulate_keystrokes("up");
    assert!(adding(&list, cx));
    cx.simulate_keystrokes("tab");
    assert_ne!(at(cx, &[&ring]).map(|stop| stop.0), Some(0));
    cx.simulate_keystrokes("alt-a");
    assert!(adding(&list, cx));
    assert_eq!(at(cx, &[&ring]), Some((0, 0)));
}

#[gpui::test]
fn a_click_on_a_button_chooses_it_and_takes_the_focus(cx: &mut TestAppContext) {
    let (mut cx, list, ring) = question(cx);
    let cx = &mut cx;
    let to = cx.debug_bounds("type-ascertain-to").expect("the button");
    cx.simulate_click(to.center(), gpui::Modifiers::none());
    assert!(!adding(&list, cx));
    assert_eq!(at(cx, &[&ring]), Some((0, 1)));
}

pub(super) fn list_focused(
    list: &gpui::Entity<super::list::TypeList>,
    cx: &mut VisualTestContext,
) -> bool {
    let focus = list.read_with(cx, |l, _| l.focus.clone());
    cx.update(|window, _| focus.is_focused(window))
}

fn list_is_open(cx: &mut VisualTestContext) -> bool {
    let view = view_of(cx);
    view.read_with(cx, |v, _| v.doc_type.list.is_some())
}

#[gpui::test]
fn escape_closes_the_question_and_the_list_keeps_its_keys(cx: &mut TestAppContext) {
    // the focus on the group (one Tab), on the colors (two Tabs), or not moved
    for tabs in [0, 1, 2] {
        let (mut cx, list, _ring) = question(cx);
        let cx = &mut cx;
        for _ in 0..tabs {
            cx.simulate_keystrokes("tab");
        }
        cx.simulate_keystrokes("escape");
        assert!(list.read_with(cx, |l, _| l.ascertain.is_none()), "{tabs}");
        assert!(list_focused(&list, cx), "after Escape, {tabs} Tabs");
        assert!(list_is_open(cx), "{tabs}");
        // the list has its keys: the second Escape closes it
        cx.simulate_keystrokes("escape");
        assert!(!list_is_open(cx), "second Escape, {tabs} Tabs");
    }
}

#[gpui::test]
fn ok_from_a_tab_stop_of_the_question_gives_the_focus_back_to_the_list(cx: &mut TestAppContext) {
    for tabs in [1, 2] {
        let (mut cx, list, _ring) = question(cx);
        let cx = &mut cx;
        for _ in 0..tabs {
            cx.simulate_keystrokes("tab");
        }
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert!(list.read_with(cx, |l, _| l.ascertain.is_none()), "{tabs}");
        // a prompt may report the import: answer it if there is one
        if cx.has_pending_prompt() {
            cx.simulate_prompt_answer("OK");
        }
        assert!(list_focused(&list, cx), "after OK, {tabs} Tabs");
        cx.simulate_keystrokes("escape");
        assert!(!list_is_open(cx), "Escape reaches the list, {tabs} Tabs");
    }
}
