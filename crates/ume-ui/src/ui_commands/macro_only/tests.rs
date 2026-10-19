//! Tests of the macro-only functions that need the window, in GPUI's headless window. They run the
//! way a key macro does (`run_function`, and `.mac` files) and the way a script does (the bridge
//! cannot run them, so the window does: `macros/ui_calls.rs`).

use std::cell::Cell;
use std::path::Path;
use std::process::{Command, Stdio};

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Pos};
use ume_core::encoding::Encoding;
use ume_core::model::DocumentModel;
use ume_plugin::session::Language;

use super::{WindowChrome, maximize, minimize, zoom_back};
use crate::document::SharedDocument;
use crate::settings::WrapMethod;
use crate::view::EditorView;

struct Fixture {
    document: Entity<SharedDocument>,
    view: Entity<EditorView>,
}

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

fn setup<'a>(cx: &'a mut TestAppContext, text: &str) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let shared = document.clone();
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.run_until_parked();
    (Fixture { document, view }, vcx)
}

impl Fixture {
    /// Runs a Sakura function the way a menu item or a key macro does. The result says whether it
    /// succeeded.
    fn run(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg]) -> bool {
        let code = code_of(function);
        let ok = self
            .view
            .update_in(cx, |v, window, cx| v.run_function(code, args, window, cx));
        cx.run_until_parked();
        ok
    }

    fn text(&self, cx: &mut VisualTestContext) -> String {
        self.document.read_with(cx, |shared, _| {
            let doc = shared.doc();
            doc.text_between(Pos::new(0, 0), doc.end_pos().unwrap())
                .unwrap()
        })
    }

    fn output(&self, cx: &mut VisualTestContext) -> String {
        self.view.read_with(cx, |v, _| v.macro_output_text())
    }

    fn notice(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view.read_with(cx, |v, _| v.notice.clone())
    }

    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view.read_with(cx, |v, _| v.cursor().caret())
    }

    fn selection(&self, cx: &mut VisualTestContext) -> Option<(Pos, Pos)> {
        self.view.read_with(cx, |v, _| v.cursor().selection())
    }

    fn top(&self, cx: &mut VisualTestContext) -> usize {
        self.view.read_with(cx, |v, _| v.top_row())
    }

    fn select(&self, cx: &mut VisualTestContext, anchor: Pos, caret: Pos) {
        self.view.update(cx, |v, cx| {
            v.run_with(cx, |c| c.select_range(anchor, caret)).unwrap();
        });
    }

    fn put_caret(&self, cx: &mut VisualTestContext, at: Pos) {
        self.view.update(cx, |v, cx| {
            v.run_with(cx, |c| c.move_caret(at, false)).unwrap();
        });
    }

    fn can_undo(&self, cx: &mut VisualTestContext) -> bool {
        self.document
            .read_with(cx, |shared, _| shared.model().history().can_undo())
    }
}

fn text(s: &str) -> Arg {
    Arg::Text(s.to_owned())
}

fn path_arg(path: &Path) -> Arg {
    text(&path.to_string_lossy())
}

fn lines_of(n: usize) -> String {
    (0..n)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---- TraceOut ----

#[gpui::test]
fn trace_out_adds_lines_to_the_output_panel(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    assert!(!f.view.read_with(cx, |v, _| v.macros.output.is_open()));
    assert!(f.run(cx, "F_TRACEOUT", &[text("hello")]));
    assert!(f.view.read_with(cx, |v, _| v.macros.output.is_open()));
    assert!(f.run(cx, "F_TRACEOUT", &[text("日本語"), Arg::Int(0)]));
    assert_eq!(f.output(cx), "hello\n日本語");
    assert_eq!(f.text(cx), "abc", "the document is not touched");
}

#[gpui::test]
fn option_2_leaves_the_line_open(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    f.run(cx, "F_TRACEOUT", &[text("a"), Arg::Int(2)]);
    f.run(cx, "F_TRACEOUT", &[text("b"), Arg::Int(2)]);
    f.run(cx, "F_TRACEOUT", &[text("c"), Arg::Int(0)]);
    assert_eq!(f.output(cx), "abc");
    f.run(cx, "F_TRACEOUT", &[text("d"), Arg::Int(0)]);
    assert_eq!(f.output(cx), "abc\nd");
}

#[gpui::test]
fn a_line_break_in_the_text_starts_a_new_line(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    f.run(cx, "F_TRACEOUT", &[text("one\r\ntwo\nthree"), Arg::Int(0)]);
    assert_eq!(f.output(cx), "one\ntwo\nthree");
}

#[gpui::test]
fn option_1_expands_the_parameters(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc");
    f.put_caret(cx, Pos::new(1, 0));
    // `$y` is the line of the caret, `$$` is a dollar sign
    f.run(cx, "F_TRACEOUT", &[text("line $y of $$"), Arg::Int(1)]);
    assert_eq!(f.output(cx), "line 2 of $");
    // without the option the text is written as it is
    f.run(cx, "F_TRACEOUT", &[text("$y"), Arg::Int(0)]);
    assert_eq!(f.output(cx), "line 2 of $\n$y");
    // the caret did not move
    assert_eq!(f.caret(cx), Pos::new(1, 0));
}

#[gpui::test]
fn trace_out_needs_a_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    assert!(!f.run(cx, "F_TRACEOUT", &[]));
    assert!(f.notice(cx).unwrap_or_default().contains("TraceOut"));
    assert!(!f.run(cx, "F_TRACEOUT", &[Arg::Int(1)]));
    assert!(!f.run(cx, "F_TRACEOUT", &[text("a"), text("b")]));
    assert_eq!(f.output(cx), "");
}

