//! Tests of 外部コマンド実行 in GPUI's headless test window: the macro `ExecCommand` with the
//! option bits, the output in the document and in the output panel, the selection and the
//! document on the standard input, the encodings, the folder, a command that fails or does not
//! exist, 中断, the dialog and the histories it keeps. The rules without a window are tested in
//! `ume_core::exec` and `ume_config`, the process in `ume_platform::exec`.
//!
//! The commands are the same on every OS (`echo`, `exit`, `sort`) or picked per OS below.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::Pos;

use super::{ExecPrefs, flags as saved_flags, history, install};
use crate::document::SharedDocument;
use crate::file_ops::windows::{LoadRequest, editors, open_editor_window};
use crate::ui_command::UiRegistry;
use crate::view::EditorView;

/// A program that copies its input to its output.
const COPY: &str = if cfg!(windows) {
    "findstr \"^\""
} else {
    "cat"
};
/// A command that runs for half a minute.
const LONG: &str = if cfg!(windows) {
    "ping -n 30 127.0.0.1"
} else {
    "sleep 30"
};
/// A command that writes the lines a, b, c.
const ABC: &str = if cfg!(windows) {
    "cmd /C \"echo a&echo b&echo c\""
} else {
    "printf 'a\\nb\\nc\\n'"
};
/// Lists the file names of the current folder, one per line.
const LIST: &str = if cfg!(windows) { "cmd /C dir /B" } else { "ls" };

// the bits of the option
const OUT: i64 = 0x01;
const WINDOW: i64 = 0x02;
const IN: i64 = 0x04;
const OUT_UTF8: i64 = 0x80;
const IN_UTF8: i64 = 0x100;
const NO_INFO: i64 = 0x40;
const INFO: i64 = 0x20;
const DIR: i64 = 0x200;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

struct Win {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
}

impl Win {
    fn call(&self, cx: &mut TestAppContext, function: &str, args: &[Arg]) {
        let code = code_of(function);
        let view = self.view.clone();
        self.handle
            .update(cx, |_, window, app| {
                view.update(app, |view, cx| view.run_function(code, args, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
    }

    fn exec(&self, cx: &mut TestAppContext, command: &str, option: i64) {
        self.call(
            cx,
            "F_EXECMD",
            &[Arg::Text(command.into()), Arg::Int(option)],
        );
    }

    fn exec_in(&self, cx: &mut TestAppContext, command: &str, option: i64, folder: &str) {
        self.call(
            cx,
            "F_EXECMD",
            &[
                Arg::Text(command.into()),
                Arg::Int(option),
                Arg::Text(folder.into()),
            ],
        );
    }

    /// The text of the document, with its line breaks as `\n`.
    fn text(&self, cx: &mut TestAppContext) -> String {
        self.view.read_with(cx, |view, app| {
            let doc = view.document().read(app).doc().clone();
            let mut text = String::new();
            for line in doc.lines(0..doc.line_count()) {
                let line = line.unwrap();
                text.push_str(&line.text);
                if line.eol.is_some() {
                    text.push('\n');
                }
            }
            text
        })
    }

    fn output(&self, cx: &mut TestAppContext) -> String {
        self.view.read_with(cx, |view, _| {
            view.macros.output.text().replace("\r\n", "\n")
        })
    }

    fn running(&self, cx: &mut TestAppContext) -> bool {
        self.view
            .read_with(cx, |view, _| view.support.exec_running.is_some())
    }

    fn select(&self, cx: &mut TestAppContext, from: Pos, to: Pos) {
        self.view.update(cx, |view, _| view.cursor.select(from, to));
    }

    fn selection(&self, cx: &mut TestAppContext) -> Option<(Pos, Pos)> {
        self.view.read_with(cx, |view, _| view.cursor().selection())
    }

    fn go_to(&self, cx: &mut TestAppContext, pos: Pos) {
        self.view.update(cx, |view, cx| view.go_to(pos, cx));
    }

    fn cx<'a>(&self, cx: &'a mut TestAppContext) -> &'a mut VisualTestContext {
        VisualTestContext::from_window(self.handle, cx).into_mut()
    }

    fn prompt(&self, cx: &mut TestAppContext) -> Option<String> {
        self.cx(cx).pending_prompt().map(|(message, _)| message)
    }

    fn dialog(&self, cx: &mut TestAppContext) -> Option<Entity<super::dialog::ExecDialog>> {
        self.view
            .read_with(cx, |view, _| view.support.exec_dialog.clone())
    }
}

/// A window with `text` as a file `doc.txt` in a folder of its own.
fn open(cx: &mut TestAppContext, text: &str) -> (tempfile::TempDir, Win) {
    cx.update(gpui_component::init);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    // Windows has CR LF in its files, and its `sort` takes a last LF alone for a last empty line
    let text = if cfg!(windows) {
        text.replace('\n', "\r\n")
    } else {
        text.to_owned()
    };
    std::fs::write(&path, text).unwrap();
    (dir, open_file(cx, &path))
}

fn open_file(cx: &mut TestAppContext, path: &Path) -> Win {
    let request = LoadRequest {
        path: path.to_owned(),
        encoding: None,
    };
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), Some(request)))
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

