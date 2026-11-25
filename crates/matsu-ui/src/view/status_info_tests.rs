//! Tests of what the status bar says about the caret and the selection (`status_info.rs`).

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_core::buffer::{Document, Pos};
use matsu_core::encoding::Encoding;
use matsu_core::model::DocumentModel;

use super::EditorView;
use super::status_info::measure;
use crate::document::SharedDocument;
use crate::status_options::{SelectCountMode, StatusbarOptions};

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Entity<EditorView>, &'a mut VisualTestContext) {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    })
}

fn put_caret(view: &Entity<EditorView>, cx: &mut VisualTestContext, line: usize, col: usize) {
    view.update(cx, |v, _| v.cursor.move_to(Pos::new(line, col), false));
}

fn select(view: &Entity<EditorView>, cx: &mut VisualTestContext, a: Pos, b: Pos) {
    view.update(cx, |v, _| v.cursor.select(a, b));
}

fn message(view: &Entity<EditorView>, cx: &mut VisualTestContext, bytes: bool) -> Option<String> {
    view.update(cx, |v, cx| {
        let options = StatusbarOptions {
            select_count_by_byte: bytes,
            ..StatusbarOptions::default()
        };
        v.selection_message(options, cx)
    })
}

fn code(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.update(cx, |v, cx| {
        v.caret_char_code(StatusbarOptions::default(), cx)
    })
}

#[gpui::test]
fn the_code_at_the_caret(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "aあ\r\n😀");
    assert_eq!(code(&v, cx), "U+0061");
    put_caret(&v, cx, 0, 1);
    assert_eq!(code(&v, cx), "U+3042");
    // at the line ending: its name; the end of the line has no character
    put_caret(&v, cx, 0, 4);
    assert_eq!(code(&v, cx), "CRLF");
    put_caret(&v, cx, 1, 0);
    assert_eq!(code(&v, cx), "U+1F600");
    // the end of the file
    put_caret(&v, cx, 1, 4);
    assert_eq!(code(&v, cx), "");
}

#[gpui::test]
fn an_empty_document_has_no_code(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "");
    assert_eq!(code(&v, cx), "");
    v.read_with(cx, |v, cx| {
        assert_eq!(v.status(cx).position_text(), "    1 行    1 桁");
    });
}

#[gpui::test]
fn the_code_is_a_part_of_the_status_bar(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "あ");
    v.read_with(cx, |v, cx| {
        let status = v.status(cx);
        assert_eq!(status.text(crate::status::Part::CharCode), "U+3042");
    });
}

#[gpui::test]
fn the_selection_in_characters(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "ab\r\ncde\nf");
    assert_eq!(message(&v, cx, false), None);
    select(&v, cx, Pos::new(0, 1), Pos::new(0, 2));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("1 chars (1 lines) selected.")
    );
    // over a line: "b" + CRLF + "cd" = 5 units, 2 lines
    select(&v, cx, Pos::new(0, 1), Pos::new(1, 2));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("5 chars (2 lines) selected.")
    );
    // to the start of a line: that line is not counted
    select(&v, cx, Pos::new(0, 0), Pos::new(1, 0));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("4 chars (1 lines) selected.")
    );
    // the caret before the anchor
    select(&v, cx, Pos::new(1, 3), Pos::new(0, 0));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("7 chars (2 lines) selected.")
    );
}

#[gpui::test]
fn a_surrogate_pair_is_two_characters(cx: &mut TestAppContext) {
    // Sakura counts the units of its UTF-16 buffer.
    let (v, cx) = setup(cx, "😀あ");
    select(&v, cx, Pos::new(0, 0), Pos::new(0, 7));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("3 chars (1 lines) selected.")
    );
}

#[gpui::test]
fn the_selection_in_bytes(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "aあ\r\nb");
    select(&v, cx, Pos::new(0, 0), Pos::new(1, 1));
    // UTF-8: 1 + 3 + 2 + 1
    assert_eq!(
        message(&v, cx, true).as_deref(),
        Some("7 bytes (2 lines) selected.")
    );
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("5 chars (2 lines) selected.")
    );
}

#[gpui::test]
fn growing_the_selection_counts_the_difference(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "abc\ndef\nghi");
    select(&v, cx, Pos::new(0, 1), Pos::new(0, 2));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("1 chars (1 lines) selected.")
    );
    select(&v, cx, Pos::new(0, 1), Pos::new(1, 2));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("5 chars (2 lines) selected.")
    );
    // shrinking at the same end
    select(&v, cx, Pos::new(0, 1), Pos::new(0, 3));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("2 chars (1 lines) selected.")
    );
    // the other end moves
    select(&v, cx, Pos::new(0, 0), Pos::new(0, 3));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("3 chars (1 lines) selected.")
    );
}

