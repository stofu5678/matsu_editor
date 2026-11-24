//! 結果出力 and 結果出力形式 of the Grep dialog (`IDD_GREP` in `sakura_core/sakura_rc.rc`): two groups of
//! radio buttons, the first of each with `WS_GROUP | WS_TABSTOP`. Tab stops on each group once at the
//! checked button, the arrow keys move and choose and go round, the digits choose directly.

use gpui::{TestAppContext, VisualTestContext};

use super::access_key_tests::{open, press, setup};
use super::format::{LineType, OutputStyle};
use crate::radio_group::testing::{assert_stops_at_checked, at, tab_to, trail};

/// The order the buttons are drawn in (`IDC_RADIO_OUTPUTLINE`, `_OUTPUTMARKED`, `_NOHIT`).
const LINE_TYPES: [LineType; 3] = [LineType::Line, LineType::MatchPart, LineType::NoHitLine];
const STYLES: [OutputStyle; 3] = [
    OutputStyle::Normal,
    OutputStyle::PerFile,
    OutputStyle::ResultOnly,
];

#[gpui::test]
fn tab_stops_on_each_group_once_at_the_checked_button(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let panel = open(&view, cx, "F_GREP_DIALOG");
    let (lines, styles) =
        panel.read_with(cx, |p, _| (p.line_type_ring.clone(), p.style_ring.clone()));
    // 該当行 and ノーマル are checked at first
    let forward = trail(cx, &[&lines, &styles], "tab", 60);
    assert_stops_at_checked(&forward, &[0, 0]);
    let back = trail(cx, &[&lines, &styles], "shift-tab", 60);
    assert_stops_at_checked(&back, &[0, 0]);
    // the stops follow the choices (the digits are the access keys)
    press(cx, "alt-3");
    press(cx, "alt-6");
    let forward = trail(cx, &[&lines, &styles], "tab", 60);
    assert_stops_at_checked(&forward, &[2, 2]);
}

#[gpui::test]
fn the_arrow_keys_move_choose_and_go_round_in_both_groups(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let panel = open(&view, cx, "F_GREP_DIALOG");
    let (lines, styles) =
        panel.read_with(cx, |p, _| (p.line_type_ring.clone(), p.style_ring.clone()));
    let chosen = |cx: &mut VisualTestContext| {
        panel.read_with(cx, |p, _| (p.settings.line_type, p.settings.style))
    };
    assert_eq!(tab_to(cx, &[&lines, &styles], 0, 60), 0);
    for (key, to) in [
        ("down", 1),
        ("right", 2),
        // round the end to the first one, and from there to the last
        ("down", 0),
        ("up", 2),
        ("left", 1),
    ] {
        press(cx, key);
        assert_eq!(chosen(cx), (LINE_TYPES[to], OutputStyle::Normal), "{key}");
        assert_eq!(at(cx, &[&lines, &styles]), Some((0, to)), "{key}");
    }
    // Tab leaves the group, and Shift+Tab comes back to the one that is chosen now
    press(cx, "tab");
    assert_ne!(at(cx, &[&lines, &styles]).map(|stop| stop.0), Some(0));
    assert_eq!(tab_to(cx, &[&lines, &styles], 0, 60), 1);
    // the group of the form
    assert_eq!(tab_to(cx, &[&lines, &styles], 1, 60), 0);
    for (key, to) in [("down", 1), ("down", 2), ("down", 0), ("up", 2)] {
        press(cx, key);
        assert_eq!(chosen(cx), (LineType::MatchPart, STYLES[to]), "{key}");
        assert_eq!(at(cx, &[&lines, &styles]), Some((1, to)), "{key}");
    }
}
