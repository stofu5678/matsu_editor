//! Tests of 「編集中のテキストから検索」 (`:HWND:`): the search of the text of a window, in the
//! engine without a window and in GPUI's headless test window with the dialog and the jump.
//!
//! Expectations cite `CDlgGrep.cpp` (`SetDataFromThisText`, `GetData`), `CDlgGrepReplace.cpp`
//! (`SetData`) and `CGrepAgent.cpp` (`GetHwndTitle`, `DoGrep`, `CFileLoadOrWnd`).

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::{Document, Pos};
use matsu_core::encoding::Encoding;
use matsu_core::model::DocumentModel;

use super::engine::{GrepError, GrepInfo, ReplaceInfo, Sink, WindowSource, check, run};
use super::enumerate::join_path;
use super::format::Layout;
use super::settings::GrepSettings;
use super::window::{THIS_DOCUMENT, parse_window_token, window_token};
use crate::document::SharedDocument;
use crate::file_ops::windows::editors;
use crate::search::state::SearchOptions;
use crate::view::EditorView;

// ---- the engine ----

struct Text(String);

impl Sink for Text {
    fn write(&mut self, text: &str) {
        self.0.push_str(text);
    }
}

fn source(text: &str, folder: &str, name: &str, title: &str) -> WindowSource {
    WindowSource {
        document: Document::from_utf8(text.as_bytes().to_vec()).unwrap(),
        encoding: Encoding::Utf8,
        folder: folder.to_owned(),
        name: name.to_owned(),
        title: title.to_owned(),
    }
}

fn info(key: &str, window: WindowSource) -> GrepInfo {
    GrepInfo {
        key: key.to_owned(),
        file: window_token(7),
        // the folder box is not used: a folder that is not there is no mistake
        folder: "/no/such/folder".to_owned(),
        options: SearchOptions::default(),
        sub_folder: true,
        charset: None,
        layout: Layout::default(),
        file_only: false,
        except_file_regex: false,
        replace: None,
        mime_decode: false,
        window: Some(window),
        window_gone: false,
    }
}

fn search(info: &GrepInfo) -> String {
    let mut sink = Text(String::new());
    run(info, &AtomicBool::new(false), &mut sink).expect("the search starts");
    sink.0
}

#[test]
fn a_window_with_a_file_is_searched_and_shown_as_that_file() {
    let w = source(
        "alpha\nbeta hit\nhit hit\n",
        "/proj",
        "memo.txt",
        "memo.txt",
    );
    let full = join_path("/proj", "memo.txt");
    let text = search(&info("hit", w));
    // `Window:[name]` in place of the patterns, and no folder
    assert!(
        text.contains("検索対象       Window:[memo.txt]\r\nフォルダー       \r\n"),
        "{text}"
    );
    assert!(
        text.ends_with(&format!(
            "{full}(2,6)  [UTF-8]: beta hit\r\n{full}(3,1)  [UTF-8]: hit hit\r\n2 個が検索されました。\r\n"
        )),
        "{text}"
    );
}

#[test]
fn a_window_without_a_file_is_named_by_its_tag() {
    // `szWindowName` is `:HWND:[id]title`; the path is empty
    let name = ":HWND:[0000000000000007](無題)1";
    let w = source("one\ntwo hit\n", "", name, "(無題)1");
    let text = search(&info("hit", w));
    assert!(text.contains("Window:[(無題)1]"), "{text}");
    assert!(
        text.ends_with(&format!(
            "{name}(2,5)  [UTF-8]: two hit\r\n1 個が検索されました。\r\n"
        )),
        "{text}"
    );
}

#[test]
fn the_lines_keep_their_breaks_for_the_match() {
    // CR LF, LF and no break at the end, as the lines of a window come with their breaks
    let full = join_path("/p", "x");
    let i = info("hit", source("a\r\nb hit\nc hit", "/p", "x", "x"));
    let text = search(&i);
    assert!(
        text.ends_with(&format!(
            "{full}(2,3)  [UTF-8]: b hit\r\n{full}(3,3)  [UTF-8]: c hit\r\n2 個が検索されました。\r\n"
        )),
        "{text}"
    );
}

