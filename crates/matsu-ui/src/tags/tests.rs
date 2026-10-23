//! Tests of the tags file functions in GPUI's headless test window: the direct tag jump with one
//! and several hits, `F_TAGJUMP` falling back to the tags file, the keyword dialog and its pages,
//! the stack of `F_TAGJUMPBACK`, and making a tags file with a fake ctags. The rules of the tags
//! file are tested in `matsu_core::tag_file` and `matsu_core::tags_make`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::Pos;

use super::list::{SHORT_DELAY, TYPING_DELAY};
use super::{prefs, remember_prefs};
use crate::document::SharedDocument;
use crate::file_ops::windows::{LoadRequest, editors, open_editor_window};
use crate::view::EditorView;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

struct Win {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
}

impl Win {
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

    fn run(&self, cx: &mut TestAppContext, function: &str) {
        self.run_with(cx, function, &[]);
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

    /// The keywords of the list, the chosen row, whether there is a next page and the first hit
    /// shown, if the list is open.
    fn list(&self, cx: &mut TestAppContext) -> Option<(Vec<String>, Option<usize>, bool, usize)> {
        self.view.read_with(cx, |v, app| {
            v.support.tag_list.as_ref().map(|list| {
                let list = list.read(app);
                (
                    list.items().iter().map(|i| i.keyword.clone()).collect(),
                    list.selected(),
                    list.has_next(),
                    list.top(),
                )
            })
        })
    }

    fn lines(&self, cx: &mut TestAppContext) -> Vec<usize> {
        self.view.read_with(cx, |v, app| {
            v.support
                .tag_list
                .as_ref()
                .map(|l| l.read(app).items().iter().map(|i| i.line).collect())
                .unwrap_or_default()
        })
    }
}

fn open(cx: &mut TestAppContext, path: &Path) -> Win {
    cx.update(gpui_component::init);
    let request = LoadRequest {
        path: path.to_owned(),
        encoding: None,
    };
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), Some(request)))
        .unwrap();
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

fn newest(cx: &mut TestAppContext) -> Win {
    let (handle, _) = cx.update(editors).pop().unwrap();
    window_of(cx, handle)
}

fn settle(cx: &mut TestAppContext, wait: Duration) {
    cx.executor().allow_parking();
    cx.executor().advance_clock(wait);
    cx.run_until_parked();
}

const LIB: &str = "line1\nline2\nint foo(void) {\n}\nint bar(void) {\n}\n";

/// A folder with `lib.c`, a tags file that names its functions, and the document `main.c` with
/// `text`.
fn project(text: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lib.c"), LIB).unwrap();
    let tags = "!_TAG_FILE_FORMAT\t2\t/extended format/\n!_TAG_FILE_SORTED\t1\t/sorted/\n\
        bar\tlib.c\t5;\"\tf\n\
        dup\tlib.c\t1;\"\tf\n\
        dup\tlib.c\t2;\"\tv\n\
        foo\tlib.c\t3;\"\tf\n\
        foobar\tlib.c\t5;\"\tf\n";
    std::fs::write(dir.path().join("tags"), tags).unwrap();
    let main = dir.path().join("main.c");
    std::fs::write(&main, text).unwrap();
    (dir, main)
}

#[gpui::test]
fn the_direct_tag_jump_goes_to_the_one_hit_and_back(cx: &mut TestAppContext) {
    let (_dir, main) = project("int x = foo();\n");
    let w = open(cx, &main);
    w.go_to(cx, Pos::new(0, 9));
    w.run(cx, "F_DIRECT_TAGJUMP");
    assert_eq!(windows(cx), 2);
    let opened = newest(cx);
    assert_eq!(opened.name(cx).as_deref(), Some("lib.c"));
    // the tag has a line and no column: the start of line 3
    assert_eq!(opened.caret(cx), Pos::new(2, 0));
    assert_eq!(w.notice(cx), None);

    // the jump was pushed on the stack of F_TAGJUMPBACK
    w.go_to(cx, Pos::new(0, 0));
    opened.run(cx, "F_TAGJUMPBACK");
    assert_eq!(w.caret(cx), Pos::new(0, 9));
}

