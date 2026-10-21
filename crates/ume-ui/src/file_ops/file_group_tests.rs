//! Tests of the rest of Sakura's file functions in GPUI's headless test window: the quiet save
//! and save-all, view mode, the header/source pair, the property dialog, and the functions that
//! hand the file to the OS (only the cases that stop before the OS is called: GPUI's test platform has no file manager).

use std::path::{Path, PathBuf};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;

use super::windows::{LoadRequest, editors, open_editor_window};
use crate::document::SharedDocument;
use crate::view::EditorView;

const OK: &str = "OK";
const NEW_LINE: &str = "\n";

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// A window with its view.
struct Win {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
}

impl Win {
    fn run(&self, cx: &mut TestAppContext, function: &str) {
        self.run_with(cx, function, &[]);
    }

    fn run_with(&self, cx: &mut TestAppContext, function: &str, args: &[Arg]) {
        let code = code_of(function);
        let view = self.view.clone();
        self.handle
            .update(cx, |_, window, app| {
                view.update(app, |view, cx| view.run_function(code, args, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
    }

    fn cx<'a>(&self, cx: &'a mut TestAppContext) -> &'a mut VisualTestContext {
        VisualTestContext::from_window(self.handle, cx).into_mut()
    }

    /// Types text as the keyboard does.
    fn type_text(&self, cx: &mut TestAppContext, text: &str) {
        self.cx(cx).simulate_input(text);
    }

    fn text(&self, cx: &mut TestAppContext) -> String {
        self.view.read_with(cx, |view, app| {
            let doc = view.document().read(app).doc();
            (0..doc.line_count())
                .map(|i| doc.line(i).unwrap().unwrap().text)
                .collect::<Vec<_>>()
                .join(NEW_LINE)
        })
    }

    fn notice(&self, cx: &mut TestAppContext) -> Option<String> {
        self.view.read_with(cx, |view, _| view.notice.clone())
    }

    fn title(&self, cx: &mut TestAppContext) -> String {
        self.view.read_with(cx, |view, app| {
            crate::file_names::title::plain_title(view, app)
        })
    }

    fn modified(&self, cx: &mut TestAppContext) -> bool {
        self.view
            .read_with(cx, |v, app| v.document().read(app).model().is_modified())
    }

    fn view_mode(&self, cx: &mut TestAppContext) -> bool {
        self.view
            .read_with(cx, |v, app| v.document().read(app).view_mode())
    }

    fn pending_prompt(&self, cx: &mut TestAppContext) -> Option<(String, String)> {
        self.cx(cx).pending_prompt()
    }
}

/// An untitled window.
fn untitled(cx: &mut TestAppContext) -> Win {
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    window_of(cx, handle)
}

/// A window that has loaded `path`.
fn with_file(cx: &mut TestAppContext, path: &Path) -> Win {
    cx.update(gpui_component::init);
    let request = LoadRequest {
        path: path.to_owned(),
        encoding: None,
    };
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), Some(request)))
        .unwrap();
    cx.run_until_parked();
    window_of(cx, handle)
}

fn window_of(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Win {
    cx.run_until_parked();
    let view = cx.update(|app| {
        editors(app)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, view)| view)
            .unwrap()
    });
    Win { handle, view }
}

fn file(dir: &tempfile::TempDir, name: &str, text: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, text).unwrap();
    path
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

// ---- F_FILESAVE_QUIET and F_FILESAVEALL ----

#[gpui::test]
fn the_quiet_save_writes_a_modified_file_and_asks_nothing(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "a.txt", "abc");
    let w = with_file(cx, &path);
    w.type_text(cx, "x");
    assert!(w.modified(cx));
    w.run(cx, "F_FILESAVE_QUIET");
    assert_eq!(read(&path), "xabc");
    assert!(!w.modified(cx));
    assert!(w.pending_prompt(cx).is_none());
}