#[test]
fn the_layout_options_apply_to_the_window_as_to_a_file() {
    let mut i = info(
        "hit",
        source("a hit\nb hit\n", "/proj", "memo.txt", "memo.txt"),
    );
    i.layout.style = super::format::OutputStyle::PerFile;
    let text = search(&i);
    let full = join_path("/proj", "memo.txt");
    assert!(
        text.ends_with(&format!(
            "■\"{full}\"  [UTF-8]\r\n・(     1,3     ): a hit\r\n・(     2,3     ): b hit\r\n2 個が検索されました。\r\n"
        )) || text.contains(&format!("■\"{full}\"")),
        "{text}"
    );
}

#[test]
fn a_chosen_encoding_is_not_written_in_front_of_a_hit() {
    let mut i = info("hit", source("x hit\n", "/proj", "memo.txt", "memo.txt"));
    i.charset = Some(Encoding::Utf8);
    let full = join_path("/proj", "memo.txt");
    let text = search(&i);
    assert!(
        text.contains(&format!("\r\n{full}(1,3): x hit\r\n")),
        "{text}"
    );
}

#[test]
fn an_empty_pattern_lists_the_window_as_the_one_file() {
    let i = info("", source("x\n", "/proj", "memo.txt", "memo.txt"));
    let full = join_path("/proj", "memo.txt");
    let text = search(&i);
    assert!(text.contains("「ファイル検索」"), "{text}");
    assert!(
        text.ends_with(&format!("{full}  [UTF-8]\r\n1 個が検索されました。\r\n")),
        "{text}"
    );
}

#[test]
fn the_search_of_a_window_stops_when_asked() {
    let i = info("hit", source("a hit\n", "/proj", "x", "x"));
    let mut sink = Text(String::new());
    let outcome = run(&i, &AtomicBool::new(true), &mut sink).unwrap();
    assert!(outcome.stopped);
}

#[test]
fn a_window_that_is_gone_ends_the_heading_with_the_error() {
    // `CGrepAgent.cpp`: after the condition (and the replacement) "HWND handle error.\n", then
    // `return 0`: no target, no folders, no count
    let mut i = info("hit", source("a hit\n", "/proj", "x", "x"));
    i.window = None;
    i.window_gone = true;
    assert_eq!(search(&i), "\r\n□検索条件  \"hit\"\r\nHWND handle error.\n");
    i.key.clear();
    assert_eq!(
        search(&i),
        "\r\n□検索条件  「ファイル検索」\r\nHWND handle error.\n"
    );
}

#[test]
fn a_replacement_of_a_window_is_refused() {
    // The dialog greys the check box out; the search of a window never replaces, and Sakura
    // would still say that it did.
    let mut i = info("hit", source("a hit\n", "/proj", "x", "x"));
    i.replace = Some(ReplaceInfo {
        text: "miss".to_owned(),
        paste: false,
        backup: false,
    });
    assert!(matches!(check(&i), Err(GrepError::WindowReplace)));
    let mut sink = Text(String::new());
    assert!(run(&i, &AtomicBool::new(false), &mut sink).is_err());
    assert!(sink.0.is_empty());
}

#[test]
fn the_name_of_a_window_in_the_file_box() {
    assert_eq!(window_token(0xab), ":HWND:00000000000000ab");
    assert_eq!(
        parse_window_token(":HWND:00000000000000ab"),
        Some(Some(0xab))
    );
    // what follows the number is not part of it
    assert_eq!(parse_window_token(":HWND:1f;!*.obj"), Some(Some(0x1f)));
    // `swscanf_s` finds no number
    assert_eq!(parse_window_token(":HWND:"), Some(None));
    assert_eq!(parse_window_token(":HWND:zz"), Some(None));
    assert_eq!(parse_window_token("*.txt"), None);
}

#[test]
fn the_dialog_checks_only_the_pattern_for_a_window() {
    let mut s = GrepSettings {
        key: "[".to_owned(),
        folder: "/no/such/folder".to_owned(),
        from_this_text: true,
        ..GrepSettings::default()
    };
    s.options.regex = true;
    assert!(s.check_from_this_text(3).unwrap_err().regex);
    s.options.regex = false;
    let info = s.check_from_this_text(3).unwrap();
    assert_eq!(info.file, ":HWND:0000000000000003");
    assert!(info.folder.is_empty());
    // the macro records the same
    let recorded = s.for_window(3).macro_args();
    assert!(matches!(&recorded[1], Arg::Text(t) if t == ":HWND:0000000000000003"));
    assert!(matches!(&recorded[2], Arg::Text(t) if t.is_empty()));
}

// ---- the windows ----

struct Fixture {
    view: Entity<EditorView>,
}

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
    path: Option<std::path::PathBuf>,
) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let shared = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), path));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.executor().allow_parking();
    (Fixture { view }, vcx)
}

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..50 {
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(60));
    }
}

