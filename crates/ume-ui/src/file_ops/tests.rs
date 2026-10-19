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

/// The name of the line ending a new document has: CRLF on Windows, LF elsewhere.
fn os_eol_name() -> &'static str {
    if cfg!(windows) { "CRLF" } else { "LF" }
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
        self.view
            .read_with(cx, crate::file_names::title::plain_title)
    }

    fn format(&self, cx: &mut VisualTestContext) -> String {
        self.view
            .read_with(cx, |v, app| v.status(app).file_format())
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
    // The options dialog of the Save As: Enter is OK.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(read(&path), b"hello");
    assert_eq!(f.title(cx), "saved.txt - UME Editor");
    assert_eq!(f.format(cx), format!("UTF-8  {}", os_eol_name()));
    assert!(f.notice(cx).unwrap().contains("saved.txt"));
}

/// `CODE_JIS` of `F_FILEOPEN`'s second argument.
const JIS: i64 = 1;
const MIME_MAIL: &[u8] = b"=?ISO-2022-JP?B?GyRCRnxLXDhsGyhC?=";

#[gpui::test]
fn opening_a_jis_file_keeps_its_mime_words_by_default(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("mail.txt", MIME_MAIL);
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(JIS));
    assert_eq!(f.lines(cx), ["=?ISO-2022-JP?B?GyRCRnxLXDhsGyhC?="]);
}

#[gpui::test]
fn opening_a_jis_file_decodes_its_mime_words_when_the_common_setting_is_on(
    cx: &mut TestAppContext,
) {
    let dir = TempDir::new();
    let path = dir.write("mail.txt", MIME_MAIL);
    let (f, cx) = setup(cx);
    cx.update(|_, app| {
        app.set_global(super::prefs::FilePrefs {
            auto_mime_decode: true,
            ..super::prefs::FilePrefs::default()
        });
    });
    f.open(cx, &path, Some(JIS));
    assert_eq!(f.lines(cx), ["日本語"]);
}

#[gpui::test]
fn cancelling_the_save_dialog_leaves_the_document_modified(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    cx.simulate_input("hello");
    f.run(cx, "F_FILESAVE", &[]);
    cx.simulate_new_path_selection(|_| None);
    cx.run_until_parked();
    assert!(f.modified(cx));
    assert_eq!(f.title(cx), "(無題)(更新) - UME Editor");
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
    assert_eq!(f.format(cx), format!("UTF-16 BOM付  {}", os_eol_name()));
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
fn closing_a_window_whose_encoding_was_changed_asks_first(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "日本語".as_bytes());
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let editor = cx.update(|_, app| super::windows::editors(app)[0].1.clone());
    let f = Fixture { view: editor };
    f.open(cx, &path, Some(UTF8));
    // `IsChgCodeSet`: the text is unmodified, the encoding is not the one that was read
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(SJIS)]);
    assert!(!cx.simulate_close(), "the window waits for the answer");
    cx.run_until_parked();
    answer(cx, DONT_SAVE);
    assert!(!cx.windows().contains(&handle));
    assert_eq!(read(&path), "日本語".as_bytes());
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
    cx.simulate_keystrokes("enter");
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

#[gpui::test]
fn the_version_keeps_counting_when_the_document_is_replaced(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let a = dir.write("a.txt", b"A");
    let b = dir.write("b.txt", b"B");
    let (f, cx) = setup(cx);
    let version = |f: &Fixture, cx: &mut VisualTestContext| {
        f.view
            .read_with(cx, |v, app| v.document().read(app).version())
    };
    let mut seen = vec![version(&f, cx)];
    for step in 0..3 {
        match step {
            0 => f.open(cx, &a, None),
            1 => f.run(cx, "F_FILE_REOPEN", &[Arg::Int(1)]),
            _ => f.run(cx, "F_FILECLOSE_OPEN", &[]),
        }
        if step == 2 {
            let chosen = b.clone();
            cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
            cx.run_until_parked();
        }
        seen.push(version(&f, cx));
    }
    assert!(seen.windows(2).all(|w| w[0] < w[1]), "{seen:?}");
}