#[gpui::test]
fn a_key_macro_can_trace_out(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mac = dir.path().join("t.mac");
    std::fs::write(&mac, "TraceOut('from the macro', 0);\nTraceOut('x', 2);\n").unwrap();
    let (f, cx) = setup(cx, "");
    f.run(cx, "F_EXECEXTMACRO", &[path_arg(&mac)]);
    assert_eq!(f.output(cx), "from the macro\nx");
}

// ---- StatusMsg ----

#[gpui::test]
fn status_msg_sets_the_notice(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    assert!(f.run(cx, "F_STATUSMSG", &[text("done")]));
    assert_eq!(f.notice(cx).as_deref(), Some("done"));
    assert!(f.run(cx, "F_STATUSMSG", &[text("2"), Arg::Int(2)]));
    assert_eq!(f.notice(cx).as_deref(), Some("2"));
    // 3 shows nothing (any command clears the notice that was there)
    assert!(f.run(cx, "F_STATUSMSG", &[text("3"), Arg::Int(3)]));
    assert_eq!(f.notice(cx), None);
}

#[gpui::test]
fn status_msg_option_1_opens_a_box_when_the_status_bar_is_hidden(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    f.view.update(cx, |v, _| v.prefs.show_status_bar = false);
    f.run(cx, "F_STATUSMSG", &[text("hidden"), Arg::Int(1)]);
    assert!(cx.has_pending_prompt());
    assert_eq!(cx.pending_prompt().unwrap().0, "UME Editor");
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert_eq!(f.notice(cx), None);
    // shown: the status bar gets it
    f.view.update(cx, |v, _| v.prefs.show_status_bar = true);
    f.run(cx, "F_STATUSMSG", &[text("shown"), Arg::Int(1)]);
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.notice(cx).as_deref(), Some("shown"));
    // 2 has no status bar to write to
    f.view.update(cx, |v, _| v.prefs.show_status_bar = false);
    f.run(cx, "F_STATUSMSG", &[text("two"), Arg::Int(2)]);
    assert_eq!(f.notice(cx), None);
    assert!(!cx.has_pending_prompt());
}

#[gpui::test]
fn status_msg_needs_a_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    assert!(!f.run(cx, "F_STATUSMSG", &[]));
    assert!(!f.run(cx, "F_STATUSMSG", &[Arg::Int(3)]));
}

// ---- MsgBeep ----

#[gpui::test]
fn msg_beep_takes_an_optional_number(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    assert!(f.run(cx, "F_MSGBEEP", &[]));
    for kind in [-1, 0, 1, 2, 3, 4, 99] {
        assert!(f.run(cx, "F_MSGBEEP", &[Arg::Int(kind)]), "{kind}");
    }
    assert!(!f.run(cx, "F_MSGBEEP", &[text("1")]));
}

// ---- SetViewTop and SetViewLeft ----

#[gpui::test]
fn set_view_top_scrolls_without_moving_the_caret(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &lines_of(500));
    assert_eq!(f.top(cx), 0);
    assert!(f.run(cx, "F_SETVIEWTOP", &[Arg::Int(100)]));
    assert_eq!(f.top(cx), 99, "the first row is 1");
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    // 0 and 1 are the first row
    f.run(cx, "F_SETVIEWTOP", &[Arg::Int(0)]);
    assert_eq!(f.top(cx), 0);
    f.run(cx, "F_SETVIEWTOP", &[Arg::Int(50)]);
    f.run(cx, "F_SETVIEWTOP", &[Arg::Int(1)]);
    assert_eq!(f.top(cx), 0);
}

#[gpui::test]
fn the_calls_after_set_view_top_do_not_scroll_back_to_the_caret(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mac = dir.path().join("v.mac");
    // commands that move nothing, after the scroll
    std::fs::write(
        &mac,
        "SetViewTop(200);\nClipboardEmpty();\nSetMsgQuoteStr('> ');\nSetUndoBuffer();\n",
    )
    .unwrap();
    let (f, cx) = setup(cx, &lines_of(500));
    f.run(cx, "F_EXECEXTMACRO", &[path_arg(&mac)]);
    assert_eq!(f.top(cx), 199);
    // a move of the caret brings it into view, as in Sakura
    f.run(cx, "F_MOVECURSOR", &[Arg::Int(1), Arg::Int(3), Arg::Int(0)]);
    assert_eq!(f.top(cx), 0);
}

#[gpui::test]
fn outside_a_macro_every_command_brings_the_caret_into_view(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &lines_of(500));
    f.run(cx, "F_SETVIEWTOP", &[Arg::Int(200)]);
    f.run(cx, "F_GOFILETOP", &[]);
    assert_eq!(f.top(cx), 0);
}

