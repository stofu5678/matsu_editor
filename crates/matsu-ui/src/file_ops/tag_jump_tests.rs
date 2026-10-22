//! Tests of the tag jump functions in GPUI's headless test window: the target is found from the
//! caret line in Sakura's Grep format, the file is opened (or its window comes to the front) at the
//! place, and `F_TAGJUMPBACK` returns. The rules of the formats are tested in `matsu_core::tag_jump`.

use std::path::{Path, PathBuf};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::Pos;

use super::windows::{LoadRequest, editors, open_editor_window};
use crate::document::SharedDocument;
use crate::view::EditorView;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

struct Win {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
}

impl Win {
    fn run(&self, cx: &mut TestAppContext, function: &str) {
        let code = code_of(function);
        let view = self.view.clone();
        self.handle
            .update(cx, |_, window, app| {
                view.update(app, |view, cx| view.run_function(code, &[], window, cx))
            })
            .unwrap();
        cx.run_until_parked();
    }

    fn caret(&self, cx: &mut TestAppContext) -> Pos {
        self.view.read_with(cx, |view, _| view.cursor().caret())
    }

    fn go_to(&self, cx: &mut TestAppContext, pos: Pos) {
        self.view.update(cx, |view, cx| view.go_to(pos, cx));
    }

    fn notice(&self, cx: &mut TestAppContext) -> Option<String> {
        self.view.read_with(cx, |view, _| view.notice.clone())
    }

    fn name(&self, cx: &mut TestAppContext) -> Option<String> {
        self.view
            .read_with(cx, |v, app| v.document().read(app).file_name())
    }

    fn cx<'a>(&self, cx: &'a mut TestAppContext) -> &'a mut VisualTestContext {
        VisualTestContext::from_window(self.handle, cx).into_mut()
    }
}

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

fn windows(cx: &mut TestAppContext) -> usize {
    cx.update(|app| editors(app).len())
}

/// The window that was opened last.
fn newest(cx: &mut TestAppContext) -> Win {
    let (handle, _) = cx.update(editors).pop().unwrap();
    window_of(cx, handle)
}

/// A folder with `target.txt` (a few lines, one with Japanese text and an emoji) and `output.txt`,
/// whose text is made from the path of `target.txt`.
fn project(output: impl Fn(&str) -> String) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.txt");
    std::fs::write(&target, "one\ntwo\n日本語😀のテキスト\nfour\n").unwrap();
    let out = dir.path().join("output.txt");
    std::fs::write(&out, output(target.to_str().unwrap())).unwrap();
    (dir, out)
}

#[gpui::test]
fn a_grep_line_opens_the_file_in_a_new_window_at_the_line_and_column(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("{t}(2,3): two\n"));
    let w = with_file(cx, &out);
    assert_eq!(windows(cx), 1);

    w.run(cx, "F_TAGJUMP");
    assert_eq!(windows(cx), 2);
    let opened = newest(cx);
    assert_eq!(opened.name(cx).as_deref(), Some("target.txt"));
    // column 3 of "two"
    assert_eq!(opened.caret(cx), Pos::new(1, 2));
    assert_eq!(w.notice(cx), None);
}

#[gpui::test]
fn the_column_counts_utf16_code_units(cx: &mut TestAppContext) {
    // "日本語😀のテキスト": the emoji takes two units, so column 6 (1-based) is the character
    // after it.
    let (_dir, out) = project(|t| format!("{t}(3,6): x\n"));
    let w = with_file(cx, &out);
    w.run(cx, "F_TAGJUMP");
    assert_eq!(newest(cx).caret(cx), Pos::new(2, "日本語😀".len()));
}

#[gpui::test]
fn a_line_without_a_position_leaves_the_caret_at_the_top(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("error in {t}\n"));
    let w = with_file(cx, &out);
    w.run(cx, "F_TAGJUMP");
    assert_eq!(newest(cx).caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn an_open_file_comes_to_the_front_with_the_caret_moved(cx: &mut TestAppContext) {
    let (dir, out) = project(|t| format!("{t}(4,2): four\n"));
    let target = with_file(cx, &dir.path().join("target.txt"));
    let w = with_file(cx, &out);
    assert_eq!(windows(cx), 2);

    w.run(cx, "F_TAGJUMP");
    assert_eq!(windows(cx), 2, "no third window");
    assert_eq!(target.caret(cx), Pos::new(3, 1));
}

#[gpui::test]
fn the_per_file_form_with_headings(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("■\"{t}\"\n・(     2,2    ): two\n"));
    let w = with_file(cx, &out);
    w.go_to(cx, Pos::new(1, 0));
    w.run(cx, "F_TAGJUMP");
    let opened = newest(cx);
    assert_eq!(opened.name(cx).as_deref(), Some("target.txt"));
    assert_eq!(opened.caret(cx), Pos::new(1, 1));
}

