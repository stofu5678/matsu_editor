//! Tests of editing a box selection. The expectations come from the Sakura sources named in
//! [`box_edit`](super::box_edit): `GetSelectedData`, `DeleteData`, `Command_INDENT`,
//! `Command_PASTEBOX`, `ConvSelectedArea`.
//!
//! A box is written as its two corners, each `(row, display column)`, with the display columns
//! of [`PlainLayout`]: tabs to multiples of 4, full-width chars two columns.

use matsu_core::buffer::{Document, Eol, Pos};
use matsu_core::cursor::Cursor;
use matsu_core::model::DocumentModel;

use crate::clipboard::{ClipKind, MemoryClipboard};
use crate::command::{Arg, CommandError};
use crate::context::CommandContext;
use crate::layout::{Layout, PlainLayout};
use crate::registry::Registry;
use crate::testing::Editor;

/// An editor with a box selected: the anchor and the caret corners as `(row, display column)`.
/// Rows past the last one are the empty row at the end.
fn boxed(text: &str, anchor: (usize, usize), caret: (usize, usize)) -> Editor {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let layout = PlainLayout::default();
    let pos = |(line, col): (usize, usize)| {
        layout
            .pos_at_col(&doc, Pos::new(line, 0), col)
            .expect("a position")
    };
    let mut cursor = Cursor::default();
    cursor.move_to(pos(anchor), false);
    cursor.begin_box(anchor.1, false);
    cursor.move_box_to(pos(caret), caret.1);
    Editor::from_parts(text, cursor)
}

fn text_of(e: &Editor) -> String {
    let doc = e.model().doc();
    doc.text_between(Pos::new(0, 0), doc.end_pos().unwrap())
        .unwrap()
}

fn clip(e: &Editor) -> (String, ClipKind) {
    let data = e.clipboard().expect("something on the clipboard");
    (data.text.clone(), data.kind)
}

// ---- copy and cut ----

#[test]
fn copy_gives_the_text_of_each_row_with_the_box_mark() {
    let mut e = boxed("abcd\nefgh\nijkl", (0, 1), (2, 3));
    e.run("copy");
    assert_eq!(clip(&e), ("bc\r\nfg\r\njk\r\n".to_owned(), ClipKind::Box));
    // the selection and the text stay
    e.assert_box((0, 1), (2, 3));
    assert_eq!(text_of(&e), "abcd\nefgh\nijkl");
    assert!(!e.changes().beep);
}

#[test]
fn copy_works_in_either_direction() {
    let mut e = boxed("abcd\nefgh\nijkl", (2, 3), (0, 1));
    e.run("copy");
    assert_eq!(clip(&e).0, "bc\r\nfg\r\njk\r\n");
}

#[test]
fn copy_uses_crlf_whatever_the_document_has() {
    for text in ["abc\ndef", "abc\rdef", "abc\r\ndef"] {
        let mut e = boxed(text, (0, 1), (1, 2));
        e.run("copy");
        assert_eq!(clip(&e).0, "b\r\ne\r\n", "{text:?}");
    }
}

#[test]
fn copy_never_takes_a_line_ending() {
    // the box reaches past the text, over the line endings
    for text in ["ab\ncd\nef", "ab\r\ncd\r\nef", "ab\rcd\ref"] {
        let mut e = boxed(text, (0, 1), (2, 9));
        e.run("copy");
        assert_eq!(clip(&e).0, "b\r\nd\r\nf\r\n", "{text:?}");
    }
}

#[test]
fn copy_short_rows_and_rows_past_the_text_are_empty_lines() {
    // line 1 is shorter than the box, line 2 is the empty row after the final line ending
    let mut e = boxed("abcdef\nab\n", (0, 3), (2, 5));
    e.run("copy");
    assert_eq!(clip(&e).0, "de\r\n\r\n\r\n");
}

#[test]
fn copy_a_box_that_covers_no_text_is_all_empty_lines() {
    let mut e = boxed("ab\ncd", (0, 5), (1, 7));
    e.run("copy");
    assert_eq!(clip(&e), ("\r\n\r\n".to_owned(), ClipKind::Box));
}

#[test]
fn copy_full_width_chars_are_taken_by_the_column_that_covers_them() {
    // 日(0-1) 本(2-3) 語(4-5) a(6)
    let mut e = boxed("日本語a\nabcdefg", (0, 2), (1, 4));
    e.run("copy");
    assert_eq!(clip(&e).0, "本\r\ncd\r\n");
    // the left edge inside a char takes that char; the right edge inside a char leaves it out
    let mut e = boxed("日本語a\nabcdefg", (0, 1), (1, 3));
    e.run("copy");
    assert_eq!(clip(&e).0, "日\r\nbc\r\n");
    let mut e = boxed("日本語a\nabcdefg", (0, 1), (1, 5));
    e.run("copy");
    assert_eq!(clip(&e).0, "日本\r\nbcde\r\n");
}

#[test]
fn copy_tabs_count_by_the_columns_they_cover() {
    // a(0) tab(1-3) b(4) c(5)
    let mut e = boxed("a\tbc\nabcdefgh", (0, 2), (1, 6));
    e.run("copy");
    assert_eq!(clip(&e).0, "\tbc\r\ncdef\r\n");
}

