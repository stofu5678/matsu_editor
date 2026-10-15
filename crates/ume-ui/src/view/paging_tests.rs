//! Tests of paging and scrolling (`paging.rs`) in the headless window. The expectations follow
//! `Command_1PageUp/Down`, `Command_HalfPageUp/Down`, `Command_WndScrollUp/Down`, `MoveViewTopLine`
//! and `OnMOUSEWHEEL2` of Sakura (see `paging.rs` for the files).
//!
//! The text area has the size that the test window's last paint measured, so the tests ask the view
//! for its number of rows (`n`) and expect results in terms of it.

use gpui::{
    AppContext as _, Entity, Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase,
    VisualTestContext, point, px,
};
use ume_commands::keymap::{Chord, Key, MouseAction, Resolved};
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use super::EditorView;
use crate::document::SharedDocument;
use crate::geometry::max_scroll_y;

const F_1PAGE_UP: u16 = 30342;
const F_1PAGE_DOWN: u16 = 30343;
const F_CURLINECENTER: u16 = 30360;
const F_CURLINETOP: u16 = 30361;
const F_CURLINEBOTTOM: u16 = 30362;
const F_WND_SCROLL_DOWN: u16 = 30380;
const F_WND_SCROLL_UP: u16 = 30381;

const LINES: usize = 1000;

/// The lines one notch of the wheel is reported as: GPUI's Windows and Linux backends multiply a
/// notch by the lines per notch (3 by default on Windows, 3 on Linux).
const NOTCH: f32 = if cfg!(target_os = "macos") { 1.0 } else { 3.0 };

fn setup(cx: &mut TestAppContext) -> (Entity<EditorView>, &mut VisualTestContext) {
    let text = (0..LINES)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let doc = Document::from_utf8(text.into_bytes()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.run_until_parked();
    (view, vcx)
}

/// The rows of the view.
fn rows(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> usize {
    view.read_with(cx, |v, _| v.metrics().visible_rows())
}

fn max_top(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> usize {
    view.read_with(cx, |v, _| {
        (max_scroll_y(v.metrics(), v.layout().row_count()) / v.metrics().line_height).round()
            as usize
    })
}

fn run(view: &Entity<EditorView>, cx: &mut VisualTestContext, code: u16) {
    let view = view.clone();
    cx.update(|window, cx| view.update(cx, |v, cx| v.run_function(code, &[], window, cx)));
}

fn place(view: &Entity<EditorView>, cx: &mut VisualTestContext, line: usize, top: usize) {
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(Pos::new(line, 0), false))
            .unwrap();
        v.scroll_top_to(top as isize, cx);
    });
}