#[gpui::test]
fn set_view_top_stays_inside_the_scroll_range(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &lines_of(500));
    f.run(cx, "F_SETVIEWTOP", &[Arg::Int(100_000)]);
    let top = f.top(cx);
    assert!(top > 0 && top < 500, "{top}");
}

#[gpui::test]
fn set_view_top_refuses_what_is_not_a_number_of_0_or_more(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &lines_of(500));
    f.run(cx, "F_SETVIEWTOP", &[Arg::Int(10)]);
    for args in [vec![], vec![Arg::Int(-1)], vec![text("10")]] {
        assert!(!f.run(cx, "F_SETVIEWTOP", &args), "{args:?}");
        assert_eq!(f.top(cx), 9);
    }
}

#[gpui::test]
fn set_view_left_scrolls_by_columns(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &lines_of(10));
    f.view.update(cx, |v, _| v.widest_cols = 500);
    let cell = f.view.read_with(cx, |v, _| v.metrics().cell_width);
    assert!(f.run(cx, "F_SETVIEWLEFT", &[Arg::Int(11)]));
    assert_eq!(f.view.read_with(cx, |v, _| v.scroll_x), 10.0 * cell);
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    f.run(cx, "F_SETVIEWLEFT", &[Arg::Int(0)]);
    assert_eq!(f.view.read_with(cx, |v, _| v.scroll_x), 0.0);
    // not past the widest row (the paint measures it again)
    f.view.update(cx, |v, _| v.widest_cols = 500);
    f.run(cx, "F_SETVIEWLEFT", &[Arg::Int(1_000_000)]);
    let x = f.view.read_with(cx, |v, _| v.scroll_x);
    assert!(x > 0.0 && x < 1_000_000.0 * cell, "{x}");
    assert!(!f.run(cx, "F_SETVIEWLEFT", &[Arg::Int(-3)]));
    assert!(!f.run(cx, "F_SETVIEWLEFT", &[]));
}

// ---- TextWrapMethod ----

#[gpui::test]
fn text_wrap_method_sets_the_method_of_this_view(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    let method =
        |f: &Fixture, cx: &mut VisualTestContext| f.view.read_with(cx, |v, _| v.prefs.wrap_method);
    f.run(cx, "F_TEXTWRAPMETHOD", &[Arg::Int(2)]);
    assert_eq!(method(&f, cx), WrapMethod::WindowWidth);
    f.run(cx, "F_TEXTWRAPMETHOD", &[Arg::Int(1)]);
    assert_eq!(method(&f, cx), WrapMethod::SettingWidth);
    f.run(cx, "F_TEXTWRAPMETHOD", &[Arg::Int(0)]);
    assert_eq!(method(&f, cx), WrapMethod::None);
    // another number does nothing
    f.run(cx, "F_TEXTWRAPMETHOD", &[Arg::Int(2)]);
    assert!(f.run(cx, "F_TEXTWRAPMETHOD", &[Arg::Int(7)]));
    assert!(f.run(cx, "F_TEXTWRAPMETHOD", &[Arg::Int(-1)]));
    assert_eq!(method(&f, cx), WrapMethod::WindowWidth);
    // no number is 0
    f.run(cx, "F_TEXTWRAPMETHOD", &[]);
    assert_eq!(method(&f, cx), WrapMethod::None);
    assert!(!f.run(cx, "F_TEXTWRAPMETHOD", &[text("1")]));
}

#[gpui::test]
fn text_wrap_method_agrees_with_the_temporary_wrap_commands(cx: &mut TestAppContext) {
    let (a, cx_a) = setup(cx, "abc");
    a.run(cx_a, "F_TEXTWRAPMETHOD", &[Arg::Int(1)]);
    let one = a
        .view
        .read_with(cx_a, |v, _| (v.prefs.wrap_method, v.prefs.wrap_columns));
    a.run(cx_a, "F_TEXTWRAPMETHOD", &[Arg::Int(0)]);
    a.run(cx_a, "F_TMPWRAPSETTING", &[]);
    let two = a
        .view
        .read_with(cx_a, |v, _| (v.prefs.wrap_method, v.prefs.wrap_columns));
    assert_eq!(one, two);
}

// ---- PutFile ----

#[gpui::test]
fn put_file_writes_the_document_in_an_encoding(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "あい\r\nう");
    let out = dir.path().join("out.txt");
    assert!(f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(0)]));
    // Shift_JIS, with the line ending as it is
    assert_eq!(
        std::fs::read(&out).unwrap(),
        [0x82, 0xA0, 0x82, 0xA2, 0x0D, 0x0A, 0x82, 0xA4]
    );
    f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(4), Arg::Int(0)]);
    assert_eq!(std::fs::read(&out).unwrap(), "あい\r\nう".as_bytes());
    // the document is not the file, and not changed
    assert_eq!(
        f.document
            .read_with(cx, |s, _| s.path().map(Path::to_owned)),
        None
    );
    assert_eq!(f.text(cx), "あい\r\nう");
    assert!(!f.document.read_with(cx, |s, _| s.model().is_modified()));
    assert_eq!(
        f.document.read_with(cx, |s, _| s.file().encoding),
        Encoding::Utf8
    );
}

