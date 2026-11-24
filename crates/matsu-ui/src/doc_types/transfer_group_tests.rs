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
fn question(
    cx: &mut TestAppContext,
) -> (VisualTestContext, gpui::Entity<super::list::TypeList>, Ring) {
    let (ini, handle) = setup(cx, "");
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

fn adding(list: &gpui::Entity<super::list::TypeList>, cx: &mut VisualTestContext) -> bool {
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
    // before the group has the focus, the arrow keys are the colors' (they move the list)
    cx.simulate_keystrokes("down");
    assert_eq!(
        list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().color),
        1
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
        1
    );
    // The group is the only Tab stop of the question (the colors and the buttons are not), so
    // Tab and Shift+Tab stay at the button that is chosen now, and never on the other one
    cx.simulate_keystrokes("tab");
    assert_eq!(at(cx, &[&ring]), Some((0, 1)));
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(at(cx, &[&ring]), Some((0, 1)));
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
