//! Tests of the vertical scroll bar's arrows and of holding the arrows and the track of either bar
//! down (`vscroll.rs`, `scroll_repeat.rs`), in the headless window. The clock of the test executor
//! stands in for time.

use std::time::Duration;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

use super::hscroll_tests::{press, release, settle};
use super::scroll_repeat::{FIRST_DELAY, INTERVAL};
use super::tests::{Fixture, setup};
use crate::geometry::{BarPart, HSCROLL_ARROW, Scrollbar, VSCROLL_ARROW, max_scroll_y};

fn long_text() -> String {
    (0..400).map(|i| format!("line {i}\n")).collect()
}

/// Two long lines among short ones, so that the horizontal bar has something to scroll.
fn wide_text() -> String {
    format!("short\n{}\nshort\n", "x".repeat(2000))
}

fn scroll_y(f: &Fixture, cx: &mut VisualTestContext) -> f32 {
    f.view.read_with(cx, |v, _| v.scroll_y)
}

fn line_height(f: &Fixture, cx: &mut VisualTestContext) -> f32 {
    f.view.read_with(cx, |v, _| v.metrics.line_height)
}

/// A point on the vertical bar, `dy` pixels from its top end (negative: from the bottom end).
fn on_vbar(f: &Fixture, cx: &mut VisualTestContext, dy: f32) -> (f32, f32) {
    f.view.read_with(cx, |v, _| {
        let paint = v.paint.expect("painted");
        let y = if dy >= 0.0 {
            paint.vbar_top + dy
        } else {
            paint.vbar_top + paint.vbar_len + dy
        };
        (paint.scrollbar_left + 3.0, y)
    })
}

fn wait(cx: &mut VisualTestContext, time: Duration) {
    cx.executor().advance_clock(time);
    cx.run_until_parked();
}

fn rows_per_arrow(cx: &mut VisualTestContext) -> f32 {
    cx.update(|_, cx| crate::common_settings::scroll_options(cx).repeated_lines as f32)
}

#[gpui::test]
fn the_vertical_bar_has_arrows_that_move_the_rows_of_a_held_key(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    settle(cx);
    let lh = line_height(&f, cx);
    let rows = rows_per_arrow(cx);
    assert!(rows >= 1.0);
    let (x, down) = on_vbar(&f, cx, -VSCROLL_ARROW / 2.0);
    let (_, up) = on_vbar(&f, cx, VSCROLL_ARROW / 2.0);
    // `OnVScroll`: `SB_LINEUP` at the first row does nothing.
    press(cx, x, up);
    release(cx, x, up);
    assert_eq!(scroll_y(&f, cx), 0.0);
    // `SB_LINEDOWN`, `nRepeatedScrollLineNum` rows.
    press(cx, x, down);
    release(cx, x, down);
    assert_eq!(scroll_y(&f, cx), rows * lh);
    press(cx, x, down);
    release(cx, x, down);
    assert_eq!(scroll_y(&f, cx), 2.0 * rows * lh);
    press(cx, x, up);
    release(cx, x, up);
    assert_eq!(scroll_y(&f, cx), rows * lh);
}

#[gpui::test]
fn the_down_arrow_stops_at_the_end(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    settle(cx);
    let limit = f
        .view
        .read_with(cx, |v, _| max_scroll_y(&v.metrics, v.layout.row_count()));
    f.view.update(cx, |v, _| v.scroll_y = limit - 1.0);
    let (x, down) = on_vbar(&f, cx, -VSCROLL_ARROW / 2.0);
    press(cx, x, down);
    release(cx, x, down);
    assert_eq!(scroll_y(&f, cx), limit);
    press(cx, x, down);
    release(cx, x, down);
    assert_eq!(scroll_y(&f, cx), limit);
}

#[gpui::test]
fn the_thumb_and_the_track_sit_between_the_arrows(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    settle(cx);
    let (track, bar_len) = f.view.read_with(cx, |v, _| {
        let bar = Scrollbar::new(&v.metrics, v.layout.row_count(), v.scroll_y);
        (bar.track_height, v.paint.unwrap().vbar_len)
    });
    assert_eq!(bar_len - 2.0 * VSCROLL_ARROW, track);
    // A press on the thumb takes it, right under the top arrow.
    let (x, y) = on_vbar(&f, cx, VSCROLL_ARROW + 1.0);
    press(cx, x, y);
    assert!(
        f.view
            .read_with(cx, |v, _| matches!(v.drag, super::Drag::Thumb { .. }))
    );
    release(cx, x, y);
}

#[gpui::test]
fn a_held_arrow_moves_at_once_waits_and_then_repeats(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    settle(cx);
    let step = rows_per_arrow(cx) * line_height(&f, cx);
    let (x, down) = on_vbar(&f, cx, -VSCROLL_ARROW / 2.0);
    press(cx, x, down);
    // At once.
    assert_eq!(scroll_y(&f, cx), step);
    // Not before the first wait is over.
    wait(cx, FIRST_DELAY - Duration::from_millis(1));
    assert_eq!(scroll_y(&f, cx), step);
    wait(cx, Duration::from_millis(1));
    assert_eq!(scroll_y(&f, cx), 2.0 * step);
    // Then every interval.
    wait(cx, INTERVAL - Duration::from_millis(1));
    assert_eq!(scroll_y(&f, cx), 2.0 * step);
    wait(cx, Duration::from_millis(1));
    assert_eq!(scroll_y(&f, cx), 3.0 * step);
    wait(cx, INTERVAL * 4);
    assert_eq!(scroll_y(&f, cx), 7.0 * step);
    // Letting go ends it.
    release(cx, x, down);
    wait(cx, INTERVAL * 10);
    assert_eq!(scroll_y(&f, cx), 7.0 * step);
}