#[gpui::test]
fn a_box_shows_its_width_and_height(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "abcd\nefgh\nijkl");
    v.update(cx, |v, _| {
        v.cursor.begin_box(64, false);
        v.cursor.move_box_to(Pos::new(2, 3), 3 * 64);
    });
    let m = message(&v, cx, false).expect("a message");
    assert!(m.starts_with("2 col ("), "{m}");
    assert!(m.ends_with("px) * 3 lines selected."), "{m}");
}

#[gpui::test]
fn selecting_without_text_says_so(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "abc");
    v.update(cx, |v, _| v.cursor.set_selecting_lock(true));
    assert_eq!(message(&v, cx, false).as_deref(), Some("selecting"));
    v.update(cx, |v, _| {
        v.cursor.set_selecting_lock(false);
        v.cursor.begin_box(0, false);
    });
    assert_eq!(message(&v, cx, false).as_deref(), Some("box selecting"));
}

#[gpui::test]
fn the_mode_of_the_window_wins_over_the_setting(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "aあ");
    select(&v, cx, Pos::new(0, 0), Pos::new(0, 4));
    v.update(cx, |v, _| v.prefs.select_count = SelectCountMode::Bytes);
    // the setting says characters, the window says bytes
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("4 bytes (1 lines) selected.")
    );
    v.update(cx, |v, _| v.prefs.select_count = SelectCountMode::Chars);
    assert_eq!(
        message(&v, cx, true).as_deref(),
        Some("2 chars (1 lines) selected.")
    );
}

#[gpui::test]
fn the_column_by_characters(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx, "\tあい");
    put_caret(&v, cx, 0, 4);
    v.read_with(cx, |v, cx| {
        let ruler = StatusbarOptions::default();
        let chars = StatusbarOptions {
            column_by_char: true,
            ..ruler
        };
        // tab (4 cells) and a wide character (2 cells): the ruler column
        assert_eq!(v.status_position(ruler, cx), (1, 7));
        // the characters before the caret: the tab and one letter
        assert_eq!(v.status_position(chars, cx), (1, 3));
    });
}

#[gpui::test]
fn the_function_toggles_the_mode_and_the_menu_shows_it(cx: &mut TestAppContext) {
    // F_SELECT_COUNT_MODE
    const CODE: u16 = 31144;
    let (v, cx) = setup(cx, "ab");
    let checked = |v: &Entity<EditorView>, cx: &mut VisualTestContext| {
        v.read_with(cx, |v, cx| {
            crate::menu_state::FuncStates::of_view(v, false, false, 1, cx).is_checked(CODE)
        })
    };
    assert!(!checked(&v, cx));
    v.update_in(cx, |v, window, cx| v.run_function(CODE, &[], window, cx));
    assert!(checked(&v, cx));
    assert_eq!(
        v.read_with(cx, |v, _| v.prefs.select_count),
        SelectCountMode::Bytes
    );
    v.update_in(cx, |v, window, cx| v.run_function(CODE, &[], window, cx));
    assert!(!checked(&v, cx));
    // a number: 2 is bytes
    let two = [matsu_commands::command::Arg::Int(2)];
    v.update_in(cx, |v, window, cx| v.run_function(CODE, &two, window, cx));
    assert!(checked(&v, cx));
}

#[test]
fn the_bytes_of_other_encodings() {
    let doc = Document::from_utf8("aあ\nい".as_bytes().to_vec()).unwrap();
    let end = Pos::new(1, 3);
    let start = Pos::new(0, 0);
    // Shift_JIS: 1 + 2 + 1 (LF) + 2
    assert_eq!(measure(&doc, Encoding::ShiftJis, start, end, true), 6);
    // EUC-JP is the same width
    assert_eq!(measure(&doc, Encoding::EucJp, start, end, true), 6);
    // UTF-16: every character is two bytes
    assert_eq!(measure(&doc, Encoding::Utf16Le, start, end, true), 8);
    // characters do not depend on the encoding
    assert_eq!(measure(&doc, Encoding::ShiftJis, start, end, false), 4);
}

// ---- the ends of a selection at the end of a wrapped row ----
//
// Sakura keeps the ends as layout points: the end of a wrapped row is on that row, not at the start
// of the next (`CViewSelect::PrintSelectionInfoMsg`: `select_line = To.y - From.y + 1`, one less
// when `To` is at index 0 of its row). The line wraps every 10 columns:
// rows `0..10`, `10..20`, `20..30`, `30..40` and `40..44`.

const WRAPPED: &str = "abcdefghijklmnopqrstuvwxyz0123456789ABCD\nxy";

fn wrapped(cx: &mut TestAppContext) -> (Entity<EditorView>, &mut VisualTestContext) {
    let (v, cx) = setup(cx, WRAPPED);
    v.update(cx, |v, cx| v.set_wrap_columns(10, cx));
    cx.run_until_parked();
    (v, cx)
}

