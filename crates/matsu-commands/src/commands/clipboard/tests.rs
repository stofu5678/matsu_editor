//! Tests of the clipboard commands, on the in-memory clipboard. The expectations come from
//! `sakura_core/cmd/CViewCommander_Clipboard.cpp` and the functions it calls (see each command).

use matsu_core::buffer::{Document, Pos};
use matsu_core::cursor::Cursor;
use matsu_core::model::DocumentModel;

use crate::clipboard::{ClipData, ClipKind, MemoryClipboard};
use crate::command::{Arg, CommandError};
use crate::context::CommandContext;
use crate::context::TextChange;
use crate::input_options::InputOptions;
use crate::layout::{Layout, PlainLayout, X};
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
fn copy_crlf_follows_the_endings_the_document_has_not_the_setting() {
    // Made with `bEnableExtEol` off: the NEL is a character. Turning the setting on afterwards
    // does not make it an ending (Sakura copies with the ending recorded in each line).
    let mut e = Editor::new("|");
    e.model().doc().set_ext_eol(false);
    for c in ['a', '\u{85}', 'b'] {
        e.run_with("input_char", &[Arg::Char(c)]);
    }
    assert_eq!(e.model().doc().line_count(), 1);
    e.model().doc().set_ext_eol(true);
    e.run("select_all");
    e.run("copy_crlf");
    assert_eq!(clip(&e), normal("a\u{85}b"));
    // Made with it on: the NEL is an ending, and stays one when the setting is turned off.
    let mut e = Editor::new("a\u{85}b|");
    e.model().doc().set_ext_eol(false);
    e.run("select_all");
    e.run("copy_crlf");
    assert_eq!(clip(&e), normal("a\r\nb"));
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
fn a_line_copy_that_ends_with_an_extended_character_gets_a_line_ending_by_default() {
    // `bEnableExtEol` is off by default: U+2028 is not a line ending (`WCODE::IsLineDelimiter`),
    // so the row copy still needs one.
    let mut e = Editor::new("a|b").with_clipboard("XY\u{2028}", ClipKind::Line);
    let os = if cfg!(windows) { "\r\n" } else { "\n" };
    e.run("paste").assert_marked(&format!("XY\u{2028}{os}a|b"));
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
    // width: max(nLineNumWidth, digits of the row count) + 2
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
fn the_line_number_width_follows_the_type() {
    // `nLineNumWidth` 5: the numbers are padded to 5 digits plus the blank and the colon
    let input = InputOptions {
        line_num_width: 5,
        ..Default::default()
    };
    let mut e = Editor::new("[ab\ncd|").with_input_options(input);
    e.run("copy_lines_with_line_number");
    assert_eq!(clip(&e), normal("     1:ab\n     2:cd\n"));
}

/// The first copied row of `copy_lines_with_line_number` over the whole of `text`.
fn first_numbered_row(text: &str) -> String {
    let mut e = Editor::new(&format!("[{text}|"));
    e.run("copy_lines_with_line_number");
    clip(&e).0.lines().next().unwrap().to_owned()
}

#[test]
fn the_width_changes_at_the_digit_boundaries_of_the_row_count() {
    // `LINENUMWIDTH_MIN` is 2 digits: 99 rows still have 2, 100 have 3
    assert_eq!(first_numbered_row(&"x\n".repeat(9)), "  1:x");
    assert_eq!(first_numbered_row(&"x\n".repeat(10)), "  1:x");
    assert_eq!(first_numbered_row(&"x\n".repeat(99)), "  1:x");
    assert_eq!(first_numbered_row(&"x\n".repeat(100)), "   1:x");
    // the empty row after a final line ending is no row ...
    assert_eq!(
        first_numbered_row(&format!("{}x", "x\n".repeat(98))),
        "  1:x"
    );
    // ... but a blank last line is one: 99 lines and a blank one are 100 rows
    assert_eq!(
        first_numbered_row(&format!("{}\n", "x\n".repeat(99))),
        "   1:x"
    );
    assert_eq!(
        first_numbered_row(&format!("{}\n\n", "x\n".repeat(98))),
        "   1:x"
    );
}

/// A layout that wraps every line after three chars (rows "abc", "def", ...).
struct Wrap3(PlainLayout);

type R<T> = matsu_core::buffer::Result<T>;

impl Layout for Wrap3 {
    fn edited(&mut self, doc: &Document, change: &TextChange) {
        self.0.edited(doc, change);
    }
    fn reset(&mut self, doc: &Document) {
        self.0.reset(doc);
    }
    fn display_x(&self, _: &Document, pos: Pos) -> R<X> {
        Ok(X::from_raw((pos.col % 3) as u32))
    }
    fn row_start(&self, _: &Document, pos: Pos) -> R<Pos> {
        Ok(Pos::new(pos.line, pos.col / 3 * 3))
    }
    fn row_end(&self, doc: &Document, pos: Pos) -> R<Pos> {
        let len = doc.line_len(pos.line)?.unwrap_or(0);
        Ok(Pos::new(pos.line, (pos.col / 3 * 3 + 3).min(len)))
    }
    fn next_row_start(&self, doc: &Document, pos: Pos) -> R<Option<Pos>> {
        let len = doc.line_len(pos.line)?.unwrap_or(0);
        let next = pos.col / 3 * 3 + 3;
        if next < len {
            return Ok(Some(Pos::new(pos.line, next)));
        }
        self.0.next_row_start(doc, pos)
    }
    fn pos_at_x(&self, doc: &Document, row_pos: Pos, x: X) -> R<Pos> {
        let len = doc.line_len(row_pos.line)?.unwrap_or(0);
        Ok(Pos::new(
            row_pos.line,
            (row_pos.col + x.raw() as usize).min(len),
        ))
    }
    fn move_rows(&self, doc: &Document, pos: Pos, rows: isize, goal: X) -> R<Pos> {
        self.0.move_rows(doc, pos, rows, goal)
    }
    fn tab_width(&self) -> usize {
        4
    }
    fn tab_x(&self) -> X {
        self.0.tab_x()
    }
    fn column_x(&self) -> X {
        self.0.column_x()
    }
}

/// Runs `id` on `text` with the selection `anchor` to `caret` on the layout that wraps after 3
/// chars; the text that went to the clipboard.
fn run_wrapped(id: &str, text: &str, anchor: Pos, caret: Pos) -> Option<String> {
    let mut model = DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap());
    // the line ending of the document (`GetNewLineCode`); set, as the default follows the OS
    model.set_new_line(matsu_core::buffer::Eol::Lf);
    let mut cursor = Cursor::default();
    cursor.select(anchor, caret);
    let mut layout = Wrap3(PlainLayout::default());
    let mut clipboard = MemoryClipboard::default();
    let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None)
        .with_clipboard(&mut clipboard);
    Registry::global().run(id, &mut cx, &[]).unwrap();
    drop(cx);
    clipboard.data().map(|d| d.text.clone())
}