#[gpui::test]
fn the_quiet_save_leaves_an_unmodified_file_and_an_untitled_document_alone(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "a.txt", "abc");
    let w = with_file(cx, &path);
    // A change made behind the editor's back would be undone by a write.
    std::fs::write(&path, "changed").unwrap();
    w.run(cx, "F_FILESAVE_QUIET");
    assert_eq!(read(&path), "changed");

    // Untitled: no name dialog, nothing happens (`Command_FILESAVE(false, false)`).
    let u = untitled(cx);
    u.type_text(cx, "hello");
    u.run(cx, "F_FILESAVE_QUIET");
    assert!(u.modified(cx));
    assert!(!u.cx(cx).did_prompt_for_new_path());
    assert!(u.pending_prompt(cx).is_none());
}

#[gpui::test]
fn save_all_saves_every_window_that_has_a_file_and_changes(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let a = file(&dir, "a.txt", "a");
    let b = file(&dir, "b.txt", "b");
    let c = file(&dir, "c.txt", "c");
    let wa = with_file(cx, &a);
    let wb = with_file(cx, &b);
    let wc = with_file(cx, &c);
    let wu = untitled(cx);
    wa.type_text(cx, "1");
    wb.type_text(cx, "2");
    wu.type_text(cx, "untitled");
    // c is unmodified; change it on disk to see that it is not written
    std::fs::write(&c, "disk").unwrap();

    // Any window starts it; all of them are saved.
    wc.run(cx, "F_FILESAVEALL");
    assert_eq!(read(&a), "1a");
    assert_eq!(read(&b), "2b");
    assert_eq!(read(&c), "disk");
    assert!(!wa.modified(cx) && !wb.modified(cx));
    // The untitled window is untouched and nothing was asked.
    assert!(wu.modified(cx));
    assert_eq!(wu.text(cx), "untitled");
    assert!(!wu.cx(cx).did_prompt_for_new_path());
    assert!(wu.pending_prompt(cx).is_none());
}

#[gpui::test]
fn save_all_in_a_single_untitled_window_does_nothing(cx: &mut TestAppContext) {
    let w = untitled(cx);
    w.type_text(cx, "x");
    w.run(cx, "F_FILESAVEALL");
    assert!(w.modified(cx));
    assert!(!w.cx(cx).did_prompt_for_new_path());
}

#[gpui::test]
fn save_all_does_not_overwrite_the_file_of_a_view_mode_window(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let a = file(&dir, "a.txt", "a");
    let b = file(&dir, "b.txt", "b");
    let wa = with_file(cx, &a);
    let wb = with_file(cx, &b);
    wa.type_text(cx, "1");
    wb.type_text(cx, "2");
    wb.run(cx, "F_VIEWMODE");
    assert!(wb.view_mode(cx));

    wa.run(cx, "F_FILESAVEALL");
    assert_eq!(read(&a), "1a");
    // Sakura refuses with a message in that window (`CSaveAgent::OnCheckSave`).
    assert_eq!(read(&b), "b");
    let (message, _) = wb.pending_prompt(cx).expect("the refusal is shown");
    assert!(
        message.contains("ビューモードでは同一ファイルへの保存はできません"),
        "{message}"
    );
    wb.cx(cx).simulate_prompt_answer(OK);
    cx.run_until_parked();
    assert!(wb.modified(cx));
}

// ---- the check before saving (`CSaveAgent::OnCheckSave`) ----

#[gpui::test]
fn save_as_refuses_a_file_another_window_has_open(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let a = file(&dir, "a.txt", "a");
    let b = file(&dir, "b.txt", "b");
    let _wa = with_file(cx, &a);
    let wb = with_file(cx, &b);
    wb.type_text(cx, "x");
    wb.run_with(cx, "F_FILESAVEAS", &[Arg::Text(a.display().to_string())]);
    let (message, _) = wb.pending_prompt(cx).expect("the refusal is shown");
    assert!(message.contains("他のウィンドウで使用中です"), "{message}");
    assert!(message.contains(&a.display().to_string()), "{message}");
    wb.cx(cx).simulate_prompt_answer(OK);
    cx.run_until_parked();
    assert_eq!(read(&a), "a");
    assert_eq!(read(&b), "b");
    assert!(wb.modified(cx));
}