#[gpui::test]
fn a_word_that_is_not_a_tag_says_so(cx: &mut TestAppContext) {
    let (_dir, main) = project("int x = foo();\n");
    let w = open(cx, &main);
    w.go_to(cx, Pos::new(0, 4));
    w.run(cx, "F_DIRECT_TAGJUMP");
    assert_eq!(windows(cx), 1);
    assert_eq!(w.notice(cx).as_deref(), Some("タグジャンプできません"));
    assert!(w.list(cx).is_none());
}

#[gpui::test]
fn a_document_without_a_tags_file_says_so(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main.c");
    std::fs::write(&main, "foo\n").unwrap();
    let w = open(cx, &main);
    w.run(cx, "F_DIRECT_TAGJUMP");
    assert_eq!(w.notice(cx).as_deref(), Some("タグジャンプできません"));
}

#[gpui::test]
fn several_hits_open_the_list_and_the_chosen_one_is_jumped_to(cx: &mut TestAppContext) {
    let (_dir, main) = project("dup\n");
    let w = open(cx, &main);
    w.run(cx, "F_DIRECT_TAGJUMP");
    let (keywords, selected, next, top) = w.list(cx).expect("the list is open");
    assert_eq!(keywords, ["dup", "dup"]);
    assert_eq!(selected, Some(0));
    assert!(!next);
    assert_eq!(top, 0);
    assert_eq!(w.lines(cx), [1, 2]);
    assert_eq!(windows(cx), 1, "nothing is jumped to before the choice");

    w.cx(cx).simulate_keystrokes("down enter");
    cx.run_until_parked();
    assert!(w.list(cx).is_none());
    assert_eq!(windows(cx), 2);
    assert_eq!(newest(cx).caret(cx), Pos::new(1, 0));
}

#[gpui::test]
fn escape_leaves_the_list_without_a_jump_or_a_message(cx: &mut TestAppContext) {
    let (_dir, main) = project("dup\n");
    let w = open(cx, &main);
    w.run(cx, "F_DIRECT_TAGJUMP");
    assert!(w.list(cx).is_some());
    w.cx(cx).simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(w.list(cx).is_none());
    assert_eq!(windows(cx), 1);
    assert_eq!(w.notice(cx), None);
    // and the stack is not touched
    w.run(cx, "F_TAGJUMPBACK");
    assert_eq!(
        w.notice(cx).as_deref(),
        Some("タグジャンプバックできません")
    );
}

#[gpui::test]
fn a_line_that_names_no_file_falls_back_to_the_tags_file(cx: &mut TestAppContext) {
    // 2011.11.29 Moca: the tags file is searched after the Grep forms fail
    let (_dir, main) = project("call foo here\n");
    let w = open(cx, &main);
    w.go_to(cx, Pos::new(0, 6));
    w.run(cx, "F_TAGJUMP");
    assert_eq!(windows(cx), 2);
    assert_eq!(newest(cx).caret(cx), Pos::new(2, 0));

    // a word that is no tag: the usual message
    let (_dir2, main2) = project("call nothing here\n");
    let w2 = open(cx, &main2);
    w2.go_to(cx, Pos::new(0, 6));
    w2.run(cx, "F_TAGJUMP");
    assert_eq!(w2.notice(cx).as_deref(), Some("タグジャンプできません"));
}

