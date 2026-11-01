//! Tests of the horizontal scroll bar (`hscroll.rs`) in the headless window: whether it is there,
//! its range and position against the scroll of the view, and the arrows, the track and the thumb
//! (`CEditView::OnHScroll`).

use gpui::VisualTestContext;
use gpui::{Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext, point, px};

use super::hscroll;
use super::tests::{Fixture, setup};
use crate::geometry::{HSCROLL_ARROW, HSCROLLBAR_HEIGHT, SCROLLBAR_WIDTH};

/// A line of 500 columns among short ones.
fn wide_text() -> String {
    format!("short\n{}\nshort\n", "x".repeat(500))
}

/// Paints until the width of the text is known.
fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn bar(
    f: &Fixture,
    cx: &mut VisualTestContext,
) -> Option<(crate::geometry::HScrollbar, hscroll::BarPaint)> {
    f.view.read_with(cx, |v, _| v.h_scrollbar())
}

fn cell(f: &Fixture, cx: &mut VisualTestContext) -> f32 {
    f.view.read_with(cx, |v, _| v.metrics.cell_width)
}

fn scroll_x(f: &Fixture, cx: &mut VisualTestContext) -> f32 {
    f.view.read_with(cx, |v, _| v.scroll_x)
}

fn press(cx: &mut VisualTestContext, x: f32, y: f32) {
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: point(px(x), px(y)),
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
}

fn release(cx: &mut VisualTestContext, x: f32, y: f32) {
    cx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: point(px(x), px(y)),
        modifiers: Modifiers::default(),
        click_count: 1,
    });
}

/// A point on the bar, `dx` pixels from its left end.
fn on_bar(f: &Fixture, cx: &mut VisualTestContext, dx: f32) -> (f32, f32) {
    let (_, at) = bar(f, cx).expect("the view has the bar");
    (at.left + dx, at.top + HSCROLLBAR_HEIGHT / 2.0)
}

#[gpui::test]
fn the_view_has_the_bar_while_the_setting_is_on(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    assert!(bar(&f, cx).is_some(), "Sakura's default is on");
    cx.update(|_, cx| hscroll::set_shown(cx, false));
    f.view.update(cx, |_, cx| cx.notify());
    settle(cx);
    assert!(bar(&f, cx).is_none());
    assert!(f.view.read_with(cx, |v, _| v.paint.unwrap().hbar.is_none()));
    cx.update(|_, cx| hscroll::set_shown(cx, true));
    f.view.update(cx, |_, cx| cx.notify());
    settle(cx);
    assert!(bar(&f, cx).is_some());
}

#[gpui::test]
fn the_bar_takes_the_bottom_of_the_view(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    let with = f.view.read_with(cx, |v, _| v.metrics.view_height);
    cx.update(|_, cx| hscroll::set_shown(cx, false));
    f.view.update(cx, |_, cx| cx.notify());
    settle(cx);
    let without = f.view.read_with(cx, |v, _| v.metrics.view_height);
    assert_eq!(without - with, HSCROLLBAR_HEIGHT);
}

#[gpui::test]
fn the_range_is_the_widest_line_and_three_columns(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    let (bar, at) = bar(&f, cx).unwrap();
    // GetRightEdgeForScrollBar: no wrap, so the widest line (500) and 3 more.
    assert_eq!(bar.range, 503);
    assert_eq!(bar.page, f.view.read_with(cx, |v, _| v.view_columns()));
    assert!(bar.enabled());
    assert_eq!(bar.pos, 0.0);
    // The bar is as wide as the view without the vertical bar, and the arrows are in it.
    let width = f.view.read_with(cx, |v, _| {
        v.metrics.view_width + v.paint.unwrap().origin_x - at.left + SCROLLBAR_WIDTH
    });
    assert!(
        (at.width + SCROLLBAR_WIDTH - width).abs() < 1.5,
        "{} {}",
        at.width,
        width
    );
}

#[gpui::test]
fn the_position_follows_the_scroll_of_the_view(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    let c = cell(&f, cx);
    f.view.update(cx, |v, _| v.scroll_to_col(40.0));
    let (bar, _) = bar(&f, cx).unwrap();
    assert_eq!(bar.pos, 40.0);
    assert_eq!(scroll_x(&f, cx), 40.0 * c);
    // Not past what ScrollAtH allows: the right edge, the room for the wrap mark and the
    // hanging punctuation, less the columns in view.
    f.view.update(cx, |v, _| v.scroll_to_col(1e6));
    let limit = f.view.read_with(cx, |v, _| {
        (503 + v.prefs.settings.wrap_overhang).saturating_sub(v.view_columns()) as f32
    });
    assert_eq!(scroll_x(&f, cx), limit * c);
}

#[gpui::test]
fn the_arrows_move_four_columns(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    let c = cell(&f, cx);
    let (x, y) = on_bar(&f, cx, HSCROLL_ARROW / 2.0);
    // The left arrow at the left end does nothing.
    press(cx, x, y);
    release(cx, x, y);
    assert_eq!(scroll_x(&f, cx), 0.0);
    let (_, at) = bar(&f, cx).unwrap();
    let (rx, ry) = on_bar(&f, cx, at.width - HSCROLL_ARROW / 2.0);
    press(cx, rx, ry);
    release(cx, rx, ry);
    assert_eq!(scroll_x(&f, cx), 4.0 * c);
    press(cx, rx, ry);
    release(cx, rx, ry);
    assert_eq!(scroll_x(&f, cx), 8.0 * c);
    press(cx, x, y);
    release(cx, x, y);
    assert_eq!(scroll_x(&f, cx), 4.0 * c);
}

