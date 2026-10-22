//! Tests of the history in GPUI's headless test windows. The files are real files in a private
//! temporary folder; the history is in memory unless a test says it is in a file.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::history::{FileEntry, History, HistoryStore};
use matsu_core::buffer::Pos;
use matsu_core::encoding::Encoding;

use super::view::{encoding_of, path_text, record_all};
use super::{HistoryOptions, HistoryState, current};
use crate::document::SharedDocument;
use crate::view::{EditorView, OpenRecentFile};

/// A folder that is removed at the end of the test.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("matsu-ui-history-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

struct Fixture {
    view: Entity<EditorView>,
}

fn setup_with(cx: &mut TestAppContext, state: HistoryState) -> (Fixture, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    cx.update(|cx| cx.set_global(state));
    let shared = cx.new(|_| SharedDocument::empty());
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (Fixture { view }, vcx)
}

fn setup(cx: &mut TestAppContext) -> (Fixture, &mut VisualTestContext) {
    setup_with(cx, HistoryState::in_memory(HistoryOptions::default()))
}

impl Fixture {
    fn run(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
        let code = code_of(function);
        self.view.update_in(cx, |view, window, cx| {
            view.run_function(code, args, window, cx)
        });
        cx.run_until_parked();
    }

    fn open(&self, cx: &mut VisualTestContext, path: &Path, encoding: Option<i64>) {
        let mut args = vec![Arg::Text(path.display().to_string())];
        if let Some(code) = encoding {
            args.push(Arg::Int(code));
        }
        self.run(cx, "F_FILEOPEN", &args);
    }

    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view.read_with(cx, |v, _| v.cursor().caret())
    }

    fn go_to(&self, cx: &mut VisualTestContext, pos: Pos) {
        self.view.update(cx, |v, cx| v.go_to(pos, cx));
    }

    fn marks(&self, cx: &mut VisualTestContext) -> Vec<usize> {
        self.view.read_with(cx, |v, app| {
            v.document().read(app).model().bookmarks().iter().collect()
        })
    }

    fn mark(&self, cx: &mut VisualTestContext, line: usize) {
        self.view.update(cx, |v, cx| {
            v.document().update(cx, |shared, _| {
                shared.parts_mut().0.bookmarks_mut().set(line, true);
            });
        });
    }

    fn encoding(&self, cx: &mut VisualTestContext) -> Encoding {
        self.view
            .read_with(cx, |v, app| v.document().read(app).file().encoding)
    }

    fn text(&self, cx: &mut VisualTestContext) -> String {
        self.view.read_with(cx, |v, app| {
            let doc = v.document().read(app).doc();
            (0..doc.line_count())
                .map(|i| doc.line(i).unwrap().unwrap().text)
                .collect::<Vec<_>>()
                .join("\n")
        })
    }
}

fn history(cx: &mut VisualTestContext) -> History {
    cx.update(|_, app| current(app).expect("a history"))
}

fn set_history(cx: &mut VisualTestContext, change: impl FnOnce(&mut History)) {
    cx.update(|_, app| {
        super::update(app, change);
    });
}

fn answer(cx: &mut VisualTestContext, button: &str) {
    assert!(cx.has_pending_prompt(), "no prompt to answer {button:?}");
    cx.simulate_prompt_answer(button);
    cx.run_until_parked();
}

const THREE_LINES: &[u8] = b"first\nsecond line\nthird\n";

#[gpui::test]
fn opening_a_file_puts_it_and_its_folder_at_the_front(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let a = dir.write("a.txt", THREE_LINES);
    let b = dir.write("b.txt", THREE_LINES);
    let (f, cx) = setup(cx);
    f.open(cx, &a, None);
    let h = history(cx);
    assert_eq!(h.file_paths(), [path_text(&a)]);
    assert_eq!(h.folders.len(), 1);
    assert!(h.folder_paths()[0].ends_with(std::path::MAIN_SEPARATOR));
    // The next file opens in a new window, and goes in front.
    f.open(cx, &b, None);
    assert_eq!(history(cx).file_paths(), [path_text(&b), path_text(&a)]);
    assert_eq!(history(cx).folders.len(), 1, "the same folder");
}

#[gpui::test]
fn a_file_that_fails_to_open_is_not_remembered(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    // A folder is not a file.
    let (f, cx) = setup(cx);
    f.open(cx, &dir.0, None);
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert!(history(cx).files.is_empty());
}