#[gpui::test]
fn a_held_arrow_pauses_while_the_pointer_is_off_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    settle(cx);
    let step = rows_per_arrow(cx) * line_height(&f, cx);
    let (x, down) = on_vbar(&f, cx, -VSCROLL_ARROW / 2.0);
    press(cx, x, down);
    // Off the arrow, onto the track.
    let (_, track) = on_vbar(&f, cx, -VSCROLL_ARROW - 20.0);
    cx.simulate_mouse_move(
        point(px(x), px(track)),
        MouseButton::Left,
        Modifiers::default(),
    );
    wait(cx, FIRST_DELAY + INTERVAL * 3);
    assert_eq!(scroll_y(&f, cx), step);
    // Back on it: it goes on.
    cx.simulate_mouse_move(
        point(px(x), px(down)),
        MouseButton::Left,
        Modifiers::default(),
    );
    wait(cx, INTERVAL);
    assert_eq!(scroll_y(&f, cx), 2.0 * step);
    release(cx, x, down);
}

#[gpui::test]
fn a_held_track_pages_until_the_thumb_comes_under_the_pointer(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    settle(cx);
    let page = f.view.read_with(cx, |v, _| {
        v.metrics.line_height * (v.metrics.visible_rows() as f32 - 1.0)
    });
    // A point on the track a third of the way from the thumb to the end.
    let (track, thumb) = f.view.read_with(cx, |v, _| {
        let bar = Scrollbar::new(&v.metrics, v.layout.row_count(), v.scroll_y);
        (bar.track_height, bar.thumb_height)
    });
    let (x, y) = on_vbar(&f, cx, VSCROLL_ARROW + thumb + (track - thumb) / 3.0);
    let pointer = f.view.read_with(cx, |v, _| y - v.paint.unwrap().vbar_top);
    assert_eq!(
        f.view.read_with(cx, |v, cx| v.v_part_at(pointer, cx)),
        Some(BarPart::PageForward)
    );
    press(cx, x, y);
    assert_eq!(scroll_y(&f, cx), page);
    wait(cx, FIRST_DELAY);
    wait(cx, INTERVAL * 60);
    let stopped = scroll_y(&f, cx);
    // It went on a page at a time, and it did not go to the end: the thumb is under the pointer.
    assert!(stopped >= 2.0 * page, "{stopped}");
    let limit = f
        .view
        .read_with(cx, |v, _| max_scroll_y(&v.metrics, v.layout.row_count()));
    assert!(stopped < limit, "{stopped} {limit}");
    assert_eq!(
        f.view.read_with(cx, |v, cx| v.v_part_at(pointer, cx)),
        Some(BarPart::Thumb)
    );
    // It stays there however long it is held.
    wait(cx, INTERVAL * 20);
    assert_eq!(scroll_y(&f, cx), stopped);
    release(cx, x, y);
}

#[gpui::test]
fn the_horizontal_arrows_and_track_repeat_too(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    let c = f.view.read_with(cx, |v, _| v.metrics.cell_width);
    let x_of = |f: &Fixture, cx: &mut VisualTestContext, dx: f32| {
        f.view.read_with(cx, |v, _| {
            let at = v.paint.unwrap().hbar.unwrap();
            (at.left + dx, at.top + 5.0)
        })
    };
    let width = f
        .view
        .read_with(cx, |v, _| v.paint.unwrap().hbar.unwrap().width);
    let (x, y) = x_of(&f, cx, width - HSCROLL_ARROW / 2.0);
    press(cx, x, y);
    assert_eq!(f.view.read_with(cx, |v, _| v.scroll_x), 4.0 * c);
    wait(cx, FIRST_DELAY - Duration::from_millis(1));
    assert_eq!(f.view.read_with(cx, |v, _| v.scroll_x), 4.0 * c);
    wait(cx, Duration::from_millis(1));
    assert_eq!(f.view.read_with(cx, |v, _| v.scroll_x), 8.0 * c);
    wait(cx, INTERVAL);
    assert_eq!(f.view.read_with(cx, |v, _| v.scroll_x), 12.0 * c);
    release(cx, x, y);
    wait(cx, INTERVAL * 5);
    assert_eq!(f.view.read_with(cx, |v, _| v.scroll_x), 12.0 * c);

    // The track: pages right until the thumb reaches the pointer.
    f.view.update(cx, |v, _| v.scroll_x = 0.0);
    let (bar0, _) = f.view.read_with(cx, |v, _| v.h_scrollbar().unwrap());
    let third = bar0.thumb_width + (bar0.track_width - bar0.thumb_width) / 3.0;
    let (px_, py) = x_of(&f, cx, HSCROLL_ARROW + third);
    let pointer = px_
        - f.view
            .read_with(cx, |v, _| v.paint.unwrap().hbar.unwrap().left);
    press(cx, px_, py);
    wait(cx, FIRST_DELAY + INTERVAL * 80);
    let (bar, _) = f.view.read_with(cx, |v, _| v.h_scrollbar().unwrap());
    assert_eq!(bar.part_at(pointer), BarPart::Thumb);
    assert!(bar.pos > 0.0 && bar.pos < (bar.range - bar.page) as f32);
    let held = f.view.read_with(cx, |v, _| v.scroll_x);
    wait(cx, INTERVAL * 10);
    assert_eq!(f.view.read_with(cx, |v, _| v.scroll_x), held);
    release(cx, px_, py);
}