/// Another program that holds the lock of the file (here a lock of the same kind, as another
/// UME Editor would take) stops the save: Windows by the share mode, the others by the `flock`
/// that `check_writable` tries.
#[gpui::test]
fn save_is_refused_while_another_program_holds_the_file(cx: &mut TestAppContext) {
    use ume_platform::file_lock::{FileLock, ShareMode};
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "a.txt", "abc");
    let w = with_file(cx, &path);
    w.type_text(cx, "x");
    let other = FileLock::acquire(&path, ShareMode::DenyWrite).unwrap();

    w.run(cx, "F_FILESAVE");
    let (message, _) = w.pending_prompt(cx).expect("the refusal is shown");
    assert!(
        message.contains("他のアプリケーションで使用されている可能性があります"),
        "{message}"
    );
    w.cx(cx).simulate_prompt_answer(OK);
    cx.run_until_parked();
    assert_eq!(read(&path), "abc");
    assert!(w.modified(cx));

    // Once the other lock is gone, the same save goes through.
    drop(other);
    w.run(cx, "F_FILESAVE");
    assert_eq!(read(&path), "xabc");
    assert!(!w.modified(cx));
}

// ---- F_VIEWMODE ----

#[gpui::test]
fn view_mode_toggles_and_the_title_shows_it(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let w = with_file(cx, &file(&dir, "a.txt", "abc"));
    assert_eq!(w.title(cx), "a.txt - UME Editor");
    w.run(cx, "F_VIEWMODE");
    assert!(w.view_mode(cx));
    assert_eq!(w.title(cx), "a.txt - UME Editor (ビューモード)");
    w.run(cx, "F_VIEWMODE");
    assert!(!w.view_mode(cx));
    assert_eq!(w.title(cx), "a.txt - UME Editor");
}

#[gpui::test]
fn view_mode_refuses_typing_deleting_pasting_and_undo(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let w = with_file(cx, &file(&dir, "a.txt", "abc\ndef"));
    w.type_text(cx, "x"); // an undo step to be refused later
    assert_eq!(w.text(cx), "xabc\ndef");
    w.run(cx, "F_VIEWMODE");

    w.type_text(cx, "y");
    w.type_text(cx, "あ");
    for function in [
        "F_DELETE",
        "F_DELETE_BACK",
        "F_DELETE_LINE",
        "F_UNDO",
        "F_REDO",
        "F_CUT",
        "F_PASTE",
        "F_TOUPPER",
        "F_CHGMOD_INS",
    ] {
        w.run(cx, function);
        assert_eq!(w.text(cx), "xabc\ndef", "{function}");
    }
    // Enter and Tab go through the same door.
    w.cx(cx).simulate_keystrokes("enter tab backspace");
    assert_eq!(w.text(cx), "xabc\ndef");
    assert!(w.modified(cx), "still the change from before");

    // Off again: everything works, the undo history was kept.
    w.run(cx, "F_VIEWMODE");
    w.type_text(cx, "y");
    assert_eq!(w.text(cx), "xyabc\ndef");
    w.run(cx, "F_UNDO");
    assert_eq!(w.text(cx), "xabc\ndef");
}

#[gpui::test]
fn view_mode_allows_moving_selecting_copying_and_searching(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let w = with_file(cx, &file(&dir, "a.txt", "foo bar\nbaz foo"));
    w.run(cx, "F_VIEWMODE");
    let caret = |cx: &mut TestAppContext| w.view.read_with(cx, |v, _| v.cursor().caret());

    w.cx(cx).simulate_keystrokes("down right");
    assert_eq!((caret(cx).line, caret(cx).col), (1, 1));
    w.run(cx, "F_GOFILETOP");
    assert_eq!((caret(cx).line, caret(cx).col), (0, 0));
    w.run(cx, "F_SELECTALL");
    assert!(w.view.read_with(cx, |v, _| v.cursor().has_selection()));
    w.run(cx, "F_COPY");
    assert!(w.notice(cx).is_none_or(|n| !n.contains("not available")));

    w.view.update(cx, |v, _| v.search.set_pattern("foo"));
    w.run(cx, "F_GOFILETOP");
    w.run(cx, "F_SEARCH_NEXT");
    let selection = w.view.read_with(cx, |v, _| v.cursor().selection());
    assert!(selection.is_some(), "the search selected a match");
    assert_eq!(w.text(cx), "foo bar\nbaz foo");
    assert!(!w.modified(cx));
}