#[test]
fn cut_copies_the_box_and_deletes_it_in_one_undo_step() {
    let mut e = boxed("abcd\nefgh\nijkl", (0, 1), (2, 3));
    e.run("cut");
    assert_eq!(clip(&e), ("bc\r\nfg\r\njk\r\n".to_owned(), ClipKind::Box));
    e.assert_marked("a|d\neh\nil");
    assert!(e.cursor().mode() == matsu_core::cursor::SelectionMode::Normal);
    e.run("undo");
    assert_eq!(text_of(&e), "abcd\nefgh\nijkl");
    e.assert_box((0, 1), (2, 3));
    e.run("redo");
    assert_eq!(text_of(&e), "ad\neh\nil");
}

#[test]
fn cut_a_box_of_empty_space_copies_empty_lines_and_changes_nothing() {
    let mut e = boxed("ab\ncd", (0, 4), (1, 6));
    e.run("cut");
    assert_eq!(clip(&e), ("\r\n\r\n".to_owned(), ClipKind::Box));
    assert_eq!(text_of(&e), "ab\ncd");
    assert!(!e.changes().text_changed());
    // the selection ended, the caret is at the top left corner (past the text: the end of it)
    assert!(!e.cursor().has_selection());
    assert_eq!(e.cursor().caret(), Pos::new(0, 2));
}

// ---- delete ----

#[test]
fn delete_and_backspace_delete_the_box() {
    for id in ["delete", "delete_back", "word_delete"] {
        let mut e = boxed("abcd\nefgh\nijkl", (0, 1), (2, 3));
        e.run(id);
        e.assert_marked("a|d\neh\nil");
        assert!(!e.changes().beep, "{id}");
        assert_eq!(e.changes().edits.len(), 3, "{id}");
        e.run("undo");
        assert_eq!(text_of(&e), "abcd\nefgh\nijkl", "{id}");
    }
}

#[test]
fn delete_takes_only_the_text_in_short_rows_and_keeps_line_endings() {
    let mut e = boxed("abcdef\nab\n\ncdefg", (0, 2), (3, 4));
    e.run("delete");
    assert_eq!(text_of(&e), "abef\nab\n\ncdg");
    // the caret is at the top left corner
    assert_eq!(e.cursor().caret(), Pos::new(0, 2));
}

#[test]
fn delete_over_japanese_text_and_tabs() {
    // 日(0-1) 本(2-3) 語(4-5) a(6): columns 1..5 take 日 (covers 1), 本, and stop before 語
    let mut e = boxed("日本語a\nabcdefg", (0, 1), (1, 5));
    e.run("delete");
    assert_eq!(text_of(&e), "語a\nafg");
    // a box inside one tab (columns 1-3) takes nothing of it, as both edges are at the same char
    let mut e = boxed("a\tbc\nabcdefgh", (0, 2), (1, 3));
    e.run("delete");
    assert_eq!(text_of(&e), "a\tbc\nabdefgh");
    // a box that starts inside the tab and ends in b takes the tab
    let mut e = boxed("a\tbc\nabcdefgh", (0, 2), (1, 5));
    e.run("delete");
    assert_eq!(text_of(&e), "ac\nabfgh");
}

#[test]
fn delete_with_crlf_and_the_last_line_without_a_line_ending() {
    let mut e = boxed("abc\r\ndef\r\nghi", (0, 1), (2, 2));
    e.run("delete");
    assert_eq!(text_of(&e), "ac\r\ndf\r\ngi");
}

#[test]
fn delete_a_box_of_empty_space_only_ends_the_box() {
    let mut e = boxed("ab\ncd", (0, 4), (1, 6));
    e.run("delete");
    assert_eq!(text_of(&e), "ab\ncd");
    assert!(!e.changes().text_changed());
    assert!(!e.changes().beep);
    assert!(!e.cursor().has_selection());
    assert!(!e.cursor().is_box());
}

#[test]
fn delete_a_box_of_no_width_ends_the_box() {
    let mut e = boxed("abc\ndef", (0, 1), (1, 1));
    e.run("delete");
    assert_eq!(text_of(&e), "abc\ndef");
    assert!(!e.cursor().is_box());
    assert_eq!(e.cursor().caret(), Pos::new(0, 1));
}

#[test]
fn delete_a_box_down_to_the_empty_row_at_the_end() {
    let mut e = boxed("abc\ndef\n", (0, 1), (2, 2));
    e.run("delete");
    assert_eq!(text_of(&e), "ac\ndf\n");
}

#[test]
fn word_delete_to_the_ends_still_beep_with_a_box() {
    let mut e = boxed("abc\ndef", (0, 1), (1, 2));
    for id in ["word_delete_to_start", "word_delete_to_end"] {
        assert!(e.try_run(id, &[]).is_ok(), "{id}");
        assert!(e.changes().beep);
        e.assert_box((0, 1), (1, 2));
    }
}

