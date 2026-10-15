//! Tests of the file flows in GPUI's headless test window. The native dialogs and prompts are
//! answered with `simulate_prompt_answer`, `simulate_new_path_selection` and
//! `simulate_path_prompt_response`; the files are real files in a private temporary folder.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::Pos;

use super::load::progress_text;
use super::windows::{LoadRequest, open_editor_window};
use crate::document::SharedDocument;
use crate::view::EditorView;

// The labels of the buttons (`ask.rs` and the flows).
const SAVE: &str = "保存する(Y)";
const DONT_SAVE: &str = "保存しない(N)";
const CANCEL: &str = "キャンセル";
const CONTINUE: &str = "続行(Y)";
const STOP: &str = "中断(N)";
const GO_TO_PLACE: &str = "該当位置に移動";
const UNIFY: &str = "統一する(Y)";
const KEEP_MIXED: &str = "そのまま保存(N)";
const RELOAD: &str = "再読み込みする";

/// A folder that is removed at the end of the test.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ume-ui-files-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Fixture {
    view: Entity<EditorView>,
}

fn setup(cx: &mut TestAppContext) -> (Fixture, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    let shared = cx.new(|_| SharedDocument::empty());
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (Fixture { view }, vcx)
}

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

impl Fixture {
    /// Runs a Sakura function the way a key or the menu does, and lets the flow run until it
    /// waits for the user.
    fn run(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
        let code = code_of(function);
        self.view.update_in(cx, |view, window, cx| {
            view.run_function(code, args, window, cx)
        });
        cx.run_until_parked();
    }

    fn lines(&self, cx: &mut VisualTestContext) -> Vec<String> {
        self.view.read_with(cx, |view, app| {
            let doc = view.document().read(app).doc();
            (0..doc.line_count())
                .map(|i| doc.line(i).unwrap().unwrap().text)
                .collect()
        })
    }

    fn modified(&self, cx: &mut VisualTestContext) -> bool {
        self.view
            .read_with(cx, |v, app| v.document().read(app).model().is_modified())
    }

    fn title(&self, cx: &mut VisualTestContext) -> String {
        self.view.read_with(cx, |v, app| v.title(app))
    }

    fn format(&self, cx: &mut VisualTestContext) -> String {
        self.view.read_with(cx, |v, app| v.status(app).file_format)
    }

    fn notice(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view.read_with(cx, |v, _| v.notice.clone())
    }

    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view.read_with(cx, |v, _| v.cursor().caret())
    }

    /// Opens `path` into this (untitled) window through `F_FILEOPEN` with its arguments.
    fn open(&self, cx: &mut VisualTestContext, path: &Path, encoding: Option<i64>) {
        let mut args = vec![Arg::Text(path.display().to_string())];
        if let Some(code) = encoding {
            args.push(Arg::Int(code));
        }
        self.run(cx, "F_FILEOPEN", &args);
    }
}

fn answer(cx: &mut VisualTestContext, button: &str) {
    assert!(cx.has_pending_prompt(), "no prompt to answer {button:?}");
    cx.simulate_prompt_answer(button);
    cx.run_until_parked();
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

const SJIS_NIHONGO: &[u8] = b"\x93\xFA\x96\x7B\x8C\xEA"; // 日本語
const SJIS: i64 = 0;
const UTF8: i64 = 4;

#[gpui::test]
fn a_file_opens_into_the_untitled_window_with_its_encoding(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &[SJIS_NIHONGO, b"\r\nabc"].concat());
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(SJIS));
    assert_eq!(f.lines(cx), ["日本語", "abc"]);
    assert_eq!(f.title(cx), "a.txt - UME Editor");
    assert_eq!(f.format(cx), "SJIS  CRLF");
    assert!(!f.modified(cx));
    assert_eq!(cx.windows().len(), 1, "no new window");
}

#[gpui::test]
fn the_status_shows_the_bom_and_the_line_ending(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("bom.txt", b"\xEF\xBB\xBFabc\ndef\n");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    assert_eq!(f.format(cx), "UTF-8 BOM付  LF");
    assert_eq!(f.lines(cx)[0], "abc", "the BOM is not part of the text");
}

#[gpui::test]
fn a_missing_file_opens_as_a_new_file_with_that_name(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.path("new.txt");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    assert_eq!(f.title(cx), "new.txt - UME Editor");
    cx.simulate_input("hello");
    f.run(cx, "F_FILESAVE", &[]);
    assert!(!cx.has_pending_prompt(), "it has a name, so no dialog");
    assert_eq!(read(&path), b"hello");
    assert!(!f.modified(cx));
}

