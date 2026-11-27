//! The vertical position of a view whose document has very many rows (ADR-0020, chapter 6): an
//! `f32` of pixels cannot tell the rows apart there (past 2^24 px, about 500,000 rows, a step of
//! the `f32` is more than a pixel; at 2x10^8 rows it is 16 rows), so the position is a row and
//! the pixels cut off it (`crate::scroll_pos`).
//!
//! The view tests use a document of [`LINES`] lines (each is a row), which an `f32` of pixels
//! would get wrong by several rows, and a text that is made in memory (60 MB); the arithmetic of
//! the same steps at 2x10^8 rows, and more, is in `scroll_pos` and `geometry`, which need no
//! document.

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use matsu_commands::sakura::FUNCTIONS;

use super::hscroll_tests::{press, release, settle};
use super::tests::{Fixture, setup};
use crate::geometry::{BarPart, Scrollbar, VSCROLL_ARROW};

/// Enough rows for the `f32` of pixels to be several rows off at the end (3x10^7 rows of 20 or
/// more pixels are 6x10^8 pixels; a step of the `f32` there is 64).
const LINES: usize = 30_000_000;

fn huge(cx: &mut TestAppContext) -> (Fixture, &mut VisualTestContext) {
    let (f, cx) = setup(cx, &"a\n".repeat(LINES));
    settle(cx);
    (f, cx)
}

fn run(f: &Fixture, cx: &mut VisualTestContext, function: &str) {
    let code = FUNCTIONS.iter().find(|x| x.name == function).unwrap().code;
    let view = f.view.clone();
    cx.update(|window, cx| view.update(cx, |v, cx| v.run_function(code, &[], window, cx)));
    settle(cx);
}

/// The first row of the view, and the rows of the document.
fn top_and_rows(f: &Fixture, cx: &mut VisualTestContext) -> (usize, usize) {
    f.view
        .read_with(cx, |v, _| (v.vscroll.row(), v.layout.row_count()))
}

fn visible_rows(f: &Fixture, cx: &mut VisualTestContext) -> usize {
    f.view.read_with(cx, |v, _| v.metrics.visible_rows())
}

/// The wheel: `rows` rows down (negative: up).
fn scroll_by_rows(f: &Fixture, cx: &mut VisualTestContext, rows: f32) {
    f.view.update(cx, |v, _| {
        let lh = v.metrics.line_height;
        // a positive `dy` moves the text up
        v.scroll_by(0.0, -rows * lh);
    });
}

#[gpui::test]
fn the_end_of_a_document_of_many_rows_is_on_the_screen(cx: &mut TestAppContext) {
    let (f, cx) = huge(cx);
    run(&f, cx, "F_GOFILEEND");
    let (top, rows) = top_and_rows(&f, cx);
    let visible = visible_rows(&f, cx);
    // The last line, the row after it (the end of the text) and the caret.
    assert_eq!(rows, LINES + 1);
    f.view.read_with(cx, |v, app| {
        assert_eq!(v.cursor.caret().line, LINES);
        let caret = v.caret_row(app).row;
        assert_eq!(caret, rows - 1, "the caret is on the row of the end");
        let lh = v.metrics.line_height;
        for row in [rows - 2, rows - 1] {
            let y = v.vscroll.y_of_row(row, lh);
            assert!(
                y >= 0.0 && y + lh <= v.metrics.view_height + 0.01,
                "row {row} at {y} of {}",
                v.metrics.view_height
            );
        }
    });
    assert!(
        top + visible >= rows && top < rows,
        "{top} {visible} {rows}"
    );
    // What is painted: the rows from the top of the view reach the end of the text.
    let last = f.view.read_with(cx, |v, app| {
        let doc = v.document.read(app).doc();
        v.layout
            .rows(doc, top, visible)
            .unwrap()
            .last()
            .unwrap()
            .row
    });
    assert_eq!(last, rows - 1);
}