#[test]
fn view_mode_refuses_the_edits_of_a_box() {
    let mut e = boxed("abc\ndef", (0, 1), (1, 2)).with_clipboard("x", ClipKind::Normal);
    e.set_view_mode(true);
    for id in ["delete", "cut", "indent_tab"] {
        assert!(e.try_run(id, &[]).is_ok(), "{id}");
        assert_eq!(text_of(&e), "abc\ndef", "{id}");
    }
    assert!(e.try_run("input_char", &[Arg::Char('x')]).is_ok());
    assert!(e.try_run("pastebox", &[]).is_ok());
}

// ---- typing and inserting ----

fn typed(e: &mut Editor, c: char) {
    e.run_with("input_char", &[Arg::Char(c)]);
}

#[test]
fn typing_puts_the_char_before_the_box_in_every_row() {
    let mut e = boxed("abcd\nefgh\nijkl", (0, 1), (2, 3));
    typed(&mut e, 'X');
    assert_eq!(text_of(&e), "aXbcd\neXfgh\niXjkl");
    // the box moved right by the width of the char; the caret is at its top left corner
    assert_eq!(e.box_state(), Some(((2, 4), (0, 2))));
    assert_eq!(e.cursor().caret(), Pos::new(0, 2));
    // a second char goes before the box again
    typed(&mut e, 'Y');
    assert_eq!(text_of(&e), "aXYbcd\neXYfgh\niXYjkl");
    assert_eq!(e.box_state(), Some(((2, 5), (0, 3))));
}

#[test]
fn typing_is_one_undo_step_and_undo_brings_the_box_back() {
    let mut e = boxed("abcd\nefgh\nijkl", (0, 1), (2, 3));
    typed(&mut e, 'X');
    assert_eq!(e.changes().edits.len(), 3);
    e.run("undo");
    assert_eq!(text_of(&e), "abcd\nefgh\nijkl");
    e.assert_box((0, 1), (2, 3));
}

#[test]
fn typing_pads_a_row_that_is_shorter_than_the_box() {
    // row 1 "a" ends before column 3; row 2 is empty; row 3 does not exist (the empty row)
    let mut e = boxed("abcde\na\n\n", (0, 3), (3, 5));
    typed(&mut e, 'X');
    assert_eq!(text_of(&e), "abcXde\na  X\n   X\n   X");
    e.assert_box((3, 6), (0, 4));
}

#[test]
fn typing_at_the_end_of_the_text_pads_nothing() {
    // the box starts exactly at the end of the text of row 1
    let mut e = boxed("abc\nab", (0, 2), (1, 5));
    typed(&mut e, 'X');
    assert_eq!(text_of(&e), "abXc\nabX");
}

#[test]
fn typing_a_char_to_the_left_of_a_full_width_char_goes_before_that_char() {
    // the left edge is inside the full-width char 日 (0-1)
    let mut e = boxed("日本\nabcd", (0, 1), (1, 3));
    typed(&mut e, 'X');
    assert_eq!(text_of(&e), "X日本\naXbcd");
    // the box moved right by one column
    e.assert_box((1, 4), (0, 2));
}

#[test]
fn typing_a_full_width_char_moves_the_box_by_two_columns() {
    let mut e = boxed("abcd\nefgh", (0, 1), (1, 3));
    typed(&mut e, '日');
    assert_eq!(text_of(&e), "a日bcd\ne日fgh");
    e.assert_box((1, 5), (0, 3));
}

#[test]
fn typing_a_tab_moves_the_box_by_the_least_width_of_the_rows() {
    // the tab at column 1 is 3 wide; at column 3 it is 1 wide: the box moves by 1
    let mut e = boxed("abcd\nabcd", (0, 1), (1, 3));
    typed(&mut e, '\t');
    assert_eq!(text_of(&e), "a\tbcd\na\tbcd");
    e.assert_box((1, 6), (0, 4));
    let mut e = boxed("a\tbcd\nabcdef", (0, 1), (1, 2));
    // row 0: the tab covers column 1, so the new tab goes before it at column 1 (3 wide);
    // row 1: a tab at column 1 (3 wide)
    typed(&mut e, '\t');
    assert_eq!(text_of(&e), "a\t\tbcd\na\tbcdef");
    e.assert_box((1, 5), (0, 4));
}

#[test]
fn typing_in_overwrite_mode_does_not_delete_the_box() {
    let mut e = boxed("abcd\nefgh", (0, 1), (1, 3));
    e.set_overwrite(true);
    typed(&mut e, 'X');
    assert_eq!(text_of(&e), "aXbcd\neXfgh");
}

#[test]
fn typing_with_crlf() {
    let mut e = boxed("ab\r\ncd\r\n", (0, 1), (2, 2));
    typed(&mut e, 'X');
    // the last row is the empty row at the end: the char is padded to the column of the box
    assert_eq!(text_of(&e), "aXb\r\ncXd\r\n X");
}

#[test]
fn enter_deletes_the_box_and_breaks_the_line() {
    let mut e = boxed("abcd\nefgh", (0, 1), (1, 3));
    typed(&mut e, '\n');
    assert_eq!(text_of(&e), "a\nd\neh");
    assert!(!e.cursor().is_box());
    // the caret is after the new line ending, at the start of the line; the old Enter keeps
    // the indentation, which is empty here
    assert_eq!(e.cursor().caret(), Pos::new(1, 0));
}