#[gpui::test]
fn the_keyword_dialog_follows_what_is_typed(cx: &mut TestAppContext) {
    let (_dir, main) = project("x\n");
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("fo".into())]);
    // the list is filled 0.05 second after the dialog opens
    assert_eq!(w.list(cx).unwrap().0, Vec::<String>::new());
    settle(cx, SHORT_DELAY);
    let (keywords, selected, _, _) = w.list(cx).unwrap();
    assert_eq!(keywords, ["foo", "foobar"]);
    assert_eq!(selected, Some(0));

    // 0.7 second after the last key
    // the keyword opens selected (as in Sakura); End drops the selection so the typing appends
    w.cx(cx).simulate_keystrokes("end");
    w.cx(cx).simulate_input("ob");
    settle(cx, Duration::from_millis(300));
    assert_eq!(w.list(cx).unwrap().0, ["foo", "foobar"], "not yet");
    settle(cx, TYPING_DELAY);
    assert_eq!(w.list(cx).unwrap().0, ["foobar"]);

    w.cx(cx).simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(windows(cx), 2);
    assert_eq!(newest(cx).caret(cx), Pos::new(4, 0));
    // the keyword is remembered for the next time
    assert_eq!(cx.update(|app| prefs(app).recent), ["foob"]);
}

#[gpui::test]
fn the_tag_jump_list_function_opens_the_keyword_dialog(cx: &mut TestAppContext) {
    // `F_TAGJUMP_LIST` is the help topic of the dialog in Sakura; here it opens the dialog.
    let (_dir, main) = project("x\n");
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_LIST", &[Arg::Text("fo".into())]);
    settle(cx, SHORT_DELAY);
    assert_eq!(w.list(cx).unwrap().0, ["foo", "foobar"]);
}

#[gpui::test]
fn the_check_boxes_change_what_matches_and_are_kept(cx: &mut TestAppContext) {
    let (_dir, main) = project("x\n");
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("OBA".into())]);
    settle(cx, SHORT_DELAY);
    assert_eq!(w.list(cx).unwrap().0, Vec::<String>::new());
    let set = |cx: &mut TestAppContext, ignore_case: bool, partial: bool| {
        w.view.update(cx, |v, cx| {
            v.support
                .tag_list
                .as_ref()
                .unwrap()
                .update(cx, |l, cx| l.set_options(ignore_case, partial, cx))
        });
        settle(cx, SHORT_DELAY);
    };
    set(cx, true, false);
    assert_eq!(
        w.list(cx).unwrap().0,
        Vec::<String>::new(),
        "not a beginning"
    );
    set(cx, true, true);
    assert_eq!(w.list(cx).unwrap().0, ["foobar"]);
    set(cx, false, true);
    assert_eq!(w.list(cx).unwrap().0, Vec::<String>::new());
    set(cx, true, true);
    w.cx(cx).simulate_keystrokes("escape");
    cx.run_until_parked();
    // cancelling keeps nothing
    assert!(!cx.update(|app| prefs(app).partial));

    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("OBA".into())]);
    set(cx, true, true);
    w.cx(cx).simulate_keystrokes("enter");
    cx.run_until_parked();
    let kept = cx.update(|app| prefs(app));
    assert!(kept.ignore_case && kept.partial);
    // the next dialog starts with them
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("OBA".into())]);
    settle(cx, SHORT_DELAY);
    assert_eq!(w.list(cx).unwrap().0, ["foobar"]);
}

#[gpui::test]
fn nothing_found_says_so_and_enter_closes_without_a_jump(cx: &mut TestAppContext) {
    let (_dir, main) = project("x\n");
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("zzz".into())]);
    settle(cx, SHORT_DELAY);
    assert!(w.list(cx).unwrap().0.is_empty());
    assert_eq!(w.list(cx).unwrap().1, None);
    w.cx(cx).simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(w.list(cx).is_none());
    assert_eq!(windows(cx), 1);
}