/// Lets the window, the clock and the threads of the command run until `done` is true.
fn wait_until(
    cx: &mut TestAppContext,
    what: &str,
    mut done: impl FnMut(&mut TestAppContext) -> bool,
) {
    cx.executor().allow_parking();
    let begun = std::time::Instant::now();
    while begun.elapsed() < Duration::from_secs(60) {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("never happened: {what}");
}

/// Waits for the end of a run whose output has `marker` in the panel.
fn wait_for_output(cx: &mut TestAppContext, w: &Win, marker: &str) {
    wait_until(cx, marker, |cx| {
        w.output(cx).contains(marker) && !w.running(cx)
    });
}

// ---- the macro ExecCommand ----

#[gpui::test]
fn the_output_goes_to_the_output_panel_with_the_extra_information(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "text\n");
    w.exec(cx, "echo hello", OUT);
    wait_for_output(cx, &w, "終了コード: 0");
    let output = w.output(cx);
    assert!(output.contains("#===="), "{output}");
    assert!(output.contains("#DateTime : "), "{output}");
    assert!(output.contains("#CmdLine  : echo hello"), "{output}");
    assert!(output.contains("hello"), "{output}");
    // the panel is open, the document is as it was
    assert!(w.view.read_with(cx, |v, _| v.macros.output.is_open()));
    assert_eq!(w.text(cx), "text\n");
}

#[gpui::test]
fn the_extra_information_can_be_left_out(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, "echo hello", OUT | NO_INFO);
    wait_for_output(cx, &w, "hello");
    assert_eq!(w.output(cx).trim(), "hello");
}

#[gpui::test]
fn the_error_is_in_the_output_too(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, "echo problem 1>&2", OUT | NO_INFO);
    wait_for_output(cx, &w, "problem");
}

#[gpui::test]
fn the_exit_code_is_reported(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, "exit 3", OUT);
    wait_for_output(cx, &w, "終了コード: 3");
}

#[gpui::test]
fn a_command_that_does_not_exist_is_told_by_the_shell(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, "ume-no-such-command-qzx", OUT);
    wait_for_output(cx, &w, "終了コード: ");
    assert!(!w.output(cx).contains("終了コード: 0"));
    assert!(w.prompt(cx).is_none());
}

#[gpui::test]
fn a_command_that_cannot_start_is_told_in_a_box(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let missing = dir.path().join("no-such-folder");
    w.exec_in(cx, "echo hi", OUT | DIR, &missing.display().to_string());
    wait_until(cx, "the box", |cx| w.prompt(cx).is_some());
    let message = w.prompt(cx).unwrap();
    assert!(
        message.contains("コマンド実行は失敗しました。"),
        "{message}"
    );
    assert!(message.contains("echo hi"), "{message}");
    w.cx(cx).simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert!(!w.running(cx));
}