#[gpui::test]
fn put_file_without_a_code_writes_shift_jis(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "日本語");
    let out = dir.path().join("out.txt");
    f.run(cx, "F_PUTFILE", &[path_arg(&out)]);
    assert_eq!(std::fs::read(&out).unwrap(), b"\x93\xFA\x96\x7B\x8C\xEA");
}

#[gpui::test]
fn put_file_99_keeps_the_encoding_of_the_document_and_its_bom(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "aあ");
    f.document
        .update(cx, |s, _| s.set_format(Encoding::Utf16Le, true));
    let out = dir.path().join("out.txt");
    f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(99)]);
    assert_eq!(
        std::fs::read(&out).unwrap(),
        [0xFF, 0xFE, b'a', 0x00, 0x42, 0x30]
    );
    // an encoding with no BOM gets none, and one that has a BOM gets that of the document
    f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(0)]);
    assert_eq!(std::fs::read(&out).unwrap(), b"a\x82\xA0");
    f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(4)]);
    assert_eq!(std::fs::read(&out).unwrap(), "\u{feff}aあ".as_bytes());
    // the document keeps its own format
    assert_eq!(
        f.document
            .read_with(cx, |s, _| (s.file().encoding, s.file().bom)),
        (Encoding::Utf16Le, true)
    );
}

#[gpui::test]
fn characters_that_the_encoding_cannot_write_become_question_marks(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "a😀b");
    let out = dir.path().join("out.txt");
    assert!(f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(0)]));
    assert_eq!(std::fs::read(&out).unwrap(), b"a?b");
    assert_eq!(f.output(cx), "", "no message: Sakura writes them as it can");
}

#[gpui::test]
fn put_file_option_1_writes_the_selection(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "abc\r\ndef\r\nghi");
    f.select(cx, Pos::new(0, 1), Pos::new(1, 2));
    let out = dir.path().join("out.txt");
    f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(4), Arg::Int(1)]);
    assert_eq!(std::fs::read(&out).unwrap(), b"bc\r\nde");
}

#[gpui::test]
fn put_file_option_1_without_a_selection_writes_nothing(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "abc");
    let out = dir.path().join("out.txt");
    f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(4), Arg::Int(1)]);
    assert!(!out.exists(), "no bytes, no file");
    // a file that is there stays
    std::fs::write(&out, "old").unwrap();
    f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(4), Arg::Int(1)]);
    assert_eq!(std::fs::read(&out).unwrap(), b"old");
    // with a BOM the BOM alone is written
    f.document
        .update(cx, |s, _| s.set_format(Encoding::Utf8, true));
    f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(4), Arg::Int(1)]);
    assert_eq!(std::fs::read(&out).unwrap(), b"\xEF\xBB\xBF");
}

#[gpui::test]
fn put_file_option_1_writes_the_rows_of_a_box(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "abcd\nefgh\nijkl");
    f.put_caret(cx, Pos::new(0, 1));
    f.run(cx, "F_MOVECURSOR", &[Arg::Int(3), Arg::Int(4), Arg::Int(2)]);
    let out = dir.path().join("out.txt");
    f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(4), Arg::Int(1)]);
    assert_eq!(std::fs::read(&out).unwrap(), b"bc\r\nfg\r\njk\r\n");
}

#[gpui::test]
fn put_file_with_an_empty_name_does_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    assert!(f.run(cx, "F_PUTFILE", &[text(""), Arg::Int(4)]));
    assert_eq!(f.output(cx), "");
    assert!(!f.run(cx, "F_PUTFILE", &[]));
    assert!(!f.run(cx, "F_PUTFILE", &[text("x"), text("4")]));
    // not an encoding
    assert!(!f.run(cx, "F_PUTFILE", &[text("x"), Arg::Int(12345)]));
}

#[gpui::test]
fn a_failed_write_is_shown_and_the_macro_goes_on(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "abc");
    // the folder is not there
    let out = dir.path().join("no").join("such").join("out.txt");
    assert!(f.run(cx, "F_PUTFILE", &[path_arg(&out), Arg::Int(4)]));
    assert!(f.output(cx).contains("out.txt"), "{}", f.output(cx));
    assert!(!out.exists());
}

// ---- InsFile ----

#[gpui::test]
fn ins_file_inserts_at_the_caret_in_the_given_encoding(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.txt");
    std::fs::write(&file, b"\x82\xA0\x82\xA2").unwrap();
    let (f, cx) = setup(cx, "xy");
    f.put_caret(cx, Pos::new(0, 1));
    assert!(f.run(cx, "F_INSFILE", &[path_arg(&file), Arg::Int(0)]));
    assert_eq!(f.text(cx), "xあいy");
    assert_eq!(f.caret(cx), Pos::new(0, "xあい".len()));
    // without a code it is Shift_JIS too
    f.run(cx, "F_INSFILE", &[path_arg(&file)]);
    assert_eq!(f.text(cx), "xあいあいy");
}