#[gpui::test]
fn an_empty_named_file_is_created_by_save_even_when_unmodified(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.path("empty.txt");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    assert!(!f.modified(cx));
    f.run(cx, "F_FILESAVE", &[]);
    assert!(path.is_file());
    assert_eq!(read(&path), b"");
}

#[gpui::test]
fn a_file_that_is_still_loading_is_not_opened_twice(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    cx.update(gpui_component::init);
    let request = LoadRequest {
        path: path.clone(),
        encoding: None,
    };
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), Some(request)))
        .unwrap();
    // The read has not run yet: the document has no path.
    let found = cx.update(|cx| super::windows::find_open(cx, &path).map(|(h, _)| h));
    assert_eq!(found, Some(handle));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.run_until_parked();
    let found = cx.update(|_, cx| super::windows::find_open(cx, &path).map(|(h, _)| h));
    assert_eq!(found, Some(handle), "and after the read, by its path");
}

#[gpui::test]
fn saving_an_untitled_document_asks_for_a_name(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.path("saved.txt");
    let (f, cx) = setup(cx);
    cx.simulate_input("hello");
    f.run(cx, "F_FILESAVE", &[]);
    let target = path.clone();
    cx.simulate_new_path_selection(move |_| Some(target));
    cx.run_until_parked();
    assert_eq!(read(&path), b"hello");
    assert_eq!(f.title(cx), "saved.txt - UME Editor");
    assert_eq!(f.format(cx), "UTF-8  CRLF");
    assert!(f.notice(cx).unwrap().contains("saved.txt"));
}

#[gpui::test]
fn cancelling_the_save_dialog_leaves_the_document_modified(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    cx.simulate_input("hello");
    f.run(cx, "F_FILESAVE", &[]);
    cx.simulate_new_path_selection(|_| None);
    cx.run_until_parked();
    assert!(f.modified(cx));
    assert_eq!(f.title(cx), "(無題) * - UME Editor");
}

#[gpui::test]
fn a_plain_save_keeps_the_encoding_and_the_bytes_of_the_rest(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &[SJIS_NIHONGO, b"\r\nabc"].concat());
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(SJIS));
    cx.simulate_input("x");
    assert!(f.modified(cx));
    f.run(cx, "F_FILESAVE", &[]);
    assert_eq!(
        read(&path),
        [b"x".as_slice(), SJIS_NIHONGO, b"\r\nabc"].concat()
    );
    assert!(!f.modified(cx));
}

#[gpui::test]
fn saving_an_unmodified_document_writes_nothing(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    // Change the file behind the editor's back: a write would undo it.
    std::fs::write(&path, b"changed").unwrap();
    f.run(cx, "F_FILESAVE", &[]);
    assert_eq!(read(&path), b"changed");
    assert!(f.notice(cx).unwrap().contains("変更がない"));
}

#[gpui::test]
fn save_as_with_arguments_changes_the_encoding_and_the_line_ending(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"\xE6\x97\xA5\xE6\x9C\xAC\xE8\xAA\x9E\r\nabc");
    let target = dir.path("b.txt");
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    // F_FILESAVEAS_DIALOG: (default name, encoding, line ending).
    f.run(
        cx,
        "F_FILESAVEAS_DIALOG",
        &[Arg::Text(String::new()), Arg::Int(SJIS), Arg::Int(2)],
    );
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert_eq!(read(&target), [SJIS_NIHONGO, b"\nabc"].concat());
    assert_eq!(f.format(cx), "SJIS  LF");
    assert_eq!(f.title(cx), "b.txt - UME Editor");
    assert_eq!(
        read(&path),
        b"\xE6\x97\xA5\xE6\x9C\xAC\xE8\xAA\x9E\r\nabc",
        "the old file is untouched"
    );
}

#[gpui::test]
fn save_as_without_a_dialog_takes_the_name_and_the_line_ending(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("c.txt");
    let (f, cx) = setup(cx);
    cx.simulate_input("a");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("b");
    f.run(
        cx,
        "F_FILESAVEAS",
        &[
            Arg::Text(target.display().to_string()),
            Arg::Int(99),
            Arg::Int(2),
        ],
    );
    assert_eq!(read(&target), b"a\nb");
    assert!(!cx.has_pending_prompt());
}