#[gpui::test]
fn an_empty_command_is_told_and_a_missing_one_stops_the_macro(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, "", OUT);
    wait_until(cx, "the box", |cx| w.prompt(cx).is_some());
    w.cx(cx).simulate_prompt_answer("OK");
    cx.run_until_parked();
    // `Argument[0] == nullptr`: the macro stops with an error
    let code = code_of("F_EXECMD");
    let view = w.view.clone();
    let answer = w
        .handle
        .update(cx, |_, window, app| {
            let reg = UiRegistry::global().get_by_code(code).unwrap();
            view.update(app, |view, cx| reg.run(view, &[], window, cx))
        })
        .unwrap();
    assert!(answer.is_err());
    let answer = w
        .handle
        .update(cx, |_, window, app| {
            let reg = UiRegistry::global().get_by_code(code).unwrap();
            view.update(app, |view, cx| {
                reg.run(
                    view,
                    &[Arg::Text("x".into()), Arg::Text("1".into())],
                    window,
                    cx,
                )
            })
        })
        .unwrap();
    assert!(answer.is_err(), "the option is a number");
}

#[gpui::test]
fn the_output_goes_into_the_document_at_the_caret(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "ab");
    w.go_to(cx, Pos::new(0, 1));
    w.exec(cx, "echo X", OUT | WINDOW);
    wait_until(cx, "the text", |cx| {
        w.text(cx).replace("\r\n", "\n") == "aX\nb" && !w.running(cx)
    });
    // no extra information in the document by default
    assert!(!w.output(cx).contains("終了コード"));
}

#[gpui::test]
fn the_extra_information_can_be_asked_for_in_the_document(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, "echo X", OUT | WINDOW | INFO);
    wait_until(cx, "the text", |cx| {
        w.text(cx).contains("終了コード: 0") && !w.running(cx)
    });
    let text = w.text(cx);
    assert!(text.contains("#CmdLine  : echo X"), "{text}");
    assert!(text.contains("X\n"), "{text}");
}

#[gpui::test]
fn all_the_output_is_one_undo_step(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "start\n");
    w.go_to(cx, Pos::new(0, 5));
    w.exec(cx, ABC, OUT | WINDOW);
    wait_until(cx, "the text", |cx| {
        w.text(cx).replace("\r\n", "\n") == "starta\nb\nc\n\n" || {
            w.text(cx).replace("\r\n", "\n") == "starta\nb\nc\n" && !w.running(cx)
        }
    });
    let after = w.text(cx);
    assert!(after.starts_with("starta"), "{after}");
    w.call(cx, "F_UNDO", &[]);
    assert_eq!(w.text(cx), "start\n");
    w.call(cx, "F_REDO", &[]);
    assert_eq!(w.text(cx), after);
}

#[gpui::test]
fn a_selection_is_replaced_and_the_output_is_selected(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "abcd");
    w.select(cx, Pos::new(0, 1), Pos::new(0, 3));
    w.exec(cx, "echo X", OUT | WINDOW);
    wait_until(cx, "the text", |cx| {
        w.text(cx).replace("\r\n", "\n") == "aX\nd" && !w.running(cx)
    });
    assert_eq!(w.selection(cx), Some((Pos::new(0, 1), Pos::new(1, 0))));
}

#[gpui::test]
fn without_a_selection_the_whole_document_goes_through_the_command(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "b\nc\na\n");
    w.exec(cx, "sort", OUT | WINDOW | IN);
    wait_until(cx, "the sorted text", |cx| {
        w.text(cx).replace("\r\n", "\n") == "a\nb\nc\n" && !w.running(cx)
    });
}

#[gpui::test]
fn a_selection_alone_goes_through_the_command(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "z\nb\na\nz\n");
    w.select(cx, Pos::new(1, 0), Pos::new(2, 1));
    w.exec(cx, "sort", OUT | WINDOW | IN);
    // "b\na" is replaced by "a\nb" (sort writes its own line break after the last line)
    wait_until(cx, "the sorted text", |cx| {
        w.text(cx).replace("\r\n", "\n").starts_with("z\na\nb\n") && !w.running(cx)
    });
}

