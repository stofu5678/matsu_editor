//! Tests of the DIFF and compare flows in GPUI's headless test windows: the dialogs, the marks of
//! small differences (Japanese text, CRLF, files in other encodings), the marks after edits, the
//! jumps, the reset, and the comparison of two windows. The difference itself is tested in
//! `matsu_core::diff`, and the rules of the jumps in `matsu_commands`.

use std::path::{Path, PathBuf};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::{Document, Pos};
use matsu_core::diff::DiffMark;
use matsu_core::model::DocumentModel;

use super::{Remembered, Target, common_part, name_score, remember};
use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window};
use crate::view::EditorView;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

struct Win {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
}

impl Win {
    /// This document as the other side of a comparison.
    fn doc(&self, cx: &mut TestAppContext) -> super::DocRef {
        super::DocRef {
            handle: self.handle,
            document: self.view.read_with(cx, |v, _| v.document().entity_id()),
        }
    }

    fn run(&self, cx: &mut TestAppContext, function: &str) -> bool {
        self.run_with(cx, function, &[])
    }

    fn run_with(&self, cx: &mut TestAppContext, function: &str, args: &[Arg]) -> bool {
        let code = code_of(function);
        let view = self.view.clone();
        let ok = self
            .handle
            .update(cx, |_, window, app| {
                view.update(app, |view, cx| view.run_function(code, args, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
        ok
    }

    fn cx<'a>(&self, cx: &'a mut TestAppContext) -> &'a mut VisualTestContext {
        VisualTestContext::from_window(self.handle, cx).into_mut()
    }

    fn marks(&self, cx: &mut TestAppContext) -> Vec<(usize, DiffMark)> {
        self.view.read_with(cx, |view, cx| {
            view.document()
                .read(cx)
                .model()
                .diff_marks()
                .iter()
                .collect()
        })
    }

    fn in_use(&self, cx: &mut TestAppContext) -> bool {
        self.view.read_with(cx, |view, cx| {
            view.document().read(cx).model().diff_marks().in_use()
        })
    }

    fn caret(&self, cx: &mut TestAppContext) -> Pos {
        self.view.read_with(cx, |view, _| view.cursor.caret())
    }

    fn notice(&self, cx: &mut TestAppContext) -> Option<String> {
        self.view.read_with(cx, |view, _| view.notice.clone())
    }

    /// The box this window shows, if any: its text. It is answered.
    fn take_box(&self, cx: &mut TestAppContext) -> Option<String> {
        let cx = self.cx(cx);
        let shown = cx.pending_prompt().map(|(_, detail)| detail);
        if shown.is_some() {
            cx.simulate_prompt_answer("OK");
        }
        shown
    }

    fn go_to(&self, cx: &mut TestAppContext, line: usize, col: usize) {
        self.view.update(cx, |view, cx| {
            view.run_with(cx, |c| c.move_caret(Pos::new(line, col), false))
                .unwrap();
        });
    }

    fn set_wrap(&self, cx: &mut TestAppContext, wrap: bool, notify: bool) {
        self.view.update(cx, |view, _| {
            let mut o = view.search.options();
            o.wrap = wrap;
            o.notify_not_found = notify;
            view.search.set_options(o);
        });
    }
}

/// A window with `text` and the name `path` (the file does not need to exist).
fn window(cx: &mut TestAppContext, text: &str, path: Option<&str>) -> Win {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let shared = SharedDocument::new(DocumentModel::new(doc), path.map(PathBuf::from));
    let handle = cx
        .update(|cx| open_editor_window(cx, shared, None))
        .unwrap();
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

fn remember_options(cx: &mut TestAppContext, flags: u32, tile: bool) {
    cx.update(|app| remember(app, Remembered { flags, tile }));
}

/// Chooses `other` in the DIFF dialog of `w` and presses OK.
fn diff_with_window(cx: &mut TestAppContext, w: &Win, other: &Win) {
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    let panel = w
        .view
        .read_with(cx, |v, _| v.diff.panel.clone())
        .expect("the dialog");
    let target = other.doc(cx);
    w.handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.select_window(target, cx);
                p.ok(window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("matsu-ui-diff-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn text_arg(path: &Path) -> Arg {
    Arg::Text(path.to_string_lossy().into_owned())
}

#[gpui::test]
fn the_dialog_marks_the_lines_that_differ_from_another_window(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "a\nb\nc\n", None);
    let other = window(cx, "a\nX\nc\nd\n", None);
    diff_with_window(cx, &w, &other);
    // line 1 changed; the line "d" of the other window is missing after the last line
    assert_eq!(
        w.marks(cx),
        [(1, DiffMark::Change), (2, DiffMark::DeleteEx)]
    );
    assert!(w.in_use(cx));
    // the dialog closed, the other window is untouched
    assert!(!w.view.read_with(cx, |v, _| v.diff_panel_open()));
    assert!(other.marks(cx).is_empty());
    assert_eq!(w.take_box(cx), None);
}

#[gpui::test]
fn a_window_is_compared_as_it_is_on_the_screen(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "日本語\r\nです\r\n", None);
    let other = window(cx, "日本語\r\nでした\r\n", None);
    // the other window has an edit that is not saved: it counts
    other.go_to(cx, 0, 0);
    other.view.update(cx, |view, cx| {
        view.run_with(cx, |c| c.insert_text("先頭\r\n")).unwrap();
    });
    diff_with_window(cx, &w, &other);
    // 先頭 is not in this window (a gap before line 0), and the second line differs
    assert_eq!(w.marks(cx), [(0, DiffMark::Delete), (1, DiffMark::Change)]);
}

#[gpui::test]
fn equal_texts_say_so(cx: &mut TestAppContext) {
    remember_options(cx, 0x40, false);
    let w = window(cx, "a\r\nb\r\n", None);
    let other = window(cx, "a\r\nb\r\n", None);
    diff_with_window(cx, &w, &other);
    assert!(w.marks(cx).is_empty());
    assert!(!w.in_use(cx));
    assert_eq!(
        w.take_box(cx).as_deref(),
        Some("DIFF差分は見つかりませんでした。")
    );
}

#[gpui::test]
fn the_options_of_the_dialog_reach_the_comparison(cx: &mut TestAppContext) {
    // 大文字小文字の同一視 and 空白無視
    remember_options(cx, 0x0043, false);
    let w = window(cx, "Hello World\nsame\n", None);
    let other = window(cx, "hello  world\nsame\n", None);
    diff_with_window(cx, &w, &other);
    assert!(w.marks(cx).is_empty());
    assert!(w.take_box(cx).is_some());
    // without them the first line differs
    remember_options(cx, 0, false);
    diff_with_window(cx, &w, &other);
    assert_eq!(w.marks(cx), [(0, DiffMark::Change)]);
}

#[gpui::test]
fn a_new_diff_replaces_the_marks_of_the_old_one(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "a\nb\nc\n", None);
    let first = window(cx, "X\nb\nc\n", None);
    let second = window(cx, "a\nb\nY\n", None);
    diff_with_window(cx, &w, &first);
    assert_eq!(w.marks(cx), [(0, DiffMark::Change)]);
    diff_with_window(cx, &w, &second);
    assert_eq!(w.marks(cx), [(2, DiffMark::Change)]);
}

#[gpui::test]
fn the_macro_form_compares_with_a_file(cx: &mut TestAppContext) {
    let dir = temp_dir("macro");
    let path = dir.join("other.txt");
    // CRLF and Japanese: only the second line differs
    std::fs::write(&path, "日本語\r\nちがう\r\nおわり\r\n").unwrap();
    let w = window(cx, "日本語\r\nそのまま\r\nおわり\r\n", None);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path), Arg::Int(0)]));
    assert_eq!(w.marks(cx), [(1, DiffMark::Change)]);
    // the flags after the file are optional, and 0x20 makes this window the old file
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path), Arg::Int(0x20)]));
    assert_eq!(w.marks(cx), [(1, DiffMark::Change)]);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path)]));
    assert_eq!(w.marks(cx), [(1, DiffMark::Change)]);
    // a file without line breaks at the end differs from one with them
    std::fs::write(&path, "日本語\r\nそのまま\r\nおわり").unwrap();
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path)]));
    assert_eq!(w.marks(cx), [(2, DiffMark::Change)]);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_file_in_another_encoding_is_decoded_as_when_it_is_opened(cx: &mut TestAppContext) {
    let dir = temp_dir("utf16");
    let path = dir.join("utf16.txt");
    // UTF-16 with a BOM
    let text = "日本語\nです\n";
    let mut bytes = vec![0xff, 0xfe];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&path, bytes).unwrap();
    let w = window(cx, text, None);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path), Arg::Int(0x40)]));
    assert!(w.marks(cx).is_empty());
    assert!(w.take_box(cx).is_some());
    let other = window(cx, "日本語\nでした\n", None);
    assert!(other.run_with(cx, "F_DIFF", &[text_arg(&path)]));
    assert_eq!(other.marks(cx), [(1, DiffMark::Change)]);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_file_that_is_not_there_is_reported(cx: &mut TestAppContext) {
    let w = window(cx, "a\n", None);
    let missing = Path::new("/nonexistent/matsu-diff-test-file.txt");
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(missing)]));
    assert_eq!(
        w.take_box(cx).as_deref(),
        Some("差分コマンド実行は失敗しました。\n\n比較するファイルが見つかりません。")
    );
    assert!(w.marks(cx).is_empty());
    // a folder is no file either
    let dir = temp_dir("folder");
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&dir)]));
    assert!(w.take_box(cx).is_some());
    std::fs::remove_dir_all(dir).ok();
    // the macro needs its file
    assert!(!w.run(cx, "F_DIFF"));
}

