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
use matsu_commands::command::Arg;
use matsu_commands::keymap::{Chord, Key, MouseAction, Resolved};
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;

use super::EditorView;
use super::paging::{
    F_WHEELDOWN, F_WHEELLEFT, F_WHEELPAGEDOWN, F_WHEELPAGELEFT, F_WHEELPAGERIGHT, F_WHEELPAGEUP,
    F_WHEELRIGHT, F_WHEELUP,
};
use crate::document::SharedDocument;
use crate::geometry::max_scroll_y;

const F_1PAGE_UP: u16 = 30342;
const F_1PAGE_DOWN: u16 = 30343;
const F_HALF_PAGE_UP_BOX: u16 = 30540;
const F_HALF_PAGE_DOWN_BOX: u16 = 30541;
const F_1PAGE_UP_BOX: u16 = 30542;
const F_1PAGE_DOWN_BOX: u16 = 30543;
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
    // The auto scroll runs only in the active window.
    vcx.update(|window, _| window.activate_window());
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

/// The corners of the box selection of the view as `(line, display column)`.
fn box_corners(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
) -> Option<((usize, usize), (usize, usize))> {
    view.read_with(cx, |v, _| {
        let ((a, ac), (c, cc)) = v.cursor.box_corners()?;
        Some(((a.line, ac), (c.line, cc)))
    })
}

#[gpui::test]
fn the_box_pages_begin_a_box_and_move_the_view_like_the_others(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    place(&view, cx, 100, 98);
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(Pos::new(100, 3), false))
            .unwrap();
    });
    run(&view, cx, F_1PAGE_DOWN_BOX);
    // The caret and the view move a page less one row, in the same column, and the box grows.
    assert_eq!(at(&view, cx), (100 + n - 1, 98 + n - 1));
    assert_eq!(box_corners(&view, cx), Some(((100, 3), (100 + n - 1, 3))));
    assert!(view.read_with(cx, |v, _| v.cursor.is_box()));
    run(&view, cx, F_1PAGE_UP_BOX);
    // back at the corner of the box: the box is still there but covers nothing
    assert_eq!(at(&view, cx), (100, 98));
    assert!(view.read_with(cx, |v, _| v.cursor.is_box() && !v.cursor.has_selection()));
    assert_eq!(view.read_with(cx, |v, cx| v.status(cx).notice), None);
}

#[gpui::test]
fn the_box_half_pages_move_only_the_caret(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    place(&view, cx, 10, 0);
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(Pos::new(10, 2), false))
            .unwrap();
    });
    run(&view, cx, F_HALF_PAGE_DOWN_BOX);
    assert_eq!(box_corners(&view, cx), Some(((10, 2), (10 + n / 2, 2))));
    assert_eq!(top(&view, cx), 0);
    // going back up passes the corner of the box and the box follows
    run(&view, cx, F_HALF_PAGE_UP_BOX);
    run(&view, cx, F_HALF_PAGE_UP_BOX);
    assert_eq!(
        box_corners(&view, cx),
        Some(((10, 2), (10usize.saturating_sub(n / 2), 2)))
    );
}