#[gpui::test]
fn close_and_open_does_not_open_a_file_twice(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    cx.update(gpui_component::init);
    let request = LoadRequest {
        path: path.clone(),
        encoding: None,
    };
    let first = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), Some(request)))
        .unwrap();
    let second = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(second, cx);
    cx.run_until_parked();
    let editor = cx.update(|_, app| {
        super::windows::editors(app)
            .into_iter()
            .find(|(h, _)| *h == second)
            .unwrap()
            .1
    });
    let f = Fixture { view: editor };
    f.run(cx, "F_FILECLOSE_OPEN", &[]);
    let chosen = path.clone();
    cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
    cx.run_until_parked();
    assert_eq!(
        f.title(cx),
        "(無題) - UME Editor",
        "this window is untouched"
    );
    assert!(cx.windows().contains(&first));
    assert_eq!(cx.windows().len(), 2);
}

#[gpui::test]
fn a_file_being_read_into_a_window_counts_as_open(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.path("a.txt");
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let found = |cx: &mut TestAppContext| {
        cx.update(|cx| super::windows::find_open(cx, &path).map(|(h, _)| h))
    };
    assert_eq!(found(cx), None);
    let view = cx.update(|cx| super::windows::editors(cx)[0].1.downgrade());
    cx.update(|cx| super::windows::begin_pending(cx, handle, &view, path.clone()));
    assert_eq!(found(cx), Some(handle));
    cx.update(|cx| super::windows::finish_pending(cx, &view));
    assert_eq!(found(cx), None);
}

#[gpui::test]
fn exit_all_asks_again_when_a_checked_window_changed(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let a = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let b = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let cx_a = &mut VisualTestContext::from_window(a, cx);
    cx_a.simulate_input("a");
    let cx_b = &mut VisualTestContext::from_window(b, cx_a);
    cx_b.simulate_input("b");
    let editor_a = cx_b.update(|_, app| super::windows::editors(app)[0].1.clone());
    let f = Fixture { view: editor_a };
    f.run(cx_b, "F_EXITALL", &[]);
    // The first window is checked and discarded; the second one is asked.
    answer(cx_b, DONT_SAVE);
    assert!(cx_b.has_pending_prompt());
    // The first window is edited while the second question is open.
    let cx_a = &mut VisualTestContext::from_window(a, cx_b);
    cx_a.simulate_input("more");
    answer(cx_a, DONT_SAVE);
    // Quitting without a new question would lose "more": the first window is asked again.
    assert!(cx_a.has_pending_prompt());
    answer(cx_a, CANCEL);
    assert!(cx_a.windows().contains(&a));
    assert_eq!(f.lines(cx_a), ["amore"]);
}

#[gpui::test]
fn the_same_file_on_the_command_line_gets_one_window(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    cx.update(gpui_component::init);
    let same = dir.0.join(".").join("a.txt");
    cx.update(|cx| super::windows::open_from_outside(cx, vec![path, same]))
        .unwrap();
    assert_eq!(cx.windows().len(), 1);
    let none: Vec<PathBuf> = Vec::new();
    let cx2 = cx;
    cx2.update(|cx| super::windows::open_from_outside(cx, none))
        .unwrap();
    assert_eq!(cx2.windows().len(), 2, "no files: one untitled window");
}

/// A file handed over by a later launch (ADR-0016) that is open already brings its window to the
/// front (`CNormalProcess::ActiveAlreadyOpenedWindow`), and no second window is made for it.
#[gpui::test]
fn a_forwarded_open_of_an_open_file_activates_its_window(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let a = dir.write("a.txt", b"abc");
    cx.update(gpui_component::init);
    cx.update(|cx| super::windows::open_from_outside(cx, vec![a.clone()]))
        .unwrap();
    cx.run_until_parked();
    let first = cx.windows()[0];
    // Another window is the active one now.
    let other = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.update(|cx| super::windows::activate(cx, other));
    assert_eq!(cx.update(|cx| cx.active_window()), Some(other));
    // The launch named the file in another way (a longer path to the same file).
    let same = dir.0.join(".").join("a.txt");
    cx.update(|cx| super::windows::open_from_outside(cx, vec![same]))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(cx.windows().len(), 2, "no second window for the same file");
    assert_eq!(cx.update(|cx| cx.active_window()), Some(first));
}

