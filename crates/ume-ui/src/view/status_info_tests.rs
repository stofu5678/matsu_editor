//! Tests of what the status bar says about the caret and the selection (`status_info.rs`).

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_core::buffer::{Document, Pos};
use ume_core::encoding::Encoding;
use ume_core::model::DocumentModel;

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
        v.selection_message(&options, cx)
    })
}

fn code(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.update(cx, |v, cx| {
        v.caret_char_code(&StatusbarOptions::default(), cx)
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
        v.cursor.begin_box(1, false);
        v.cursor.move_box_to(Pos::new(2, 3), 3);
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
        assert_eq!(v.status_position(&ruler, cx), (1, 7));
        // the characters before the caret: the tab and one letter
        assert_eq!(v.status_position(&chars, cx), (1, 3));
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
    let two = [ume_commands::command::Arg::Int(2)];
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