#[gpui::test]
fn reopening_restores_the_caret_and_the_bookmarks(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", THREE_LINES);
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    f.go_to(cx, Pos::new(1, 4));
    f.mark(cx, 0);
    f.mark(cx, 2);
    // Closing the file remembers where the caret is.
    f.run(cx, "F_FILECLOSE", &[]);
    let entry = history(cx).file(&path_text(&path)).cloned().unwrap();
    assert_eq!((entry.caret_x, entry.caret_y), (4, 1));
    assert_eq!(entry.marks, ":001");
    f.open(cx, &path, None);
    assert_eq!(f.caret(cx), Pos::new(1, 4));
    assert_eq!(f.marks(cx), [0, 2]);
}

#[gpui::test]
fn the_caret_column_is_kept_in_utf16_units(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    // 𠮷 is one char, four bytes in UTF-8 and two UTF-16 units; 野 is three bytes and one unit.
    let path = dir.write("j.txt", "日本\n𠮷野家\n".as_bytes());
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(4));
    f.go_to(cx, Pos::new(1, 7)); // after 𠮷野
    f.run(cx, "F_FILECLOSE", &[]);
    let entry = history(cx).file(&path_text(&path)).cloned().unwrap();
    assert_eq!((entry.caret_x, entry.caret_y), (3, 1));
    f.open(cx, &path, Some(4));
    assert_eq!(f.caret(cx), Pos::new(1, 7));
}

#[gpui::test]
fn the_options_can_turn_the_restoring_off(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", THREE_LINES);
    let options = HistoryOptions {
        restore_cur_position: false,
        restore_bookmarks: false,
        query_if_code_change: true,
    };
    let (f, cx) = setup_with(cx, HistoryState::in_memory(options));
    f.open(cx, &path, None);
    f.go_to(cx, Pos::new(2, 1));
    f.mark(cx, 1);
    f.run(cx, "F_FILECLOSE", &[]);
    f.open(cx, &path, None);
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    assert!(f.marks(cx).is_empty());
}

#[gpui::test]
fn a_caret_past_the_end_of_the_file_goes_to_the_end(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"one\ntwo");
    let (f, cx) = setup(cx);
    let mut entry = FileEntry::new(path_text(&path));
    entry.caret_x = 2;
    entry.caret_y = 50;
    set_history(cx, |h| h.add_file(entry));
    f.open(cx, &path, None);
    assert_eq!(f.caret(cx), Pos::new(1, 3));
}

#[gpui::test]
fn a_column_past_the_end_of_the_line_goes_to_the_end_of_the_line(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"one\ntwo\n");
    let (f, cx) = setup(cx);
    let mut entry = FileEntry::new(path_text(&path));
    entry.caret_x = 40;
    entry.caret_y = 1;
    set_history(cx, |h| h.add_file(entry));
    f.open(cx, &path, None);
    assert_eq!(f.caret(cx), Pos::new(1, 3));
}

#[gpui::test]
fn a_bookmark_past_the_end_of_the_file_is_dropped(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"one\ntwo\n");
    let (f, cx) = setup(cx);
    let mut entry = FileEntry::new(path_text(&path));
    entry.marks = matsu_config::history::marks::encode_marks([0, 1, 7]);
    set_history(cx, |h| h.add_file(entry));
    f.open(cx, &path, None);
    assert_eq!(f.marks(cx), [0, 1]);
}

#[gpui::test]
fn the_encoding_of_last_time_can_be_taken_or_not(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    // UTF-8 text with Japanese in it: detected as UTF-8.
    let path = dir.write("u.txt", "日本語のテキストです\n".as_bytes());
    let (f, cx) = setup(cx);
    let mut entry = FileEntry::new(path_text(&path));
    entry.encoding_code = Encoding::ShiftJis.sakura_code();
    set_history(cx, |h| h.add_file(entry.clone()));
    // The detection and the history differ: the user is asked (`bQueryIfCodeChange`).
    f.open(cx, &path, None);
    assert!(cx.has_pending_prompt());
    answer(cx, "いいえ(N)");
    assert_eq!(f.encoding(cx), Encoding::Utf8);
    assert_eq!(f.text(cx), "日本語のテキストです");
    // Yes takes the encoding of last time.
    f.run(cx, "F_FILECLOSE", &[]);
    set_history(cx, |h| h.add_file(entry));
    f.open(cx, &path, None);
    answer(cx, "はい(Y)");
    assert_eq!(f.encoding(cx), Encoding::ShiftJis);
}