#[gpui::test]
fn a_binary_file_is_not_compared(cx: &mut TestAppContext) {
    let dir = temp_dir("binary");
    let path = dir.join("data.bin");
    std::fs::write(&path, b"a\0b\n").unwrap();
    let w = window(cx, "a\nb\n", None);
    // the old marks go before the check, as in Sakura
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&dir.join("missing"))]));
    w.take_box(cx);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path)]));
    assert_eq!(
        w.take_box(cx).as_deref(),
        Some("DIFF差分を行おうとしたファイルはバイナリファイルです。")
    );
    assert!(w.marks(cx).is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_marks_follow_the_edits_of_the_document(cx: &mut TestAppContext) {
    let dir = temp_dir("edits");
    let path = dir.join("other.txt");
    std::fs::write(&path, "a\nX\nc\nY\n").unwrap();
    let w = window(cx, "a\nb\nc\nd\n", None);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path)]));
    assert_eq!(w.marks(cx), [(1, DiffMark::Change), (3, DiffMark::Change)]);
    // a line comes in at the top: the marks move down with their lines
    w.go_to(cx, 0, 0);
    w.view.update(cx, |view, cx| {
        view.run_with(cx, |c| c.insert_text("new\n")).unwrap();
    });
    assert_eq!(w.marks(cx), [(2, DiffMark::Change), (4, DiffMark::Change)]);
    // the marked line is deleted: its mark goes with it
    w.go_to(cx, 2, 0);
    assert!(w.run(cx, "F_DELETE_LINE"));
    assert_eq!(w.marks(cx), [(3, DiffMark::Change)]);
    // undo brings the line back without the mark
    assert!(w.run(cx, "F_UNDO"));
    assert_eq!(w.marks(cx), [(4, DiffMark::Change)]);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_jumps_go_to_the_marked_lines(cx: &mut TestAppContext) {
    let dir = temp_dir("jumps");
    let path = dir.join("other.txt");
    std::fs::write(&path, "a\nX\nc\nd\nY\nf\n").unwrap();
    let w = window(cx, "a\nb\nc\nd\ne\nf\n", None);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path)]));
    assert_eq!(w.marks(cx), [(1, DiffMark::Change), (4, DiffMark::Change)]);
    w.set_wrap(cx, false, true);
    assert!(w.run(cx, "F_DIFF_NEXT"));
    assert_eq!(w.caret(cx), Pos::new(1, 0));
    assert!(w.run(cx, "F_DIFF_NEXT"));
    assert_eq!(w.caret(cx), Pos::new(4, 0));
    assert_eq!(w.notice(cx), None);
    assert!(w.run(cx, "F_DIFF_PREV"));
    assert_eq!(w.caret(cx), Pos::new(1, 0));
    // nothing before it: the status bar and the box say so
    assert!(w.run(cx, "F_DIFF_PREV"));
    assert_eq!(w.caret(cx), Pos::new(1, 0));
    assert_eq!(w.notice(cx).as_deref(), Some("△見つかりませんでした"));
    assert_eq!(
        w.take_box(cx).as_deref(),
        Some("後方(↑) に差分が見つかりません。")
    );
    // past the last one
    w.go_to(cx, 4, 0);
    assert!(w.run(cx, "F_DIFF_NEXT"));
    assert_eq!(w.notice(cx).as_deref(), Some("▽見つかりませんでした"));
    assert_eq!(
        w.take_box(cx).as_deref(),
        Some("前方(↓) に差分が見つかりません。")
    );
    // the option of the search dialogs: no box, or going on from the other end
    w.set_wrap(cx, true, false);
    assert!(w.run(cx, "F_DIFF_NEXT"));
    assert_eq!(w.caret(cx), Pos::new(1, 0));
    assert_eq!(w.notice(cx).as_deref(), Some("▼先頭から再検索しました"));
    assert_eq!(w.take_box(cx), None);
    assert!(w.run(cx, "F_DIFF_PREV"));
    assert_eq!(w.caret(cx), Pos::new(4, 0));
    assert_eq!(w.notice(cx).as_deref(), Some("▲末尾から再検索しました"));
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn reset_removes_the_marks(cx: &mut TestAppContext) {
    let dir = temp_dir("reset");
    let path = dir.join("other.txt");
    std::fs::write(&path, "a\nX\n").unwrap();
    let w = window(cx, "a\nb\n", None);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path)]));
    assert!(!w.marks(cx).is_empty());
    assert!(w.run(cx, "F_DIFF_RESET"));
    assert!(w.marks(cx).is_empty());
    assert!(!w.in_use(cx));
    // there is nothing to jump to now
    w.set_wrap(cx, true, true);
    assert!(w.run(cx, "F_DIFF_NEXT"));
    assert_eq!(
        w.take_box(cx).as_deref(),
        Some("前方(↓) に差分が見つかりません。")
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_dialog_can_be_cancelled_and_needs_a_choice(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "a\nb\n", None);
    let other = window(cx, "x\ny\n", None);
    // cancel
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    let panel = w.view.read_with(cx, |v, _| v.diff.panel.clone()).unwrap();
    w.handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.cancel(window, cx))
        })
        .unwrap();
    assert!(!w.view.read_with(cx, |v, _| v.diff_panel_open()));
    assert!(w.marks(cx).is_empty());
    // OK with the empty file box (no window chosen): closes and does nothing
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    let panel = w.view.read_with(cx, |v, _| v.diff.panel.clone()).unwrap();
    w.handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.set_file("", window, cx);
                p.ok(window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert!(!w.view.read_with(cx, |v, _| v.diff_panel_open()));
    assert!(w.marks(cx).is_empty() && !w.in_use(cx));
    let _ = other;
    assert_eq!(w.take_box(cx), None);
}