fn text_of(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, app| {
        let doc = view.document().read(app).doc();
        (0..doc.line_count())
            .map(|i| {
                let line = doc.line(i).unwrap().unwrap();
                format!(
                    "{}{}",
                    line.text,
                    line.eol.map_or("", matsu_core::model::eol_text)
                )
            })
            .collect()
    })
}

fn others(cx: &mut VisualTestContext, not: &Entity<EditorView>) -> Vec<Entity<EditorView>> {
    cx.update(|_, app| editors(app))
        .into_iter()
        .map(|(_, view)| view)
        .filter(|view| view != not)
        .collect()
}

/// Runs a function in the window that has `view` (the result of a Grep is in another window
/// than the one the test was made in).
fn run_in_window(cx: &mut VisualTestContext, view: &Entity<EditorView>, function: &str) {
    let code = code_of(function);
    let handle = cx
        .update(|_, app| editors(app))
        .into_iter()
        .find(|(_, v)| v == view)
        .expect("a window")
        .0;
    let view = view.clone();
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    settle(cx);
}

impl Fixture {
    fn id(&self, cx: &mut VisualTestContext) -> u64 {
        self.view
            .read_with(cx, |v, _| v.document().entity_id().as_u64())
    }

    fn run(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
        let code = code_of(function);
        self.view.update_in(cx, |view, window, cx| {
            view.run_function(code, args, window, cx)
        });
        settle(cx);
    }

    /// The macro `Grep` over the text of this window.
    fn grep_this(&self, cx: &mut VisualTestContext, key: &str) {
        let settings = GrepSettings {
            key: key.to_owned(),
            ..GrepSettings::default()
        };
        let id = self.id(cx);
        let mut args = settings.for_window(id).macro_args();
        // the macro gets the folder box as it is: empty
        args[2] = Arg::Text(String::new());
        self.run(cx, "F_GREP", &args);
    }
}

#[gpui::test]
fn the_unsaved_text_of_an_untitled_window_is_searched(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", None);
    cx.simulate_input("typed but not saved: needle");
    assert!(f.view.read_with(cx, |v, app| {
        v.document().read(app).model().is_modified() && v.document().read(app).path().is_none()
    }));
    f.grep_this(cx, "needle");
    // the window has changes, so the result has its own window
    let result = others(cx, &f.view);
    assert_eq!(result.len(), 1);
    let text = text_of(&result[0], cx);
    let id = f.id(cx);
    assert!(text.contains("検索対象       Window:["), "{text}");
    assert!(
        text.contains(&format!(":HWND:[{id:016x}]")),
        "the hit is named by the window: {text}"
    );
    assert!(
        text.contains("(1,22)  [UTF-8]: typed but not saved: needle"),
        "{text}"
    );
    assert!(text.ends_with("1 個が検索されました。\r\n"), "{text}");
}

#[gpui::test]
fn the_changes_of_a_saved_file_are_searched_not_the_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memo.txt");
    std::fs::write(&path, "on disk: old word\n").unwrap();
    let (f, cx) = setup(cx, "on disk: old word\n", Some(path.clone()));
    cx.simulate_keystrokes("ctrl-end");
    cx.simulate_input("new word");
    f.grep_this(cx, "new word");
    let result = others(cx, &f.view);
    assert_eq!(result.len(), 1);
    let text = text_of(&result[0], cx);
    let shown = path.to_string_lossy().into_owned();
    assert!(text.contains("Window:[memo.txt]"), "{text}");
    assert!(
        text.contains(&format!("{shown}(2,1)  [UTF-8]: new word")),
        "{text}"
    );
    // the old text is still there to be found too, and the file on disk was not read
    f.grep_this(cx, "old word");
    let results = others(cx, &f.view);
    assert!(
        results
            .iter()
            .any(|view| text_of(view, cx).contains("(1,10)  [UTF-8]: on disk: old word"))
    );
}