#[gpui::test]
fn a_relative_name_is_relative_to_the_folder_of_the_document(cx: &mut TestAppContext) {
    let (_dir, out) = project(|_| "target.txt(1,2): one\n".to_owned());
    let w = with_file(cx, &out);
    w.run(cx, "F_TAGJUMP");
    let opened = newest(cx);
    assert_eq!(opened.name(cx).as_deref(), Some("target.txt"));
    assert_eq!(opened.caret(cx), Pos::new(0, 1));
}

#[gpui::test]
fn a_line_that_names_nothing_says_so(cx: &mut TestAppContext) {
    let (_dir, out) = project(|_| "nothing here (1,2)\n".to_owned());
    let w = with_file(cx, &out);
    w.run(cx, "F_TAGJUMP");
    assert_eq!(windows(cx), 1);
    assert_eq!(w.notice(cx).as_deref(), Some("タグジャンプできません"));
}

#[gpui::test]
fn jumping_back_returns_to_the_window_and_the_place(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("first\n{t}(2,3): two\n"));
    let w = with_file(cx, &out);
    w.go_to(cx, Pos::new(1, 4));
    w.run(cx, "F_TAGJUMP");
    let opened = newest(cx);
    assert_eq!(opened.caret(cx), Pos::new(1, 2));

    // Back from the other window; the first window's caret was moved meanwhile.
    w.go_to(cx, Pos::new(0, 0));
    opened.run(cx, "F_TAGJUMPBACK");
    assert_eq!(w.caret(cx), Pos::new(1, 4));
    assert_eq!(windows(cx), 2);
    // The stack is empty now.
    opened.run(cx, "F_TAGJUMPBACK");
    assert_eq!(
        opened.notice(cx).as_deref(),
        Some("タグジャンプバックできません")
    );
}

#[gpui::test]
fn jumping_back_with_nothing_on_the_stack_says_so(cx: &mut TestAppContext) {
    let (_dir, out) = project(|_| String::new());
    let w = with_file(cx, &out);
    w.run(cx, "F_TAGJUMPBACK");
    assert_eq!(
        w.notice(cx).as_deref(),
        Some("タグジャンプバックできません")
    );
}

#[gpui::test]
fn the_newest_jump_is_undone_first(cx: &mut TestAppContext) {
    // Two jumps inside one window: both start from this window, at different carets.
    let (_dir, out) = project(|t| format!("{t}(1,1): a\n{t}(2,1): b\n"));
    let w = with_file(cx, &out);
    w.run(cx, "F_TAGJUMP");
    w.go_to(cx, Pos::new(1, 2));
    w.run(cx, "F_TAGJUMP");
    assert_eq!(windows(cx), 2, "the file is open after the first jump");

    w.go_to(cx, Pos::new(0, 0));
    w.run(cx, "F_TAGJUMPBACK");
    assert_eq!(w.caret(cx), Pos::new(1, 2));
    w.run(cx, "F_TAGJUMPBACK");
    assert_eq!(w.caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn the_header_source_jump_can_be_undone_with_jump_back(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("main.cpp");
    std::fs::write(&source, "int a;\nint b;\n").unwrap();
    std::fs::write(dir.path().join("main.h"), "h").unwrap();
    let w = with_file(cx, &source);
    w.go_to(cx, Pos::new(1, 4));
    w.run(cx, "F_OPEN_HfromtoC");
    let header = newest(cx);
    assert_eq!(header.name(cx).as_deref(), Some("main.h"));
    w.go_to(cx, Pos::new(0, 0));
    header.run(cx, "F_TAGJUMPBACK");
    assert_eq!(w.caret(cx), Pos::new(1, 4));
}

#[gpui::test]
fn jump_and_close_closes_the_window_it_came_from(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("{t}(2,1): two\n"));
    let w = with_file(cx, &out);
    w.run(cx, "F_TAGJUMP_CLOSE");
    cx.run_until_parked();
    assert_eq!(windows(cx), 1, "the output window is closed");
    let opened = newest(cx);
    assert_eq!(opened.name(cx).as_deref(), Some("target.txt"));
    assert_eq!(opened.caret(cx), Pos::new(1, 0));
}

#[gpui::test]
fn jump_and_close_asks_about_a_modified_document(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("{t}(2,1): two\nlast\n"));
    let w = with_file(cx, &out);
    // Change the second line, and jump from the first.
    w.go_to(cx, Pos::new(1, 0));
    w.cx(cx).simulate_input("x");
    w.go_to(cx, Pos::new(0, 0));
    w.run(cx, "F_TAGJUMP_CLOSE");
    cx.run_until_parked();
    // The jump was made; the window stays until the question is answered.
    assert_eq!(windows(cx), 2);
    assert!(w.cx(cx).has_pending_prompt());
    w.cx(cx).simulate_prompt_answer("キャンセル");
    cx.run_until_parked();
    assert_eq!(windows(cx), 2, "cancelled: the window stays");
}
