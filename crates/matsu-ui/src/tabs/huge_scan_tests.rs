//! Moving in a huge file whose scan has not finished (ADR-0020, chapter 2): a jump to the end or
//! to a line beyond the scan, the scroll bar over the whole file, the window on the file that
//! moves with the view, and the line numbers that become exact when the scan catches up.
//!
//! The files are real ones of 1.2 MiB (read by ranges from a threshold of 1 MiB, scanned a block
//! of 64 KiB every 300 ms), so the tests look at the editor while the scan is in its first part.
//! The same movements over a file of 10 GB are tried in `matsu-core`
//! (`buffer/block/window_tests.rs`), where the bytes are made from the line number.

use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext};
use matsu_commands::command::Arg;
use matsu_core::buffer::{Pos, WINDOW_BYTES};

use super::huge_tests::{
    big_text, caret_line, line_count, notice, opened, scanning, wait_for_scan,
};
use super::tests::{active_view, run};
use crate::document::ScanWindowInfo;
use crate::view::EditorView;

/// How far the "about" line numbers may be from the true ones: the estimate comes from the first
/// 64 KiB of a file of 1.2 MB, so it is off by a share of that.
const ABOUT: usize = 40;

/// The window on the file that the document is, if it is one.
fn window_of(cx: &mut TestAppContext, view: &Entity<EditorView>) -> Option<ScanWindowInfo> {
    view.read_with(cx, |v, cx| v.document().read(cx).scan_window())
}

/// The line of the status bar, and whether it is "about".
fn status_line(cx: &mut TestAppContext, view: &Entity<EditorView>) -> (usize, bool) {
    view.read_with(cx, |v, cx| {
        let status = v.status(cx);
        (status.line, status.line_approx)
    })
}

/// The byte of the file that the caret is at.
fn caret_byte(cx: &mut TestAppContext, view: &Entity<EditorView>) -> u64 {
    view.read_with(cx, |v, cx| {
        let shared = v.document().read(cx);
        shared.frame_start() + shared.doc().pos_to_offset(v.cursor.caret()).unwrap() as u64
    })
}

fn caret_text(cx: &mut TestAppContext, view: &Entity<EditorView>) -> String {
    view.read_with(cx, |v, cx| {
        let at = v.cursor.caret().line;
        v.document().read(cx).doc().line(at).unwrap().unwrap().text
    })
}

