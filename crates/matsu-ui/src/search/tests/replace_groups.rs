//! The two groups of radio buttons of the replace panel (`IDD_REPLACE`: 置換対象
//! `IDC_RADIO_REPLACE`.. `IDC_RADIO_LINEDELETE` and 範囲 `IDC_RADIO_SELECTEDAREA`,
//! `IDC_RADIO_ALLAREA`; the first of each has `WS_GROUP | WS_TABSTOP`): Tab stops on each group once
//! at the checked button, the arrow keys move and choose and go round, the access keys choose.

use gpui::{TestAppContext, VisualTestContext};
use matsu_core::search::ReplaceTarget;

use super::{Fixture, setup};
use crate::radio_group::Ring;
use crate::radio_group::testing::{assert_stops_at_checked, at, tab_to, trail};

const TARGETS: [ReplaceTarget; 4] = [
    ReplaceTarget::Match,
    ReplaceTarget::InsertBefore,
    ReplaceTarget::InsertAfter,
    ReplaceTarget::DeleteLine,
];

fn rings(f: &Fixture, cx: &mut VisualTestContext) -> (Ring, Ring) {
    f.view.read_with(cx, |v, cx| {
        let panel = v.panel.clone().unwrap();
        let panel = panel.read(cx);
        (panel.target_ring.clone(), panel.area_ring.clone())
    })
}

fn chosen(f: &Fixture, cx: &mut VisualTestContext) -> (ReplaceTarget, bool) {
    f.view.read_with(cx, |v, _| {
        (v.search.replace.target, v.search.replace.in_selection)
    })
}

#[gpui::test]
fn tab_stops_on_each_group_once_at_the_checked_button(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aa");
    cx.simulate_keystrokes("ctrl-r");
    let (targets, areas) = rings(&f, cx);
    // 選択文字 and ファイル全体 are checked at first
    let forward = trail(cx, &[&targets, &areas], "tab", 40);
    assert_stops_at_checked(&forward, &[0, 1]);
    let back = trail(cx, &[&targets, &areas], "shift-tab", 40);
    assert_stops_at_checked(&back, &[0, 1]);
    // the stops are the checked buttons after a choice too
    cx.simulate_keystrokes("alt-2");
    cx.simulate_keystrokes("alt-s");
    assert_eq!(chosen(&f, cx), (ReplaceTarget::InsertAfter, true));
    let forward = trail(cx, &[&targets, &areas], "tab", 40);
    assert_stops_at_checked(&forward, &[2, 0]);
}

#[gpui::test]
fn the_arrow_keys_move_among_the_targets_choose_and_go_round(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aa");
    cx.simulate_keystrokes("ctrl-r");
    let (targets, areas) = rings(&f, cx);
    assert_eq!(tab_to(cx, &[&targets, &areas], 0, 40), 0);
    for (key, to) in [
        ("down", 1),
        ("right", 2),
        ("down", 3),
        // round the end to the first one, and from there to the last
        ("down", 0),
        ("up", 3),
        ("left", 2),
    ] {
        cx.simulate_keystrokes(key);
        assert_eq!(chosen(&f, cx), (TARGETS[to], false), "{key}");
        assert_eq!(at(cx, &[&targets, &areas]), Some((0, to)), "{key}");
    }
    // Tab leaves the group, and Shift+Tab comes back to the one that is chosen now
    cx.simulate_keystrokes("tab");
    assert_ne!(at(cx, &[&targets, &areas]).map(|stop| stop.0), Some(0));
    assert_eq!(tab_to(cx, &[&targets, &areas], 0, 40), 2);
}

#[gpui::test]
fn the_arrow_keys_move_between_the_two_areas_and_go_round(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aa");
    cx.simulate_keystrokes("ctrl-r");
    let (targets, areas) = rings(&f, cx);
    // ファイル全体 is the second button and is checked at first
    assert_eq!(tab_to(cx, &[&targets, &areas], 1, 40), 1);
    for (key, selection, to) in [
        // round the end to the first one (選択範囲), and back to the last
        ("down", true, 0),
        ("up", false, 1),
        ("left", true, 0),
        ("right", false, 1),
    ] {
        cx.simulate_keystrokes(key);
        assert_eq!(chosen(&f, cx), (ReplaceTarget::Match, selection), "{key}");
        assert_eq!(at(cx, &[&targets, &areas]), Some((1, to)), "{key}");
    }
}

#[gpui::test]
fn the_access_keys_choose_each_button(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aa");
    cx.simulate_keystrokes("ctrl-r");
    for (key, expected) in [
        ("alt-1", (ReplaceTarget::InsertBefore, false)),
        ("alt-3", (ReplaceTarget::DeleteLine, false)),
        ("alt-s", (ReplaceTarget::DeleteLine, true)),
        ("alt-0", (ReplaceTarget::Match, true)),
        ("alt-o", (ReplaceTarget::Match, false)),
    ] {
        cx.simulate_keystrokes(key);
        assert_eq!(chosen(&f, cx), expected, "{key}");
    }
}