#[gpui::test]
fn ins_file_keeps_the_line_endings_of_the_file_and_drops_the_bom(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.txt");
    std::fs::write(&file, "\u{feff}a\nb\r\nc\r").unwrap();
    let (f, cx) = setup(cx, "");
    f.run(
        cx,
        "F_INSFILE",
        &[path_arg(&file), Arg::Int(4), Arg::Int(0)],
    );
    assert_eq!(f.text(cx), "a\nb\r\nc\r");
}

#[gpui::test]
fn ins_file_replaces_a_selection_and_selects_what_it_inserted(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.txt");
    std::fs::write(&file, "XY\nZ").unwrap();
    let (f, cx) = setup(cx, "abcd");
    f.select(cx, Pos::new(0, 1), Pos::new(0, 3));
    f.run(cx, "F_INSFILE", &[path_arg(&file), Arg::Int(4)]);
    assert_eq!(f.text(cx), "aXY\nZd");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 1), Pos::new(1, 1))));
}

#[gpui::test]
fn ins_file_takes_a_big_file_in_pieces_and_it_is_still_one_step(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("big.txt");
    let big: String = (0..30_000).map(|i| format!("line {i}\r\n")).collect();
    assert!(big.len() > 4 * super::insfile::CHUNK_BYTES);
    std::fs::write(&file, &big).unwrap();
    let (f, cx) = setup(cx, "xy");
    f.put_caret(cx, Pos::new(0, 1));
    f.run(cx, "F_INSFILE", &[path_arg(&file), Arg::Int(4)]);
    assert_eq!(f.text(cx), format!("x{big}y"));
    f.run(cx, "F_UNDO", &[]);
    assert_eq!(f.text(cx), "xy");
    assert!(!f.can_undo(cx));
}

#[gpui::test]
fn ins_file_into_a_box_inserts_line_by_line(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.txt");
    std::fs::write(&file, "A\nB\nC\n").unwrap();
    // The expected result: the lines inserted one after the other (Sakura's loop).
    let (expected, cx_e) = setup(cx, "abc\ndef\nghi");
    expected.put_caret(cx_e, Pos::new(0, 1));
    expected.run(
        cx_e,
        "F_MOVECURSOR",
        &[Arg::Int(2), Arg::Int(3), Arg::Int(2)],
    );
    expected.view.update(cx_e, |v, cx| {
        for line in ["A\n", "B\n", "C\n"] {
            v.run_with(cx, |c| c.insert_text(line)).unwrap();
        }
    });
    let want = expected.text(cx_e);
    let (f, cx) = setup(cx_e, "abc\ndef\nghi");
    f.put_caret(cx, Pos::new(0, 1));
    f.run(cx, "F_MOVECURSOR", &[Arg::Int(2), Arg::Int(3), Arg::Int(2)]);
    f.run(cx, "F_INSFILE", &[path_arg(&file), Arg::Int(4)]);
    assert_eq!(f.text(cx), want);
}

#[gpui::test]
fn ins_file_is_one_undo_step(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.txt");
    std::fs::write(&file, "1\n2\n3\n").unwrap();
    let (f, cx) = setup(cx, "ab");
    f.run(cx, "F_INSFILE", &[path_arg(&file), Arg::Int(4)]);
    assert_eq!(f.text(cx), "1\n2\n3\nab");
    assert!(f.can_undo(cx));
    f.run(cx, "F_UNDO", &[]);
    assert_eq!(f.text(cx), "ab");
    assert!(!f.can_undo(cx));
}

#[gpui::test]
fn ins_file_of_a_missing_file_is_shown_and_the_macro_goes_on(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "ab");
    let missing = dir.path().join("missing.txt");
    assert!(f.run(cx, "F_INSFILE", &[path_arg(&missing), Arg::Int(4)]));
    assert!(f.output(cx).contains("missing.txt"), "{}", f.output(cx));
    assert_eq!(f.text(cx), "ab");
    // an empty name is nothing at all
    let before = f.output(cx);
    assert!(f.run(cx, "F_INSFILE", &[text("")]));
    assert_eq!(f.output(cx), before);
    assert!(!f.run(cx, "F_INSFILE", &[]));
}

#[gpui::test]
fn ins_file_is_refused_in_view_mode(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.txt");
    std::fs::write(&file, "X").unwrap();
    let (f, cx) = setup(cx, "ab");
    f.document.update(cx, |s, cx| s.set_view_mode(true, cx));
    assert!(!f.run(cx, "F_INSFILE", &[path_arg(&file), Arg::Int(4)]));
    assert_eq!(f.text(cx), "ab");
}

#[gpui::test]
fn put_file_and_ins_file_round_trip_in_a_key_macro(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let tmp = dir.path().join("tmp.txt");
    let mac = dir.path().join("m.mac");
    let quoted = tmp.to_string_lossy().replace('\\', "\\\\");
    std::fs::write(
        &mac,
        format!("PutFile('{quoted}', 4, 0);\nGoFileEnd(0);\nInsFile('{quoted}', 4, 0);\n"),
    )
    .unwrap();
    let (f, cx) = setup(cx, "日本語");
    f.run(cx, "F_EXECEXTMACRO", &[path_arg(&mac)]);
    assert_eq!(f.text(cx), "日本語日本語");
}

