use std::path::PathBuf;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::Document;
use matsu_core::encoding::Encoding;
use matsu_core::model::DocumentModel;

use crate::document::SharedDocument;
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
    fn run(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg]) -> bool {
        let code = code_of(function);
        let ok = self
            .view
            .update_in(cx, |v, window, cx| v.run_function(code, args, window, cx));
        cx.run_until_parked();
        ok
    }

    fn format(&self, cx: &mut VisualTestContext) -> (Encoding, bool) {
        self.document
            .read_with(cx, |s, _| (s.file().encoding, s.file().bom))
    }

    fn set_format(&self, cx: &mut VisualTestContext, encoding: Encoding, bom: bool) {
        self.document.update(cx, |s, _| s.set_format(encoding, bom));
    }

    fn dialog_open(&self, cx: &mut VisualTestContext) -> bool {
        self.view.read_with(cx, |v, _| v.save_as_dialog.is_some())
    }

    fn keys(&self, cx: &mut VisualTestContext, keys: &str) {
        cx.simulate_keystrokes(keys);
        cx.run_until_parked();
    }

    /// What a save under `name` in `dir` writes.
    fn saved(&self, cx: &mut VisualTestContext, dir: &tempfile::TempDir, name: &str) -> Vec<u8> {
        let path: PathBuf = dir.path().join(name);
        self.run(
            cx,
            "F_FILESAVEAS",
            &[Arg::Text(path.to_string_lossy().into_owned())],
        );
        std::fs::read(path).unwrap()
    }
}

#[gpui::test]
fn an_encoding_argument_sets_what_a_save_writes(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "日本語");
    assert!(f.run(cx, "F_CHG_CHARSET", &[Arg::Int(0)]));
    assert_eq!(f.format(cx), (Encoding::ShiftJis, false));
    assert_eq!(f.saved(cx, &dir, "a.txt"), b"\x93\xFA\x96\x7B\x8C\xEA");
}

#[gpui::test]
fn the_second_argument_is_the_bom_for_an_encoding_that_has_one(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "a");
    // UTF-8 with a BOM, and without
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(4), Arg::Int(1)]);
    assert_eq!(f.format(cx), (Encoding::Utf8, true));
    assert_eq!(f.saved(cx, &dir, "a.txt"), b"\xEF\xBB\xBFa");
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(4), Arg::Int(0)]);
    assert_eq!(f.format(cx), (Encoding::Utf8, false));
    assert_eq!(f.saved(cx, &dir, "b.txt"), b"a");
    // a missing BOM argument is off
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(3)]);
    assert_eq!(f.format(cx), (Encoding::Utf16Le, false));
    // UTF-16 with a BOM
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(3), Arg::Int(1)]);
    assert_eq!(f.saved(cx, &dir, "c.txt"), b"\xFF\xFEa\x00");
    // Shift_JIS has no BOM, whatever the argument says (`UseBom() & bBom`)
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(0), Arg::Int(1)]);
    assert_eq!(f.format(cx), (Encoding::ShiftJis, false));
}

#[gpui::test]
fn it_neither_modifies_the_document_nor_converts_the_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "日本語\r\nabc");
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(0)]);
    f.document.read_with(cx, |s, _| {
        assert!(!s.model().is_modified());
        assert!(!s.model().history().can_undo());
        assert_eq!(s.doc().line(0).unwrap().unwrap().text, "日本語");
    });
}

#[gpui::test]
fn the_status_bar_shows_the_new_encoding(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a");
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(3), Arg::Int(1)]);
    let charset = f.view.read_with(cx, |v, app| v.status(app).charset);
    assert_eq!(charset, "UTF-16 BOM付");
}

#[gpui::test]
fn a_number_that_is_not_an_encoding_is_a_mistake(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a");
    f.set_format(cx, Encoding::Utf8, true);
    assert!(!f.run(cx, "F_CHG_CHARSET", &[Arg::Int(12345)]));
    assert!(!f.run(cx, "F_CHG_CHARSET", &[Arg::Text("utf8".to_owned())]));
    assert_eq!(f.format(cx), (Encoding::Utf8, true));
    assert!(!f.dialog_open(cx));
}

#[gpui::test]
fn without_an_encoding_the_dialog_asks_and_starts_from_the_document(cx: &mut TestAppContext) {
    for args in [
        vec![],
        vec![Arg::Int(99)],
        vec![Arg::Int(-1)],
        vec![Arg::Text(String::new())],
    ] {
        let (f, cx) = setup(cx, "a");
        f.set_format(cx, Encoding::Utf8, true);
        f.run(cx, "F_CHG_CHARSET", &args);
        assert!(f.dialog_open(cx), "{args:?}");
        // OK keeps the encoding and the BOM the document has
        f.keys(cx, "enter");
        assert!(!f.dialog_open(cx));
        assert_eq!(f.format(cx), (Encoding::Utf8, true), "{args:?}");
    }
}