/// A file that is not open yet gets a window of its own and its contents.
#[gpui::test]
fn a_forwarded_open_of_a_new_file_opens_a_window(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let a = dir.write("a.txt", b"abc");
    let b = dir.write("b.txt", "日本語".as_bytes());
    cx.update(gpui_component::init);
    cx.update(|cx| super::windows::open_from_outside(cx, vec![a]))
        .unwrap();
    cx.run_until_parked();
    cx.update(|cx| super::windows::open_from_outside(cx, vec![b.clone()]))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(cx.windows().len(), 2);
    let found = cx.update(|cx| {
        super::windows::find_open(cx, &b).map(|(_, view)| {
            let doc = view.read(cx).document().read(cx).doc();
            doc.line(0).unwrap().unwrap().text
        })
    });
    assert_eq!(found.as_deref(), Some("日本語"));
}

/// A launch without a file opens a new, empty window, also when windows are open already
/// (`CControlTray::OpenNewEditor`).
#[gpui::test]
fn a_forwarded_open_without_a_file_opens_an_empty_window(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    cx.update(|cx| super::windows::open_from_outside(cx, Vec::new()))
        .unwrap();
    cx.update(|cx| super::windows::open_from_outside(cx, Vec::new()))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(cx.windows().len(), 2);
    let empty = cx.update(|cx| {
        super::windows::editors(cx)
            .iter()
            .all(|(_, view)| view.read(cx).document().read(cx).path().is_none())
    });
    assert!(empty);
}

#[gpui::test]
fn a_window_that_is_reading_a_file_is_not_reused_for_another(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let other = dir.write("b.txt", b"B");
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let editor = cx.update(|_, app| super::windows::editors(app)[0].1.clone());
    cx.update(|_, app| {
        super::windows::begin_pending(app, handle, &editor.downgrade(), dir.path("a.txt"))
    });
    let f = Fixture { view: editor };
    f.open(cx, &other, None);
    assert_eq!(cx.windows().len(), 2, "B gets its own window");
    assert_eq!(f.lines(cx).len(), 0, "the busy window is untouched");
}

// ---- the options dialog of the Save As (save_as_dialog.rs) ----

impl Fixture {
    fn dialog_open(&self, cx: &mut VisualTestContext) -> bool {
        self.view.read_with(cx, |v, _| v.save_as_dialog.is_some())
    }

    /// Runs a Save As and answers the path dialog, so that the options dialog is open.
    fn save_as_to(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg], to: &Path) {
        self.run(cx, function, args);
        let target = to.to_owned();
        cx.simulate_new_path_selection(move |_| Some(target));
        cx.run_until_parked();
    }

    fn keys(&self, cx: &mut VisualTestContext, keys: &str) {
        cx.simulate_keystrokes(keys);
        cx.run_until_parked();
    }
}

/// A window with `bytes` read from `dir` as UTF-8.
fn utf8_fixture<'a>(
    cx: &'a mut TestAppContext,
    dir: &TempDir,
    bytes: &[u8],
) -> (Fixture, &'a mut VisualTestContext) {
    let path = dir.write("a.txt", bytes);
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    (f, cx)
}

const NIHONGO_UTF8: &[u8] = "日本語".as_bytes();

#[gpui::test]
fn the_options_open_after_the_path_and_ok_keeps_the_current_values(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, &[NIHONGO_UTF8, b"\r\nabc"].concat());
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    assert!(f.dialog_open(cx));
    assert!(!target.exists(), "nothing is written before OK");
    f.keys(cx, "enter");
    assert!(!f.dialog_open(cx));
    assert_eq!(read(&target), [NIHONGO_UTF8, b"\r\nabc"].concat());
    assert_eq!(f.title(cx), "b.txt - UME Editor");
}