// ---- FileOpen ----

#[gpui::test]
fn file_open_opens_a_file_with_an_encoding(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, b"\x93\xFA\x96\x7B\x8C\xEA").unwrap();
    let (f, cx) = setup(cx, "");
    assert!(f.run(cx, "F_FILEOPEN2", &[path_arg(&file), Arg::Int(0)]));
    assert_eq!(f.text(cx), "日本語");
    assert_eq!(
        f.document.read_with(cx, |s, _| s.file().encoding),
        Encoding::ShiftJis
    );
    assert!(!f.document.read_with(cx, |s, _| s.view_mode()));
    assert_eq!(cx.windows().len(), 1, "it went into the untitled window");
}

#[gpui::test]
fn file_open_detects_the_encoding_by_default_and_for_a_number_that_is_none(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "日本語".as_bytes()).unwrap();
    let (f, cx) = setup(cx, "");
    assert!(f.run(cx, "F_FILEOPEN2", &[path_arg(&file), Arg::Int(54321)]));
    assert_eq!(f.text(cx), "日本語");
    assert_eq!(
        f.document.read_with(cx, |s, _| s.file().encoding),
        Encoding::Utf8
    );
}

#[gpui::test]
fn file_open_with_the_third_argument_opens_in_view_mode(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "abc").unwrap();
    let (f, cx) = setup(cx, "");
    f.run(
        cx,
        "F_FILEOPEN2",
        &[path_arg(&file), Arg::Int(99), Arg::Int(1)],
    );
    assert_eq!(f.text(cx), "abc");
    assert!(f.document.read_with(cx, |s, _| s.view_mode()));
    // an edit is refused there
    assert!(!f.run(cx, "F_INSFILE", &[path_arg(&file), Arg::Int(4)]));
}

#[gpui::test]
fn the_calls_after_file_open_work_on_the_new_text(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "from the file").unwrap();
    let mac = dir.path().join("m.mac");
    let quoted = file.to_string_lossy().replace('\\', "\\\\");
    std::fs::write(
        &mac,
        format!("FileOpen('{quoted}', 4, 0, '');\nGoFileEnd(0);\nChar(33);\n"),
    )
    .unwrap();
    let (f, cx) = setup(cx, "");
    f.run(cx, "F_EXECEXTMACRO", &[path_arg(&mac)]);
    assert_eq!(f.text(cx), "from the file!");
    assert_eq!(cx.windows().len(), 1);
}

#[gpui::test]
fn file_open_of_a_file_that_cannot_be_read_is_shown_and_the_macro_goes_on(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    // a folder is not a file
    let (f, cx) = setup(cx, "");
    assert!(f.run(cx, "F_FILEOPEN2", &[path_arg(dir.path()), Arg::Int(4)]));
    assert!(
        f.output(cx).contains(&*dir.path().to_string_lossy()),
        "{}",
        f.output(cx)
    );
    assert_eq!(f.text(cx), "");
}

#[gpui::test]
fn file_open_needs_a_file_name_argument(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    assert!(!f.run(cx, "F_FILEOPEN2", &[Arg::Int(1)]));
    assert!(!f.run(cx, "F_FILEOPEN2", &[text("x"), text("y")]));
}

#[gpui::test]
fn a_key_bound_to_file_open_shows_the_open_dialog(cx: &mut TestAppContext) {
    use ume_commands::keymap::user::parse_entries;
    use ume_commands::keymap::{KeyMap, KeyMapOptions, Platform};
    // A `[KeyBind]` row with the macro name `FileOpen`, which Sakura reads as `F_FILEOPEN2`.
    let (rows, warnings) = parse_entries([("KeyBind[000]", "0071,FileOpen,0,0,0,0,0,0,0,F2")]);
    assert!(warnings.is_empty(), "{warnings:?}");
    let mut map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
    map.apply_user_rows(&rows);
    let (f, cx) = setup(cx, "");
    f.view.update(cx, |view, _| view.keymap = map);
    cx.simulate_keystrokes("f2");
    cx.run_until_parked();
    assert!(cx.did_prompt_for_paths(), "the open dialog");
    assert_eq!(f.output(cx), "", "not an error");
}

#[gpui::test]
fn file_open_with_an_empty_name_asks_for_a_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "chosen").unwrap();
    let (f, cx) = setup(cx, "");
    f.run(cx, "F_FILEOPEN2", &[text("")]);
    assert!(cx.did_prompt_for_paths());
    cx.simulate_path_prompt_response(move |_| Some(vec![file]));
    cx.run_until_parked();
    assert_eq!(f.text(cx), "chosen");
}