#[gpui::test]
fn half_a_page_moves_only_the_caret(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    cx.simulate_keystrokes("ctrl-pagedown");
    // The caret is still in view, so the view stays.
    assert_eq!(at(&view, cx), (n / 2, 0));
    cx.simulate_keystrokes("ctrl-shift-pagedown");
    assert_eq!(caret_line(&view, cx), 2 * (n / 2));
    assert!(view.read_with(cx, |v, _| v.cursor.has_selection()));
    cx.simulate_keystrokes("ctrl-pageup");
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
    // Plan A (ADR-0008): Control is Sakura's Ctrl on every OS.
    let ctrl = Modifiers::control();
    let sakura_ctrl = matsu_commands::keymap::Modifiers::CTRL;
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

fn run_with_args(view: &Entity<EditorView>, cx: &mut VisualTestContext, code: u16, args: &[Arg]) {
    let view = view.clone();
    cx.update(|window, cx| view.update(cx, |v, cx| v.run_function(code, args, window, cx)));
}

#[gpui::test]
fn the_wheel_functions_run_as_commands(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    // OnMOUSEWHEEL2 through Command_WHEELDOWN: one notch (zDelta 120) is three rows.
    run(&view, cx, F_WHEELDOWN);
    assert_eq!(top(&view, cx), 3);
    run(&view, cx, F_WHEELUP);
    assert_eq!(top(&view, cx), 0);
    // zDelta 240 is two notches; the sign of zDelta does not matter, the function decides.
    run_with_args(&view, cx, F_WHEELDOWN, &[Arg::Int(240)]);
    assert_eq!(top(&view, cx), 6);
    run_with_args(&view, cx, F_WHEELDOWN, &[Arg::Int(-120)]);
    assert_eq!(top(&view, cx), 9);
    // half a notch makes 1.5 rows: one now, the rest is kept for the next one
    run_with_args(&view, cx, F_WHEELUP, &[Arg::Int(60)]);
    assert_eq!(top(&view, cx), 8);
    // the page functions scroll a page less one row per notch
    run(&view, cx, F_WHEELPAGEDOWN);
    assert_eq!(top(&view, cx), 8 + n - 1);
    run(&view, cx, F_WHEELPAGEUP);
    assert_eq!(top(&view, cx), 8);
    // none of them moves the caret
    assert_eq!(caret_line(&view, cx), 0);
}

#[gpui::test]
fn the_horizontal_wheel_functions_run_as_commands(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    view.update(cx, |v, _| v.widest_cols = 500);
    let cell = view.read_with(cx, |v, _| v.metrics().cell_width);
    run(&view, cx, F_WHEELRIGHT);
    assert_eq!(view.read_with(cx, |v, _| v.scroll_x), 3.0 * cell);
    run(&view, cx, F_WHEELLEFT);
    assert_eq!(view.read_with(cx, |v, _| v.scroll_x), 0.0);
    view.update(cx, |v, _| v.widest_cols = 500);
    run(&view, cx, F_WHEELPAGERIGHT);
    let page = view.read_with(cx, |v, _| v.scroll_x / v.metrics().cell_width);
    assert!(page > 3.0, "a page is more than a notch: {page}");
    run(&view, cx, F_WHEELPAGELEFT);
    assert_eq!(view.read_with(cx, |v, _| v.scroll_x), 0.0);
}

// ---- F_AUTOSCROLL (`view/autoscroll.rs`) ----

const F_AUTOSCROLL: u16 = 30384;

fn autoscrolling(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> bool {
    view.read_with(cx, |v, _| v.is_autoscrolling())
}

fn tick(cx: &mut VisualTestContext) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(200));
    cx.run_until_parked();
}

#[gpui::test]
fn the_function_starts_and_ends_the_auto_scroll(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    run(&view, cx, F_AUTOSCROLL);
    assert!(autoscrolling(&view, cx));
    run(&view, cx, F_AUTOSCROLL);
    assert!(!autoscrolling(&view, cx));
}

#[gpui::test]
fn a_text_that_fits_the_view_does_not_start_it(cx: &mut TestAppContext) {
    let doc = Document::from_utf8(b"short".to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    cx.run_until_parked();
    // `AutoScrollEnter`: neither axis has anything to scroll.
    run(&view, cx, F_AUTOSCROLL);
    assert!(!autoscrolling(&view, cx));
}

#[gpui::test]
fn the_pointer_below_the_start_scrolls_down_every_200_ms(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    cx.simulate_mouse_move(point(px(100.), px(100.)), None, Modifiers::default());
    run(&view, cx, F_AUTOSCROLL);
    assert!(autoscrolling(&view, cx));
    // 15 pixels: inside the dead zone (`relPos / 8` less one is 0).
    cx.simulate_mouse_move(point(px(100.), px(115.)), None, Modifiers::default());
    tick(cx);
    assert_eq!(top(&view, cx), 0);
    // 80 pixels down: 80 / 8 - 1 = 9 rows a tick.
    cx.simulate_mouse_move(point(px(100.), px(180.)), None, Modifiers::default());
    tick(cx);
    assert_eq!(top(&view, cx), 9);
    tick(cx);
    assert_eq!(top(&view, cx), 18);
    // Above the start it scrolls back up.
    cx.simulate_mouse_move(point(px(100.), px(20.)), None, Modifiers::default());
    tick(cx);
    assert_eq!(top(&view, cx), 9);
}

#[gpui::test]
fn another_function_a_click_or_a_key_ends_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    run(&view, cx, F_AUTOSCROLL);
    run(&view, cx, F_CURLINETOP);
    assert!(!autoscrolling(&view, cx));
    run(&view, cx, F_AUTOSCROLL);
    cx.simulate_keystrokes("down");
    assert!(!autoscrolling(&view, cx));
    run(&view, cx, F_AUTOSCROLL);
    cx.simulate_click(point(px(300.), px(60.)), Modifiers::default());
    assert!(!autoscrolling(&view, cx));
}