#[gpui::test]
fn characters_the_encoding_cannot_write_are_asked_about(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &[SJIS_NIHONGO, b"\r\n"].concat());
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(SJIS));
    cx.simulate_input("😀");
    let before = read(&path);

    // Stop: nothing is written.
    f.run(cx, "F_FILESAVE", &[]);
    let (message, detail) = cx.pending_prompt().unwrap();
    assert!(message.contains("SJIS"), "{message}");
    assert!(detail.contains("U+1F600"), "{detail}");
    answer(cx, STOP);
    assert_eq!(read(&path), before);
    assert!(f.modified(cx));

    // Go to the place: the caret moves, nothing is written.
    cx.simulate_keystrokes("ctrl-end");
    assert_ne!(f.caret(cx), Pos::new(0, 0));
    f.run(cx, "F_FILESAVE", &[]);
    answer(cx, GO_TO_PLACE);
    assert_eq!(
        f.caret(cx),
        Pos::new(0, 0),
        "the emoji is at the start of the first line"
    );
    assert_eq!(read(&path), before);

    // Continue: the character becomes "?".
    f.run(cx, "F_FILESAVE", &[]);
    answer(cx, CONTINUE);
    assert_eq!(
        read(&path),
        [b"?".as_slice(), SJIS_NIHONGO, b"\r\n"].concat()
    );
    assert!(!f.modified(cx));
    assert!(f.notice(cx).unwrap().contains("? にした"));
}

#[gpui::test]
fn mixed_line_endings_ask_to_unify(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"a\r\nb\nc");
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    cx.simulate_input("x");

    // Cancel: nothing is written.
    f.run(cx, "F_FILESAVE", &[]);
    let (message, _) = cx.pending_prompt().unwrap();
    assert!(message.contains("CRLF"), "{message}");
    answer(cx, CANCEL);
    assert_eq!(read(&path), b"a\r\nb\nc");

    // Save as it is.
    f.run(cx, "F_FILESAVE", &[]);
    answer(cx, KEEP_MIXED);
    assert_eq!(read(&path), b"xa\r\nb\nc");

    // Unify to the first line's ending.
    cx.simulate_input("y");
    f.run(cx, "F_FILESAVE", &[]);
    answer(cx, UNIFY);
    assert_eq!(read(&path), b"xya\r\nb\r\nc");
    // The choice is remembered: the next plain save does not ask again.
    cx.simulate_input("z");
    f.run(cx, "F_FILESAVE", &[]);
    assert!(!cx.has_pending_prompt());
    assert_eq!(read(&path), b"xyza\r\nb\r\nc");
}

#[gpui::test]
fn closing_a_modified_file_asks_and_cancel_keeps_everything(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    cx.simulate_input("x");

    f.run(cx, "F_FILECLOSE", &[]);
    let (message, _) = cx.pending_prompt().unwrap();
    assert!(
        message.contains("a.txt") && message.contains("変更されています"),
        "{message}"
    );
    answer(cx, CANCEL);
    assert_eq!(f.lines(cx), ["xabc"]);
    assert!(f.modified(cx));

    // Save and close: the file is written and the window is untitled and empty.
    f.run(cx, "F_FILECLOSE", &[]);
    answer(cx, SAVE);
    assert_eq!(read(&path), b"xabc");
    assert_eq!(f.title(cx), "(無題) - UME Editor");
    assert!(!f.modified(cx));
}

#[gpui::test]
fn closing_without_saving_discards_the_changes(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    cx.simulate_input("x");
    f.run(cx, "F_FILECLOSE", &[]);
    answer(cx, DONT_SAVE);
    assert_eq!(read(&path), b"abc");
    assert_eq!(f.title(cx), "(無題) - UME Editor");
    assert!(!f.modified(cx));
}

#[gpui::test]
fn save_then_cancelling_the_name_dialog_stops_the_close(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    cx.simulate_input("keepme");
    f.run(cx, "F_FILECLOSE", &[]);
    answer(cx, SAVE);
    cx.simulate_new_path_selection(|_| None);
    cx.run_until_parked();
    assert_eq!(f.lines(cx), ["keepme"]);
    assert!(f.modified(cx));
}

#[gpui::test]
fn an_unmodified_file_closes_without_asking(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    f.run(cx, "F_FILECLOSE", &[]);
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.title(cx), "(無題) - UME Editor");
}