#[test]
fn inserting_text_puts_its_first_line_before_the_box_in_every_row() {
    let mut e = boxed("abcd\nefgh\nijkl", (0, 1), (2, 3)).with_clipboard("xy\nz", ClipKind::Normal);
    e.run("paste");
    assert_eq!(text_of(&e), "axybcd\nexyfgh\nixyjkl");
    e.assert_box((2, 5), (0, 3));
    // an empty first line inserts nothing
    let mut e = boxed("abcd\nefgh", (0, 1), (1, 3)).with_clipboard("\nz", ClipKind::Normal);
    e.run("paste");
    assert_eq!(text_of(&e), "abcd\nefgh");
}

#[test]
fn a_row_copy_is_pasted_into_the_box_like_other_text() {
    let mut e = boxed("abcd\nefgh", (0, 1), (1, 3)).with_clipboard("xy\n", ClipKind::Line);
    e.run("paste");
    assert_eq!(text_of(&e), "axybcd\nexyfgh");
}

#[test]
fn typing_in_a_box_that_covers_nothing_pads_the_row_to_the_caret() {
    let mut e = Editor::new("ab\ncd|");
    e.run("begin_box")
        .run("cursor_right_box")
        .run("cursor_right_box");
    // a box one row high from column 2 to 4 of "cd" is empty space only; type into it
    typed(&mut e, 'X');
    assert_eq!(text_of(&e), "ab\ncdX");
    // a box of no width on text: the char goes at the caret and the box ends
    let mut e = Editor::new("ab|cd");
    e.run("begin_box");
    typed(&mut e, 'X');
    e.assert_marked("abX|cd");
    assert!(!e.cursor().is_box());
}

#[test]
fn typing_at_the_empty_space_caret_pads_with_spaces() {
    let mut e = Editor::new("ab|\ncd");
    e.run("begin_box");
    for _ in 0..3 {
        e.run("cursor_right_box");
    }
    // the box is the empty space of row 0, columns 2..5; the caret is at column 5
    e.assert_box((0, 2), (0, 5));
    typed(&mut e, 'X');
    // the box starts at column 2: the char goes there (the end of the text), no padding
    assert_eq!(text_of(&e), "abX\ncd");
}

// ---- indent ----

#[test]
fn indent_tab_inserts_a_tab_before_the_box_in_rows_that_have_text_there() {
    let mut e = boxed("abcd\n\nef\ngh", (0, 1), (3, 2));
    e.run("indent_tab");
    // row 1 is empty and row 2 ends before... it has text at column 1, so it is indented
    assert_eq!(text_of(&e), "a\tbcd\n\ne\tf\ng\th");
    assert!(!e.changes().beep);
}

#[test]
fn a_soft_tab_before_the_box_fills_up_to_the_next_tab_stop_in_each_row() {
    let mut e = boxed("abcd\n\nef\ngh", (0, 1), (3, 2)).with_ins_space(true);
    e.run("indent_tab");
    assert_eq!(text_of(&e), "a   bcd\n\ne   f\ng   h");
    assert!(!e.changes().beep);
    e.run("undo");
    assert_eq!(text_of(&e), "abcd\n\nef\ngh");
}

#[test]
fn a_soft_tab_in_a_box_counts_full_width_chars_as_two_columns() {
    // Column 2 is between the two chars of the Japanese row, so that row gets a different
    // number of spaces from the same column.
    let mut e = boxed("abcdef\nあいう\nxyz", (0, 2), (2, 3)).with_ins_space(true);
    e.run("indent_tab");
    assert_eq!(text_of(&e), "ab  cdef\nあ  いう\nxy  z");
    // Column 1 is inside the first full-width char: the spaces go before it, from column 0.
    let mut e = boxed("abcdef\nあいう\nxyz", (0, 1), (2, 2)).with_ins_space(true);
    e.run("indent_tab");
    assert_eq!(text_of(&e), "a   bcdef\n    あいう\nx   yz");
}

#[test]
fn a_soft_tab_in_a_box_uses_the_tab_width_of_the_layout() {
    let mut e = boxed("abcdef\nxyz", (0, 2), (1, 3))
        .with_tab_width(8)
        .with_ins_space(true);
    e.run("indent_tab");
    assert_eq!(text_of(&e), "ab      cdef\nxy      z");
}

#[test]
fn a_soft_tab_moves_the_box_right_by_the_least_width() {
    let mut e = boxed("abcdef\nxyz", (0, 1), (1, 3)).with_ins_space(true);
    e.run("indent_tab");
    // both rows got 3 spaces; the box moved by 3
    assert_eq!(text_of(&e), "a   bcdef\nx   yz");
    e.run("indent_tab");
    // the box now starts on the tab stop at column 4: a whole tab width
    assert_eq!(text_of(&e), "a       bcdef\nx       yz");
}