#[gpui::test]
fn the_middle_button_starts_it_on_release_and_ends_it_on_the_next_press(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let position = point(px(120.), px(80.));
    let down = gpui::MouseDownEvent {
        button: gpui::MouseButton::Middle,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    };
    let up = gpui::MouseUpEvent {
        button: gpui::MouseButton::Middle,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
    };
    let v = view.clone();
    cx.update(|window, cx| v.update(cx, |v, cx| v.on_middle_mouse_down(&down, window, cx)));
    // Pressed only: mode 1, not scrolling yet.
    assert!(!autoscrolling(&view, cx));
    let v = view.clone();
    cx.update(|window, cx| v.update(cx, |v, cx| v.on_middle_mouse_up(&up, window, cx)));
    assert!(autoscrolling(&view, cx));
    let v = view.clone();
    cx.update(|window, cx| v.update(cx, |v, cx| v.on_middle_mouse_down(&down, window, cx)));
    assert!(!autoscrolling(&view, cx));
    // The release that follows the ending press does nothing.
    let v = view.clone();
    cx.update(|window, cx| v.update(cx, |v, cx| v.on_middle_mouse_up(&up, window, cx)));
    assert!(!autoscrolling(&view, cx));
}

#[gpui::test]
fn losing_the_focus_ends_it_at_the_next_tick(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    run(&view, cx, F_AUTOSCROLL);
    assert!(autoscrolling(&view, cx));
    cx.update(|window, app| window.blur(app));
    tick(cx);
    assert!(!autoscrolling(&view, cx));
}

#[gpui::test]
fn a_middle_release_outside_finishes_only_a_press_that_waited(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let position = point(px(120.), px(80.));
    let down = gpui::MouseDownEvent {
        button: gpui::MouseButton::Middle,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    };
    let up = gpui::MouseUpEvent {
        button: gpui::MouseButton::Middle,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
    };
    let v = view.clone();
    cx.update(|window, cx| v.update(cx, |v, cx| v.on_middle_mouse_up_outside(&up, window, cx)));
    assert!(!autoscrolling(&view, cx));
    let v = view.clone();
    cx.update(|window, cx| v.update(cx, |v, cx| v.on_middle_mouse_down(&down, window, cx)));
    let v = view.clone();
    cx.update(|window, cx| v.update(cx, |v, cx| v.on_middle_mouse_up_outside(&up, window, cx)));
    assert!(autoscrolling(&view, cx));
}

#[gpui::test]
fn an_inactive_window_ends_it_at_the_next_tick(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    run(&view, cx, F_AUTOSCROLL);
    assert!(autoscrolling(&view, cx));
    // The focused element stays, but the window is no longer the active one.
    cx.deactivate_window();
    assert!(view.read_with(cx, |v, _| v.is_autoscrolling()));
    tick(cx);
    assert!(!autoscrolling(&view, cx));
}

#[gpui::test]
fn keys_that_no_command_takes_end_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    // An unbound key (F13 has no function by default).
    run(&view, cx, F_AUTOSCROLL);
    cx.simulate_keystrokes("f13");
    assert!(!autoscrolling(&view, cx));
    // Typed text: the path of a character key that is not a command.
    run(&view, cx, F_AUTOSCROLL);
    cx.simulate_input("x");
    assert!(!autoscrolling(&view, cx));
    // The completion list takes the key first.
    run(&view, cx, F_AUTOSCROLL);
    cx.simulate_keystrokes("ctrl-space");
    assert!(!autoscrolling(&view, cx));
}

#[gpui::test]
fn a_composition_ends_it(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler as _;
    let (view, cx) = setup(cx);
    run(&view, cx, F_AUTOSCROLL);
    view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", None, window, cx);
    });
    assert!(!autoscrolling(&view, cx));
}

#[gpui::test]
fn a_key_bound_to_the_function_toggles_it_off(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    run(&view, cx, F_AUTOSCROLL);
    view.update(cx, |v, _| {
        v.autoscroll_key_pressed(&gpui::Keystroke::parse("f13").unwrap());
    });
    assert!(!autoscrolling(&view, cx));
}

// ---- the scroll options of the common settings 「全般」 ----

fn set_options(cx: &mut VisualTestContext, options: crate::common_settings::ScrollOptions) {
    cx.update(|_, cx| cx.set_global(options));
}

fn hold_middle_button(view: &Entity<EditorView>, cx: &mut VisualTestContext) {
    view.update(cx, |v, _| v.held_buttons.middle = true);
}

fn key_down(view: &Entity<EditorView>, cx: &mut VisualTestContext, key: &str, held: bool) {
    let event = gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
        is_held: held,
        prefer_character_input: false,
    };
    let view = view.clone();
    cx.update(|window, cx| view.update(cx, |v, cx| v.on_key_down(&event, window, cx)));
}