/// Lets the scan go on until `done` says it can stop.
fn advance_until(
    cx: &mut TestAppContext,
    view: &Entity<EditorView>,
    done: impl Fn(&mut TestAppContext, &Entity<EditorView>) -> bool,
) {
    for _ in 0..6000 {
        if done(cx, view) {
            return;
        }
        cx.executor().advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the condition was not reached");
}

#[gpui::test]
fn a_jump_to_the_end_shows_the_end_at_once_and_the_numbers_become_exact_later(
    cx: &mut TestAppContext,
) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    assert!(scanning(cx, &view));
    run(cx, handle, "F_GOFILEEND");
    // The end is on the screen while the scan is at the start: a window on the file there, which
    // is less than the file, and read-only.
    let window = window_of(cx, &view).expect("a window was made");
    assert!(scanning(cx, &view));
    let total = big_text().len() as u64;
    assert_eq!(window.end, total);
    let shown = view.read_with(cx, |v, cx| v.document().read(cx).doc().raw_len()) as u64;
    assert_eq!(shown, window.end - window.start);
    assert!(shown <= WINDOW_BYTES as u64 && window.start > 0);
    assert!(!window.exact, "the scan has not passed the window");
    // The caret is at the end of the text, after the last line; the numbers are "about".
    let (line, approx) = status_line(cx, &view);
    assert!(approx);
    assert!(line.abs_diff(100_001) <= ABOUT, "{line}");
    view.update(cx, |v, cx| v.type_char('x', cx));
    assert_eq!(
        notice(cx, &view).as_deref(),
        Some("改行位置を調べている間は編集できません")
    );
    // The bar covers the whole file: the thumb is at the bottom.
    let bar = view.read_with(cx, |v, cx| v.scan_scrollbar(&v.metrics, cx).unwrap());
    let travel = bar.track_height - bar.thumb_height;
    assert!(bar.thumb_top > travel * 0.95, "{bar:?}");
    // When the scan is done the whole file takes the place of the window, at the same place.
    wait_for_scan(cx, &view);
    assert_eq!(window_of(cx, &view), None);
    assert_eq!(line_count(cx, &view), 100_000);
    assert_eq!(
        view.read_with(cx, |v, _| v.cursor.caret()),
        Pos::new(100_000, 0),
        "the end of the text, after the last line"
    );
    assert_eq!(status_line(cx, &view), (100_001, false));
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_jump_to_a_line_the_scan_has_not_reached_goes_there_and_the_scan_makes_it_exact(
    cx: &mut TestAppContext,
) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    // A line the scan has reached is a jump as usual.
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(2)], cx));
    assert_eq!(caret_line(cx, &view), 1);
    assert_eq!(window_of(cx, &view), None);
    // A line it has not reached: the window is there, with the caret on the line (about).
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(70_000)], cx));
    let window = window_of(cx, &view).expect("a window was made");
    assert!(!window.exact && window.start > 0);
    assert!(scanning(cx, &view));
    assert_eq!(status_line(cx, &view), (70_000, true));
    let shown: usize = caret_text(cx, &view)["line ".len()..].parse().unwrap();
    assert!(shown.abs_diff(69_999) <= ABOUT, "{shown}");
    // The scan comes to the window: its numbers are exact now, while the scan goes on, and the
    // line the caret is on is the line of the file.
    advance_until(cx, &view, |cx, view| {
        window_of(cx, view).is_none_or(|w| w.exact)
    });
    let window = window_of(cx, &view).expect("still a window");
    assert!(window.exact && scanning(cx, &view));
    assert_eq!(window.lines_before, window.start / 12);
    let (line, approx) = status_line(cx, &view);
    assert!(!approx && line.abs_diff(70_000) <= ABOUT, "{line}");
    assert_eq!(caret_text(cx, &view), format!("line {:06}", line - 1));
    wait_for_scan(cx, &view);
    assert_eq!(window_of(cx, &view), None);
    // The caret is on the same line of the whole file.
    assert_eq!(caret_line(cx, &view), line - 1);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_pl_sql_count_and_the_count_of_rows_still_wait_for_the_scan(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    // They cannot be placed in a start of the file (or a window): they wait.
    view.update(cx, |v, cx| {
        v.run_command("jump", &[Arg::Int(3), Arg::Int(2)], cx)
    });
    assert_eq!(caret_line(cx, &view), 0);
    assert_eq!(window_of(cx, &view), None);
    assert_eq!(
        notice(cx, &view).as_deref(),
        Some("改行位置を調べ終えてから移動します")
    );
    wait_for_scan(cx, &view);
    assert_eq!(
        caret_line(cx, &view),
        2,
        "the jump ran when the scan was done"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_move_to_the_start_that_extends_a_selection_waits_when_a_window_is_shown(
    cx: &mut TestAppContext,
) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_some());
    let before = view.read_with(cx, |v, _| v.cursor.caret());
    // The top of the window is not the top of the file: the move waits for the scan.
    run(cx, handle, "F_GOFILETOP_SEL");
    assert_eq!(view.read_with(cx, |v, _| v.cursor.caret()), before);
    assert_eq!(
        notice(cx, &view).as_deref(),
        Some("改行位置を調べ終えてから移動します")
    );
    wait_for_scan(cx, &view);
    assert_eq!(caret_line(cx, &view), 0);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_bookmarks_of_the_file_are_not_shown_in_a_window_and_come_back_with_the_whole_file(
    cx: &mut TestAppContext,
) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    let marks = |cx: &mut TestAppContext| -> Vec<usize> {
        view.read_with(cx, |v, app| {
            v.document().read(app).model().bookmarks().iter().collect()
        })
    };
    view.update(cx, |v, cx| {
        v.document().update(cx, |shared, _| {
            shared.parts_mut().0.bookmarks_mut().set(5, true);
        });
    });
    assert_eq!(marks(cx), [5]);
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_some());
    // Line 5 of the file is not line 5 of the window.
    assert!(marks(cx).is_empty());
    wait_for_scan(cx, &view);
    assert_eq!(marks(cx), [5]);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_go_to_line_dialog_shows_a_line_beyond_the_scan_too(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    view.update(cx, |v, cx| v.go_to_line(90_000, cx));
    assert!(window_of(cx, &view).is_some());
    assert_eq!(status_line(cx, &view), (90_000, true));
    wait_for_scan(cx, &view);
    assert!(caret_line(cx, &view).abs_diff(89_999) <= ABOUT);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_thumb_of_the_bar_covers_the_whole_file_and_moves_the_view_there(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    let total = big_text().len() as u64;
    let bar = |cx: &mut TestAppContext| {
        view.read_with(cx, |v, cx| v.scan_scrollbar(&v.metrics, cx).unwrap())
    };
    let top = bar(cx);
    assert_eq!(top.thumb_top, 0.0);
    // The thumb is as long as the view is of the file (30 rows of 100 000), not of what is
    // scanned: the least length.
    assert!((top.thumb_height - 24.0).abs() < 1e-3, "{top:?}");
    let travel = top.track_height - top.thumb_height;
    // Half way down: the view is at half of the file (by bytes).
    view.update(cx, |v, cx| v.drag_scan_thumb(&top, travel / 2.0, cx));
    let window = window_of(cx, &view).expect("a window was made");
    assert!(
        window.start <= total / 2 && total / 2 < window.end,
        "{window:?}"
    );
    let (first, _) = view.read_with(cx, |v, cx| v.visible_bytes(&v.metrics, cx));
    assert!(first.abs_diff(total / 2) <= 12, "{first} for {}", total / 2);
    let half = bar(cx);
    assert!((half.thumb_top - travel / 2.0).abs() < 1.0, "{half:?}");
    // All the way down: the end of the file; all the way up: its start.
    view.update(cx, |v, cx| v.drag_scan_thumb(&half, travel, cx));
    assert_eq!(window_of(cx, &view).unwrap().end, total);
    let (_, last) = view.read_with(cx, |v, cx| v.visible_bytes(&v.metrics, cx));
    assert!(last >= total - 12, "{last}");
    view.update(cx, |v, cx| v.drag_scan_thumb(&half, 0.0, cx));
    assert_eq!(window_of(cx, &view).map(|w| w.start), Some(0));
    let (first, _) = view.read_with(cx, |v, cx| v.visible_bytes(&v.metrics, cx));
    assert_eq!(first, 0);
    // Ctrl+Home from a window goes to the start of the file.
    run(cx, handle, "F_GOFILEEND");
    run(cx, handle, "F_GOFILETOP");
    assert_eq!(caret_line(cx, &view), 0);
    assert_eq!(status_line(cx, &view), (1, false));
    wait_for_scan(cx, &view);
    assert!(window_of(cx, &view).is_none());
    assert!(!view.read_with(cx, |v, cx| v.document().read(cx).is_partial()));
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn scrolling_to_the_edge_of_a_window_moves_the_window_with_the_view(cx: &mut TestAppContext) {
    // 3.6 MB, so that a window of 1 MiB has room on both sides.
    let text: String = (0..300_000).map(|i| format!("line {i:06}\n")).collect();
    let (handle, dir) = opened(cx, "big.txt", text.as_bytes());
    let view = active_view(cx, handle);
    let total = text.len() as u64;
    // A window in the middle of the file, with room on both sides.
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(150_000)], cx));
    let window = window_of(cx, &view).expect("a window was made");
    assert!(window.start > 0 && window.end < total, "{window:?}");
    let caret_before = caret_byte(cx, &view);
    // Scroll to the end of the window, as a wheel does.
    let rows = view.read_with(cx, |v, _| v.layout.row_count());
    view.update(cx, |v, cx| {
        v.scroll_by(0.0, -(rows as f32) * 20.0);
        v.queue_scan_window_shift(true, cx);
    });
    let before = view.read_with(cx, |v, cx| v.visible_bytes(&v.metrics, cx));
    cx.run_until_parked();
    let moved = window_of(cx, &view).unwrap();
    assert!(
        moved.start > window.start && moved.end > window.end,
        "{moved:?}"
    );
    // The view did not jump: the same lines are at the top.
    let after = view.read_with(cx, |v, cx| v.visible_bytes(&v.metrics, cx));
    assert_eq!(before.0, after.0, "{moved:?}");
    // The caret is still inside the window: it stays on its byte.
    assert_eq!(caret_byte(cx, &view), caret_before);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn another_pane_of_the_document_follows_the_window_to_the_same_place(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    // A second view of the same document, as a split pane is.
    let document = view.read_with(cx, |v, _| v.document().clone());
    let pane = cx.new(|cx| EditorView::new(document.clone(), cx));
    pane.update(cx, |v, cx| v.go_to_line(95_000, cx));
    cx.run_until_parked();
    assert!(window_of(cx, &pane).is_some());
    // (The pane that has the focus may move the window meanwhile, which makes a new estimate.)
    let (line, _) = status_line(cx, &pane);
    assert!(line.abs_diff(95_000) <= ABOUT, "{line}");
    // The first pane had its caret at the start of the file, which is not in the window: it
    // goes to the top of what it shows.
    assert!(view.read_with(cx, |v, cx| v.visible_bytes(&v.metrics, cx).0) > 0);
    assert_eq!(view.read_with(cx, |v, _| v.cursor.caret().col), 0);
    wait_for_scan(cx, &view);
    cx.run_until_parked();
    assert!(caret_line(cx, &pane).abs_diff(94_999) <= ABOUT);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_file_cut_short_while_a_window_is_shown_stops_the_scan_and_keeps_the_view(
    cx: &mut TestAppContext,
) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_some());
    // Another program cuts the file short.
    std::fs::OpenOptions::new()
        .write(true)
        .open(dir.join("big.txt"))
        .unwrap()
        .set_len(200_000)
        .unwrap();
    advance_until(cx, &view, |cx, view| !scanning(cx, view));
    // What is on the screen stays, read-only, and says why.
    assert!(view.read_with(cx, |v, cx| v.document().read(cx).is_partial()));
    let text = notice(cx, &view).unwrap_or_default();
    assert!(text.contains("読み取り専用のまま"), "{text}");
    // A jump the file cannot give any more does nothing, and does not stop the editor.
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(70_000)], cx));
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(2)], cx));
    std::fs::remove_dir_all(dir).ok();
}
