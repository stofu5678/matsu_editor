//! Up or Down held in the box of a number (`spin.rs`, `KeyHold`): key-downs with no key-up between
//! them, sent through the real key dispatch (`Window::dispatch_key_event`), with the test clock moved
//! between them. The box is フォント高 (7 to 200). It has the focus after a click on its arrow.

use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{Number, open, setup, shown, type_number};
use crate::print_setting::dialog::PrintSettingDialog;

fn focus_box(cx: &mut VisualTestContext) {
    let bounds = cx
        .debug_bounds("print-setting-spin-0-up")
        .expect("the up arrow");
    cx.simulate_click(bounds.center(), gpui::Modifiers::none());
    cx.run_until_parked();
}

fn down(cx: &mut VisualTestContext, key: &str, held: bool) {
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
        is_held: held,
        prefer_character_input: false,
    });
    cx.run_until_parked();
}

fn up(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    cx.run_until_parked();
}

fn wait(cx: &mut VisualTestContext, ms: u64) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(ms));
}

/// One key-down every 100 ms for `ms` (the first at once), no key-up.
fn hold(cx: &mut VisualTestContext, key: &str, ms: u64) {
    for n in 0..=ms / 100 {
        if n > 0 {
            wait(cx, 100);
        }
        down(cx, key, n > 0);
    }
}

fn number(dialog: &Entity<PrintSettingDialog>, cx: &mut VisualTestContext) -> i32 {
    shown(dialog, Number::FontHeight, cx).parse().unwrap()
}

fn start<'a>(
    cx: &'a mut TestAppContext,
    value: &str,
) -> (Entity<PrintSettingDialog>, &'a mut VisualTestContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    cx.run_until_parked();
    focus_box(cx);
    type_number(&dialog, Number::FontHeight, value, cx);
    (dialog, cx)
}

#[gpui::test]
fn up_goes_by_one_then_by_five_lined_up_then_by_twenty_lined_up(cx: &mut TestAppContext) {
    let (dialog, cx) = start(cx, "7");
    // 0 to 1.9 s: by 1 (20 key-downs)
    hold(cx, "up", 1900);
    assert_eq!(number(&dialog, cx), 27);
    // 2.0 s: by 5, lined up on a multiple of 5 (27 + 5 goes to 35), then by whole steps
    wait(cx, 100);
    down(cx, "up", true);
    assert_eq!(number(&dialog, cx), 35);
    wait(cx, 100);
    down(cx, "up", true);
    assert_eq!(number(&dialog, cx), 40);
}

#[gpui::test]
fn after_five_seconds_it_goes_by_twenty_lined_up(cx: &mut TestAppContext) {
    // one key-down a second (the longest first repeat delay of Windows), so that the steps
    // 1, 5 and 20 each show: at 0 s 7 goes to 8, 1 s 9, 2 s (by 5) 15, 3 s 20, 4 s 25,
    // 5 s (by 20) 25 + 20 = 45 goes to 60, 6 s 80
    let (dialog, cx) = start(cx, "7");
    let mut seen = vec![];
    for second in 0..=6 {
        if second > 0 {
            wait(cx, 1000);
        }
        down(cx, "up", second > 0);
        seen.push(number(&dialog, cx));
    }
    assert_eq!(seen, [8, 9, 15, 20, 25, 60, 80]);
}

#[gpui::test]
fn a_key_up_and_a_new_press_start_at_one_again(cx: &mut TestAppContext) {
    let (dialog, cx) = start(cx, "10");
    hold(cx, "up", 2500);
    let held = number(&dialog, cx);
    assert_eq!(held % 5, 0);
    up(cx, "up");
    wait(cx, 100);
    down(cx, "up", false);
    assert_eq!(number(&dialog, cx), held + 1);
    wait(cx, 100);
    down(cx, "up", true);
    assert_eq!(number(&dialog, cx), held + 2);
}

#[gpui::test]
fn down_goes_by_five_lined_down_after_two_seconds(cx: &mut TestAppContext) {
    let (dialog, cx) = start(cx, "100");
    hold(cx, "down", 1900);
    assert_eq!(number(&dialog, cx), 80);
    wait(cx, 100);
    down(cx, "down", true);
    // 80 - 5 is 75, a multiple of 5 already
    assert_eq!(number(&dialog, cx), 75);
    // a number set by hand during the hold: floor(73 / 5) * 5
    type_number(&dialog, Number::FontHeight, "78", cx);
    wait(cx, 100);
    down(cx, "down", true);
    assert_eq!(number(&dialog, cx), 70);
}

#[gpui::test]
fn a_wait_longer_than_a_repeat_ends_a_hold_whose_key_up_was_missed(cx: &mut TestAppContext) {
    let (dialog, cx) = start(cx, "10");
    hold(cx, "up", 2500);
    let held = number(&dialog, cx);
    wait(cx, 2000);
    down(cx, "up", false);
    assert_eq!(number(&dialog, cx), held + 1);
}

#[gpui::test]
fn the_other_direction_starts_a_new_hold(cx: &mut TestAppContext) {
    let (dialog, cx) = start(cx, "10");
    hold(cx, "up", 2500);
    let held = number(&dialog, cx);
    wait(cx, 100);
    down(cx, "down", false);
    assert_eq!(number(&dialog, cx), held - 1);
}

#[gpui::test]
fn the_hold_ends_when_the_window_is_deactivated(cx: &mut TestAppContext) {
    let (dialog, cx) = start(cx, "10");
    // a window that is active, so that the deactivation is a change the box sees
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    hold(cx, "up", 2500);
    let held = number(&dialog, cx);
    assert_eq!(held % 5, 0);
    // the key-up goes to another program, and the window comes back within `KEY_GAP`
    cx.deactivate_window();
    wait(cx, 100);
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    down(cx, "up", false);
    assert_eq!(number(&dialog, cx), held + 1);
}