#[test]
fn indent_skips_rows_where_the_box_is_to_the_right_of_the_text() {
    let mut e = boxed("abcd\na\nabc", (0, 2), (2, 3));
    e.run("indent_space");
    assert_eq!(text_of(&e), "ab cd\na\nab c");
}

#[test]
fn indent_a_box_of_no_width_only_grows_it_to_the_right_edge() {
    let mut e = boxed("abc\ndef", (0, 1), (1, 1));
    e.run("indent_tab");
    assert_eq!(text_of(&e), "abc\ndef");
    assert_eq!(
        e.box_state(),
        Some(((0, 1), (1, crate::context::MAX_BOX_COL)))
    );
    // the next press indents
    e.run("indent_tab");
    assert_eq!(text_of(&e), "a\tbc\nd\tef");
}

#[test]
fn indent_tab_on_a_begin_box_with_nothing_selected_grows_the_box() {
    let mut e = Editor::new("ab|c");
    e.run("begin_box");
    e.run("indent_tab");
    assert_eq!(text_of(&e), "abc");
    e.assert_box((0, 2), (0, crate::context::MAX_BOX_COL));
}

#[test]
fn indent_space_of_a_one_column_box_leaves_full_width_chars_that_fit_alone() {
    // Sakura's `alignFullWidthChar`: row 0 has 日 at columns 0-1 (the box edge is inside it),
    // row 1 has b exactly at column 1. Only row 0 gets a space.
    let mut e = boxed("日本\nabcd", (0, 1), (1, 2));
    e.run("indent_space");
    assert_eq!(text_of(&e), " 日本\nabcd");
    // the box does not move when a row was left alone
    e.assert_box((1, 2), (0, 1));
}

#[test]
fn indent_space_does_the_indent_again_when_every_row_was_left_alone() {
    let mut e = boxed("abc\ndef", (0, 1), (1, 2));
    e.run("indent_space");
    assert_eq!(text_of(&e), "a bc\nd ef");
    e.assert_box((1, 3), (0, 2));
}

#[test]
fn indent_tab_does_not_indent_when_the_box_lies_inside_an_indent_char() {
    // columns 1..2 are inside the tab (1-3) in row 0
    let mut e = boxed("a\tb\nabcd", (0, 1), (1, 2));
    e.run("indent_tab");
    assert_eq!(text_of(&e), "a\tb\na\tbcd");
}

#[test]
fn indent_is_one_undo_step() {
    let mut e = boxed("abc\ndef", (0, 1), (1, 2));
    e.run("indent_tab");
    e.run("undo");
    assert_eq!(text_of(&e), "abc\ndef");
    e.assert_box((0, 1), (1, 2));
}

// ---- conversions ----

#[test]
fn a_conversion_changes_the_text_of_each_row_of_the_box() {
    let mut e = boxed("abcd\nefgh\nij", (0, 1), (2, 3));
    e.run("to_upper");
    assert_eq!(text_of(&e), "aBCd\neFGh\niJ");
    // the box is not selected afterwards; the caret is at its top left corner
    assert!(!e.cursor().has_selection());
    assert!(!e.cursor().is_box());
    assert_eq!(e.cursor().caret(), Pos::new(0, 1));
    assert!(!e.changes().beep);
    e.run("undo");
    assert_eq!(text_of(&e), "abcd\nefgh\nij");
    e.assert_box((0, 1), (2, 3));
}

#[test]
fn a_conversion_of_a_width_changing_kind_works_row_by_row() {
    let mut e = boxed("abc\ndef", (0, 1), (1, 3));
    e.run("to_zenkaku_eisu");
    assert_eq!(text_of(&e), "a\u{ff42}\u{ff43}\nd\u{ff45}\u{ff46}");
}

#[test]
fn tab_to_space_gets_the_column_where_each_row_starts() {
    // the tab at column 1 of "a\tb" is 3 columns wide when the row starts at column 1
    let mut e = boxed("a\tb\nabcd", (0, 1), (1, 5));
    e.run("tab_to_space");
    assert_eq!(text_of(&e), "a   b\nabcd");
}

#[test]
fn ltrim_and_rtrim_trim_each_row_of_the_box() {
    let mut e = boxed("a  bc\nd e  f", (0, 1), (1, 4));
    e.run("ltrim");
    assert_eq!(text_of(&e), "abc\nde  f");
    let mut e = boxed("ab  c\nd e  f", (0, 1), (1, 4));
    e.run("rtrim");
    assert_eq!(text_of(&e), "abc\nd e f");
}

#[test]
fn a_conversion_with_a_box_that_covers_nothing_does_nothing() {
    let mut e = Editor::new("ab|c");
    e.run("begin_box").run("to_upper");
    assert_eq!(text_of(&e), "abc");
    assert!(!e.changes().beep);
}

#[test]
fn merge_returns_at_once_with_a_box_selected() {
    let mut e = boxed("a\na\nb", (0, 0), (1, 1));
    e.run("merge");
    assert_eq!(text_of(&e), "a\na\nb");
    assert!(!e.changes().beep);
    e.assert_box((0, 0), (1, 1));
}

// ---- paste as a box ----