#[gpui::test]
fn the_selection_is_the_input_and_the_output_goes_to_the_panel(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "ab\ncd\nef\n");
    w.select(cx, Pos::new(1, 0), Pos::new(1, 2));
    w.exec(cx, COPY, OUT | IN | NO_INFO);
    wait_for_output(cx, &w, "cd");
    assert_eq!(w.output(cx).trim(), "cd");
    assert_eq!(w.text(cx), "ab\ncd\nef\n");
}

#[gpui::test]
fn a_document_that_is_not_selected_is_the_input_whole(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "ab\ncd\n");
    w.exec(cx, COPY, OUT | IN | NO_INFO);
    wait_for_output(cx, &w, "cd");
    assert_eq!(w.output(cx).trim(), "ab\ncd");
}

#[gpui::test]
fn japanese_text_goes_through_in_shift_jis_by_default(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "日本語\n");
    w.exec(cx, COPY, OUT | IN | NO_INFO);
    wait_for_output(cx, &w, "日本語");
}

#[gpui::test]
fn the_encodings_of_the_input_and_the_output_are_chosen_by_their_bits(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "日本語\n");
    // UTF-8 both ways: the same text
    w.exec(cx, COPY, OUT | IN | NO_INFO | OUT_UTF8 | IN_UTF8);
    wait_for_output(cx, &w, "日本語");
    // UTF-8 in and Shift_JIS out: the bytes are read as the wrong encoding, not as the text
    let (_dir2, v) = open(cx, "日本語\n");
    v.exec(cx, COPY, OUT | IN | NO_INFO | IN_UTF8);
    wait_until(cx, "some output", |cx| {
        !v.output(cx).trim().is_empty() && !v.running(cx)
    });
    assert!(!v.output(cx).contains("日本語"));
}

#[gpui::test]
fn the_folder_is_used_only_when_asked_for(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let other = tempfile::tempdir().unwrap();
    std::fs::write(other.path().join("marker-qzx.txt"), "x").unwrap();
    // asked for: the folder of the argument
    w.exec_in(
        cx,
        LIST,
        OUT | NO_INFO | DIR,
        &other.path().display().to_string(),
    );
    wait_for_output(cx, &w, "marker-qzx.txt");
    assert!(!w.output(cx).contains("doc.txt"));
    // not asked for: the argument is ignored and the command runs in the folder of the file
    let (_dir2, v) = open_beside(cx, dir.path());
    v.exec_in(cx, LIST, OUT | NO_INFO, &other.path().display().to_string());
    wait_for_output(cx, &v, "doc.txt");
    assert!(!v.output(cx).contains("marker-qzx.txt"));
}

/// A window on `doc.txt` of `dir` (the folder stays the caller's).
fn open_beside(cx: &mut TestAppContext, dir: &Path) -> (PathBuf, Win) {
    (dir.to_path_buf(), open_file(cx, &dir.join("doc.txt")))
}

#[gpui::test]
fn the_name_of_the_file_is_expanded(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, "echo $F", OUT | NO_INFO);
    wait_for_output(cx, &w, "doc.txt");
}

#[gpui::test]
fn a_document_in_view_mode_gets_the_output_in_the_panel(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "text\n");
    w.view.update(cx, |view, cx| {
        view.document()
            .clone()
            .update(cx, |shared, cx| shared.set_view_mode(true, cx));
    });
    w.exec(cx, "echo viewed", OUT | WINDOW | NO_INFO);
    wait_for_output(cx, &w, "viewed");
    assert_eq!(w.text(cx), "text\n");
}