#[gpui::test]
fn file_open_starts_the_dialog_where_the_fourth_argument_says(cx: &mut TestAppContext) {
    use crate::file_dialog::tests::asked;
    use ume_platform::dialog::Kind;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("メモ.txt");
    std::fs::write(&file, "chosen").unwrap();
    let (f, cx) = setup(cx, "");
    asked();
    let open = |cx: &mut gpui::VisualTestContext, default: String| {
        f.run(
            cx,
            "F_FILEOPEN2",
            &[text(""), Arg::Int(99), Arg::Int(0), Arg::Text(default)],
        );
        let mut shown = asked();
        assert_eq!(shown.len(), 1, "{shown:?}");
        cx.simulate_path_prompt_response(|_| None);
        cx.run_until_parked();
        shown.pop().unwrap()
    };
    // a folder (with the separator): the dialog starts in it
    let folder = format!("{}{}", dir.path().display(), std::path::MAIN_SEPARATOR);
    let shown = open(cx, folder);
    assert_eq!(shown.kind, Kind::OpenFile { multiple: true });
    assert_eq!(shown.start.as_deref(), Some(dir.path()));
    assert_eq!(shown.name, None);
    // a file in a folder that exists: its folder, and its name
    let shown = open(cx, file.display().to_string());
    assert_eq!(shown.start.as_deref(), Some(dir.path()));
    assert_eq!(shown.name.as_deref(), Some("メモ.txt"));
    // a folder that does not exist: where the OS wants, and no name
    let gone = dir.path().join("gone").join("a.txt");
    let shown = open(cx, gone.display().to_string());
    assert_eq!(shown.start, None);
    assert_eq!(shown.name, None);
    // without the argument: the setting (the current folder here)
    f.run(cx, "F_FILEOPEN2", &[text("")]);
    let shown = asked();
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert_eq!(shown[0].start, std::env::current_dir().ok());
    assert_eq!(shown[0].name, None);
}

// ---- the window ----

struct FakeWindow {
    maximized: Cell<bool>,
    minimized: Cell<bool>,
    toggles: Cell<usize>,
}

impl FakeWindow {
    fn new(maximized: bool) -> Self {
        Self {
            maximized: Cell::new(maximized),
            minimized: Cell::new(false),
            toggles: Cell::new(0),
        }
    }
}

impl WindowChrome for FakeWindow {
    fn is_maximized(&self) -> bool {
        self.maximized.get()
    }

    fn toggle_zoom(&self) {
        self.maximized.set(!self.maximized.get());
        self.toggles.set(self.toggles.get() + 1);
    }

    fn minimize(&self) {
        self.minimized.set(true);
    }
}

#[test]
fn maximize_does_not_toggle_a_maximized_window_back() {
    let window = FakeWindow::new(false);
    maximize(&window);
    assert!(window.is_maximized());
    maximize(&window);
    assert!(window.is_maximized(), "SW_MAXIMIZE on a maximized window");
    assert_eq!(window.toggles.get(), 1);
}

#[test]
fn minimize_minimizes() {
    let window = FakeWindow::new(true);
    minimize(&window);
    assert!(window.minimized.get());
}

#[test]
fn zoom_back_restores_a_maximized_window_only() {
    let window = FakeWindow::new(true);
    zoom_back(&window);
    assert!(!window.is_maximized());
    zoom_back(&window);
    assert!(!window.is_maximized());
    assert_eq!(window.toggles.get(), 1);
}

#[test]
fn the_window_functions_have_commands() {
    use crate::ui_command::UiRegistry;
    for name in ["F_WINMAXIMIZE", "F_WINMINIMIZE", "F_WINRESTORE"] {
        let code = code_of(name);
        assert!(UiRegistry::global().get_by_code(code).is_some(), "{name}");
    }
}

// ---- the functions of the core commands, through the window ----

#[gpui::test]
fn the_undo_buffer_functions_group_a_key_macro(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mac = dir.path().join("u.mac");
    std::fs::write(
        &mac,
        "AddRefUndoBuffer();\nChar(97);\nChar(98);\nChar(99);\nSetUndoBuffer();\nChar(100);\n",
    )
    .unwrap();
    let (f, cx) = setup(cx, "");
    f.run(cx, "F_EXECEXTMACRO", &[path_arg(&mac)]);
    assert_eq!(f.text(cx), "abcd");
    f.run(cx, "F_UNDO", &[]);
    assert_eq!(f.text(cx), "abc");
    f.run(cx, "F_UNDO", &[]);
    assert_eq!(f.text(cx), "", "a, b and c are one step");
}

#[gpui::test]
fn a_block_that_the_macro_left_open_is_closed_when_it_ends(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mac = dir.path().join("u.mac");
    std::fs::write(&mac, "AddRefUndoBuffer();\nChar(97);\nChar(98);\n").unwrap();
    let (f, cx) = setup(cx, "");
    f.run(cx, "F_EXECEXTMACRO", &[path_arg(&mac)]);
    assert_eq!(
        f.document.read_with(cx, |s, _| s.model().history().depth()),
        0
    );
    // what the macro typed is a step, and the next typing is another
    f.run(cx, "F_WCHAR", &[Arg::Char('z')]);
    f.run(cx, "F_UNDO", &[]);
    assert_eq!(f.text(cx), "ab");
    f.run(cx, "F_UNDO", &[]);
    assert_eq!(f.text(cx), "");
}