#[test]
fn pastebox_puts_each_line_into_the_next_row_at_the_caret_column() {
    let mut e = Editor::new("a|bc\ndef\nghi").with_clipboard("12\n34", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "a12bc\nd34ef\nghi");
    // the caret is where it was
    e.assert_marked("a|12bc\nd34ef\nghi");
}

#[test]
fn pastebox_pads_short_rows_with_spaces_and_adds_rows() {
    let mut e = Editor::new("ab|\ncdef\ng").with_clipboard("12\n34\n56\n78", ClipKind::Normal);
    e.run("pastebox");
    // "g" is padded to the column; the last line gets a line ending; two rows are added
    assert_eq!(text_of(&e), "ab12\ncd34ef\ng 56\n  78\n");
    e.assert_marked("ab|12\ncd34ef\ng 56\n  78\n");
}

#[test]
fn pastebox_adds_the_line_ending_of_the_document_to_a_last_line_that_has_none() {
    let mut e = Editor::new("a\r\n|b").with_clipboard("X", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "a\r\nXb\r\n");
}

#[test]
fn pastebox_a_text_that_ends_with_a_line_ending_has_no_extra_row() {
    let mut e = Editor::new("|ab\ncd\nef").with_clipboard("1\n2\n", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "1ab\n2cd\nef");
    // an empty line in the text inserts nothing in its row
    let mut e = Editor::new("|ab\ncd\nef").with_clipboard("1\n\n3", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "1ab\ncd\n3ef\n");
}

#[test]
fn pastebox_into_the_empty_row_at_the_end_creates_lines() {
    let mut e = Editor::new("ab\n|").with_clipboard("x\ny", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "ab\nx\ny\n");
    // from a column past the start
    let mut e = Editor::new("abc|")
        .with_new_line(Eol::Lf)
        .with_clipboard("x\ny", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "abcx\n   y\n");
}

#[test]
fn pastebox_uses_display_columns_for_full_width_chars_and_tabs() {
    // the caret is after 日 (column 2); row 1 has two ASCII chars before that column
    let mut e = Editor::new("日|本\nabcd").with_clipboard("X\nY", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "日X本\nabYcd\n");
    // a tab covers the column: the text goes before the tab
    let mut e = Editor::new("ab|\n\tc\nabcd").with_clipboard("X\nY\nZ", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "abX\nY\tc\nabZcd\n");
}

#[test]
fn pastebox_with_crlf_text() {
    let mut e = Editor::new("a|b\r\ncd").with_clipboard("1\r\n2", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "a1b\r\nc2d\r\n");
}

#[test]
fn pastebox_deletes_the_selection_first() {
    let mut e = Editor::new("a[bc|d\nefgh").with_clipboard("X\nY", ClipKind::Normal);
    e.run("pastebox");
    // "bc" is gone, the caret is where the selection started
    assert_eq!(text_of(&e), "aXd\neYfgh\n");
}

#[test]
fn pastebox_deletes_a_box_selection_first() {
    let mut e = boxed("abcd\nefgh\nijkl", (0, 1), (1, 3)).with_clipboard("X\nY", ClipKind::Normal);
    e.run("pastebox");
    // the box (b, c / f, g) is deleted; the paste is at its top left corner
    assert_eq!(text_of(&e), "aXd\neYh\nijkl");
}

#[test]
fn pastebox_is_one_undo_step() {
    let mut e = Editor::new("ab|\ncd").with_clipboard("1\n2\n3", ClipKind::Normal);
    e.run("pastebox");
    e.run("undo");
    assert_eq!(text_of(&e), "ab\ncd");
    e.assert_marked("ab|\ncd");
}

#[test]
fn pastebox_beeps_without_text_on_the_clipboard() {
    let mut e = Editor::new("a|b");
    e.run("pastebox");
    assert!(e.changes().beep);
    assert_eq!(text_of(&e), "ab");
}

#[test]
fn pastebox_with_an_empty_text_changes_nothing() {
    let mut e = Editor::new("a|b").with_clipboard("", ClipKind::Normal);
    e.run("pastebox");
    assert_eq!(text_of(&e), "ab");
}

#[test]
fn insboxtext_pastes_its_argument_as_a_box() {
    let mut e = Editor::new("a|bc\ndef");
    e.run_with("insboxtext", &[Arg::Text("12\n34".into())]);
    assert_eq!(text_of(&e), "a12bc\nd34ef\n");
    assert!(matches!(
        e.try_run("insboxtext", &[]),
        Err(CommandError::BadArgs(_))
    ));
}

#[test]
fn paste_of_a_box_copy_pastes_a_box() {
    let mut e = Editor::new("a|bc\ndef\nghi").with_clipboard("12\r\n34\r\n", ClipKind::Box);
    e.run("paste");
    assert_eq!(text_of(&e), "a12bc\nd34ef\nghi");
    // not with the option that says no (0x20)
    let mut e = Editor::new("a|bc\ndef").with_clipboard("12\r\n34\r\n", ClipKind::Box);
    e.run_with("paste", &[Arg::Int(0x20)]);
    assert_eq!(text_of(&e), "a12\r\n34\r\nbc\ndef");
    // the option that says yes (0x10) does not make a normal copy a box
    let mut e = Editor::new("a|bc\ndef").with_clipboard("12\n34", ClipKind::Normal);
    e.run_with("paste", &[Arg::Int(0x10)]);
    assert_eq!(text_of(&e), "a12\n34bc\ndef");
}