#[gpui::test]
fn the_dialog_chooses_a_file(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let dir = temp_dir("dialog-file");
    let path = dir.join("other.txt");
    std::fs::write(&path, "a\nB\n").unwrap();
    let w = window(cx, "a\nb\n", None);
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    let panel = w.view.read_with(cx, |v, _| v.diff.panel.clone()).unwrap();
    // without other windows the file box is the choice
    assert_eq!(
        panel.read_with(cx, |p, cx| p.target(cx)),
        None,
        "nothing typed yet"
    );
    w.handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.set_file(&path.to_string_lossy(), window, cx);
                assert_eq!(p.target(cx), Some(Target::File(path.clone())));
                p.ok(window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(w.marks(cx), [(1, DiffMark::Change)]);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_dialog_selects_the_window_with_the_most_alike_name(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "a\n", Some("/work/memo.txt"));
    let _first = window(cx, "a\n", Some("/work/zzz.rs"));
    let like = window(cx, "a\n", Some("/old/memo.md"));
    let _last = window(cx, "a\n", Some("/work/other.txt"));
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    let panel = w.view.read_with(cx, |v, _| v.diff.panel.clone()).unwrap();
    assert_eq!(
        panel.read_with(cx, |p, cx| p.target(cx)),
        Some(Target::Window(like.doc(cx)))
    );
}

#[test]
fn names_that_share_a_part_score_higher() {
    assert_eq!(common_part("memo", "MEMO"), 4);
    assert_eq!(common_part("memo", "my_memo2"), 4);
    assert_eq!(common_part("日本語", "本語"), 2);
    assert_eq!(common_part("abc", "xyz"), 0);
    assert_eq!(common_part("", "abc"), 0);
    let score = |a: &str, b: &str| name_score(Path::new(a), Path::new(b));
    assert_eq!(score("/a/memo.txt", "/b/memo.txt"), 4 + 4);
    assert_eq!(score("/a/memo.txt", "/b/memo.md"), 4 + 1);
    assert!(score("/a/memo.txt", "/b/note.rs") < score("/a/memo.txt", "/b/memo.md"));
}

#[gpui::test]
fn compare_moves_both_carets_to_the_first_difference(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "abc\n日本語\nend\n", None);
    let other = window(cx, "abc\n日本人\nend\n", None);
    assert!(w.run(cx, "F_COMPARE"));
    assert!(w.view.read_with(cx, |v, _| v.compare_panel_open()));
    let panel = w.view.read_with(cx, |v, _| v.diff.compare.clone()).unwrap();
    let target = other.doc(cx);
    w.handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.select_window(target, cx);
                p.ok(window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert!(!w.view.read_with(cx, |v, _| v.compare_panel_open()));
    // after 日本 (6 bytes) in line 1 of both
    assert_eq!(w.caret(cx), Pos::new(1, 6));
    assert_eq!(other.caret(cx), Pos::new(1, 6));
    // the window compared with says so
    assert_eq!(
        other.take_box(cx).as_deref(),
        Some("異なる箇所が見つかりました。")
    );
    assert_eq!(w.take_box(cx), None);
}

#[gpui::test]
fn compare_says_when_nothing_differs(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "a\r\nb\r\n", None);
    let other = window(cx, "a\r\nb\r\n", None);
    assert!(w.run(cx, "F_COMPARE"));
    let panel = w.view.read_with(cx, |v, _| v.diff.compare.clone()).unwrap();
    let target = other.doc(cx);
    w.handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.select_window(target, cx);
                p.ok(window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(w.caret(cx), Pos::new(0, 0));
    assert_eq!(
        other.take_box(cx).as_deref(),
        Some("異なる箇所は見つかりませんでした。")
    );
}

#[gpui::test]
fn compare_starts_at_the_carets_and_sees_line_endings(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "x\r\ny\r\nz\r\n", None);
    let other = window(cx, "q\r\ny\nz\r\n", None);
    // the first lines differ, but the carets are past them
    w.go_to(cx, 1, 0);
    other.go_to(cx, 1, 0);
    assert!(w.run(cx, "F_COMPARE"));
    let panel = w.view.read_with(cx, |v, _| v.diff.compare.clone()).unwrap();
    let target = other.doc(cx);
    w.handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.select_window(target, cx);
                p.ok(window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    // CRLF against LF: the difference is at the end of the text of line 1
    assert_eq!(w.caret(cx), Pos::new(1, 1));
    assert_eq!(other.caret(cx), Pos::new(1, 1));
    other.take_box(cx);
}

#[gpui::test]
fn compare_without_a_choice_does_nothing(cx: &mut TestAppContext) {
    let w = window(cx, "a\n", None);
    // no other window: the list is empty and OK closes the dialog
    assert!(w.run(cx, "F_COMPARE"));
    let panel = w.view.read_with(cx, |v, _| v.diff.compare.clone()).unwrap();
    w.handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.ok(window, cx))
        })
        .unwrap();
    assert!(!w.view.read_with(cx, |v, _| v.compare_panel_open()));
    assert_eq!(w.caret(cx), Pos::new(0, 0));
    assert_eq!(w.take_box(cx), None);
}