#[gpui::test]
fn choosing_the_encoding_and_the_line_ending_with_the_keys(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, &[NIHONGO_UTF8, b"\r\nabc"].concat());
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    // The list has the focus, on UTF-8. Home goes to Shift_JIS, the first one.
    f.keys(cx, "home");
    // Tab skips the BOM box (disabled for SJIS) and reaches the line endings: Down is CR+LF,
    // Down again is LF.
    f.keys(cx, "tab down down");
    f.keys(cx, "enter");
    assert_eq!(read(&target), [SJIS_NIHONGO, b"\nabc"].concat());
    assert_eq!(f.format(cx), "SJIS  LF");
}

#[gpui::test]
fn the_bom_box_is_reached_with_tab_and_toggled_with_space(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    f.keys(cx, "tab space enter");
    assert_eq!(read(&target), b"\xEF\xBB\xBFabc");
    // "abc" has no line ending, so the document has the one of the OS
    assert_eq!(f.format(cx), format!("UTF-8 BOM付  {}", os_eol_name()));
}

#[gpui::test]
fn utf16_defaults_to_a_bom_and_a_line_ending_is_applied(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"a\r\nb");
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    // The order is SJIS, JIS, EUC, Latin1, UTF-16, UTF-16BE, UTF-8: two up from UTF-8.
    f.keys(cx, "up up");
    // Tab (BOM box), Tab (line endings), Right Right: LF.
    f.keys(cx, "tab tab right right enter");
    assert_eq!(read(&target), b"\xFF\xFEa\x00\n\x00b\x00");
}

#[gpui::test]
fn space_on_the_ok_button_saves(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    // List, BOM box, line endings, OK.
    f.keys(cx, "tab tab tab space");
    assert!(!f.dialog_open(cx));
    assert_eq!(read(&target), b"abc");
}

#[gpui::test]
fn escape_and_the_cancel_button_stop_the_save(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    f.keys(cx, "escape");
    assert!(!f.dialog_open(cx));
    assert!(!target.exists());
    assert_eq!(f.title(cx), "a.txt - UME Editor", "still the old file");

    // Shift+Tab from the list wraps to Cancel; Enter on it cancels.
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    f.keys(cx, "shift-tab enter");
    assert!(!f.dialog_open(cx));
    assert!(!target.exists());

    // The dialog can be opened again after cancelling.
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    assert!(f.dialog_open(cx));
    f.keys(cx, "enter");
    assert_eq!(read(&target), b"abc");
}

#[gpui::test]
fn cancelling_the_path_dialog_never_shows_the_options(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    cx.simulate_new_path_selection(|_| None);
    cx.run_until_parked();
    assert!(!f.dialog_open(cx));
}

#[gpui::test]
fn the_editor_takes_no_keys_while_the_options_are_open(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    cx.simulate_input("zzz");
    assert_eq!(f.lines(cx), ["abc"]);
    f.keys(cx, "escape");
    cx.simulate_input("x");
    assert_eq!(f.lines(cx), ["xabc"], "the view has the focus again");
}

#[gpui::test]
fn the_existing_questions_come_after_the_options(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, "日本語😀\r\nabc\nx".as_bytes());
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    f.keys(cx, "home enter");
    // The line endings are mixed, and Shift_JIS cannot write the emoji.
    let (message, _) = cx.pending_prompt().unwrap();
    assert!(message.contains("改行コードが混在"), "{message}");
    answer(cx, KEEP_MIXED);
    let (message, _) = cx.pending_prompt().unwrap();
    assert!(message.contains("SJIS"), "{message}");
    answer(cx, STOP);
    assert!(!target.exists());
}

#[gpui::test]
fn a_macro_with_arguments_skips_the_options(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    f.save_as_to(
        cx,
        "F_FILESAVEAS_DIALOG",
        &[Arg::Text(String::new()), Arg::Int(SJIS)],
        &target,
    );
    assert!(!f.dialog_open(cx));
    assert_eq!(read(&target), b"abc");
    assert_eq!(f.format(cx), format!("SJIS  {}", os_eol_name()));

    // The Save As that takes a path never shows it either.
    let target2 = dir.path("c.txt");
    f.run(
        cx,
        "F_FILESAVEAS",
        &[Arg::Text(target2.display().to_string())],
    );
    assert!(!f.dialog_open(cx));
    assert!(target2.exists());
}