#[gpui::test]
fn without_the_question_the_encoding_of_last_time_wins(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("u.txt", "日本語のテキストです\n".as_bytes());
    let options = HistoryOptions {
        query_if_code_change: false,
        ..HistoryOptions::default()
    };
    let (f, cx) = setup_with(cx, HistoryState::in_memory(options));
    let mut entry = FileEntry::new(path_text(&path));
    entry.encoding_code = Encoding::ShiftJis.sakura_code();
    set_history(cx, |h| h.add_file(entry));
    f.open(cx, &path, None);
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.encoding(cx), Encoding::ShiftJis);
}

#[gpui::test]
fn an_encoding_given_by_the_user_is_not_questioned(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("u.txt", "日本語のテキストです\n".as_bytes());
    let (f, cx) = setup(cx);
    let mut entry = FileEntry::new(path_text(&path));
    entry.encoding_code = Encoding::ShiftJis.sakura_code();
    set_history(cx, |h| h.add_file(entry));
    f.open(cx, &path, Some(4));
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.encoding(cx), Encoding::Utf8);
}

#[gpui::test]
fn a_file_that_is_not_there_is_new_in_the_encoding_of_last_time(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.0.join("gone.txt");
    let (f, cx) = setup(cx);
    let mut entry = FileEntry::new(path_text(&path));
    entry.encoding_code = Encoding::Utf16Le.sakura_code();
    set_history(cx, |h| h.add_file(entry));
    f.open(cx, &path, None);
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.encoding(cx), Encoding::Utf16Le);
}

#[gpui::test]
fn the_encoding_is_remembered_with_the_file(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("u.txt", "日本語\n".as_bytes());
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(4));
    let entry = history(cx).file(&path_text(&path)).cloned().unwrap();
    assert_eq!(encoding_of(&entry), Some(Encoding::Utf8));
}

#[gpui::test]
fn reopen_keeps_the_caret_and_does_not_ask(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", THREE_LINES);
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    f.go_to(cx, Pos::new(2, 2));
    f.run(cx, "F_FILE_REOPEN_UTF8", &[Arg::Int(1)]);
    assert_eq!(f.caret(cx), Pos::new(2, 2));
    assert!(!cx.has_pending_prompt());
}

#[gpui::test]
fn the_recent_file_menu_opens_the_file(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let a = dir.write("a.txt", THREE_LINES);
    let b = dir.write("b.txt", b"bbb\n");
    let (f, cx) = setup(cx);
    let mut entry = FileEntry::new(path_text(&b));
    entry.encoding_code = Encoding::Utf8.sakura_code();
    set_history(cx, |h| {
        h.add_file(entry);
        h.add_file(FileEntry::new(path_text(&a)));
    });
    // The list is a, b; the untitled window takes b.
    cx.dispatch_action(OpenRecentFile {
        path: path_text(&b),
    });
    cx.run_until_parked();
    assert_eq!(f.text(cx), "bbb");
    assert_eq!(
        history(cx).file_paths(),
        [path_text(&b), path_text(&a)],
        "opening moves it to the front"
    );
    // The item opens the file it showed, whatever the history is now.
    set_history(cx, |h| h.files.clear());
    cx.dispatch_action(OpenRecentFile {
        path: path_text(&a),
    });
    cx.run_until_parked();
    assert_eq!(cx.windows().len(), 2, "a opened in a new window");
}

#[gpui::test]
fn the_list_is_recorded_when_the_application_quits(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let a = dir.write("a.txt", THREE_LINES);
    let b = dir.write("b.txt", THREE_LINES);
    let (f, cx) = setup(cx);
    f.open(cx, &a, None);
    f.open(cx, &b, None);
    // `b` is in a window that the application knows (the first window is the test's own).
    let (_, view) = cx
        .update(|_, app| crate::file_ops::windows::editors(app))
        .into_iter()
        .find(|(_, view)| view.entity_id() != f.view.entity_id())
        .expect("the second window");
    view.update(cx, |v, cx| v.go_to(Pos::new(1, 3), cx));
    // The entry is from the load: the caret was at the top.
    let entry = history(cx).file(&path_text(&b)).cloned().unwrap();
    assert_eq!((entry.caret_x, entry.caret_y), (0, 0));
    cx.update(|_, app| record_all(app));
    let entry = history(cx).file(&path_text(&b)).cloned().unwrap();
    assert_eq!((entry.caret_x, entry.caret_y), (3, 1));
}