#[gpui::test]
fn view_mode_refuses_replace_all(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let w = with_file(cx, &file(&dir, "a.txt", "foo foo"));
    w.run(cx, "F_VIEWMODE");
    w.view.update(cx, |v, _| {
        v.search.set_pattern("foo");
        v.search.replace.text = "bar".to_owned();
    });
    w.run(cx, "F_REPLACE_ALL");
    assert_eq!(w.text(cx), "foo foo");
    assert!(!w.modified(cx));
}

#[gpui::test]
fn view_mode_does_not_overwrite_the_file_but_saves_under_another_name(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "a.txt", "abc");
    let w = with_file(cx, &path);
    w.type_text(cx, "x");
    w.run(cx, "F_VIEWMODE");

    w.run(cx, "F_FILESAVE");
    let (message, _) = w.pending_prompt(cx).expect("refused with a message");
    assert!(message.contains("ビューモード"), "{message}");
    w.cx(cx).simulate_prompt_answer(OK);
    cx.run_until_parked();
    assert_eq!(read(&path), "abc");

    // Save As to another file works, and the mode stays.
    let other = dir.path().join("b.txt");
    w.run_with(
        cx,
        "F_FILESAVEAS",
        &[Arg::Text(other.display().to_string())],
    );
    assert_eq!(read(&other), "xabc");
    assert!(w.view_mode(cx));
}

#[gpui::test]
fn reopening_keeps_view_mode_and_a_new_document_clears_it(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "a.txt", "abc");
    let w = with_file(cx, &path);
    w.run(cx, "F_VIEWMODE");
    w.run_with(cx, "F_FILE_REOPEN_UTF8", &[]);
    assert!(w.view_mode(cx), "reopen keeps the mode");
    w.run(cx, "F_FILECLOSE");
    assert!(!w.view_mode(cx), "a new document starts without it");
    assert_eq!(w.title(cx), "(無題)1 - UME Editor");
}

// ---- F_OPEN_HfromtoC ----

#[gpui::test]
fn the_header_opens_from_the_source_in_a_new_window_with_the_same_encoding(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("main.cpp");
    // Shift_JIS: 日本語
    std::fs::write(&source, b"\x93\xFA\x96\x7B\x8C\xEA").unwrap();
    std::fs::write(dir.path().join("main.hpp"), b"\x93\xFA\x96\x7B\x8C\xEA hpp").unwrap();
    std::fs::write(dir.path().join("main.hxx"), "hxx").unwrap();
    let w = with_file(cx, &source);
    assert_eq!(cx.update(|app| editors(app).len()), 1);

    w.run(cx, "F_OPEN_HfromtoC");
    cx.run_until_parked();
    let all = cx.update(editors);
    assert_eq!(all.len(), 2, "a new window");
    let (_, opened) = &all[1];
    let (name, text) = opened.read_with(cx, |v, app| {
        let shared = v.document().read(app);
        (
            shared.file_name(),
            shared.doc().line(0).unwrap().unwrap().text,
        )
    });
    assert_eq!(name.as_deref(), Some("main.hpp"), "hpp comes before hxx");
    assert!(
        text.starts_with("日本語"),
        "the encoding of the source: {text}"
    );

    // Again: the file is open, so no third window.
    w.run(cx, "F_OPEN_HfromtoC");
    cx.run_until_parked();
    assert_eq!(cx.update(|app| editors(app).len()), 2);
}

#[gpui::test]
fn the_source_opens_from_the_header(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let header = file(&dir, "a.h", "h");
    file(&dir, "a.c", "c");
    let w = with_file(cx, &header);
    w.run(cx, "F_OPEN_HfromtoC");
    cx.run_until_parked();
    let all = cx.update(editors);
    assert_eq!(all.len(), 2);
    let name = all[1]
        .1
        .read_with(cx, |v, app| v.document().read(app).file_name());
    assert_eq!(name.as_deref(), Some("a.c"));
}