#[gpui::test]
fn another_window_is_searched_by_its_number(cx: &mut TestAppContext) {
    // The window that runs the Grep is not the one that is searched.
    let (f, cx) = setup(cx, "", None);
    let doc = Document::from_utf8(b"in the other window: needle\n".to_vec()).unwrap();
    cx.update(|_, app| {
        crate::file_ops::windows::open_editor_window(
            app,
            SharedDocument::new(DocumentModel::new(doc), None),
            None,
        )
        .unwrap();
    });
    cx.run_until_parked();
    let other = others(cx, &f.view).remove(0);
    let id = other.read_with(cx, |v, _| v.document().entity_id().as_u64());
    let settings = GrepSettings {
        key: "needle".to_owned(),
        ..GrepSettings::default()
    };
    f.run(cx, "F_GREP", &settings.for_window(id).macro_args());
    // the macro writes the result to a window of its own
    let result: Vec<_> = others(cx, &f.view)
        .into_iter()
        .filter(|view| *view != other)
        .collect();
    assert_eq!(result.len(), 1);
    let text = text_of(&result[0], cx);
    assert!(text.contains(&format!(":HWND:[{id:016x}]")), "{text}");
    assert!(
        text.contains("(1,22)  [UTF-8]: in the other window: needle"),
        "{text}"
    );
    // the other window was not touched
    assert_eq!(text_of(&other, cx), "in the other window: needle\n");
}

#[gpui::test]
fn a_window_that_is_gone_is_told_in_the_result(cx: &mut TestAppContext) {
    // `GetHwndTitle` is -1: "HWND handle error." in the result, no box, no search
    let (f, cx) = setup(cx, "x\n", None);
    let args = vec![
        Arg::Text("x".to_owned()),
        Arg::Text(window_token(0xdead_beef)),
        Arg::Text(String::new()),
        Arg::Int(5),
    ];
    f.run(cx, "F_GREP", &args);
    assert!(!cx.has_pending_prompt());
    let result = others(cx, &f.view);
    assert_eq!(result.len(), 1);
    assert_eq!(
        text_of(&result[0], cx),
        "\r\n□検索条件  \"x\"\r\nHWND handle error.\n"
    );
    assert_eq!(text_of(&f.view, cx), "x\n");
}

#[gpui::test]
fn a_grep_replace_of_a_window_is_refused_in_a_box(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", None);
    cx.simulate_input("a needle");
    let id = f.id(cx);
    let args = vec![
        Arg::Text("needle".to_owned()),
        Arg::Text("pin".to_owned()),
        Arg::Text(window_token(id)),
        Arg::Text(String::new()),
        Arg::Int(5),
    ];
    f.run(cx, "F_GREP_REPLACE", &args);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert!(others(cx, &f.view).is_empty());
    // nothing was replaced
    assert_eq!(text_of(&f.view, cx), "a needle");
}

/// A window of the application (the tag jump finds the windows of the application only) with
/// `text`, as a document with `path` or without.
fn open_window(
    cx: &mut VisualTestContext,
    text: &str,
    path: Option<std::path::PathBuf>,
) -> Entity<EditorView> {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let before = cx.update(|_, app| editors(app).len());
    cx.update(|_, app| {
        crate::file_ops::windows::open_editor_window(
            app,
            SharedDocument::new(DocumentModel::new(doc), path),
            None,
        )
        .unwrap();
    });
    cx.run_until_parked();
    let all = cx.update(|_, app| editors(app));
    assert_eq!(all.len(), before + 1);
    all.last().unwrap().1.clone()
}

fn first_line_with(cx: &mut VisualTestContext, view: &Entity<EditorView>, needle: &str) -> usize {
    view.read_with(cx, |v, app| {
        let doc = v.document().read(app).doc();
        (0..doc.line_count())
            .find(|&i| doc.line(i).unwrap().unwrap().text.contains(needle))
            .expect("a hit")
    })
}

#[gpui::test]
fn jumping_from_the_result_goes_to_the_untitled_window(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", None);
    let target = open_window(cx, "first\nsecond needle\nthird", None);
    let id = target.read_with(cx, |v, _| v.document().entity_id().as_u64());
    let settings = GrepSettings {
        key: "needle".to_owned(),
        ..GrepSettings::default()
    };
    f.run(cx, "F_GREP", &settings.for_window(id).macro_args());
    let result = others(cx, &f.view)
        .into_iter()
        .find(|view| *view != target)
        .expect("the result");
    let at = first_line_with(cx, &result, ":HWND:[");
    result.update(cx, |v, cx| v.go_to(Pos::new(at, 0), cx));
    run_in_window(cx, &result, "F_TAGJUMP");
    let caret = target.read_with(cx, |v, _| v.cursor().caret());
    assert_eq!(
        caret,
        Pos::new(1, 7),
        "the caret is at the hit of the window"
    );
}