#[gpui::test]
fn a_held_down_key_moves_the_rows_of_the_setting(cx: &mut TestAppContext) {
    // `Command_DOWN` with `bRepeat`: `nRepeatedScrollLineNum` rows (3 by default)
    let (view, cx) = setup(cx);
    key_down(&view, cx, "down", false);
    assert_eq!(caret_line(&view, cx), 1);
    key_down(&view, cx, "down", true);
    assert_eq!(caret_line(&view, cx), 4);
    key_down(&view, cx, "up", true);
    assert_eq!(caret_line(&view, cx), 1);
    set_options(
        cx,
        crate::common_settings::ScrollOptions {
            repeated_lines: 7,
            ..Default::default()
        },
    );
    key_down(&view, cx, "down", true);
    assert_eq!(caret_line(&view, cx), 8);
    // a key that is not up or down is not repeated by the setting
    key_down(&view, cx, "right", true);
    assert_eq!(view.read_with(cx, |v, _| v.cursor.caret().col), 1);
}

#[gpui::test]
fn the_middle_button_makes_the_wheel_scroll_by_pages_when_the_setting_says_so(
    cx: &mut TestAppContext,
) {
    use crate::common_settings::{ScrollOptions, WHEEL_MIDDLE_BUTTON};
    let (view, cx) = setup(cx);
    let n = rows(&view, cx);
    let down = ScrollDelta::Lines(point(0., -NOTCH));
    // not chosen: the button changes nothing
    hold_middle_button(&view, cx);
    wheel(&view, cx, down, Modifiers::default());
    assert_eq!(top(&view, cx), 3);
    assert!(!view.read_with(cx, |v, _| v.wheel_used_buttons.middle));
    // chosen: a page (rows less one) a notch, and letting go of the button is not a click
    set_options(
        cx,
        ScrollOptions {
            page_by_wheel: WHEEL_MIDDLE_BUTTON,
            ..Default::default()
        },
    );
    wheel(&view, cx, down, Modifiers::default());
    assert_eq!(top(&view, cx), 3 + n - 1);
    assert!(view.read_with(cx, |v, _| v.wheel_used_buttons.middle));
    // without the button the wheel is the plain one again
    view.update(cx, |v, _| v.held_buttons.middle = false);
    wheel(&view, cx, down, Modifiers::default());
    assert_eq!(top(&view, cx), 6 + n - 1);
}

#[gpui::test]
fn the_middle_button_makes_the_wheel_scroll_sideways_when_the_setting_says_so(
    cx: &mut TestAppContext,
) {
    use crate::common_settings::{ScrollOptions, WHEEL_MIDDLE_BUTTON};
    let (view, cx) = setup(cx);
    view.update(cx, |v, _| v.widest_cols = 500);
    let cell = view.read_with(cx, |v, _| v.metrics().cell_width);
    set_options(
        cx,
        ScrollOptions {
            hscroll_by_wheel: WHEEL_MIDDLE_BUTTON,
            ..Default::default()
        },
    );
    hold_middle_button(&view, cx);
    // wheel down is "forward": to the right, three columns a notch (`SPI_GETWHEELSCROLLCHARS`)
    wheel(
        &view,
        cx,
        ScrollDelta::Lines(point(0., -NOTCH)),
        Modifiers::default(),
    );
    assert_eq!(
        view.read_with(cx, |v, _| (v.scroll_x, v.scroll_y())),
        (3.0 * cell, 0.0)
    );
    wheel(
        &view,
        cx,
        ScrollDelta::Lines(point(0., NOTCH)),
        Modifiers::default(),
    );
    assert_eq!(view.read_with(cx, |v, _| v.scroll_x), 0.0);
}

#[gpui::test]
fn a_side_button_that_turned_the_wheel_does_not_click_when_it_is_let_go(cx: &mut TestAppContext) {
    use crate::common_settings::{ScrollOptions, WHEEL_SIDE_BUTTON_1};
    use gpui::{MouseButton, NavigationDirection};
    let (view, cx) = setup(cx);
    set_options(
        cx,
        ScrollOptions {
            page_by_wheel: WHEEL_SIDE_BUTTON_1,
            ..Default::default()
        },
    );
    view.update(cx, |v, _| v.held_buttons.side_1 = true);
    wheel(
        &view,
        cx,
        ScrollDelta::Lines(point(0., -NOTCH)),
        Modifiers::default(),
    );
    let back = MouseButton::Navigate(NavigationDirection::Back);
    view.update(cx, |v, _| {
        // `OnXLBUTTONUP` gives up the click once; the other side button was not used
        assert!(
            !v.wheel_used_buttons
                .clone()
                .take(MouseButton::Navigate(NavigationDirection::Forward))
        );
        assert!(v.wheel_used_buttons.take(back));
        assert!(!v.wheel_used_buttons.take(back));
    });
}
