//! Tests of the clipboard commands, on the in-memory clipboard. The expectations come from
//! `sakura_core/cmd/CViewCommander_Clipboard.cpp` and the functions it calls (see each command).

use ume_core::buffer::Document;
use ume_core::cursor::Cursor;
use ume_core::model::DocumentModel;

use crate::clipboard::{ClipData, ClipKind};
use crate::command::{Arg, CommandError};
use crate::context::CommandContext;
use crate::layout::PlainLayout;
use crate::registry::Registry;
use crate::testing::Editor;

fn clip(e: &Editor) -> (String, ClipKind) {
    let d = e.clipboard().expect("something on the clipboard");
    (d.text.clone(), d.kind)
}

fn normal(text: &str) -> (String, ClipKind) {
    (text.to_owned(), ClipKind::Normal)
}

fn line(text: &str) -> (String, ClipKind) {
    (text.to_owned(), ClipKind::Line)
}

#[test]
fn copy_copies_the_selection_and_keeps_it() {
    let mut e = Editor::new("a[bc\r\nd|e");
    e.run("copy").assert_marked("a[bc\r\nd|e");
    assert_eq!(clip(&e), normal("bc\r\nd"));
}

#[test]
fn copy_without_a_selection_copies_the_row_as_a_line_copy() {
    let mut e = Editor::new("ab|c\r\nde");
    e.run("copy").assert_marked("ab|c\r\nde");
    assert_eq!(clip(&e), line("abc\r\n"));
    // the last row has no line ending
    let mut e = Editor::new("abc\r\nd|e");
    e.run("copy");
    assert_eq!(clip(&e), line("de"));
}

#[test]
fn copy_on_the_empty_last_line_does_nothing_and_does_not_beep() {
    let mut e = Editor::new("ab\n|");
    e.run("copy");
    assert!(e.clipboard().is_none());
    assert!(!e.changes().beep);
}

#[test]
fn copy_crlf_changes_the_line_endings() {
    let mut e = Editor::new("[a\nb\rc|\u{2028}d");
    e.run("copy_crlf");
    assert_eq!(clip(&e), normal("a\r\nb\r\nc"));
    let mut e = Editor::new("a|b\nc");
    e.run("copy_crlf");
    assert_eq!(clip(&e), line("ab\r\n"));
}

#[test]
fn cut_copies_and_deletes_the_selection() {
    let mut e = Editor::new("a[bc|d");
    e.run("cut").assert_marked("a|d");
    assert_eq!(clip(&e), normal("bc"));
    e.run("undo").assert_marked("a[bc|d");
}

#[test]
fn cut_without_a_selection_cuts_the_row() {
    let mut e = Editor::new("ab|cd\r\nef");
    e.run("cut").assert_marked("ef|");
    assert_eq!(clip(&e), line("abcd\r\n"));
}

#[test]
fn cut_line_beeps_with_a_selection_and_without_a_row() {
    for text in ["a[b|c", "ab\n|"] {
        let mut e = Editor::new(text);
        e.run("cut_line").assert_marked(text);
        assert!(e.changes().beep, "{text:?}");
        assert!(e.clipboard().is_none(), "{text:?}");
    }
    let mut e = Editor::new("x\nab|\nc");
    e.run("cut_line").assert_marked("x\nc|");
    assert_eq!(clip(&e), line("ab\n"));
}

#[test]
fn paste_puts_the_text_in_place_of_the_selection() {
    let mut e = Editor::new("a[bc|d").with_clipboard("X\r\nY", ClipKind::Normal);
    e.run("paste").assert_marked("aX\r\nY|d");
    e.run("undo").assert_marked("a[bc|d");
}

#[test]
fn paste_does_not_convert_line_endings_by_default() {
    // bConvertEOLPaste is off by default (CShareData.cpp)
    let mut e = Editor::new("|").with_clipboard("a\nb\r\nc", ClipKind::Normal);
    e.run("paste").assert_marked("a\nb\r\nc|");
}

#[test]
fn paste_option_converts_the_line_endings_to_the_documents() {
    // Command_PASTE: option 0x01 converts to the new line code of the document. A document without
    // a line ending has the one of the OS (CRLF on Windows, as in Sakura; LF elsewhere).
    let os = if cfg!(windows) { "\r\n" } else { "\n" };
    let mut e = Editor::new("|").with_clipboard("a\nb\rc\r\nd", ClipKind::Normal);
    e.run_with("paste", &[Arg::Int(1)])
        .assert_marked(&format!("a{os}b{os}c{os}d|"));
}

#[test]
fn paste_of_a_line_copy_goes_to_the_start_of_the_row_and_the_caret_stays() {
    let mut e = Editor::new("ab|cd\r\nef").with_clipboard("XY\r\n", ClipKind::Line);
    e.run("paste").assert_marked("XY\r\nab|cd\r\nef");
    e.run("undo").assert_marked("ab|cd\r\nef");
}

#[test]
fn paste_of_a_line_copy_adds_a_line_ending_if_it_has_none() {
    // the line ending of the OS, since the document has none yet
    let os = if cfg!(windows) { "\r\n" } else { "\n" };
    let mut e = Editor::new("a|b").with_clipboard("XY", ClipKind::Line);
    e.run("paste").assert_marked(&format!("XY{os}a|b"));
}

#[test]
fn paste_of_a_line_copy_over_a_selection_deletes_the_selection_first() {
    let mut e = Editor::new("a[bc|d\nef").with_clipboard("XY\n", ClipKind::Line);
    e.run("paste").assert_marked("XY\na|d\nef");
}