fn run(v: &Entity<EditorView>, cx: &mut VisualTestContext, ids: &[&str]) {
    for id in ids {
        v.update(cx, |v, cx| v.run_command(id, &[], cx));
    }
}

#[gpui::test]
fn an_anchor_at_the_end_of_a_wrapped_row_counts_its_row(cx: &mut TestAppContext) {
    let (v, cx) = wrapped(cx);
    // End on the first row, then Shift+Down twice: the rows 0 to 2
    run(
        &v,
        cx,
        &["go_line_end", "cursor_down_sel", "cursor_down_sel"],
    );
    v.read_with(cx, |v, _| {
        assert_eq!(v.cursor.anchor(), Some(Pos::new(0, 10)));
        assert_eq!(v.cursor.caret(), Pos::new(0, 30));
        assert_eq!(v.cursor.anchor_free_col(), Some(10 * 64));
    });
    // Sakura: From.y = 0, To.y = 2, To is at x = 10 of its row: 3 lines
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("20 chars (3 lines) selected.")
    );
}

#[gpui::test]
fn a_caret_at_the_end_of_a_wrapped_row_counts_its_row_at_the_start(cx: &mut TestAppContext) {
    let (v, cx) = wrapped(cx);
    // from the middle of the third row up to the end of the first
    put_caret(&v, cx, 0, 25);
    run(
        &v,
        cx,
        &["cursor_up_sel", "cursor_up_sel", "go_line_end_sel"],
    );
    v.read_with(cx, |v, _| {
        assert_eq!(v.cursor.caret(), Pos::new(0, 10));
        assert_eq!(v.cursor.free_col(), Some(10 * 64));
        assert_eq!(v.cursor.anchor(), Some(Pos::new(0, 25)));
    });
    // the rows 0 to 2; `To` is at x = 5 of row 2
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("15 chars (3 lines) selected.")
    );
}

#[gpui::test]
fn a_caret_at_the_end_of_a_wrapped_row_ends_on_that_row(cx: &mut TestAppContext) {
    let (v, cx) = wrapped(cx);
    // from the middle of the first row to the end of the third: the rows 0 to 2. The end is
    // on row 2 and not at the start of row 3, so no line is taken off.
    put_caret(&v, cx, 0, 5);
    run(
        &v,
        cx,
        &["cursor_down_sel", "cursor_down_sel", "go_line_end_sel"],
    );
    v.read_with(cx, |v, _| {
        assert_eq!(v.cursor.caret(), Pos::new(0, 30));
        assert_eq!(v.cursor.free_col(), Some(10 * 64));
    });
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("25 chars (3 lines) selected.")
    );
    // the same text with the caret at the start of row 3: `To` is at index 0 of row 3, which is
    // not counted, so the same 3 lines
    select(&v, cx, Pos::new(0, 5), Pos::new(0, 30));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("25 chars (3 lines) selected.")
    );
}

#[gpui::test]
fn an_end_at_the_start_of_a_row_is_not_counted_and_a_stale_mark_is_ignored(
    cx: &mut TestAppContext,
) {
    let (v, cx) = wrapped(cx);
    // a mark on a position that does not start a row that continues a line moves nothing
    select(&v, cx, Pos::new(0, 3), Pos::new(0, 25));
    v.update(cx, |v, _| v.cursor.set_anchor_free_col(Some(10 * 64)));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("22 chars (3 lines) selected.")
    );
    // an end at the start of its row, with no mark: the row is not counted
    select(&v, cx, Pos::new(0, 3), Pos::new(0, 20));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("17 chars (2 lines) selected.")
    );
    // the anchor at the start of a row, with no mark: its own row
    select(&v, cx, Pos::new(0, 10), Pos::new(0, 25));
    assert_eq!(
        message(&v, cx, false).as_deref(),
        Some("15 chars (2 lines) selected.")
    );
}

#[gpui::test]
fn a_box_that_starts_at_the_end_of_a_wrapped_row_starts_in_its_gap(cx: &mut TestAppContext) {
    let (v, cx) = wrapped(cx);
    // End on row 1, then a box two rows down: the corner is in the gap after the row, and the
    // box covers rows 1 to 3
    put_caret(&v, cx, 0, 15);
    run(
        &v,
        cx,
        &[
            "go_line_end",
            "begin_box",
            "cursor_left_box",
            "cursor_down_box",
            "cursor_down_box",
        ],
    );
    v.read_with(cx, |v, _| {
        // the corner begins in the gap after row 1; Left goes onto the last char of the row
        let ((a, _), (c, _)) = v.cursor.box_corners().expect("a box");
        assert_eq!((a, c), (Pos::new(0, 20), Pos::new(0, 39)));
    });
    let text = message(&v, cx, false).expect("a box");
    assert!(text.ends_with("* 3 lines selected."), "{text}");
}