#[gpui::test]
fn closing_the_window_records_the_file(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let a = dir.write("a.txt", THREE_LINES);
    let b = dir.write("b.txt", THREE_LINES);
    let (f, cx) = setup(cx);
    f.open(cx, &a, None);
    f.open(cx, &b, None);
    // `b` is in a window of its own.
    let (handle, view) = cx
        .update(|_, app| crate::file_ops::windows::editors(app))
        .into_iter()
        .find(|(_, view)| view.entity_id() != f.view.entity_id())
        .expect("the second window");
    view.update(cx, |v, cx| v.go_to(Pos::new(2, 1), cx));
    let code = code_of("F_WINCLOSE");
    handle
        .update(cx, |_, window, app| {
            view.update(app, |v, cx| v.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    let entry = history(cx).file(&path_text(&b)).cloned().unwrap();
    assert_eq!((entry.caret_x, entry.caret_y), (1, 2));
    assert_eq!(cx.windows().len(), 1, "the window is closed");
}

#[gpui::test]
fn an_untitled_window_leaves_no_trace(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    cx.simulate_input("text");
    f.run(cx, "F_FILECLOSE", &[]);
    answer(cx, "保存しない(N)");
    assert!(history(cx).files.is_empty());
    assert!(history(cx).folders.is_empty());
}

#[gpui::test]
fn without_a_history_nothing_is_remembered(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", THREE_LINES);
    cx.update(gpui_component::init);
    let shared = cx.new(|_| SharedDocument::empty());
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    let f = Fixture { view };
    f.open(vcx, &path, None);
    assert_eq!(f.text(vcx), "first\nsecond line\nthird");
    assert!(vcx.update(|_, app| current(app)).is_none());
}

#[gpui::test]
fn the_insert_functions_write_the_list_with_the_line_ending_of_the_document(
    cx: &mut TestAppContext,
) {
    let dir = TempDir::new();
    let doc = dir.write("crlf.txt", b"x\r\n");
    let (f, cx) = setup(cx);
    set_history(cx, |h| {
        h.add_file(FileEntry::new("C:\\Users\\山田\\メモ.txt"));
        h.add_file(FileEntry::new("\\\\server\\share\\日報.log"));
    });
    f.open(cx, &doc, None);
    f.go_to(cx, Pos::new(0, 0));
    f.run(cx, "F_INS_FILE_USED_RECENTLY", &[]);
    // The newest is first. The list shows `doc` too: it was opened last.
    let text = f.text(cx);
    let want = format!(
        "{}\n\\\\server\\share\\日報.log\nC:\\Users\\山田\\メモ.txt\nx",
        path_text(&doc)
    );
    assert_eq!(text, want);
    // The line ending is the one of the document: CRLF.
    let crlf = f
        .view
        .read_with(cx, |v, app| v.document().read(app).new_line());
    assert_eq!(crlf, matsu_core::buffer::Eol::CrLf);
}

#[gpui::test]
fn the_insert_functions_use_the_numbers_the_menu_shows(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    set_history(cx, |h| {
        for n in 0..20 {
            h.add_file(FileEntry::new(format!("/dir{n}/file.txt")));
        }
    });
    f.run(cx, "F_INS_FILE_USED_RECENTLY", &[]);
    // 15 are shown (the default), the 16th to the 20th are not inserted.
    assert_eq!(f.text(cx).lines().count(), 15);
    assert!(f.text(cx).starts_with("/dir19/file.txt\n/dir18/file.txt\n"));
    // The folders: the same 20, 15 of them.
    f.run(cx, "F_INS_FOLDER_USED_RECENTLY", &[]);
    let text = f.text(cx);
    assert!(text.contains("/dir19/\n/dir18/\n"));
}

#[gpui::test]
fn the_insert_functions_with_an_empty_history_insert_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    f.run(cx, "F_INS_FILE_USED_RECENTLY", &[]);
    f.run(cx, "F_INS_FOLDER_USED_RECENTLY", &[]);
    assert_eq!(f.text(cx), "");
    let modified = f
        .view
        .read_with(cx, |v, app| v.document().read(app).model().is_modified());
    assert!(!modified);
}

#[gpui::test]
fn the_insert_functions_can_be_undone_in_one_step(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    set_history(cx, |h| {
        h.add_file(FileEntry::new("/a/b.txt"));
        h.add_file(FileEntry::new("/a/c.txt"));
    });
    f.run(cx, "F_INS_FILE_USED_RECENTLY", &[]);
    assert!(f.text(cx).contains("/a/c.txt"));
    f.run(cx, "F_UNDO", &[]);
    assert_eq!(f.text(cx), "");
}

#[gpui::test]
fn the_insert_is_refused_in_view_mode(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let (f, cx) = setup(cx);
    f.open(cx, &path, None);
    f.run(cx, "F_VIEWMODE", &[]);
    f.run(cx, "F_INS_FILE_USED_RECENTLY", &[]);
    assert_eq!(f.text(cx), "abc");
}

#[gpui::test]
fn windows_that_open_files_at_once_all_get_in_the_file(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let ini = dir.0.join("conf").join("matsu.ini");
    std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
    std::fs::write(
        &ini,
        b"\xEF\xBB\xBF[Common]\nnMRUArrNum_MAX=5\n[KeyBind]\nx=1\n",
    )
    .unwrap();
    let a = dir.write("a.txt", THREE_LINES);
    let b = dir.write("b.txt", THREE_LINES);
    let c = dir.write("c.txt", THREE_LINES);
    let state = HistoryState::on_disk(
        HistoryStore::new(&ini, super::CASE_INSENSITIVE),
        HistoryOptions::default(),
    );
    let (f, cx) = setup_with(cx, state);
    // Three files, three windows (the first takes the untitled window).
    f.run(cx, "F_FILEOPEN", &[Arg::Text(a.display().to_string())]);
    f.run(cx, "F_FILEOPEN", &[Arg::Text(b.display().to_string())]);
    // Another process adds a file in between; the window that adds the next one keeps it.
    let other = HistoryStore::new(&ini, super::CASE_INSENSITIVE);
    other
        .update(|h| h.add_file(FileEntry::new("/other/process.txt")))
        .unwrap();
    f.run(cx, "F_FILEOPEN", &[Arg::Text(c.display().to_string())]);
    assert_eq!(cx.windows().len(), 3);
    let on_disk = HistoryStore::new(&ini, super::CASE_INSENSITIVE).load();
    let paths = on_disk.file_paths().join("|");
    for p in [&a, &b, &c] {
        assert!(paths.contains(&path_text(p)), "{p:?} in {paths}");
    }
    assert!(paths.contains("/other/process.txt"), "{paths}");
    // The rest of the file is as it was, and the shown number is the user's (5).
    let text = String::from_utf8(std::fs::read(&ini).unwrap()).unwrap();
    assert!(text.contains("[KeyBind]\nx=1\n"), "{text}");
    assert_eq!(on_disk.files.view_count(), 5);
}

#[gpui::test]
fn the_favorite_function_opens_the_management_window(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let before = cx.windows().len();
    f.run(cx, "F_FAVORITE", &[]);
    assert_eq!(cx.windows().len(), before + 1);
    f.run(cx, "F_FAVORITE", &[]);
    assert_eq!(
        cx.windows().len(),
        before + 1,
        "a second call finds the first"
    );
}

#[gpui::test]
fn the_options_are_read_from_the_settings(cx: &mut TestAppContext) {
    let _ = cx;
    let profile = matsu_config::Profile::from_bytes(
        b"[Common]\nm_bRestoreCurPosition=0\nm_bRestoreBookmarks=0\nbQueryIfCodeChange=0\n",
    );
    assert_eq!(
        HistoryOptions::from_profile(&profile),
        HistoryOptions {
            restore_cur_position: false,
            restore_bookmarks: false,
            query_if_code_change: false
        }
    );
    // Without the keys, Sakura's defaults: all on.
    assert_eq!(
        HistoryOptions::from_profile(&matsu_config::Profile::new()),
        HistoryOptions::default()
    );
}