#[test]
fn a_line_copy_that_ends_with_an_extended_line_ending_gets_no_second_one() {
    let mut e = Editor::new("a|b").with_clipboard("XY\u{2028}", ClipKind::Line);
    e.run("paste").assert_marked("XY\u{2028}a|b");
}

#[test]
fn paste_option_can_turn_line_mode_off() {
    let mut e = Editor::new("ab|cd").with_clipboard("XY\n", ClipKind::Line);
    e.run_with("paste", &[Arg::Int(0x08)])
        .assert_marked("abXY\n|cd");
}

#[test]
fn paste_beeps_without_text_on_the_clipboard() {
    let mut e = Editor::new("ab|");
    e.run("paste").assert_marked("ab|");
    assert!(e.changes().beep);
}

#[test]
fn copy_then_paste_round_trips_through_the_clipboard() {
    let mut e = Editor::new("[ab|c\nd");
    e.run("copy");
    e.run("go_file_end").run("paste").assert_marked("abc\ndab|");
}

#[test]
fn copy_lines_widens_the_selection_to_whole_rows() {
    let mut e = Editor::new("a[b\r\ncd\r\ne|f\r\ngh");
    e.run("copy_lines").assert_marked("[ab\r\ncd\r\nef\r\n|gh");
    assert_eq!(clip(&e), normal("ab\r\ncd\r\nef\r\n"));
}

#[test]
fn copy_lines_does_not_take_the_row_where_the_selection_ends_at_its_start() {
    let mut e = Editor::new("a[b\r\n|cd");
    e.run("copy_lines").assert_marked("[ab\r\n|cd");
    assert_eq!(clip(&e), normal("ab\r\n"));
}

#[test]
fn copy_lines_without_a_selection_does_nothing() {
    let mut e = Editor::new("ab|c");
    e.run("copy_lines");
    assert!(e.clipboard().is_none());
    assert!(!e.changes().beep);
}

#[test]
fn copy_lines_as_passage_quotes_each_row_and_ends_the_last_one() {
    let mut e = Editor::new("[ab\r\ncd|");
    e.run("copy_lines_as_passage");
    // the last row has no line ending, so it gets the document's (CRLF)
    assert_eq!(clip(&e), normal("> ab\r\n> cd\r\n"));
}

#[test]
fn copy_lines_with_line_number_numbers_the_lines() {
    let mut e = Editor::new("[ab\ncd|");
    e.run("copy_lines_with_line_number");
    // width: max(2, digits of the line count) + 2
    assert_eq!(clip(&e), normal("  1:ab\n  2:cd\n"));
    let text = vec!["x"; 100].join("\n");
    let mut e = Editor::new(&format!("[{text}|"));
    e.run("copy_lines_with_line_number");
    let (copied, _) = clip(&e);
    assert!(copied.starts_with("   1:x\n"), "{copied:?}");
    assert!(copied.contains("\n  99:x\n"), "{copied:?}");
    assert!(copied.contains("\n 100:x\n"), "{copied:?}");
}

#[test]
fn file_name_path_and_tag() {
    let mut e = Editor::new("ab\n日本|語").with_path("/dir/sub/file.txt");
    e.run("copy_file_name");
    assert_eq!(clip(&e), normal("file.txt"));
    e.run("copy_path");
    assert_eq!(clip(&e), normal("/dir/sub/file.txt"));
    // line 2, column 3 (two UTF-16 units before the caret, counted from 1)
    e.run("copy_tag");
    assert_eq!(clip(&e), normal("/dir/sub/file.txt (2,3): "));
}

#[test]
fn file_commands_beep_without_a_file() {
    for id in ["copy_file_name", "copy_path", "copy_tag"] {
        let mut e = Editor::new("a|b");
        e.run(id);
        assert!(e.changes().beep, "{id}");
        assert!(e.clipboard().is_none(), "{id}");
    }
}

#[test]
fn line_cut_to_start_cuts_the_line_not_the_row() {
    let mut e = Editor::new("ab\ncd|ef");
    e.run("line_cut_to_start").assert_marked("ab\n|ef");
    assert_eq!(clip(&e), normal("cd"));
    // at the start of the line: beep
    let mut e = Editor::new("ab\n|cd");
    e.run("line_cut_to_start").assert_marked("ab\n|cd");
    assert!(e.changes().beep);
}

#[test]
fn line_cut_to_end_keeps_the_line_ending() {
    let mut e = Editor::new("ab|cd\r\nef");
    e.run("line_cut_to_end").assert_marked("ab|\r\nef");
    assert_eq!(clip(&e), normal("cd"));
    let mut e = Editor::new("ab|\r\nef");
    e.run("line_cut_to_end");
    assert!(e.changes().beep);
}

#[test]
fn line_cuts_with_a_selection_cut_the_selection() {
    let mut e = Editor::new("a[bc|d");
    e.run("line_cut_to_start").assert_marked("a|d");
    assert_eq!(clip(&e), normal("bc"));
    let mut e = Editor::new("a[bc|d");
    e.run("line_cut_to_end").assert_marked("a|d");
}

#[test]
fn a_context_without_a_clipboard_answers_an_error() {
    let mut model = DocumentModel::new(Document::from_utf8(b"ab".to_vec()).unwrap());
    let mut cursor = Cursor::default();
    let mut layout = PlainLayout::default();
    let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
    let result = Registry::global().run("copy", &mut cx, &[]);
    assert!(
        matches!(result, Err(CommandError::Unavailable(_))),
        "{result:?}"
    );
}

#[test]
fn the_clip_data_type_compares_text_and_kind() {
    assert_ne!(
        ClipData::new("a", ClipKind::Normal),
        ClipData::new("a", ClipKind::Line)
    );
}