#[gpui::test]
fn without_the_output_the_command_just_starts(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "text\n");
    w.exec(cx, "exit 0", 0);
    for _ in 0..20 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
    }
    assert!(!w.running(cx));
    assert_eq!(w.output(cx), "");
    assert_eq!(w.text(cx), "text\n");
    // nobody waits for the command: let it end before its folder is removed
    std::thread::sleep(Duration::from_millis(300));
}

// ---- 中断 ----

fn stop(w: &Win, cx: &mut TestAppContext) {
    w.view.update(cx, |view, cx| {
        let running = view.support.exec_running.clone().expect("a box is shown");
        running.update(cx, |running, cx| running.stop(cx));
    });
}

#[gpui::test]
fn a_long_command_is_stopped_with_the_button(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, LONG, OUT);
    wait_until(cx, "the box", |cx| w.running(cx));
    // the output goes to the panel: the box is in the corner and the keys stay with the text
    stop(&w, cx);
    wait_for_output(cx, &w, "中断しました。");
    let output = w.output(cx);
    assert!(output.contains("終了コード: "), "{output}");
}

#[cfg(unix)]
#[gpui::test]
fn a_command_that_never_stops_writing_can_still_be_stopped(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, "yes", OUT | NO_INFO);
    wait_until(cx, "some output", |cx| w.output(cx).lines().count() > 1000);
    // the window gets its turns between the batches: the button works
    stop(&w, cx);
    wait_until(cx, "the end", |cx| !w.running(cx));
}

#[gpui::test]
fn the_box_over_the_text_takes_the_keys_and_escape_stops(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "keep\n");
    w.exec(cx, LONG, OUT | WINDOW);
    wait_until(cx, "the box", |cx| w.running(cx));
    // typing does not reach the text
    w.cx(cx).simulate_input("zzz");
    cx.run_until_parked();
    w.cx(cx).simulate_keystrokes("escape");
    wait_until(cx, "the end", |cx| !w.running(cx));
    // what the command wrote (`ping` writes at once) is at the caret, before the text
    assert!(w.text(cx).contains("keep"), "{}", w.text(cx));
    assert!(!w.text(cx).contains("zzz"), "{}", w.text(cx));
}

// ---- the dialog ----

fn file_in(dir: &Path) -> PathBuf {
    dir.join("ume.ini")
}

fn set_history(cx: &mut TestAppContext, file: &Path) {
    cx.update(|app| install(app, ExecPrefs::load(None, Some(file.to_owned()))));
}

/// Types into the dialog's boxes and checks what the test asks for.
fn fill(w: &Win, cx: &mut TestAppContext, command: &str, dir: Option<&str>) {
    let dialog = w.dialog(cx).expect("the dialog is open");
    let (command, dir) = (command.to_owned(), dir.map(str::to_owned));
    w.handle
        .update(cx, |_, window, app| {
            dialog.update(app, |dialog, cx| {
                dialog
                    .command
                    .update(cx, |state, cx| state.set_value(command, window, cx));
                if let Some(dir) = dir {
                    dialog.use_cur_dir = true;
                    dialog
                        .cur_dir
                        .update(cx, |state, cx| state.set_value(dir, window, cx));
                }
            })
        })
        .unwrap();
}