#[gpui::test]
fn the_track_moves_a_page(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    let c = cell(&f, cx);
    let (bar0, _) = bar(&f, cx).unwrap();
    let page = bar0.page as f32;
    // Right of the thumb: a page right (SB_PAGERIGHT).
    let (x, y) = on_bar(
        &f,
        cx,
        HSCROLL_ARROW + bar0.thumb_left + bar0.thumb_width + 5.0,
    );
    press(cx, x, y);
    release(cx, x, y);
    assert_eq!(scroll_x(&f, cx), page * c);
    // Left of the thumb: a page left, and not before the first column.
    let (bar1, _) = bar(&f, cx).unwrap();
    let (x, y) = on_bar(&f, cx, HSCROLL_ARROW + bar1.thumb_left - 5.0);
    press(cx, x, y);
    release(cx, x, y);
    assert_eq!(scroll_x(&f, cx), 0.0);
}

#[gpui::test]
fn the_thumb_moves_the_view(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    let c = cell(&f, cx);
    let (bar0, _) = bar(&f, cx).unwrap();
    let (x, y) = on_bar(&f, cx, HSCROLL_ARROW + bar0.thumb_left + 1.0);
    press(cx, x, y);
    // Half of the travel of the thumb is half of the columns that can be scrolled.
    let travel = bar0.track_width - bar0.thumb_width;
    cx.simulate_mouse_move(
        point(px(x + travel / 2.0), px(y)),
        MouseButton::Left,
        Modifiers::default(),
    );
    let half = ((bar0.range - bar0.page) as f32 / 2.0).round();
    assert_eq!(scroll_x(&f, cx), half * c);
    let (bar1, _) = bar(&f, cx).unwrap();
    assert_eq!(bar1.pos, half);
    // Past the end of the track it stops at the last position of the bar.
    // (A pointer can leave the window while the thumb is held; the event is called directly.)
    f.view.update(cx, |v, cx| {
        v.h_scroll_drag(x + travel * 3.0, x, 0.0, cx);
    });
    assert_eq!(scroll_x(&f, cx), (bar0.range - bar0.page) as f32 * c);
    release(cx, x, y);
}

#[gpui::test]
fn a_press_on_the_bar_does_not_move_the_caret(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    let (bar0, at) = bar(&f, cx).unwrap();
    let before = f.caret(cx);
    let (x, y) = on_bar(
        &f,
        cx,
        HSCROLL_ARROW + bar0.thumb_left + bar0.thumb_width + 5.0,
    );
    press(cx, x, y);
    release(cx, x, y);
    // The corner where the two bars meet does nothing.
    let (cx_x, cx_y) = (at.left + at.width + SCROLLBAR_WIDTH / 2.0, at.top + 3.0);
    press(cx, cx_x, cx_y);
    release(cx, cx_x, cx_y);
    assert_eq!(f.caret(cx), before);
    assert!(f.view.read_with(cx, |v, _| v.drag == super::Drag::None));
}

#[gpui::test]
fn a_text_that_fits_has_a_disabled_bar_and_stays_at_the_left(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "short\nlines\n");
    settle(cx);
    let (bar0, _) = bar(&f, cx).unwrap();
    assert!(!bar0.enabled());
    // A press on the track does nothing.
    let (x, y) = on_bar(&f, cx, 100.0);
    press(cx, x, y);
    release(cx, x, y);
    assert_eq!(scroll_x(&f, cx), 0.0);
    // `AdjustScrollBars` scrolls a view back to the left when everything fits.
    f.view.update(cx, |v, _| v.scroll_x = 30.0);
    f.view.update(cx, |_, cx| cx.notify());
    settle(cx);
    assert_eq!(scroll_x(&f, cx), 0.0);
}

#[gpui::test]
fn with_a_wrap_the_range_is_the_wrap_column_and_the_room_after_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    f.view.update(cx, |v, cx| {
        v.prefs.wrap_method = crate::settings::view_settings::WrapMethod::SettingWidth;
        v.prefs.wrap_columns = 300;
        v.relayout_in(cx);
    });
    settle(cx);
    let (bar, _) = bar(&f, cx).unwrap();
    let overhang = f.view.read_with(cx, |v, _| v.prefs.settings.wrap_overhang);
    assert_eq!(bar.range, 300 + overhang);
}

#[gpui::test]
fn a_selection_to_the_right_widens_the_range_when_the_text_does_not_wrap(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\ncd\n");
    settle(cx);
    let before = f.view.read_with(cx, |v, _| v.h_right_edge());
    assert_eq!(before, 2 + 3);
    f.view.update(cx, |v, _| v.h_selection_edge = 400);
    assert_eq!(f.view.read_with(cx, |v, _| v.h_right_edge()), 403);
}

#[gpui::test]
fn editing_the_widest_line_changes_the_range(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &wide_text());
    settle(cx);
    assert_eq!(bar(&f, cx).unwrap().0.range, 503);
    // The widest line goes: the next widest is 5 columns.
    f.view.update(cx, |v, cx| {
        v.run_with(cx, |command_cx| {
            command_cx.replace(
                matsu_core::buffer::Pos::new(1, 0),
                matsu_core::buffer::Pos::new(1, 500),
                "",
            )
        })
        .ok();
    });
    settle(cx);
    assert_eq!(bar(&f, cx).unwrap().0.range, 5 + 3);
}