fn caret_line(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> usize {
    view.read_with(cx, |v, _| v.cursor.caret().line)
}

fn top(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> usize {
    view.read_with(cx, |v, _| v.top_row())
}

fn at(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> (usize, usize) {
    (caret_line(view, cx), top(view, cx))
}

#[gpui::test]
fn one_page_moves_the_caret_and_the_view_by_a_page_less_one_row(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    assert!((3..200).contains(&n), "n = {n}");
    place(&view, cx, 100, 98);
    run(&view, cx, F_1PAGE_DOWN);
    // A page less one row; the caret keeps its place on the screen.
    assert_eq!(at(&view, cx), (100 + n - 1, 98 + n - 1));
    run(&view, cx, F_1PAGE_UP);
    assert_eq!(at(&view, cx), (100, 98));
    assert_eq!(view.read_with(cx, |v, cx| v.status(cx).notice), None);
}

#[gpui::test]
fn one_page_scrolls_even_when_the_caret_cannot_move(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    // Sakura 2014.01.10 Moca: the view scrolls when the caret is already on the first row.
    place(&view, cx, 0, 2 * n);
    run(&view, cx, F_1PAGE_UP);
    assert_eq!(at(&view, cx), (0, 2 * n - (n - 1)));
}

#[gpui::test]
fn one_page_stops_at_the_end_of_the_document(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    let end = max_top(&view, cx);
    place(&view, cx, LINES - 3, end - 2);
    run(&view, cx, F_1PAGE_DOWN);
    assert_eq!(at(&view, cx), (LINES - 1, end));
    run(&view, cx, F_1PAGE_DOWN);
    assert_eq!(at(&view, cx), (LINES - 1, end));
    // The last row stays in view.
    assert!(end + n > LINES - 1);
}

#[gpui::test]
fn the_page_keys_and_their_selecting_variants(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    cx.simulate_keystrokes("pagedown");
    assert_eq!(at(&view, cx), (n - 1, n - 1));
    assert!(!view.read_with(cx, |v, _| v.cursor.has_selection()));
    cx.simulate_keystrokes("shift-pagedown");
    assert_eq!(at(&view, cx), (2 * (n - 1), 2 * (n - 1)));
    let selection = view.read_with(cx, |v, _| v.cursor.selection());
    assert_eq!(
        selection,
        Some((Pos::new(n - 1, 0), Pos::new(2 * (n - 1), 0)))
    );
    cx.simulate_keystrokes("shift-pageup");
    assert_eq!(at(&view, cx), (n - 1, n - 1));
}

#[gpui::test]
fn half_a_page_moves_only_the_caret(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    cx.simulate_keystrokes("secondary-pagedown");
    // The caret is still in view, so the view stays.
    assert_eq!(at(&view, cx), (n / 2, 0));
    cx.simulate_keystrokes("secondary-shift-pagedown");
    assert_eq!(caret_line(&view, cx), 2 * (n / 2));
    assert!(view.read_with(cx, |v, _| v.cursor.has_selection()));
    cx.simulate_keystrokes("secondary-pageup");
    assert_eq!(caret_line(&view, cx), n / 2);
}

#[gpui::test]
fn the_caret_row_goes_to_the_top_the_middle_or_the_bottom(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    place(&view, cx, 500, 0);
    run(&view, cx, F_CURLINETOP);
    assert_eq!(top(&view, cx), 500);
    run(&view, cx, F_CURLINECENTER);
    assert_eq!(top(&view, cx), 500 - n / 2);
    run(&view, cx, F_CURLINEBOTTOM);
    assert_eq!(top(&view, cx), 500 - (n - 1)); // the caret is on the last row in view
    place(&view, cx, 1, 0);
    run(&view, cx, F_CURLINEBOTTOM);
    assert_eq!(top(&view, cx), 0); // never above the first row
    assert_eq!(caret_line(&view, cx), 1);
}

#[gpui::test]
fn the_middle_of_the_view_works_near_the_end_of_the_document(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    place(&view, cx, LINES - 1, 0);
    run(&view, cx, F_CURLINECENTER);
    // MoveViewTopLine has no upper limit: the last row is the middle one, past the scroll range.
    let middle = LINES - 1 - n / 2;
    assert_eq!(top(&view, cx), middle);
    assert!(middle > max_top(&view, cx));
    // Typing keeps the caret in view and the top where it is.
    cx.simulate_input("x");
    assert_eq!(top(&view, cx), middle);
    // Any other scrolling brings the top back into the range.
    view.update(cx, |v, _| v.scroll_by(0.0, 0.0));
    assert_eq!(top(&view, cx), max_top(&view, cx));
}

#[gpui::test]
fn scrolling_the_window_pushes_the_caret_along_at_the_edge(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    let margin = (n / 20).max(1);
    // The text goes up one row; the caret on the first row would leave the margin. Sakura
    // moves it `margin + 1` rows when the new top is row 1, else by one.
    place(&view, cx, 0, 0);
    run(&view, cx, F_WND_SCROLL_UP);
    assert_eq!(at(&view, cx), (margin + 1, 1));
    place(&view, cx, 100, 100);
    run(&view, cx, F_WND_SCROLL_UP);
    assert_eq!(at(&view, cx), (101, 101));
    // The text goes down one row; the caret on the last row would leave the margin.
    place(&view, cx, 100 + n - 1, 100);
    run(&view, cx, F_WND_SCROLL_DOWN);
    assert_eq!(at(&view, cx), (100 + n - 2, 99));
    // At the first row the view does not move.
    place(&view, cx, 0, 0);
    run(&view, cx, F_WND_SCROLL_DOWN);
    assert_eq!(top(&view, cx), 0);
}

fn wheel(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
    delta: ScrollDelta,
    modifiers: Modifiers,
) {
    let event = ScrollWheelEvent {
        position: point(px(10.), px(10.)),
        delta,
        modifiers,
        touch_phase: TouchPhase::Moved,
    };
    let view = view.clone();
    cx.update(|window, cx| view.update(cx, |v, cx| v.handle_wheel(&event, window, cx)));
}

#[gpui::test]
fn a_wheel_notch_scrolls_three_rows_without_moving_the_caret(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    // Sakura's key table: the wheel without keys is F_WHEELDOWN / F_WHEELUP.
    let down = ScrollDelta::Lines(point(0., -NOTCH));
    let up = ScrollDelta::Lines(point(0., NOTCH));
    wheel(&view, cx, down, Modifiers::default());
    assert_eq!(at(&view, cx), (0, 3));
    wheel(&view, cx, down, Modifiers::default());
    assert_eq!(top(&view, cx), 6);
    wheel(&view, cx, up, Modifiers::default());
    assert_eq!(top(&view, cx), 3);
    // A part of a notch is kept until the next one makes a row (m_nWheelDelta).
    let part = ScrollDelta::Lines(point(0., -0.4 * NOTCH));
    wheel(&view, cx, part, Modifiers::default());
    assert_eq!(top(&view, cx), 4); // 1.2 rows
    wheel(&view, cx, part, Modifiers::default());
    assert_eq!(top(&view, cx), 5); // 0.07 + 1.2 rows
    wheel(
        &view,
        cx,
        ScrollDelta::Lines(point(0., 100.)),
        Modifiers::default(),
    );
    assert_eq!(top(&view, cx), 0);
}

#[gpui::test]
fn the_wheel_follows_the_key_table(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    // Ctrl+wheel is F_SETFONTSIZEUP/DOWN in Sakura's table, not a scroll.
    let (ctrl, sakura_ctrl) = if cfg!(target_os = "macos") {
        (Modifiers::command(), ume_commands::keymap::Modifiers::META)
    } else {
        (Modifiers::control(), ume_commands::keymap::Modifiers::CTRL)
    };
    wheel(&view, cx, ScrollDelta::Lines(point(0., -NOTCH)), ctrl);
    assert_eq!(top(&view, cx), 0);
    view.read_with(cx, |v, _| {
        let chord = Chord::new(Key::Mouse(MouseAction::WheelDown), sakura_ctrl);
        assert_eq!(v.keymap.resolve(chord), Resolved::Code(31123));
    });
}

#[gpui::test]
fn a_trackpad_scrolls_by_pixels(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    wheel(
        &view,
        cx,
        ScrollDelta::Pixels(point(px(0.), px(-30.))),
        Modifiers::default(),
    );
    assert_eq!(view.read_with(cx, |v, _| v.scroll_y()), 30.0);
}

#[gpui::test]
fn the_page_at_a_time_setting_of_windows_scrolls_one_page(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    // Windows reports "one screen at a time" as u32::MAX lines per notch.
    let max_lines = u32::MAX as f32;
    wheel(
        &view,
        cx,
        ScrollDelta::Lines(point(0., -max_lines)),
        Modifiers::default(),
    );
    assert_eq!(top(&view, cx), n - 1);
}

#[gpui::test]
fn a_diagonal_trackpad_move_scrolls_both_ways(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    // Some columns to scroll over.
    view.update(cx, |v, _| v.widest_cols = 500);
    wheel(
        &view,
        cx,
        ScrollDelta::Pixels(point(px(-20.), px(-30.))),
        Modifiers::default(),
    );
    assert_eq!(
        view.read_with(cx, |v, _| (v.scroll_x, v.scroll_y())),
        (20.0, 30.0)
    );
}

#[gpui::test]
fn the_two_axes_of_the_wheel_keep_their_own_leftovers(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    view.update(cx, |v, _| v.widest_cols = 500);
    // 0.6 notch down and 0.6 notch right: 1.8 rows (one row, 0.2 notch left) and 1.8 columns
    // for each axis, the same on both; nothing leaks from one axis to the other.
    let event = ScrollDelta::Lines(point(-0.6 * NOTCH, -0.6 * NOTCH));
    wheel(&view, cx, event, Modifiers::default());
    let cell = view.read_with(cx, |v, _| v.metrics().cell_width);
    assert_eq!(top(&view, cx), 1);
    assert_eq!(view.read_with(cx, |v, _| v.scroll_x), cell);
    // Opposite signs: the horizontal leftover survives a vertical move the other way (0.27 + 0.6
    // notch is two more columns).
    view.update(cx, |v, _| v.widest_cols = 500); // the paint measured the short text again
    let event = ScrollDelta::Lines(point(-0.6 * NOTCH, 0.6 * NOTCH));
    wheel(&view, cx, event, Modifiers::default());
    assert_eq!(top(&view, cx), 0);
    assert_eq!(view.read_with(cx, |v, _| v.scroll_x), 3.0 * cell);
}

#[gpui::test]
fn dragging_the_thumb_starts_from_where_it_is_drawn(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    place(&view, cx, LINES - 1, 0);
    run(&view, cx, F_CURLINECENTER);
    // The top is past the scroll range now; the thumb is drawn at the bottom of the track.
    let (x, y, limit) = view.read_with(cx, |v, _| {
        let paint = v.paint.expect("the window has been painted");
        let bar =
            crate::geometry::Scrollbar::new(v.metrics(), v.layout().row_count(), v.scroll_y());
        (
            paint.scrollbar_left + 1.0,
            paint.origin_y + bar.thumb_top + 1.0,
            max_scroll_y(v.metrics(), v.layout().row_count()),
        )
    });
    let event = gpui::MouseDownEvent {
        button: gpui::MouseButton::Left,
        position: point(px(x), px(y)),
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    };
    let v = view.clone();
    cx.update(|window, cx| v.update(cx, |v, cx| v.on_mouse_down(&event, window, cx)));
    let start = view.read_with(cx, |v, _| match v.drag {
        super::Drag::Thumb { start_scroll, .. } => start_scroll,
        other => panic!("not dragging the thumb: {other:?}"),
    });
    assert_eq!(start, limit);
}