#[test]
fn copy_lines_with_line_number_numbers_the_rows_of_a_wrapped_line() {
    // `CopySelectedAllLines`: "nLineNum + 1" is the layout row; every row ends with a line ending
    let got = run_wrapped(
        "copy_lines_with_line_number",
        "abcdefg\nxy",
        Pos::new(0, 1),
        Pos::new(1, 1),
    );
    assert_eq!(got.as_deref(), Some("  1:abc\n  2:def\n  3:g\n  4:xy\n"));
}

#[test]
fn copy_lines_ends_a_wrapped_row_only_with_add_crlf_when_copy() {
    let (text, anchor, caret) = ("abcdef", Pos::new(0, 1), Pos::new(0, 2));
    let plain = run_wrapped("copy_lines", text, anchor, caret);
    assert_eq!(plain.as_deref(), Some("abc"));
    let options = EditOptions {
        add_crlf_when_copy: true,
        ..Default::default()
    };
    let with = with_options(options, || run_wrapped("copy_lines", text, anchor, caret));
    // the line ending of the document (no CR or CRLF in it: LF)
    assert_eq!(with.as_deref(), Some("abc\n"));
}

#[test]
fn copy_lines_take_the_rows_of_a_box() {
    // the box becomes whole rows, down to the row of its last corner whatever the column
    let mut e = Editor::new("|ab\ncd\nef\n");
    e.run("begin_box")
        .run("cursor_right_box")
        .run("cursor_down_box");
    e.run("copy_lines_with_line_number");
    assert_eq!(clip(&e), normal("  1:ab\n  2:cd\n"));
    assert!(!e.cursor().is_box());
    let mut e = Editor::new("|ab\ncd\nef\n");
    e.run("begin_box")
        .run("cursor_right_box")
        .run("cursor_down_box");
    e.run("copy_lines");
    assert_eq!(clip(&e), normal("ab\ncd\n"));
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

// ---- the options of the common settings ----

use crate::edit_options::{EditOptions, with_options};

#[test]
fn copy_and_cut_add_a_line_ending_with_add_crlf_when_copy() {
    // `HandleCommand` passes `bAddCRLFWhenCopy` to `Command_COPY`; with the plain layout only the
    // last row, which has no line ending, is affected.
    let options = EditOptions {
        add_crlf_when_copy: true,
        ..Default::default()
    };
    with_options(options, || {
        let mut e = Editor::new("[abc\ndef|");
        e.run("copy");
        assert_eq!(clip(&e), normal("abc\ndef\n"));
        let mut e = Editor::new("[abc\ndef|");
        e.run("copy_crlf");
        assert_eq!(clip(&e), normal("abc\r\ndef\r\n"));
        // a row copy adds CRLF to the last row (`CopyCurLine`)
        let mut e = Editor::new("ab\ncd|");
        e.run("copy");
        assert_eq!(clip(&e), line("cd\r\n"));
        // cut and cut-line pass the option on as well
        let mut e = Editor::new("ab\n[cd|");
        e.run("cut");
        assert_eq!(clip(&e), normal("cd\n"));
        let mut e = Editor::new("ab\ncd|");
        e.run("cut_line");
        assert_eq!(clip(&e), line("cd\r\n"));
    });
    // off by default
    let mut e = Editor::new("[abc\ndef|");
    e.run("copy");
    assert_eq!(clip(&e), normal("abc\ndef"));
}

#[test]
fn copy_drops_the_selection_with_copy_and_disable_selection() {
    // `Command_COPY`: "コピーしたら選択解除"
    let options = EditOptions {
        copy_and_disable_selection: true,
        ..Default::default()
    };
    with_options(options, || {
        let mut e = Editor::new("a[bc|d");
        e.run("copy").assert_marked("abc|d");
        assert_eq!(clip(&e), normal("bc"));
        // a box too
        let mut e = Editor::new("|abc\ndef\n");
        e.run("begin_box")
            .run("cursor_right_box")
            .run("cursor_down_box");
        e.run("copy");
        assert!(!e.cursor().has_selection() && !e.cursor().is_box());
        assert_eq!(clip(&e), ("a\r\nd\r\n".to_owned(), ClipKind::Box));
        // without a selection nothing changes
        let mut e = Editor::new("a|bc");
        e.run("copy").assert_marked("a|bc");
    });
}

#[test]
fn a_last_line_that_wraps_counts_all_its_rows() {
    // 98 lines and a last line of 3 rows are 101 rows: 3 digits
    let text = format!("{}abcdefgh", "x\n".repeat(98));
    let got = run_wrapped(
        "copy_lines_with_line_number",
        &text,
        Pos::new(0, 0),
        Pos::new(98, 8),
    )
    .unwrap();
    assert!(got.starts_with("   1:x\n"), "{got:?}");
    assert!(got.ends_with(" 101:gh\n"), "{got:?}");
    // 97 lines and 3 rows are 100 ... and 96 lines are 99 rows: 2 digits
    let text = format!("{}abcdefgh", "x\n".repeat(96));
    let got = run_wrapped(
        "copy_lines_with_line_number",
        &text,
        Pos::new(0, 0),
        Pos::new(96, 8),
    )
    .unwrap();
    assert!(got.starts_with("  1:x\n"), "{got:?}");
}

/// The text of the document, without the marks of the cursor.
fn plain(e: &Editor) -> String {
    // The final line ending is not what these tests are about.
    e.marked()
        .replace(['[', '|'], "")
        .trim_end_matches(['\r', '\n'])
        .to_owned()
}

/// `bAutoColumnPaste` (`bAutoColmnPaste`) and the bits 0x10 / 0x20 of the argument of `Paste`
/// (`Command_PASTE`): the bit decides, and without one the setting does.
#[test]
fn the_text_of_a_box_copy_is_pasted_as_a_box_by_the_setting_and_the_paste_bits() {
    use crate::edit_options::{EditOptions, with_options};
    for setting in [true, false] {
        for (option, as_box) in [
            (0, setting),
            (0x10, true),
            (0x20, false),
            (0x10 | 0x20, true),
            (0x01, setting),
            (0x10 | 0x01, true),
        ] {
            let mut e = Editor::new("ab|\ncd").with_clipboard("X\nY", ClipKind::Box);
            with_options(
                EditOptions {
                    auto_column_paste: setting,
                    ..EditOptions::default()
                },
                || {
                    e.run_with("paste", &[Arg::Int(option)]);
                },
            );
            let text = plain(&e);
            let expected = if as_box { "abX\ncdY" } else { "abX\nY\ncd" };
            assert_eq!(
                text.replace("\r\n", "\n"),
                expected,
                "setting {setting}, option {option:#x}"
            );
        }
    }
}

#[test]
fn the_setting_does_not_touch_the_paste_of_other_text() {
    use crate::edit_options::{EditOptions, with_options};
    for setting in [true, false] {
        let mut e = Editor::new("ab|\ncd").with_clipboard("X\nY", ClipKind::Normal);
        with_options(
            EditOptions {
                auto_column_paste: setting,
                ..EditOptions::default()
            },
            || {
                e.run_with("paste", &[Arg::Int(0x10)]);
            },
        );
        assert_eq!(plain(&e).replace("\r\n", "\n"), "abX\nY\ncd", "{setting}");
    }
}

#[test]
fn pastebox_pastes_as_a_box_whatever_the_setting_is() {
    use crate::edit_options::{EditOptions, with_options};
    let mut e = Editor::new("ab|\ncd").with_clipboard("X\nY", ClipKind::Box);
    with_options(
        EditOptions {
            auto_column_paste: false,
            ..EditOptions::default()
        },
        || {
            e.run("pastebox");
        },
    );
    assert_eq!(plain(&e).replace("\r\n", "\n"), "abX\ncdY");
}