#[gpui::test]
fn without_a_partner_a_wrong_type_or_a_file_nothing_opens(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let alone = file(&dir, "alone.cpp", "x");
    let other = file(&dir, "notes.txt", "x");
    file(&dir, "notes.h", "x");
    let w = with_file(cx, &alone);
    w.run(cx, "F_OPEN_HfromtoC");
    assert_eq!(cx.update(|app| editors(app).len()), 1);
    assert!(w.notice(cx).unwrap().contains("ファイルがない"));

    let t = with_file(cx, &other);
    t.run(cx, "F_OPEN_HfromtoC");
    assert_eq!(cx.update(|app| editors(app).len()), 2, "a .txt has no pair");

    let u = untitled(cx);
    u.run(cx, "F_OPEN_HfromtoC");
    assert_eq!(cx.update(|app| editors(app).len()), 3);
    assert!(u.notice(cx).is_some());
}

// ---- F_PROPERTY_FILE ----

#[gpui::test]
fn the_property_dialog_shows_the_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "p.txt", "あい\nう");
    let w = with_file(cx, &path);
    w.run(cx, "F_PROPERTY_FILE");
    let (title, detail) = w.pending_prompt(cx).expect("the dialog is shown");
    assert_eq!(title, "ファイルのプロパティ");
    assert!(
        detail.contains(&format!("ファイル名  {}", path.display())),
        "{detail}"
    );
    assert!(detail.contains("文字コード  UTF-8"), "{detail}");
    assert!(detail.contains("行数  2行"), "{detail}");
    assert!(detail.contains("変更されていません。"), "{detail}");
    assert!(detail.contains("ファイルサイズ  10 バイト"), "{detail}");
    assert!(!detail.contains("ビューモード"), "{detail}");
    w.cx(cx).simulate_prompt_answer(OK);
    cx.run_until_parked();

    w.run(cx, "F_VIEWMODE");
    w.run(cx, "F_PROPERTY_FILE");
    let (_, detail) = w.pending_prompt(cx).unwrap();
    assert!(detail.contains("ビューモードで開いています。"), "{detail}");
    w.cx(cx).simulate_prompt_answer(OK);
}

#[gpui::test]
fn the_property_dialog_of_an_untitled_document_has_no_file_information(cx: &mut TestAppContext) {
    let w = untitled(cx);
    w.type_text(cx, "x");
    w.run(cx, "F_PROPERTY_FILE");
    let (_, detail) = w.pending_prompt(cx).unwrap();
    assert!(detail.contains("変更されています。"), "{detail}");
    assert!(!detail.contains("ファイル情報"), "{detail}");
    w.cx(cx).simulate_prompt_answer(OK);
}

// ---- the functions that hand the file to the OS ----

#[gpui::test]
fn a_document_without_a_file_cannot_be_handed_to_the_os(cx: &mut TestAppContext) {
    let w = untitled(cx);
    for function in [
        "F_BROWSE",
        "F_OPEN_FOLDER_IN_EXPLORER",
        "F_OPEN_COMMAND_PROMPT",
        "F_OPEN_COMMAND_PROMPT_AS_ADMIN",
        "F_OPEN_POWERSHELL",
        "F_OPEN_POWERSHELL_AS_ADMIN",
    ] {
        w.view.update(cx, |v, _| v.notice = None);
        w.run(cx, function);
        let notice = w.notice(cx);
        assert!(
            notice
                .as_deref()
                .is_some_and(|n| n.contains("ファイルがない")),
            "{function}: {notice:?}"
        );
    }
}

#[cfg(not(windows))]
#[gpui::test]
fn only_windows_opens_a_shell_as_administrator(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let w = with_file(cx, &file(&dir, "a.txt", "x"));
    for function in [
        "F_OPEN_COMMAND_PROMPT_AS_ADMIN",
        "F_OPEN_POWERSHELL_AS_ADMIN",
    ] {
        w.view.update(cx, |v, _| v.notice = None);
        w.run(cx, function);
        let notice = w.notice(cx).unwrap_or_default();
        assert!(notice.contains("使えない"), "{function}: {notice}");
    }
}
