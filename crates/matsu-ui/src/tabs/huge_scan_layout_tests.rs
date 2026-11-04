//! The window on a huge file with wrapped lines (the layout of a new document is made a part at
//! a time, so the rows are provisional at first), and with a selection (ADR-0020, chapter 2).

use gpui::{Entity, TestAppContext};
use matsu_commands::command::Arg;
use matsu_core::buffer::Pos;

use super::huge_scan_tests::window_of;
use super::huge_tests::{big_text, opened};
use super::tests::{active_view, run};
use crate::view::EditorView;

/// Wraps the lines of the view after 4 columns (`line 000000` is 3 rows), as if the user had
/// asked for a wrap at that width.
fn wrap_at_four(cx: &mut TestAppContext, view: &Entity<EditorView>) {
    view.update(cx, |v, cx| {
        v.prefs.wrap_method = crate::settings::view_settings::WrapMethod::SettingWidth;
        v.prefs.wrap_columns = 4;
        v.relayout_in(cx);
    });
}

/// Makes the layout of the document complete, as the frames do, and lets the view settle.
fn settle_layout(cx: &mut TestAppContext, view: &Entity<EditorView>) {
    view.update(cx, |v, cx| {
        let doc = v.document().read(cx).doc().clone();
        v.layout.layout_all(&doc).unwrap();
        v.hold_frame_top(cx);
        v.settle_caret(cx);
    });
}