#[gpui::test]
fn the_list_has_pages_of_fifty(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut tags = String::from("!_TAG_FILE_SORTED\t0\n");
    for i in 0..120 {
        tags += &format!("item{i:03}\tlib.c\t{};\"\tf\n", i + 1);
    }
    std::fs::write(dir.path().join("tags"), tags).unwrap();
    std::fs::write(dir.path().join("lib.c"), "x\n".repeat(200)).unwrap();
    let main = dir.path().join("main.c");
    std::fs::write(&main, "x\n").unwrap();
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("item".into())]);
    settle(cx, SHORT_DELAY);
    let page = |w: &Win, cx: &mut TestAppContext| {
        let (keywords, _, next, top) = w.list(cx).unwrap();
        (keywords.len(), keywords.first().cloned(), next, top)
    };
    assert_eq!(page(&w, cx), (50, Some("item000".into()), true, 0));
    let turn = |cx: &mut TestAppContext, next: bool| {
        w.view.update(cx, |v, cx| {
            v.support.tag_list.as_ref().unwrap().update(cx, |l, cx| {
                if next {
                    l.next_page(cx)
                } else {
                    l.previous_page(cx)
                }
            })
        });
        cx.run_until_parked();
    };
    turn(cx, true);
    assert_eq!(page(&w, cx), (50, Some("item050".into()), true, 50));
    turn(cx, true);
    assert_eq!(page(&w, cx), (20, Some("item100".into()), false, 100));
    turn(cx, true);
    assert_eq!(page(&w, cx).3, 100, "there is no page after the last");
    turn(cx, false);
    assert_eq!(page(&w, cx), (50, Some("item050".into()), true, 50));
    // a new keyword starts from the first page again
    // the keyword opens selected (as in Sakura); End drops the selection so the typing appends
    w.cx(cx).simulate_keystrokes("end");
    w.cx(cx).simulate_input("0");
    settle(cx, TYPING_DELAY);
    assert_eq!(page(&w, cx), (50, Some("item000".into()), true, 0));
}

#[gpui::test]
fn a_new_document_looks_in_the_current_folder(cx: &mut TestAppContext) {
    // Sub_PreProcTagJumpByTagsFile: a document without a file is taken as `dmy` in the current
    // folder; the folder of the test process has no tags file, so nothing is found, and the
    // dialog still opens
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let w = window_of(cx, handle);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("zzzzzz".into())]);
    assert!(w.list(cx).is_some());
}

// ---- F_TAGS_MAKE ----

/// The text of the pending prompt of the window.
fn prompt(w: &Win, cx: &mut TestAppContext) -> Option<String> {
    w.cx(cx).pending_prompt().map(|(message, _)| message)
}

fn use_ctags_in(cx: &mut TestAppContext, dirs: Vec<PathBuf>) {
    cx.update(|app| remember_prefs(app, move |p| p.ctags_dirs = Some(dirs)));
}

fn make_dialog_open(w: &Win, cx: &mut TestAppContext) -> bool {
    w.view.read_with(cx, |v, _| v.support.tags_make.is_some())
}