#[gpui::test]
fn the_dialogs_replace_each_other(cx: &mut TestAppContext) {
    let w = window(cx, "a\n", None);
    let _other = window(cx, "b\n", None);
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    assert!(w.view.read_with(cx, |v, _| v.diff_panel_open()));
    assert!(w.run(cx, "F_COMPARE"));
    assert!(
        w.view
            .read_with(cx, |v, _| v.compare_panel_open() && !v.diff_panel_open())
    );
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    assert!(
        w.view
            .read_with(cx, |v, _| v.diff_panel_open() && !v.compare_panel_open())
    );
}

#[test]
fn the_colors_of_the_marks_are_sakuras_defaults() {
    let colors = crate::settings::ViewSettings::default().colors;
    // sakura_core/doc/CDocTypeSetting.cpp: DIFF追加, DIFF変更, DIFF削除
    let want = [
        (colors.diff_append, 0x0000d2, 0xa2d0ff),
        (colors.diff_change, 0x006f00, 0xbdfdc0),
        (colors.diff_delete, 0xd56a00, 0xffe9ac),
    ];
    for (color, text, background) in want {
        assert_eq!((color.text, color.background), (text, background));
        assert!(color.shown && !color.bold);
    }
}

#[gpui::test]
fn a_reset_while_the_difference_is_computed_wins(cx: &mut TestAppContext) {
    let dir = temp_dir("race");
    let path = dir.join("other.txt");
    std::fs::write(&path, "a\nX\n").unwrap();
    let w = window(cx, "a\nb\n", None);
    // start the comparison and reset before the task is done
    let code = code_of("F_DIFF");
    let reset = code_of("F_DIFF_RESET");
    let args = [text_arg(&path)];
    let view = w.view.clone();
    w.handle
        .update(cx, |_, window, app| {
            view.update(app, |v, cx| {
                v.run_function(code, &args, window, cx);
                v.run_function(reset, &[], window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert!(w.marks(cx).is_empty() && !w.in_use(cx));
    // a newer comparison replaces an older one that is still running
    let other = dir.join("o2.txt");
    std::fs::write(&other, "Y\nb\n").unwrap();
    let args2 = [text_arg(&other)];
    w.handle
        .update(cx, |_, window, app| {
            view.update(app, |v, cx| {
                v.run_function(code, &args, window, cx);
                v.run_function(code, &args2, window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(w.marks(cx), [(0, DiffMark::Change)]);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_no_difference_message_follows_the_check_box(cx: &mut TestAppContext) {
    // box off: no message, and the old marks are cleared as before
    remember_options(cx, 0, false);
    let w = window(cx, "a\nb\n", None);
    let different = window(cx, "a\nX\n", None);
    let same = window(cx, "a\nb\n", None);
    diff_with_window(cx, &w, &different);
    assert_eq!(w.marks(cx), [(1, DiffMark::Change)]);
    diff_with_window(cx, &w, &same);
    assert!(w.marks(cx).is_empty() && !w.in_use(cx));
    assert_eq!(w.take_box(cx), None);
    // box on: the message
    remember_options(cx, 0x40, false);
    diff_with_window(cx, &w, &same);
    assert_eq!(
        w.take_box(cx).as_deref(),
        Some("DIFF差分は見つかりませんでした。")
    );
    // a difference needs no message either way
    diff_with_window(cx, &w, &different);
    assert_eq!(w.take_box(cx), None);
}

#[gpui::test]
fn the_macro_asks_for_the_message_with_the_bit_0x40(cx: &mut TestAppContext) {
    let dir = temp_dir("bit40");
    let path = dir.join("same.txt");
    std::fs::write(&path, "a\nb\n").unwrap();
    let w = window(cx, "a\nb\n", None);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path)]));
    assert_eq!(w.take_box(cx), None);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path), Arg::Int(0x01)]));
    assert_eq!(w.take_box(cx), None);
    assert!(w.run_with(cx, "F_DIFF", &[text_arg(&path), Arg::Int(0x41)]));
    assert_eq!(
        w.take_box(cx).as_deref(),
        Some("DIFF差分は見つかりませんでした。")
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_box_starts_checked_unless_the_setting_says_otherwise(cx: &mut TestAppContext) {
    // nothing remembered: on
    assert_eq!(cx.update(|app| super::remembered(app)).flags, 0x40);
    let w = window(cx, "a\n", None);
    let same = window(cx, "a\n", None);
    diff_with_window(cx, &w, &same);
    assert!(w.take_box(cx).is_some());
    // a value of 0 (a sakura.ini-style setting) is used as it is
    remember_options(cx, 0, true);
    diff_with_window(cx, &w, &same);
    assert_eq!(w.take_box(cx), None);
}