#[gpui::test]
fn with_a_wrap_the_caret_of_a_jump_is_on_the_screen_when_the_layout_is_done(
    cx: &mut TestAppContext,
) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    wrap_at_four(cx, &view);
    let on_screen = |cx: &mut TestAppContext| {
        view.read_with(cx, |v, app| {
            let row = v.caret_row(app).row as f32 * v.metrics.line_height;
            row >= v.scroll_y() - 0.5 && row < v.scroll_y() + v.metrics.view_height
        })
    };
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_some());
    settle_layout(cx, &view);
    assert!(on_screen(cx), "the end of the file");
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(70_000)], cx));
    settle_layout(cx, &view);
    assert!(on_screen(cx), "a line");
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn with_a_wrap_the_thumb_keeps_the_top_of_the_view_when_the_layout_is_done(
    cx: &mut TestAppContext,
) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    wrap_at_four(cx, &view);
    let total = big_text().len() as u64;
    let top = view.read_with(cx, |v, cx| v.scan_scrollbar(&v.metrics, cx).unwrap());
    let travel = top.track_height - top.thumb_height;
    view.update(cx, |v, cx| v.drag_scan_thumb(&top, travel / 2.0, cx));
    settle_layout(cx, &view);
    let (first, _) = view.read_with(cx, |v, cx| v.visible_bytes(&v.metrics, cx));
    assert!(first.abs_diff(total / 2) <= 12, "{first} for {}", total / 2);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_selection_keeps_the_window_where_it_is_until_it_is_dropped(cx: &mut TestAppContext) {
    let text: String = (0..300_000).map(|i| format!("line {i:06}\n")).collect();
    let (handle, dir) = opened(cx, "big.txt", text.as_bytes());
    let view = active_view(cx, handle);
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(150_000)], cx));
    let window = window_of(cx, &view).expect("a window was made");
    let a = view.read_with(cx, |v, _| v.cursor.caret());
    view.update(cx, |v, _| v.cursor.select(a, Pos::new(a.line + 5, 0)));
    let rows = view.read_with(cx, |v, _| v.layout.row_count());
    view.update(cx, |v, cx| {
        v.scroll_by(0.0, -(rows as f32) * 20.0);
        v.queue_scan_window_shift(true, cx);
    });
    cx.run_until_parked();
    // The window is not moved under the selection, which would lose it.
    assert_eq!(window_of(cx, &view), Some(window));
    assert!(view.read_with(cx, |v, _| v.cursor.has_selection()));
    view.update(cx, |v, cx| {
        v.cursor.clear_selection();
        v.queue_scan_window_shift(true, cx);
    });
    cx.run_until_parked();
    assert_ne!(window_of(cx, &view), Some(window));
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_jump_with_the_selection_lock_waits_and_the_place_left_is_in_the_jump_list(
    cx: &mut TestAppContext,
) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(3)], cx));
    // With the lock the jump extends the selection: it waits, as the jumps that select do.
    view.update(cx, |v, _| v.cursor.set_selecting_lock(true));
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_none());
    view.update(cx, |v, _| {
        v.cursor.set_selecting_lock(false);
        v.pending_scan_jump = None;
    });
    // Without it the jump shows the end at once, and the place it left is recorded.
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_some());
    super::huge_tests::wait_for_scan(cx, &view);
    run(cx, handle, "F_JUMPHIST_PREV");
    assert_eq!(super::huge_tests::caret_line(cx, &view), 2);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_selection_lock_of_a_window_is_kept_when_the_window_is_replaced(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_some());
    // A lock with nothing selected yet.
    view.update(cx, |v, _| v.cursor.set_selecting_lock(true));
    super::huge_tests::wait_for_scan(cx, &view);
    assert!(window_of(cx, &view).is_none());
    assert!(
        view.read_with(cx, |v, _| v.cursor.selecting_lock()),
        "the lock is still on when the whole file replaces the window"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_selection_lock_with_a_selection_is_kept_when_the_window_moves(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    run(cx, handle, "F_GOFILEEND");
    let window = window_of(cx, &view).expect("a window was made");
    let a = view.read_with(cx, |v, _| v.cursor.caret());
    view.update(cx, |v, _| {
        v.cursor.select(Pos::new(a.line.saturating_sub(5), 0), a);
        v.cursor.set_selecting_lock(true);
    });
    // A drag of the thumb moves the window (a selection does not stop that); the selection and
    // its lock go with it.
    let top = view.read_with(cx, |v, cx| v.scan_scrollbar(&v.metrics, cx).unwrap());
    view.update(cx, |v, cx| v.drag_scan_thumb(&top, 0.0, cx));
    assert_ne!(window_of(cx, &view), Some(window));
    assert!(view.read_with(cx, |v, _| v.cursor.selecting_lock()));
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_window_does_not_put_its_own_positions_in_the_jump_list(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(3)], cx));
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_some());
    let len = |cx: &mut TestAppContext| view.read_with(cx, |v, _| v.cursor.jump_history().len());
    // The places the jumps left are in the list; a command that records its own place (select all)
    // would put a position of the window in it, which is not a line of the file.
    run(cx, handle, "F_SELECTALL");
    super::huge_tests::wait_for_scan(cx, &view);
    // The two jumps of the test left one place each; select all left none.
    assert_eq!(len(cx), 2, "only the places that the two jumps left");
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_jump_list_put_aside_goes_when_another_document_is_loaded(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    run(cx, handle, "F_GOFILEEND");
    assert!(view.read_with(cx, |v, _| v.has_stashed_jumps()));
    view.update(cx, |v, cx| {
        let new =
            crate::document::SharedDocument::new(matsu_core::model::DocumentModel::empty(), None);
        v.load_document(new, None, cx);
    });
    assert!(
        !view.read_with(cx, |v, _| v.has_stashed_jumps()),
        "the list of the old document is not the new one's"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn with_a_wrap_numbered_by_rows_a_window_is_numbered_by_lines(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    wrap_at_four(cx, &view);
    // The type settings number the rows (`bLineNumIsCRLF` off): the rows before a window are not
    // known, so the window is numbered by the lines, as the lines before it are counted.
    view.update(cx, |v, _| v.prefs.settings.outline.line_num_is_crlf = false);
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(70_000)], cx));
    settle_layout(cx, &view);
    assert!(window_of(cx, &view).is_some());
    let (line, _) = super::huge_scan_tests::status_line(cx, &view);
    assert_eq!(line, 70_000);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn an_empty_box_selection_keeps_its_mode_when_the_scan_is_done(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    run(cx, handle, "F_GOFILEEND");
    view.update(cx, |v, _| {
        v.cursor.move_to(Pos::new(10, 1), false);
        v.cursor.begin_box(1, true);
    });
    super::huge_tests::wait_for_scan(cx, &view);
    assert!(view.read_with(cx, |v, _| v.cursor.is_box() && v.cursor.selecting_lock()));
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_line_beyond_the_end_goes_to_the_empty_row_after_the_last_line_ending(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    // The file has 100 000 lines and ends with a line ending: a line far beyond is the end.
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(900_000)], cx));
    assert!(window_of(cx, &view).is_some());
    super::huge_tests::wait_for_scan(cx, &view);
    assert_eq!(
        view.read_with(cx, |v, _| v.cursor.caret()),
        Pos::new(100_000, 0)
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_box_selection_in_a_window_is_kept_when_the_scan_is_done(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_some());
    view.update(cx, |v, _| {
        v.cursor.move_to(Pos::new(10, 1), false);
        v.cursor.begin_box(1, false);
        v.cursor.move_box_to(Pos::new(12, 3), 3);
    });
    let (anchor_line, lines) = view.read_with(cx, |v, cx| {
        (
            v.cursor.anchor().unwrap().line,
            v.document().read(cx).doc().line_count(),
        )
    });
    super::huge_tests::wait_for_scan(cx, &view);
    let (is_box, corners) = view.read_with(cx, |v, _| (v.cursor.is_box(), v.cursor.box_corners()));
    assert!(is_box);
    let ((a, ac), (c, cc)) = corners.expect("the box covers something");
    // The same lines, now counted from the start of the file: further down than the window's.
    assert_eq!((c.line - a.line, a.col, c.col, ac, cc), (2, 1, 3, 1, 3));
    assert!(a.line > anchor_line && a.line < 100_000 + lines);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_jump_list_waits_for_the_scan_while_a_window_is_shown(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(3)], cx));
    run(cx, handle, "F_GOFILEEND");
    assert!(window_of(cx, &view).is_some());
    let at = view.read_with(cx, |v, _| v.cursor.caret());
    // The place in the list is a line of the file, not of the window.
    run(cx, handle, "F_JUMPHIST_PREV");
    assert_eq!(view.read_with(cx, |v, _| v.cursor.caret()), at);
    super::huge_tests::wait_for_scan(cx, &view);
    assert_eq!(super::huge_tests::caret_line(cx, &view), 2);
    std::fs::remove_dir_all(dir).ok();
}