fn press_make(w: &Win, cx: &mut TestAppContext) {
    w.view.update(cx, |v, cx| {
        v.support
            .tags_make
            .as_ref()
            .unwrap()
            .update(cx, |d, cx| d.make(cx))
    });
    // ctags runs on a thread; give it time (it returns at once when the prompt is there, so
    // the minute only matters on a busy machine)
    cx.executor().allow_parking();
    for _ in 0..6000 {
        cx.run_until_parked();
        if prompt(w, cx).is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[gpui::test]
fn making_tags_without_ctags_says_so(cx: &mut TestAppContext) {
    let (dir, main) = project("x\n");
    std::fs::remove_file(dir.path().join("tags")).unwrap();
    let w = open(cx, &main);
    use_ctags_in(cx, Vec::new());
    w.run(cx, "F_TAGS_MAKE");
    assert!(make_dialog_open(&w, cx));
    press_make(&w, cx);
    let message = prompt(&w, cx).expect("the refusal is shown");
    assert!(
        message.contains("ctags") && message.contains("見つかりません"),
        "{message}"
    );
    w.cx(cx).simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert!(!make_dialog_open(&w, cx));
    assert!(!dir.path().join("tags").exists());
}

#[gpui::test]
fn cancelling_the_dialog_runs_nothing(cx: &mut TestAppContext) {
    let (_dir, main) = project("x\n");
    let w = open(cx, &main);
    w.run(cx, "F_TAGS_MAKE");
    w.cx(cx).simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!make_dialog_open(&w, cx));
    assert!(prompt(&w, cx).is_none());
}

#[cfg(unix)]
mod with_a_fake_ctags {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn fake(body: &str) -> tempfile::TempDir {
        let bin = tempfile::tempdir().unwrap();
        let path = bin.path().join("ctags");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    #[gpui::test]
    fn a_quiet_ctags_makes_the_file_and_says_it_is_finished(cx: &mut TestAppContext) {
        let (dir, main) = project("x\n");
        std::fs::remove_file(dir.path().join("tags")).unwrap();
        let bin = fake("echo \"$@\" > tags");
        let w = open(cx, &main);
        use_ctags_in(cx, vec![bin.path().to_path_buf()]);
        w.run(cx, "F_TAGS_MAKE");
        // the folder of the document is the start
        press_make(&w, cx);
        assert_eq!(
            prompt(&w, cx).as_deref(),
            Some("タグファイルの作成が終了しました。")
        );
        w.cx(cx).simulate_prompt_answer("OK");
        cx.run_until_parked();
        assert!(!make_dialog_open(&w, cx));
        let written = std::fs::read_to_string(dir.path().join("tags")).unwrap();
        // --excmd=n and the names in the folder (`*`)
        assert_eq!(written.trim(), "--excmd=n lib.c main.c");
    }

    #[gpui::test]
    fn the_options_of_the_dialog_reach_ctags_and_are_kept(cx: &mut TestAppContext) {
        let (dir, main) = project("x\n");
        std::fs::remove_file(dir.path().join("tags")).unwrap();
        let bin = fake("echo \"$@\" > tags");
        let w = open(cx, &main);
        use_ctags_in(cx, vec![bin.path().to_path_buf()]);
        cx.update(|app| {
            remember_prefs(app, |p| {
                p.make.recursive = true;
                p.make.command_line = "--languages=C".into();
            })
        });
        w.run(cx, "F_TAGS_MAKE");
        press_make(&w, cx);
        w.cx(cx).simulate_prompt_answer("OK");
        cx.run_until_parked();
        let written = std::fs::read_to_string(dir.path().join("tags")).unwrap();
        assert_eq!(written.trim(), "--excmd=n -R --languages=C lib.c main.c");
        assert!(cx.update(|app| prefs(app).make.recursive));
    }

    #[gpui::test]
    fn what_ctags_says_is_shown_as_an_error(cx: &mut TestAppContext) {
        let (_dir, main) = project("x\n");
        let bin = fake("echo 'unknown option' >&2");
        let w = open(cx, &main);
        use_ctags_in(cx, vec![bin.path().to_path_buf()]);
        w.run(cx, "F_TAGS_MAKE");
        press_make(&w, cx);
        let message = prompt(&w, cx).expect("the output is shown");
        assert!(message.contains("unknown option"), "{message}");
        w.cx(cx).simulate_prompt_answer("OK");
        cx.run_until_parked();
        assert!(!make_dialog_open(&w, cx));
    }
}

#[gpui::test]
fn enter_waits_for_the_list_of_the_new_keyword(cx: &mut TestAppContext) {
    let (_dir, main) = project("x\n");
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("fo".into())]);
    settle(cx, SHORT_DELAY);
    // the keyword opens selected (as in Sakura); End drops the selection so the typing appends
    w.cx(cx).simulate_keystrokes("end");
    w.cx(cx).simulate_input("ob");
    // the list still answers "fo": Enter does not take it
    w.cx(cx).simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(w.list(cx).is_some());
    assert_eq!(windows(cx), 1);
    settle(cx, TYPING_DELAY);
    w.cx(cx).simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(windows(cx), 2);
    assert_eq!(newest(cx).caret(cx), Pos::new(4, 0));
}