#[gpui::test]
fn a_row_up_and_a_row_down_near_the_end_move_exactly_one_row(cx: &mut TestAppContext) {
    let (f, cx) = huge(cx);
    run(&f, cx, "F_GOFILEEND");
    let (end, _) = top_and_rows(&f, cx);
    scroll_by_rows(&f, cx, -1.0);
    assert_eq!(top_and_rows(&f, cx).0, end - 1);
    scroll_by_rows(&f, cx, -1.0);
    assert_eq!(top_and_rows(&f, cx).0, end - 2);
    scroll_by_rows(&f, cx, 1.0);
    assert_eq!(top_and_rows(&f, cx).0, end - 1);
    scroll_by_rows(&f, cx, 1.0);
    assert_eq!(top_and_rows(&f, cx).0, end);
    // The last position stops there (one row more than the end of the text can show is the most).
    scroll_by_rows(&f, cx, 50.0);
    let (last, rows) = top_and_rows(&f, cx);
    let visible = visible_rows(&f, cx);
    assert_eq!(last, rows - (visible - 1));
}

#[gpui::test]
fn a_page_moves_exactly_a_page_in_a_document_of_many_rows(cx: &mut TestAppContext) {
    let (f, cx) = huge(cx);
    run(&f, cx, "F_GOFILEEND");
    scroll_by_rows(&f, cx, -100.0);
    let (before, _) = top_and_rows(&f, cx);
    let page = visible_rows(&f, cx) - 1;
    f.view
        .update(cx, |v, cx| v.v_scroll_step(BarPart::PageBack, cx));
    assert_eq!(top_and_rows(&f, cx).0, before - page);
    f.view
        .update(cx, |v, cx| v.v_scroll_step(BarPart::PageForward, cx));
    assert_eq!(top_and_rows(&f, cx).0, before);
    // The key of a page: the caret and the view move by a page less one row.
    let caret = f.view.read_with(cx, |v, _| v.cursor.caret().line);
    let top_before = top_and_rows(&f, cx).0;
    run(&f, cx, "F_1PageUp");
    assert_eq!(top_and_rows(&f, cx).0, top_before - page);
    assert_eq!(
        f.view.read_with(cx, |v, _| v.cursor.caret().line),
        caret - page
    );
}

#[gpui::test]
fn dragging_the_thumb_to_the_middle_shows_the_rows_around_the_middle(cx: &mut TestAppContext) {
    let (f, cx) = huge(cx);
    let (bar, paint, rows, visible) = f.view.read_with(cx, |v, _| {
        let rows = v.layout.row_count();
        (
            Scrollbar::new(&v.metrics, rows, v.vscroll),
            v.paint.unwrap(),
            rows,
            v.metrics.visible_rows(),
        )
    });
    let (x, y) = (
        paint.scrollbar_left + 3.0,
        paint.vbar_top + VSCROLL_ARROW + 1.0,
    );
    press(cx, x, y);
    let travel = bar.track_height - bar.thumb_height;
    cx.simulate_mouse_move(
        point(px(x), px(y + travel / 2.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    let (top, _) = top_and_rows(&f, cx);
    let middle = (rows - (visible - 1)) / 2;
    assert!(top.abs_diff(middle) <= 1, "{top} for {middle}");
    release(cx, x, y + travel / 2.0);
    settle(cx);
    let (after, _) = top_and_rows(&f, cx);
    assert!(after.abs_diff(middle) <= 1, "{after} for {middle}");
}

#[gpui::test]
fn the_thumb_is_where_the_view_is(cx: &mut TestAppContext) {
    let (f, cx) = huge(cx);
    let thumb = |f: &Fixture, cx: &mut VisualTestContext| {
        f.view.read_with(cx, |v, _| {
            let bar = Scrollbar::new(&v.metrics, v.layout.row_count(), v.vscroll);
            (bar.thumb_top, bar.track_height - bar.thumb_height)
        })
    };
    let (top, travel) = thumb(&f, cx);
    assert_eq!(top, 0.0);
    assert!(travel > 0.0);
    run(&f, cx, "F_GOFILEEND");
    let (end, travel_end) = thumb(&f, cx);
    assert!((travel_end - end).abs() < 0.01, "{end} of {travel_end}");
    // The middle of the rows that can be scrolled.
    f.view.update(cx, |v, cx| {
        let rows = v.layout.row_count();
        let max = rows - (v.metrics.visible_rows() - 1);
        v.scroll_top_to((max / 2) as isize, cx);
    });
    let (middle, travel_middle) = thumb(&f, cx);
    assert!(
        (middle - travel_middle / 2.0).abs() < 0.01,
        "{middle} of {travel_middle}"
    );
    run(&f, cx, "F_GOFILETOP");
    assert_eq!(thumb(&f, cx).0, 0.0);
    assert_eq!(top_and_rows(&f, cx).0, 0);
}
