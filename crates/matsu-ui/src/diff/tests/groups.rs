//! The two groups of radio buttons of the DIFF dialog (`IDD_DIFF`: 比較するファイル
//! `IDC_RADIO_DIFF_DST1`, `IDC_RADIO_DIFF_DST2` and 編集中ファイルが... `IDC_RADIO_DIFF_FILE1`,
//! `IDC_RADIO_DIFF_FILE2`; the first of each has `WS_GROUP | WS_TABSTOP`): Tab stops on each group
//! once at the checked button, the arrow keys move and choose and go round, the access keys choose.

use gpui::TestAppContext;

use super::{Target, Win, remember_options, window};
use crate::diff::panel::DiffPanel;
use crate::radio_group::Ring;
use crate::radio_group::testing::{assert_stops_at_checked, at, tab_to, trail};

/// The dialog of `w` with another window to compare with, and the two rings of its groups.
fn open_with_another_window(
    cx: &mut TestAppContext,
) -> (Win, Win, gpui::Entity<DiffPanel>, Ring, Ring) {
    remember_options(cx, 0, false);
    let w = window(cx, "a\nb\n", None);
    let other = window(cx, "a\nc\n", None);
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    let panel = w.view.read_with(cx, |v, _| v.diff.panel.clone()).unwrap();
    let (destination, side) =
        panel.read_with(cx, |p, _| (p.destination_ring.clone(), p.side_ring.clone()));
    (w, other, panel, destination, side)
}

/// 0: 外部ファイルを指定, 1: 他の編集中のファイル
fn destination(panel: &gpui::Entity<DiffPanel>, cx: &mut TestAppContext) -> usize {
    usize::from(matches!(
        panel.read_with(cx, |p, cx| p.target(cx)),
        Some(Target::Window(_))
    ))
}

/// 0: 新ファイル, 1: 旧ファイル
fn side(panel: &gpui::Entity<DiffPanel>, cx: &mut TestAppContext) -> usize {
    panel.read_with(cx, |p, _| usize::from(p.flags() & 0x20 != 0))
}

#[gpui::test]
fn tab_stops_on_each_group_once_at_the_checked_button(cx: &mut TestAppContext) {
    let (w, _other, panel, dst, own) = open_with_another_window(cx);
    let checked = [destination(&panel, cx), side(&panel, cx)];
    assert_eq!(checked[1], 0, "新ファイル is checked at first");
    let keys = w.cx(cx);
    let forward = trail(keys, &[&dst, &own], "tab", 40);
    assert_stops_at_checked(&forward, &checked);
    let back = trail(keys, &[&dst, &own], "shift-tab", 40);
    assert_stops_at_checked(&back, &checked);
    // the stop of 編集中ファイルが... follows the choice
    keys.simulate_keystrokes("alt-d");
    let forward = trail(keys, &[&dst, &own], "tab", 40);
    assert_stops_at_checked(&forward, &[checked[0], 1]);
}

#[gpui::test]
fn the_arrow_keys_choose_the_file_or_the_window_and_go_round(cx: &mut TestAppContext) {
    let (w, _other, panel, dst, own) = open_with_another_window(cx);
    let start = destination(&panel, cx);
    let keys = w.cx(cx);
    assert_eq!(tab_to(keys, &[&dst, &own], 0, 40), start);
    // two buttons: every arrow key goes to the other one, round the end
    let mut now = start;
    for key in ["down", "up", "right", "left", "down"] {
        now = 1 - now;
        w.cx(cx).simulate_keystrokes(key);
        assert_eq!(destination(&panel, cx), now, "{key}");
        assert_eq!(at(w.cx(cx), &[&dst, &own]), Some((0, now)), "{key}");
    }
    // Tab leaves the group, and Shift+Tab comes back to the one that is chosen now
    w.cx(cx).simulate_keystrokes("tab");
    assert_ne!(at(w.cx(cx), &[&dst, &own]).map(|stop| stop.0), Some(0));
    assert_eq!(tab_to(w.cx(cx), &[&dst, &own], 0, 40), now);
}

#[gpui::test]
fn the_arrow_keys_choose_the_new_or_the_old_file_and_go_round(cx: &mut TestAppContext) {
    let (w, _other, panel, dst, own) = open_with_another_window(cx);
    assert_eq!(tab_to(w.cx(cx), &[&dst, &own], 1, 40), 0);
    for (key, now) in [
        ("down", 1),
        ("down", 0),
        ("up", 1),
        ("left", 0),
        ("right", 1),
    ] {
        w.cx(cx).simulate_keystrokes(key);
        assert_eq!(side(&panel, cx), now, "{key}");
        assert_eq!(at(w.cx(cx), &[&dst, &own]), Some((1, now)), "{key}");
    }
    // the access keys of the two buttons choose them
    // and they take the focus to the button too
    w.cx(cx).simulate_keystrokes("tab");
    assert_ne!(at(w.cx(cx), &[&dst, &own]).map(|stop| stop.0), Some(1));
    w.cx(cx).simulate_keystrokes("alt-n");
    assert_eq!(side(&panel, cx), 0);
    assert_eq!(at(w.cx(cx), &[&dst, &own]), Some((1, 0)));
    w.cx(cx).simulate_keystrokes("alt-d");
    assert_eq!(side(&panel, cx), 1);
    assert_eq!(at(w.cx(cx), &[&dst, &own]), Some((1, 1)));
}

#[gpui::test]
fn without_another_file_the_window_button_cannot_be_reached(cx: &mut TestAppContext) {
    // `CDlgDiff::SetData` with `count == 0`: 外部ファイルを指定 is checked and the other one is
    // greyed out
    remember_options(cx, 0, false);
    let w = window(cx, "a\nb\n", None);
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    let panel = w.view.read_with(cx, |v, _| v.diff.panel.clone()).unwrap();
    let (dst, own) = panel.read_with(cx, |p, _| (p.destination_ring.clone(), p.side_ring.clone()));
    assert_eq!(tab_to(w.cx(cx), &[&dst, &own], 0, 40), 0);
    for key in ["down", "right", "up"] {
        w.cx(cx).simulate_keystrokes(key);
        assert_eq!(at(w.cx(cx), &[&dst, &own]), Some((0, 0)), "{key}");
        assert_eq!(destination(&panel, cx), 0, "{key}");
    }
}