fn press_ok(w: &Win, cx: &mut TestAppContext) {
    let dialog = w.dialog(cx).expect("the dialog is open");
    w.handle
        .update(cx, |_, window, app| {
            dialog.update(app, |dialog, cx| dialog.ok(window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn the_dialog_runs_the_command_and_remembers_it(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let ini = file_in(dir.path());
    set_history(cx, &ini);
    w.call(cx, "F_EXECMD_DIALOG", &[]);
    assert!(w.dialog(cx).is_some());
    // the first time: only 標準出力を得る, no history
    assert_eq!(saved_flags_of(cx), OUT as i32);
    fill(&w, cx, "echo from the dialog", None);
    // 標準出力を得る (the default) puts the output in the panel
    press_ok(&w, cx);
    wait_for_output(cx, &w, "from the dialog");
    assert!(w.dialog(cx).is_none());
    // the command, the folder and the choices are in the settings file as Sakura writes them
    let saved = String::from_utf8_lossy(&std::fs::read(&ini).unwrap()).into_owned();
    assert!(saved.contains("[Cmd]"), "{saved}");
    assert!(saved.contains("nCmdArrNum=1"), "{saved}");
    assert!(
        saved.contains("szCmdArr[00]=echo from the dialog"),
        "{saved}"
    );
    assert!(saved.contains("nExecFlgOpt=1"), "{saved}");
    assert_eq!(
        cx.update(|app| history(app).commands.items().to_vec()),
        ["echo from the dialog"]
    );
}

fn saved_flags_of(cx: &mut TestAppContext) -> i32 {
    cx.update(|app| saved_flags(app).0)
}

#[gpui::test]
fn the_dialog_starts_with_the_last_command_and_choices(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let ini = file_in(dir.path());
    std::fs::write(
        &ini,
        "[Cmd]\r\nnCmdArrNum=2\r\nszCmdArr[00]=newest\r\nszCmdArr[01]=older\r\n\
         nCurDirArrNum=1\r\nszCurDirArr[00]=/some/dir\r\n[Other]\r\nnExecFlgOpt=517\r\n",
    )
    .unwrap();
    cx.update(|app| install(app, ExecPrefs::load(Some(&ini), None)));
    w.call(cx, "F_EXECMD_DIALOG", &[]);
    let dialog = w.dialog(cx).unwrap();
    let (command, dir, flags) = dialog.read_with(cx, |d, app| {
        let choice = d.choice(app);
        (choice.command, choice.cur_dir, choice.flags.0)
    });
    assert_eq!(command, "newest");
    // 517 = 0x205: output, input, folder
    assert_eq!(dir, "/some/dir");
    assert_eq!(flags, 0x205);
}

#[gpui::test]
fn cancelling_the_dialog_runs_and_remembers_nothing(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let ini = file_in(dir.path());
    set_history(cx, &ini);
    w.call(cx, "F_EXECMD_DIALOG", &[]);
    fill(&w, cx, "echo never", None);
    w.cx(cx).simulate_keystrokes("escape");
    cx.run_until_parked();
    for _ in 0..20 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
    }
    assert!(w.dialog(cx).is_none());
    assert_eq!(w.output(cx), "");
    assert!(!w.running(cx));
    assert!(!ini.exists());
    assert!(cx.update(|app| history(app).commands.items().is_empty()));
}

#[gpui::test]
fn the_dialog_keeps_the_folder_only_when_it_is_checked(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let ini = file_in(dir.path());
    set_history(cx, &ini);
    let other = tempfile::tempdir().unwrap();
    std::fs::write(other.path().join("marker-qzx.txt"), "x").unwrap();
    w.call(cx, "F_EXECMD_DIALOG", &[]);
    fill(&w, cx, LIST, Some(&other.path().display().to_string()));
    press_ok(&w, cx);
    wait_for_output(cx, &w, "marker-qzx.txt");
    let kept = cx.update(|app| history(app).cur_dirs.items().to_vec());
    assert_eq!(kept, [other.path().display().to_string()]);
    let flags = saved_flags_of(cx);
    assert_eq!(flags & DIR as i32, DIR as i32);
    // the next time: unchecked, so no folder is kept and the old one stays
    w.call(cx, "F_EXECMD_DIALOG", &[]);
    let dialog = w.dialog(cx).unwrap();
    w.handle
        .update(cx, |_, _, app| {
            dialog.update(app, |d, _| d.use_cur_dir = false)
        })
        .unwrap();
    fill(&w, cx, "echo again", None);
    press_ok(&w, cx);
    wait_for_output(cx, &w, "again");
    assert_eq!(cx.update(|app| history(app).cur_dirs.items().len()), 1);
    assert_eq!(saved_flags_of(cx) & DIR as i32, 0);
}

#[gpui::test]
fn a_document_in_view_mode_does_not_offer_the_window(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let ini = file_in(dir.path());
    std::fs::write(&ini, "[Other]\r\nnExecFlgOpt=3\r\n").unwrap();
    cx.update(|app| install(app, ExecPrefs::load(Some(&ini), None)));
    w.view.update(cx, |view, cx| {
        view.document()
            .clone()
            .update(cx, |shared, cx| shared.set_view_mode(true, cx));
    });
    w.call(cx, "F_EXECMD_DIALOG", &[]);
    let dialog = w.dialog(cx).unwrap();
    let flags = dialog.read_with(cx, |d, app| d.choice(app).flags.0);
    assert_eq!(flags, 1, "0x02 is taken off (CDlgExec::SetData)");
}

#[gpui::test]
fn the_dialog_records_the_call_in_a_key_macro(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let rec = dir.path().join("rec.mac");
    set_history(cx, &file_in(dir.path()));
    w.view
        .update(cx, |view, cx| view.toggle_key_macro_recording(cx));
    w.call(cx, "F_EXECMD_DIALOG", &[]);
    fill(&w, cx, "echo recorded", None);
    press_ok(&w, cx);
    wait_for_output(cx, &w, "recorded");
    w.view
        .update(cx, |view, cx| view.toggle_key_macro_recording(cx));
    w.view
        .read_with(cx, |view, _| view.save_key_macro_to(&rec))
        .unwrap();
    let saved = String::from_utf8_lossy(&std::fs::read(&rec).unwrap()).into_owned();
    // `HandleCommand( F_EXECMD, true, cmd, flags, dir )` is what is recorded, without the folder
    assert!(
        saved.contains("ExecCommand('echo recorded', 1);"),
        "{saved}"
    );
    assert!(!saved.contains("ExecCommandDialog"), "{saved}");
}

#[gpui::test]
fn a_macro_does_not_change_what_the_dialog_remembers(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let ini = file_in(dir.path());
    set_history(cx, &ini);
    w.exec(cx, "echo quiet", OUT | NO_INFO);
    wait_for_output(cx, &w, "quiet");
    assert!(cx.update(|app| history(app).commands.items().is_empty()));
    assert!(!ini.exists());
}

#[gpui::test]
fn the_history_list_puts_a_row_in_the_box(cx: &mut TestAppContext) {
    let (dir, w) = open(cx, "");
    let ini = file_in(dir.path());
    std::fs::write(
        &ini,
        "[Cmd]\r\nnCmdArrNum=2\r\nszCmdArr[00]=newest\r\nszCmdArr[01]=older\r\n",
    )
    .unwrap();
    cx.update(|app| install(app, ExecPrefs::load(Some(&ini), None)));
    w.call(cx, "F_EXECMD_DIALOG", &[]);
    let dialog = w.dialog(cx).unwrap();
    w.handle
        .update(cx, |_, window, app| {
            dialog.update(app, |d, cx| d.pick_command("older", window, cx))
        })
        .unwrap();
    let command = dialog.read_with(cx, |d, app| d.choice(app).command);
    assert_eq!(command, "older");
}

#[gpui::test]
fn a_second_command_does_not_replace_the_box_of_the_first(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.exec(cx, LONG, OUT);
    wait_until(cx, "the box", |cx| w.running(cx));
    let first = w.view.read_with(cx, |v, _| {
        v.support.exec_running.clone().unwrap().entity_id()
    });
    w.exec(cx, "echo second", OUT | NO_INFO);
    wait_until(cx, "the notice", |cx| w.prompt(cx).is_some());
    let message = w.prompt(cx).unwrap();
    assert!(message.contains("別のコマンドを実行中です"), "{message}");
    w.cx(cx).simulate_prompt_answer("OK");
    cx.run_until_parked();
    // the first command still has its box, and the second did not run
    let now = w.view.read_with(cx, |v, _| {
        v.support.exec_running.clone().unwrap().entity_id()
    });
    assert_eq!(now, first);
    assert!(!w.output(cx).contains("second"));
    stop(&w, cx);
    wait_for_output(cx, &w, "終了コード: ");
}
