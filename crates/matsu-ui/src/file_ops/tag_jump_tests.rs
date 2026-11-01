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

#[test]
fn a_jump_from_the_output_opens_a_regular_file_and_not_through_a_link() {
    use super::tag_jump::is_plain_local_file;

    let dir = tempfile::tempdir().unwrap();
    // the folder for temporary files is itself behind a link on macOS (`/var` is `/private/var`)
    let root = std::fs::canonicalize(dir.path()).unwrap();
    let file = root.join("a.sql");
    std::fs::write(&file, "select 1;\n").unwrap();
    assert!(is_plain_local_file(&file, true));
    assert!(is_plain_local_file(&file, false));
    // a folder and a file that is not there are not files
    assert!(!is_plain_local_file(&root, true));
    assert!(!is_plain_local_file(&root.join("none.sql"), true));
    #[cfg(unix)]
    {
        // a link to a file, and a file in a linked folder: opened without the rule (`false`, the
        // systems where a link cannot lead to a network path), refused with it (Windows)
        let link = root.join("link.sql");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        let folder_link = root.join("folder");
        std::os::unix::fs::symlink(&root, &folder_link).unwrap();
        assert!(is_plain_local_file(&link, false));
        assert!(!is_plain_local_file(&link, true));
        assert!(is_plain_local_file(&folder_link.join("a.sql"), false));
        assert!(!is_plain_local_file(&folder_link.join("a.sql"), true));
        // a pipe is not a regular file
        let pipe = root.join("pipe");
        let made = std::process::Command::new("mkfifo").arg(&pipe).status();
        if made.is_ok_and(|s| s.success()) {
            assert!(!is_plain_local_file(&pipe, false));
        }
    }
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

// ---- Enter and the double click in the result of a Grep ----

/// Makes the window a Grep window, as a finished search leaves it.
fn make_grep_window(w: &Win, cx: &mut TestAppContext) {
    w.view.update(cx, |view, cx| {
        view.pretend_grep_running(cx);
        view.stop_pretending_grep(cx);
    });
}

fn set_grep_jumps(cx: &mut TestAppContext, enter: bool, double_click: bool) {
    cx.update(|app| {
        app.set_global(crate::search::prefs::SearchPrefs {
            grep_enter_jumps: enter,
            grep_double_click_jumps: double_click,
            ..Default::default()
        })
    });
}

#[gpui::test]
fn enter_in_the_result_of_a_grep_is_a_tag_jump(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("{t}(2,3): two\n"));
    let w = with_file(cx, &out);
    make_grep_window(&w, cx);
    set_grep_jumps(cx, true, true);
    w.cx(cx).simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(windows(cx), 2);
    assert_eq!(newest(cx).caret(cx), Pos::new(1, 2));
    // no line break was typed in the result
    let lines = w
        .view
        .read_with(cx, |v, app| v.document().read(app).doc().line_count());
    assert_eq!(lines, 1, "no line break was typed");
}

#[gpui::test]
fn enter_types_a_line_break_when_the_option_is_off(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("{t}(2,3): two\n"));
    let w = with_file(cx, &out);
    make_grep_window(&w, cx);
    set_grep_jumps(cx, false, true);
    w.cx(cx).simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(windows(cx), 1);
    let lines = w
        .view
        .read_with(cx, |v, app| v.document().read(app).doc().line_count());
    assert_eq!(lines, 2);
}

#[gpui::test]
fn enter_in_a_window_that_is_not_a_grep_result_types_a_line_break(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("{t}(2,3): two\n"));
    let w = with_file(cx, &out);
    set_grep_jumps(cx, true, true);
    w.cx(cx).simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(windows(cx), 1);
}

#[gpui::test]
fn a_double_click_in_the_result_of_a_grep_is_a_tag_jump(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("{t}(2,3): two\n"));
    let w = with_file(cx, &out);
    make_grep_window(&w, cx);
    set_grep_jumps(cx, true, true);
    double_click_on_row(&w, cx, 0);
    assert_eq!(windows(cx), 2);
    assert_eq!(newest(cx).caret(cx), Pos::new(1, 2));
}

#[gpui::test]
fn a_double_click_selects_a_word_when_the_option_is_off(cx: &mut TestAppContext) {
    let (_dir, out) = project(|t| format!("{t}(2,3): two\n"));
    let w = with_file(cx, &out);
    make_grep_window(&w, cx);
    set_grep_jumps(cx, true, false);
    double_click_on_row(&w, cx, 0);
    assert_eq!(windows(cx), 1);
    assert!(w.view.read_with(cx, |v, _| v.cursor().has_selection()));
}

#[gpui::test]
fn a_double_click_on_a_line_that_names_nothing_selects_a_word(cx: &mut TestAppContext) {
    // `OnLBUTTONDBLCLK`: when the jump fails the ordinary double click goes on.
    let (_dir, out) = project(|_| "nothing here\n".to_owned());
    let w = with_file(cx, &out);
    make_grep_window(&w, cx);
    set_grep_jumps(cx, true, true);
    double_click_on_row(&w, cx, 0);
    assert_eq!(windows(cx), 1);
    assert!(w.view.read_with(cx, |v, _| v.cursor().has_selection()));
}

/// A double click on the first cells of a row of the window.
fn double_click_on_row(w: &Win, cx: &mut TestAppContext, row: usize) {
    use gpui::{Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, point, px};
    let vcx = w.cx(cx);
    vcx.run_until_parked();
    let at = w.view.read_with(vcx, |v, _| {
        let paint = v.paint.expect("the window was painted");
        let m = v.metrics;
        point(
            px(paint.origin_x + 1.1 * m.cell_width),
            px(paint.origin_y + (row as f32 + 0.5) * m.line_height - v.scroll_y),
        )
    });
    for count in 1..=2 {
        vcx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: at,
            modifiers: Modifiers::default(),
            click_count: count,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: at,
            modifiers: Modifiers::default(),
            click_count: count,
        });
    }
    cx.run_until_parked();
}