#[gpui::test]
fn saving_an_untitled_document_shows_the_options_too(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("n.txt");
    let (f, cx) = setup(cx);
    cx.simulate_input("hi");
    f.save_as_to(cx, "F_FILESAVE", &[], &target);
    assert!(f.dialog_open(cx));
    f.keys(cx, "up up enter");
    assert_eq!(read(&target), b"\xFF\xFEh\x00i\x00");
}

#[gpui::test]
fn a_second_save_as_does_not_open_another_path_dialog(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    f.save_as_to(cx, "F_FILESAVEAS_DIALOG", &[], &target);
    // A menu or a macro starts another one while the options are open: it says so and stops.
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    assert!(f.notice(cx).unwrap().contains("途中"));
    assert!(f.dialog_open(cx), "the first dialog is still there");
    f.keys(cx, "enter");
    assert_eq!(read(&target), b"abc");
}

// ---- Base64 and uudecode of the selection (save_decoded.rs) ----

#[gpui::test]
fn base64_decode_saves_the_bytes_of_the_selection(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("out.bin");
    let (f, cx) = utf8_fixture(cx, &dir, b"aGVs\r\nbG8=");
    f.run(cx, "F_SELECTALL", &[]);
    f.run(cx, "F_BASE64DECODE", &[]);
    assert!(cx.did_prompt_for_new_path());
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert_eq!(read(&target), b"hello");
    // The document is not changed.
    assert!(!f.modified(cx));
    assert_eq!(f.title(cx), "a.txt - UME Editor");
}

#[gpui::test]
fn base64_decode_replaces_an_existing_file_without_leaving_a_temporary_one(
    cx: &mut TestAppContext,
) {
    let dir = TempDir::new();
    let target = dir.write("old.bin", b"old contents that are longer");
    let (f, cx) = utf8_fixture(cx, &dir, b"aGVsbG8=");
    f.run(cx, "F_SELECTALL", &[]);
    f.run(cx, "F_BASE64DECODE", &[]);
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert_eq!(read(&target), b"hello");
    assert!(!dir.path("old.bin.ume-tmp").exists());
}

#[gpui::test]
fn base64_decode_does_nothing_without_a_selection_or_for_other_text(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let (f, cx) = utf8_fixture(cx, &dir, b"aGVs!bG8=");
    f.run(cx, "F_BASE64DECODE", &[]);
    assert!(!cx.did_prompt_for_new_path(), "no selection");
    f.run(cx, "F_SELECTALL", &[]);
    f.run(cx, "F_BASE64DECODE", &[]);
    assert!(!cx.did_prompt_for_new_path(), "not Base64");
}

#[gpui::test]
fn cancelling_the_dialog_of_base64_decode_writes_nothing(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let (f, cx) = utf8_fixture(cx, &dir, b"aGVsbG8=");
    f.run(cx, "F_SELECTALL", &[]);
    f.run(cx, "F_BASE64DECODE", &[]);
    cx.simulate_new_path_selection(|_| None);
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert!(!f.modified(cx));
}

#[gpui::test]
fn uudecode_saves_the_bytes_of_the_selection(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let target = dir.path("cat.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"begin 644 cat.txt\r\n#0V%T\r\n`\r\nend\r\n");
    f.run(cx, "F_SELECTALL", &[]);
    f.run(cx, "F_UUDECODE", &[]);
    assert!(cx.did_prompt_for_new_path());
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert_eq!(read(&target), b"Cat");
    assert!(!f.modified(cx));
}

#[gpui::test]
fn uudecode_does_nothing_for_text_that_is_not_uu(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let (f, cx) = utf8_fixture(cx, &dir, b"begin 644 a\n#0V%T\n`\nend\n");
    f.run(cx, "F_SELECTALL", &[]);
    f.run(cx, "F_UUDECODE", &[]);
    assert!(
        !cx.did_prompt_for_new_path(),
        "the lines do not end with CR LF"
    );
}