#[test]
fn copy_then_paste_round_trips_a_box() {
    let mut e = boxed("abcd\nefgh\nijkl\n", (0, 1), (2, 3));
    e.run("copy");
    // paste it at the start of row 0 of a fresh view of the same text
    let clipboard = clip(&e);
    assert_eq!(clipboard.1, ClipKind::Box);
    let mut e = Editor::new("|xy\nz\n\n").with_clipboard(&clipboard.0, clipboard.1);
    e.run("paste");
    assert_eq!(text_of(&e), "bcxy\nfgz\njk\n");
}

#[test]
fn cut_then_paste_moves_a_box() {
    let mut e = boxed("abcd\nefgh", (0, 1), (1, 3));
    e.run("cut");
    assert_eq!(text_of(&e), "ad\neh");
    // the caret is at (0, 1); go to the end of the first row and paste there
    e.run("go_line_end");
    e.run("paste");
    assert_eq!(text_of(&e), "adbc\nehfg\n");
}

// ---- wrapped rows ----

/// A layout that wraps every line after `n` ASCII chars. Rows of a line are `[0, n)`, `[n, 2n)`,
/// and so on; positions are always buffer positions.
struct Wrap(usize);

impl Wrap {
    fn rows(&self, doc: &Document) -> Vec<Pos> {
        let mut rows = Vec::new();
        for line in 0..doc.line_count() {
            let len = doc.line_len(line).unwrap().unwrap_or(0);
            let mut col = 0;
            loop {
                rows.push(Pos::new(line, col));
                col += self.0;
                if col >= len {
                    break;
                }
            }
        }
        if doc.end_pos().unwrap().line >= doc.line_count() {
            rows.push(Pos::new(doc.line_count(), 0));
        }
        rows
    }

    fn index(&self, doc: &Document, pos: Pos) -> usize {
        let pos = doc.clamp_pos(pos).unwrap();
        let rows = self.rows(doc);
        rows.iter().rposition(|&r| r <= pos).unwrap_or(0)
    }
}

type R<T> = matsu_core::buffer::Result<T>;

impl Layout for Wrap {
    fn edited(&mut self, _: &Document, _: &crate::context::TextChange) {}
    fn reset(&mut self, _: &Document) {}
    fn tab_width(&self) -> usize {
        4
    }
    fn display_col(&self, doc: &Document, pos: Pos) -> R<usize> {
        let pos = doc.clamp_pos(pos)?;
        Ok(pos.col - self.rows(doc)[self.index(doc, pos)].col)
    }
    fn row_start(&self, doc: &Document, pos: Pos) -> R<Pos> {
        Ok(self.rows(doc)[self.index(doc, pos)])
    }
    fn row_end(&self, doc: &Document, pos: Pos) -> R<Pos> {
        let start = self.row_start(doc, pos)?;
        let len = doc.line_len(start.line)?.unwrap_or(0);
        Ok(Pos::new(start.line, (start.col + self.0).min(len)))
    }
    fn next_row_start(&self, doc: &Document, pos: Pos) -> R<Option<Pos>> {
        Ok(self.rows(doc).get(self.index(doc, pos) + 1).copied())
    }
    fn pos_at_col(&self, doc: &Document, row_pos: Pos, col: usize) -> R<Pos> {
        let start = self.row_start(doc, row_pos)?;
        let end = self.row_end(doc, row_pos)?;
        Ok(Pos::new(start.line, (start.col + col).min(end.col)))
    }
    fn move_rows(&self, doc: &Document, pos: Pos, rows: isize, goal: usize) -> R<Pos> {
        let all = self.rows(doc);
        let i = self
            .index(doc, pos)
            .saturating_add_signed(rows)
            .min(all.len() - 1);
        self.pos_at_col(doc, all[i], goal)
    }
}

/// Runs `id` on `text` with the wrapping layout and the box (corners as `(row, display column)`,
/// rows counted as the layout counts them). Returns the text and the clipboard.
fn run_wrapped(
    text: &str,
    anchor: (usize, usize),
    caret: (usize, usize),
    clipboard: &str,
    id: &str,
    args: &[Arg],
) -> (String, Option<(String, ClipKind)>, Cursor) {
    let mut model = DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap());
    model.set_new_line(Eol::Lf); // the document has no line ending; not the OS default
    let mut layout = Wrap(3);
    let rows = layout.rows(model.doc());
    let corner = |(row, col): (usize, usize)| {
        layout
            .pos_at_col(model.doc(), rows[row], col)
            .expect("a position")
    };
    let mut cursor = Cursor::default();
    cursor.move_to(corner(anchor), false);
    cursor.begin_box(anchor.1, false);
    cursor.move_box_to(corner(caret), caret.1);
    let mut mem = MemoryClipboard::with_text(clipboard);
    {
        let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None)
            .with_clipboard(&mut mem);
        Registry::global().run(id, &mut cx, args).unwrap();
    }
    let doc = model.doc();
    let out = doc
        .text_between(Pos::new(0, 0), doc.end_pos().unwrap())
        .unwrap();
    let clip = mem.data().map(|d| (d.text.clone(), d.kind));
    (out, clip, cursor)
}