#[gpui::test]
fn the_quote_string_is_shared_by_the_windows(cx: &mut TestAppContext) {
    let (a, cx_a) = setup(cx, "x\ny");
    a.run(cx_a, "F_SET_QUOTESTRING", &[text("| ")]);
    a.select(cx_a, Pos::new(0, 0), Pos::new(1, 1));
    a.run(cx_a, "F_COPYLINESASPASSAGE", &[]);
    assert_eq!(
        cx_a.read_from_clipboard().and_then(|c| c.text()),
        Some("| x\n| y\n".to_owned())
    );
    let (b, cx_b) = setup(cx_a, "p\nq");
    b.select(cx_b, Pos::new(0, 0), Pos::new(1, 1));
    b.run(cx_b, "F_COPYLINESASPASSAGE", &[]);
    assert_eq!(
        cx_b.read_from_clipboard().and_then(|c| c.text()),
        Some("| p\n| q\n".to_owned())
    );
}

#[gpui::test]
fn clipboard_empty_leaves_no_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.select(cx, Pos::new(0, 0), Pos::new(0, 2));
    f.run(cx, "F_COPY", &[]);
    assert!(cx.read_from_clipboard().is_some());
    assert!(f.run(cx, "F_CLIPBOARDEMPTY", &[]));
    f.put_caret(cx, Pos::new(0, 3));
    // a paste finds no text
    f.run(cx, "F_PASTE", &[]);
    assert_eq!(f.text(cx), "abc");
}

// ---- scripts ----

fn available(language: Language) -> bool {
    let interpreter = language.default_interpreter();
    let ok = Command::new(&interpreter)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!(
            "SKIP: {} ({}) is not available",
            language.display_name(),
            interpreter.to_string_lossy()
        );
        // On a CI runner that has every interpreter, a missing one is a failure, not a skip.
        if std::env::var_os("UME_REQUIRE_INTERPRETERS").is_some_and(|v| !v.is_empty()) {
            panic!(
                "{} is required here (UME_REQUIRE_INTERPRETERS)",
                language.display_name()
            );
        }
    }
    ok
}

/// A script that calls the functions of this group. The bridge does not know the UI commands, so
/// the window answers them (`macros/ui_calls.rs`); the core commands go through the bridge.
fn script_calls_the_functions(cx: &mut TestAppContext, language: Language, name: &str, code: &str) {
    if !available(language) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join(name);
    std::fs::write(&script, code).unwrap();
    let (f, cx) = setup(cx, "world");
    cx.executor().allow_parking();
    f.run(cx, "F_EXECEXTMACRO", &[path_arg(&script)]);
    for _ in 0..1500 {
        cx.run_until_parked();
        if !f.view.read_with(cx, |v, _| v.is_macro_running()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let output = f.output(cx);
    assert!(output.contains("from a script"), "{output}");
    assert!(output.contains("正常に終了しました"), "{output}");
    // every call was answered: the script got to its end
    assert!(output.contains("the end"), "{output}");
    // AddRefUndoBuffer and SetUndoBuffer made the two inserts one step
    assert_eq!(f.text(cx), "abworld");
    f.run(cx, "F_UNDO", &[]);
    assert_eq!(f.text(cx), "world");
}

#[gpui::test]
fn a_javascript_macro_can_call_the_functions_that_need_the_window(cx: &mut TestAppContext) {
    script_calls_the_functions(
        cx,
        Language::JavaScript,
        "all.cjs",
        "Editor.TraceOut('from a script', 0);\n\
         Editor.GoFileTop();\n\
         Editor.AddRefUndoBuffer();\n\
         Editor.InsText('a');\n\
         Editor.InsText('b');\n\
         Editor.SetUndoBuffer();\n\
         Editor.MoveCursor(1, 1, 0);\n\
         Editor.SetViewTop(1);\n\
         Editor.StatusMsg('status', 0);\n\
         console.log('the end');\n",
    );
}

#[gpui::test]
fn a_python_macro_can_call_the_functions_that_need_the_window(cx: &mut TestAppContext) {
    script_calls_the_functions(
        cx,
        Language::Python,
        "all.py",
        "Editor.TraceOut('from a script', 0)\n\
         Editor.GoFileTop()\n\
         Editor.AddRefUndoBuffer()\n\
         Editor.InsText('a')\n\
         Editor.InsText('b')\n\
         Editor.SetUndoBuffer()\n\
         Editor.MoveCursor(1, 1, 0)\n\
         Editor.SetViewTop(1)\n\
         Editor.StatusMsg('status', 0)\n\
         print('the end')\n",
    );
}

#[gpui::test]
fn a_script_is_told_when_a_function_of_the_window_fails(cx: &mut TestAppContext) {
    let language = Language::JavaScript;
    if !available(language) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("bad.cjs");
    // a negative row: Sakura's SetViewTop refuses it
    std::fs::write(
        &script,
        "try { Editor.SetViewTop(-1); console.log('no error'); } catch (e) { console.log('error', e.message); }\n",
    )
    .unwrap();
    let (f, cx) = setup(cx, "");
    cx.executor().allow_parking();
    f.run(cx, "F_EXECEXTMACRO", &[path_arg(&script)]);
    for _ in 0..1500 {
        cx.run_until_parked();
        if !f.view.read_with(cx, |v, _| v.is_macro_running()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let output = f.output(cx);
    assert!(output.contains("error"), "{output}");
    assert!(!output.contains("no error"), "{output}");
}