#[gpui::test]
fn the_dialog_chooses_the_encoding_and_the_bom(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "日本語");
    f.run(cx, "F_CHG_CHARSET", &[]);
    // Home is Shift_JIS, the first one, which has no BOM
    f.keys(cx, "home enter");
    assert_eq!(f.format(cx), (Encoding::ShiftJis, false));
    assert_eq!(f.saved(cx, &dir, "a.txt"), b"\x93\xFA\x96\x7B\x8C\xEA");
    // UTF-16 takes its default BOM: the order is SJIS, JIS, EUC, Latin1, UTF-16, UTF-16BE, UTF-8
    f.set_format(cx, Encoding::Utf8, false);
    f.run(cx, "F_CHG_CHARSET", &[]);
    f.keys(cx, "up up enter");
    assert_eq!(f.format(cx), (Encoding::Utf16Le, true));
    // Tab reaches the BOM box and Space turns it off
    f.run(cx, "F_CHG_CHARSET", &[]);
    f.keys(cx, "tab space enter");
    assert_eq!(f.format(cx), (Encoding::Utf16Le, false));
}

#[gpui::test]
fn escape_changes_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a");
    f.set_format(cx, Encoding::Utf8, true);
    f.run(cx, "F_CHG_CHARSET", &[]);
    f.keys(cx, "home escape");
    assert!(!f.dialog_open(cx));
    assert_eq!(f.format(cx), (Encoding::Utf8, true));
}

#[gpui::test]
fn the_dialog_has_no_line_ending_field(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a");
    f.set_format(cx, Encoding::Utf8, false);
    f.run(cx, "F_CHG_CHARSET", &[]);
    // Shift_JIS: the BOM box is disabled, so Tab goes list, OK, Cancel. With the line endings of
    // the Save As in between, the second Tab would stop at OK instead of Cancel.
    f.keys(cx, "home tab tab enter");
    assert!(!f.dialog_open(cx));
    assert_eq!(f.format(cx), (Encoding::Utf8, false), "Cancel was pressed");
}

// ---- a file on disk: the change is saved and asked about although the text is unmodified ----

/// A window with `bytes` read from a file in `dir` as UTF-8.
fn open_file<'a>(
    cx: &'a mut TestAppContext,
    dir: &tempfile::TempDir,
    bytes: &[u8],
) -> (Fixture, PathBuf, &'a mut VisualTestContext) {
    let (f, cx) = setup(cx, "");
    let path = dir.path().join("a.txt");
    std::fs::write(&path, bytes).unwrap();
    f.run(
        cx,
        "F_FILEOPEN",
        &[Arg::Text(path.to_string_lossy().into_owned()), Arg::Int(4)],
    );
    (f, path, cx)
}

#[gpui::test]
fn a_plain_save_writes_the_new_encoding_of_an_unmodified_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, path, cx) = open_file(cx, &dir, "日本語".as_bytes());
    // nothing changed: nothing is written
    f.run(cx, "F_FILESAVE", &[]);
    assert_eq!(std::fs::read(&path).unwrap(), "日本語".as_bytes());
    // `CDocFile::IsChgCodeSet`: the change makes the save write
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(0)]);
    assert!(f.document.read_with(cx, |s, _| s.format_changed()));
    f.run(cx, "F_FILESAVE", &[]);
    assert_eq!(std::fs::read(&path).unwrap(), b"\x93\xFA\x96\x7B\x8C\xEA");
    // and the file is as the document is now
    assert!(!f.document.read_with(cx, |s, _| s.format_changed()));
    f.run(cx, "F_FILESAVE", &[]);
    assert_eq!(std::fs::read(&path).unwrap(), b"\x93\xFA\x96\x7B\x8C\xEA");
}

#[gpui::test]
fn choosing_the_old_encoding_again_is_no_change(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, _path, cx) = open_file(cx, &dir, b"abc");
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(0)]);
    assert!(f.document.read_with(cx, |s, _| s.format_changed()));
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(4)]);
    assert!(!f.document.read_with(cx, |s, _| s.format_changed()));
    // a document without a file has nothing to change
    let (g, cx) = setup(cx, "abc");
    g.run(cx, "F_CHG_CHARSET", &[Arg::Int(0)]);
    assert!(!g.document.read_with(cx, |s, _| s.format_changed()));
}

#[gpui::test]
fn closing_asks_about_a_changed_encoding_and_cancel_gives_it_up(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, path, cx) = open_file(cx, &dir, "日本語".as_bytes());
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(0)]);
    // STR_CHANGE_CHARSET
    f.run(cx, "F_FILECLOSE", &[]);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("キャンセル");
    cx.run_until_parked();
    assert_eq!(f.format(cx), (Encoding::Utf8, false), "CancelChgCodeSet");
    // nothing to ask now
    f.run(cx, "F_FILECLOSE", &[]);
    assert!(!cx.has_pending_prompt());
    assert_eq!(std::fs::read(&path).unwrap(), "日本語".as_bytes());
}

#[gpui::test]
fn closing_can_save_the_changed_encoding(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, path, cx) = open_file(cx, &dir, "日本語".as_bytes());
    f.run(cx, "F_CHG_CHARSET", &[Arg::Int(0)]);
    f.run(cx, "F_FILECLOSE", &[]);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("保存する(Y)");
    cx.run_until_parked();
    assert_eq!(std::fs::read(&path).unwrap(), b"\x93\xFA\x96\x7B\x8C\xEA");
}