#[gpui::test]
fn opening_from_a_modified_window_uses_a_new_window(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let (f, cx) = setup(cx);
    cx.simulate_input("typed");
    f.run(cx, "F_FILEOPEN", &[]);
    assert!(cx.did_prompt_for_paths(), "no path given: the open dialog");
    let chosen = path.clone();
    cx.simulate_path_prompt_response(move |options| {
        assert!(options.files && options.multiple);
        Some(vec![chosen])
    });
    cx.run_until_parked();
    assert_eq!(f.lines(cx), ["typed"], "this window is untouched");
    assert_eq!(cx.windows().len(), 2);
}

#[gpui::test]
fn the_other_files_of_the_open_dialog_get_new_windows(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let a = dir.write("a.txt", b"A");
    let b = dir.write("b.txt", b"B");
    let c = dir.write("c.txt", b"C");
    let (f, cx) = setup(cx);
    f.run(cx, "F_FILEOPEN", &[]);
    cx.simulate_path_prompt_response(move |_| Some(vec![a, b, c]));
    cx.run_until_parked();
    assert_eq!(
        f.lines(cx),
        ["A"],
        "the first goes into the untitled window"
    );
    assert_eq!(cx.windows().len(), 3);
}

#[gpui::test]
fn close_and_open_keeps_the_file_when_the_dialog_is_cancelled(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let other = dir.write("b.txt", b"other");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    f.run(cx, "F_FILECLOSE_OPEN", &[]);
    cx.simulate_path_prompt_response(|_| None);
    cx.run_until_parked();
    assert_eq!(f.title(cx), "a.txt - UME Editor");

    f.run(cx, "F_FILECLOSE_OPEN", &[]);
    cx.simulate_path_prompt_response(move |_| Some(vec![other]));
    cx.run_until_parked();
    assert_eq!(f.lines(cx), ["other"]);
    assert_eq!(cx.windows().len(), 1);
}

#[gpui::test]
fn reopening_reads_the_file_again_in_another_encoding(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &[SJIS_NIHONGO, b"\r\nabc"].concat());
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(f.format(cx), "UTF-8  CRLF");
    assert_ne!(f.lines(cx)[0], "日本語");
    f.run(cx, "F_FILE_REOPEN_SJIS", &[]);
    assert_eq!(f.lines(cx), ["日本語", "abc"]);
    assert_eq!(f.format(cx), "SJIS  CRLF");
}

#[gpui::test]
fn reopening_a_modified_file_asks_unless_told_not_to(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    cx.simulate_input("x");

    f.run(cx, "F_FILE_REOPEN", &[]);
    let (message, _) = cx.pending_prompt().unwrap();
    assert!(message.contains("再ロード"), "{message}");
    answer(cx, CANCEL);
    assert_eq!(f.lines(cx), ["xabc"]);

    f.run(cx, "F_FILE_REOPEN", &[]);
    answer(cx, RELOAD);
    assert_eq!(f.lines(cx), ["abc"]);
    assert!(!f.modified(cx));

    cx.simulate_input("y");
    f.run(cx, "F_FILE_REOPEN_UTF8", &[Arg::Int(1)]);
    assert!(!cx.has_pending_prompt(), "bNoConfirm");
    assert_eq!(f.lines(cx), ["abc"]);
}

#[gpui::test]
fn reopening_keeps_the_caret(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"line1\r\nline2\r\nline3");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    cx.simulate_keystrokes("down down right right");
    let caret = f.caret(cx);
    assert_eq!(caret, Pos::new(2, 2));
    f.run(cx, "F_FILE_REOPEN_UTF8", &[Arg::Int(1)]);
    assert_eq!(f.caret(cx), caret);
}

#[gpui::test]
fn a_document_without_a_file_only_takes_the_encoding_on_reopen(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    cx.simulate_input("abc");
    f.run(cx, "F_FILE_REOPEN_UNICODE", &[]);
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.lines(cx), ["abc"]);
    assert_eq!(f.format(cx), "UTF-16 BOM付  CRLF");
}

#[gpui::test]
fn a_directory_shows_an_error_and_leaves_the_window_alone(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let (f, cx) = setup(cx);
    cx.simulate_input("x");
    // The window is modified, so the directory goes to a new window which fails to load.
    f.open(cx, &dir.0, None);
    assert_eq!(f.lines(cx), ["x"]);
}