#[gpui::test]
fn jumping_from_the_result_goes_to_the_open_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memo.txt");
    std::fs::write(&path, "one\ntwo\n").unwrap();
    let (f, cx) = setup(cx, "", None);
    // the window has a line that the file does not have
    let target = open_window(cx, "one\ntwo\n\nthree needle", Some(path));
    let id = target.read_with(cx, |v, _| v.document().entity_id().as_u64());
    let settings = GrepSettings {
        key: "needle".to_owned(),
        ..GrepSettings::default()
    };
    f.run(cx, "F_GREP", &settings.for_window(id).macro_args());
    let result = others(cx, &f.view)
        .into_iter()
        .find(|view| *view != target)
        .expect("the result");
    let at = first_line_with(cx, &result, "memo.txt(");
    result.update(cx, |v, cx| v.go_to(Pos::new(at, 0), cx));
    run_in_window(cx, &result, "F_TAGJUMP");
    // the jump reaches the window that has the file open, at the line that only it has
    let caret = target.read_with(cx, |v, _| v.cursor().caret());
    assert_eq!(caret, Pos::new(3, 6));
    assert_eq!(
        others(cx, &f.view).len(),
        2,
        "no window was opened for the file"
    );
}

// ---- the dialog ----

#[gpui::test]
fn the_check_box_shows_the_document_and_puts_the_boxes_back(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "text\n", None);
    f.run(cx, "F_GREP_DIALOG", &[]);
    let panel = f.view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| {
        p.file.update(cx, |s, cx| s.set_value("*.md", window, cx));
        p.folder
            .update(cx, |s, cx| s.set_value("/some/folder", window, cx));
        p.set_from_this_text(true, window, cx);
    });
    let shown = panel.read_with(cx, |p, app| {
        [&p.file, &p.folder, &p.exclude_file, &p.exclude_folder]
            .map(|state| state.read(app).value().to_string())
    });
    assert_eq!(shown, [THIS_DOCUMENT, THIS_DOCUMENT, "", ""]);
    panel.update_in(cx, |p, window, cx| p.set_from_this_text(false, window, cx));
    let back = panel.read_with(cx, |p, app| {
        [&p.file, &p.folder].map(|state| state.read(app).value().to_string())
    });
    assert_eq!(back, ["*.md", "/some/folder"]);
}

#[gpui::test]
fn the_dialog_searches_the_text_of_the_window(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "", None);
    cx.simulate_input("a needle in the unsaved text");
    f.run(cx, "F_GREP_DIALOG", &[]);
    let panel = f.view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| {
        p.key.update(cx, |s, cx| s.set_value("needle", window, cx));
        p.set_from_this_text(true, window, cx);
        // a folder is not asked for
        p.ok(window, cx);
    });
    settle(cx);
    assert!(!f.view.read_with(cx, |v, _| v.grep_panel_open()));
    let result = others(cx, &f.view);
    assert_eq!(result.len(), 1);
    let text = text_of(&result[0], cx);
    assert!(
        text.contains("(1,3)  [UTF-8]: a needle in the unsaved text"),
        "{text}"
    );
    // the file and the folder were not used, so they are not kept
    // (only the `*.*` that the opening of the dialog put there)
    assert_eq!(
        cx.update(|_, app| crate::search::history::grep_files(app)),
        ["*.*"]
    );
    assert!(
        cx.update(|_, app| crate::search::history::grep_folders(app))
            .is_empty()
    );
    // the key is
    assert_eq!(
        cx.update(|_, app| crate::search::history::search_keys(app)),
        ["needle"]
    );
}

#[gpui::test]
fn the_replace_dialog_does_not_offer_the_check_box(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "text\n", None);
    f.run(cx, "F_GREP_REPLACE_DLG", &[]);
    let panel = f.view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    assert!(panel.read_with(cx, |p, _| p.replace_mode && !p.settings.from_this_text));
}

#[gpui::test]
fn the_check_box_starts_off_each_time(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "text\n", None);
    f.run(cx, "F_GREP_DIALOG", &[]);
    let panel = f.view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| p.set_from_this_text(true, window, cx));
    panel.update_in(cx, |p, window, cx| p.cancel(window, cx));
    f.run(cx, "F_GREP_DIALOG", &[]);
    let panel = f.view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    let (on, file) = panel.read_with(cx, |p, app| {
        (
            p.settings.from_this_text,
            p.file.read(app).value().to_string(),
        )
    });
    assert!(!on);
    assert_eq!(file, "*.*");
}