#[test]
fn wrapped_rows_are_rows_of_the_box() {
    // rows: "abc" "def" (line 0), "xy" (line 1); the box is columns 1..2 of rows 0 and 1
    let (_, clip, _) = run_wrapped("abcdef\nxy", (0, 1), (1, 2), "", "copy", &[]);
    assert_eq!(clip, Some(("b\r\ne\r\n".to_owned(), ClipKind::Box)));
    // the box over all three rows; the third row is another line
    let (_, clip, _) = run_wrapped("abcdef\nxyz", (0, 0), (2, 3), "", "copy", &[]);
    assert_eq!(clip.unwrap().0, "abc\r\ndef\r\nxyz\r\n");
}

#[test]
fn deleting_in_wrapped_rows_edits_one_line() {
    let (text, _, cursor) = run_wrapped("abcdef\nxy", (0, 1), (1, 2), "", "delete", &[]);
    assert_eq!(text, "acdf\nxy");
    assert_eq!(cursor.caret(), Pos::new(0, 1));
}

#[test]
fn typing_in_wrapped_rows_inserts_in_each_row() {
    let (text, _, cursor) = run_wrapped(
        "abcdef\nxy",
        (0, 1),
        (1, 2),
        "",
        "input_char",
        &[Arg::Char('X')],
    );
    assert_eq!(text, "aXbcdXef\nxy");
    // the box moved right by one column; the caret is at its top left corner
    assert_eq!(cursor.box_caret_col(), Some(2));
    assert_eq!(cursor.box_anchor_col(), Some(3));
}

#[test]
fn copy_addcrlf_ends_the_wrapped_rows_with_the_line_ending_of_the_document() {
    // "abcdef\r\nxy": rows "abc" "def\r\n" "xy"
    let (_, clip, _) = {
        let mut model = DocumentModel::new(Document::from_utf8(b"abcdef\r\nxy".to_vec()).unwrap());
        let mut cursor = Cursor::default();
        cursor.select(Pos::new(0, 1), Pos::new(1, 2));
        let mut layout = Wrap(3);
        let mut mem = MemoryClipboard::default();
        {
            let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None)
                .with_clipboard(&mut mem);
            Registry::global()
                .run("copy_addcrlf", &mut cx, &[])
                .unwrap();
        }
        ((), mem.data().map(|d| (d.text.clone(), d.kind)), cursor)
    };
    // the first row is cut by the wrapping and gets the ending of the document (CRLF); the row
    // that has the line ending keeps it; the last line is taken whole and gets one too
    assert_eq!(
        clip,
        Some(("bc\r\ndef\r\nxy\r\n".to_owned(), ClipKind::Normal))
    );
}

#[test]
fn copy_without_the_option_keeps_the_text_as_it_is() {
    let mut e = Editor::new("[abcdef\nxy|");
    e.run("copy");
    assert_eq!(clip(&e).0, "abcdef\nxy");
    let mut e = Editor::new("[abcdef\nxy|");
    e.run("copy_addcrlf");
    // no wrapping in the plain layout; the last line gets a line ending
    assert_eq!(clip(&e).0, "abcdef\nxy\n");
}

#[test]
fn copy_addcrlf_copies_a_row_that_has_no_line_ending_with_crlf() {
    // `CopyCurLine` adds CRLF, not the line ending of the document
    let mut e = Editor::new("ab\ncd|");
    e.run("copy_addcrlf");
    assert_eq!(clip(&e), ("cd\r\n".to_owned(), ClipKind::Line));
}

#[test]
fn pastebox_into_wrapped_rows_adds_a_line_ending_at_the_wrap_point_as_sakura_does() {
    // Sakura's loop adds a line ending to any row whose last char is not one, wrapped rows
    // included (`bAddLastCR` in `Command_PASTEBOX`).
    let (text, _, cursor) = run_wrapped("abcdef", (0, 1), (0, 1), "12", "pastebox", &[]);
    // "a12bcdef" wraps into "a12" "bcd" "ef"; the row "a12" has no line ending, so it gets one
    assert_eq!(text, "a12\nbcdef");
    assert_eq!(cursor.caret(), Pos::new(0, 1));
}

#[test]
fn text_that_reaches_the_wrap_point_moves_the_box_to_the_right_edge() {
    // The end of the inserted text is on the next row, so its width cannot be told from the
    // columns; Sakura counts the columns up to the right edge (`GetMaxLineLayout()`).
    let (text, _, cursor) = run_wrapped("abcdef\nxy", (0, 1), (1, 2), "XY", "paste", &[]);
    assert_eq!(text, "aXYbcdXYef\nxy");
    assert_eq!(cursor.box_caret_col(), Some(crate::context::MAX_BOX_COL));
}