#[gpui::test]
fn a_new_window_loads_its_file_and_shows_errors_in_itself(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"hello");
    let request = LoadRequest {
        path,
        encoding: None,
    };
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), Some(request)))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.run_until_parked();
    let text = cx.update(|_, app| {
        let editors = super::windows::editors(app);
        assert_eq!(editors.len(), 1);
        let doc = editors[0].1.read(app).document().read(app);
        doc.doc().line(0).unwrap().unwrap().text
    });
    assert_eq!(text, "hello");

    // A directory cannot be opened: the error is shown in that window.
    let request = LoadRequest {
        path: dir.0.clone(),
        encoding: None,
    };
    let handle = cx
        .update(|_, app| open_editor_window(app, SharedDocument::empty(), Some(request)))
        .unwrap();
    let cx2 = &mut VisualTestContext::from_window(handle, cx);
    cx2.run_until_parked();
    let (message, _) = cx2.pending_prompt().expect("the error is shown");
    assert!(message.contains("を開けませんでした"), "{message}");
}

#[gpui::test]
fn closing_a_modified_window_asks_first(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.run_until_parked();
    // An unmodified window just closes.
    assert!(cx.simulate_close());

    let handle = cx
        .update(|_, app| open_editor_window(app, SharedDocument::empty(), None))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.simulate_input("x");
    assert!(!cx.simulate_close(), "the window waits for the answer");
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    answer(cx, CANCEL);
    assert!(cx.windows().contains(&handle));
    // Asking again after a cancel works.
    assert!(!cx.simulate_close());
    cx.run_until_parked();
    answer(cx, DONT_SAVE);
    assert!(!cx.windows().contains(&handle));
}

#[gpui::test]
fn win_close_and_save_close(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.path("s.txt");
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.simulate_input("saved");
    let editor = cx.update(|_, app| super::windows::editors(app)[0].1.clone());
    let f = Fixture { view: editor };
    f.run(cx, "F_WINCLOSE", &[]);
    answer(cx, CANCEL);
    assert!(cx.windows().contains(&handle));

    f.run(cx, "F_FILESAVECLOSE", &[]);
    let target = path.clone();
    cx.simulate_new_path_selection(move |_| Some(target));
    cx.run_until_parked();
    assert_eq!(read(&path), b"saved");
    assert!(!cx.windows().contains(&handle));
}

#[gpui::test]
fn exit_all_stops_when_one_window_is_kept(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.simulate_input("x");
    let editor = cx.update(|_, app| super::windows::editors(app)[0].1.clone());
    let f = Fixture { view: editor };
    f.run(cx, "F_EXITALL", &[]);
    answer(cx, CANCEL);
    assert!(cx.windows().contains(&handle));
    assert!(f.modified(cx));
}

#[gpui::test]
fn a_file_that_is_open_in_a_window_is_not_opened_again(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let request = LoadRequest {
        path: path.clone(),
        encoding: None,
    };
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), Some(request)))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.run_until_parked();
    let editor = cx.update(|_, app| super::windows::editors(app)[0].1.clone());
    let f = Fixture { view: editor };
    f.open(cx, &path, None);
    assert_eq!(cx.windows().len(), 1);
    assert!(!cx.has_pending_prompt());
}

#[gpui::test]
fn new_windows_for_new_files(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    f.run(cx, "F_FILENEW", &[]);
    assert_eq!(cx.windows().len(), 2);
    f.run(cx, "F_FILENEW_NEWWINDOW", &[]);
    assert_eq!(cx.windows().len(), 3);
}

#[gpui::test]
fn bad_arguments_are_reported_not_ignored(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    f.run(cx, "F_FILESAVEAS", &[]);
    assert!(f.notice(cx).unwrap().contains("file name"));
    f.run(
        cx,
        "F_FILESAVEAS_DIALOG",
        &[Arg::Text(String::new()), Arg::Int(42)],
    );
    assert!(f.notice(cx).unwrap().contains("encoding"));
    assert!(!cx.has_pending_prompt());
}

#[test]
fn progress_text_names_the_file_and_the_percent() {
    let text = progress_text(Path::new("/x/big.log"), 3 * 1024 * 1024, 12 * 1024 * 1024);
    assert_eq!(text, "big.log を読み込み中 25% (3 / 12 MB)");
    assert!(progress_text(Path::new("a"), 0, 0).contains("0%"));
}
