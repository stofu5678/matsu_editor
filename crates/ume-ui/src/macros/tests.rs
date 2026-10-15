//! Tests of key macros and macro files in GPUI's headless test window.
//!
//! The keys are Sakura's defaults (`sakura_core/func/CKeyBind.cpp`, key `M` and `L`): Ctrl+Shift+M
//! records, Ctrl+L loads, Ctrl+Shift+L plays. `secondary-` is Ctrl on Windows and
//! Linux and Cmd on macOS, where the key map lets Cmd stand for Ctrl (ADR-0008).
//!
//! The scripts run with the real Node.js and Python. A test whose interpreter is not installed
//! prints "SKIP" and passes, as the tests of `ume-plugin` do.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_core::buffer::Document;
use ume_core::model::DocumentModel;
use ume_plugin::session::Language;

use super::{F_EXECEXTMACRO, FileKind, file_kind};
use crate::document::SharedDocument;
use crate::view::EditorView;

struct Fixture {
    document: Entity<SharedDocument>,
    view: Entity<EditorView>,
}

fn setup<'a>(cx: &'a mut TestAppContext, text: &str) -> (Fixture, &'a mut VisualTestContext) {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let shared = document.clone();
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (Fixture { document, view }, vcx)
}

impl Fixture {
    fn lines(&self, cx: &mut VisualTestContext) -> Vec<String> {
        self.document.read_with(cx, |shared, _| {
            let doc = shared.doc();
            (0..doc.line_count())
                .map(|i| doc.line(i).unwrap().unwrap().text)
                .collect()
        })
    }

    fn output(&self, cx: &mut VisualTestContext) -> String {
        self.view.read_with(cx, |v, _| v.macro_output_text())
    }

    fn running(&self, cx: &mut VisualTestContext) -> bool {
        self.view.read_with(cx, |v, _| v.is_macro_running())
    }

    fn title(&self, cx: &mut VisualTestContext) -> String {
        self.view.read_with(cx, |v, cx| v.title(cx))
    }

    /// Runs a Sakura function the way a menu item does.
    fn run(&self, cx: &mut VisualTestContext, code: u16, args: &[Arg]) {
        self.view
            .update_in(cx, |v, window, cx| v.run_function(code, args, window, cx));
    }

    /// Lets the macro's process and the window talk until the macro ends.
    fn wait_until_done(&self, cx: &mut VisualTestContext) {
        cx.executor().allow_parking();
        for _ in 0..1500 {
            cx.run_until_parked();
            if !self.running(cx) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("the macro did not end: {}", self.output(cx));
    }
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

fn text_arg(path: &Path) -> Arg {
    Arg::Text(path.to_string_lossy().into_owned())
}

/// Ctrl+M is Sakura's key for it, but on macOS Cmd+M is left to the OS (minimize), so the tests
/// run the function the way a menu item does.
const F_SAVEKEYMACRO: u16 = 31251;
const REC: &str = "secondary-shift-m";
const EXEC: &str = "secondary-shift-l";

#[test]
fn files_are_told_apart_by_their_extension() {
    assert_eq!(file_kind(Path::new("a.mac")), FileKind::KeyMacro);
    assert_eq!(file_kind(Path::new("A.MAC")), FileKind::KeyMacro);
    assert_eq!(
        file_kind(Path::new("a.js")),
        FileKind::Script(Language::JavaScript)
    );
    assert_eq!(
        file_kind(Path::new("a.py")),
        FileKind::Script(Language::Python)
    );
    assert_eq!(file_kind(Path::new("a.vbs")), FileKind::Unknown);
    assert_eq!(file_kind(Path::new("macro")), FileKind::Unknown);
}

// ---- key macros ----

#[gpui::test]
fn the_same_keys_give_the_same_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "xyz");
    cx.simulate_keystrokes(REC);
    assert!(f.view.read_with(cx, |v, _| v.is_recording_key_macro()));
    cx.simulate_keystrokes("right right");
    cx.simulate_input("Q");
    cx.simulate_keystrokes(REC);
    assert!(!f.view.read_with(cx, |v, _| v.is_recording_key_macro()));
    assert_eq!(f.lines(cx), ["xyQz"]);

    cx.simulate_keystrokes("home");
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["xyQQz"]);
    // every call is its own undo step
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(f.lines(cx), ["xyQz"]);
}

#[gpui::test]
fn playing_stops_a_recording_and_plays_what_was_recorded(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    cx.simulate_keystrokes(REC);
    cx.simulate_input("ab");
    // Sakura: `Command_EXECKEYMACRO` ends the recording first.
    cx.simulate_keystrokes(EXEC);
    assert!(!f.view.read_with(cx, |v, _| v.is_recording_key_macro()));
    assert_eq!(f.lines(cx), ["abab"]);
}

#[gpui::test]
fn the_title_says_that_it_is_recording(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    assert!(!f.title(cx).contains("記録中"));
    cx.simulate_keystrokes(REC);
    assert!(
        f.title(cx).contains("【キーマクロの記録中】"),
        "{}",
        f.title(cx)
    );
    cx.simulate_keystrokes(REC);
    assert!(!f.title(cx).contains("記録中"));
}

#[gpui::test]
fn text_from_the_ime_is_recorded_as_characters(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler;
    let (f, cx) = setup(cx, "");
    cx.simulate_keystrokes(REC);
    // A committed composition: more than one character, not typed by keys.
    f.view.update_in(cx, |v, window, cx| {
        v.replace_text_in_range(None, "あい", window, cx);
    });
    cx.simulate_keystrokes(REC);
    assert_eq!(f.lines(cx), ["あい"]);
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["あいあい"]);
}

#[gpui::test]
fn a_recorded_macro_is_saved_and_loaded_through_the_dialogs(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rec.mac");
    let (f, cx) = setup(cx, "xyz");
    cx.simulate_keystrokes(REC);
    cx.simulate_keystrokes("right");
    cx.simulate_input("A");
    // stops the recording and asks for the file
    f.run(cx, F_SAVEKEYMACRO, &[]);
    assert!(!f.view.read_with(cx, |v, _| v.is_recording_key_macro()));
    assert!(cx.did_prompt_for_new_path());
    let chosen = path.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();

    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"\xEF\xBB\xBF"), "UTF-8 with a BOM");
    let saved = String::from_utf8(bytes).unwrap();
    assert!(
        saved.contains("Right();") && saved.contains("Char(65);"),
        "{saved}"
    );

    // another window, another document: load, then play
    let (g, cx) = setup(cx, "xyz");
    cx.simulate_keystrokes("secondary-l");
    assert!(cx.did_prompt_for_paths());
    let chosen = path.clone();
    cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
    cx.run_until_parked();
    cx.simulate_keystrokes(EXEC);
    assert_eq!(g.lines(cx), ["xAyz"]);
}

#[gpui::test]
fn a_cancelled_dialog_changes_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "xyz");
    cx.simulate_keystrokes("secondary-l");
    cx.simulate_path_prompt_response(|_| None);
    cx.run_until_parked();
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["xyz"]);
    assert_eq!(f.output(cx), "");
}

#[gpui::test]
fn only_a_key_macro_can_be_saved(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let script = write(dir.path(), "m.js", "// nothing\n");
    let target = dir.path().join("out.mac");
    let (f, cx) = setup(cx, "");
    // load a script as "the key macro", then try to save it (Sakura: STR_ERR_CEDITVIEW_CMD26)
    cx.simulate_keystrokes("secondary-l");
    cx.simulate_path_prompt_response(move |_| Some(vec![script]));
    cx.run_until_parked();
    f.run(cx, F_SAVEKEYMACRO, &[]);
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert!(!target.exists());
    assert!(
        f.output(cx)
            .contains("キーボードマクロ以外は保存できません")
    );
}

#[gpui::test]
fn a_mistake_in_a_key_macro_file_names_its_line(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        dir.path(),
        "bad.mac",
        "Right();\nNoSuchFunction();\nChar(65);\n",
    );
    let (f, cx) = setup(cx, "xyz");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    cx.run_until_parked();
    // nothing ran: the file is read as a whole first
    assert_eq!(f.lines(cx), ["xyz"]);
    let output = f.output(cx);
    assert!(
        output.contains("bad.mac") && output.contains("line 2"),
        "{output}"
    );
}

#[gpui::test]
fn playing_stops_at_the_first_call_that_fails(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    // `Char()` has no argument: the command refuses it
    let path = write(dir.path(), "stop.mac", "Char(65);\nChar();\nChar(66);\n");
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    cx.run_until_parked();
    assert_eq!(f.lines(cx), ["A"]);
    assert!(f.output(cx).contains("2 番目"), "{}", f.output(cx));
    assert!(!f.running(cx));
}

#[gpui::test]
fn running_a_key_macro_file_asks_for_the_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "a.mac", "Right();\nChar(66);\n");
    let (f, cx) = setup(cx, "xyz");
    f.run(cx, F_EXECEXTMACRO, &[]);
    assert!(cx.did_prompt_for_paths());
    cx.simulate_path_prompt_response(move |_| Some(vec![path]));
    cx.run_until_parked();
    assert_eq!(f.lines(cx), ["xByz"]);
}

#[gpui::test]
fn a_macro_file_run_while_recording_is_recorded_as_one_call(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let inner = write(dir.path(), "inner.mac", "Char(66);\n");
    let saved = dir.path().join("saved.mac");
    let (f, cx) = setup(cx, "");
    cx.simulate_keystrokes(REC);
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&inner)]);
    cx.run_until_parked();
    assert_eq!(f.lines(cx), ["B"]);
    // recording goes on after the macro; its own calls were not recorded
    cx.simulate_input("c");
    f.run(cx, F_SAVEKEYMACRO, &[]);
    let chosen = saved.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    let text = std::fs::read_to_string(&saved).unwrap();
    let text = text.trim_start_matches('\u{feff}');
    let calls: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with("//") && !l.trim().is_empty())
        .map(|l| l.split('\t').next().unwrap())
        .collect();
    assert_eq!(calls.len(), 2, "{text}");
    assert!(calls[0].starts_with("ExecExternalMacro("), "{text}");
    assert_eq!(calls[1], "Char(99);");

    // playing it runs the macro file again
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["BcBc"]);
}

#[gpui::test]
fn an_unknown_extension_is_refused(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "a.vbs", "WScript.Echo 1\n");
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    cx.run_until_parked();
    assert!(f.output(cx).contains("拡張子"), "{}", f.output(cx));
    assert!(!f.running(cx));
}

// ---- macro files in JavaScript and Python ----

fn available(language: Language) -> bool {
    let interpreter = language.default_interpreter();
    let ok = Command::new(&interpreter)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!(
            "SKIP: {} ({}) is not available",
            language.display_name(),
            interpreter.to_string_lossy()
        );
    }
    ok
}

/// A macro that edits the document, prints, and ends by itself.
const JS_EDIT: &str = "Editor.GoFileTop();\nEditor.InsText('hello ');\nconsole.log('done', Editor.GetLineCount(0));\n";
const PY_EDIT: &str =
    "Editor.GoFileTop()\nEditor.InsText('hello ')\nprint('done', Editor.GetLineCount(0))\n";

fn script_edits_the_document(cx: &mut TestAppContext, language: Language, name: &str, code: &str) {
    if !available(language) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), name, code);
    let (f, cx) = setup(cx, "world");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    assert!(f.running(cx), "the macro is running in another process");
    f.wait_until_done(cx);
    assert_eq!(f.lines(cx), ["hello world"]);
    let output = f.output(cx);
    assert!(output.contains("done 1"), "{output}");
    assert!(output.contains("正常に終了しました"), "{output}");
}

#[gpui::test]
fn a_javascript_macro_edits_the_document(cx: &mut TestAppContext) {
    script_edits_the_document(cx, Language::JavaScript, "edit.js", JS_EDIT);
}

#[gpui::test]
fn a_python_macro_edits_the_document(cx: &mut TestAppContext) {
    script_edits_the_document(cx, Language::Python, "edit.py", PY_EDIT);
}

fn script_error_names_its_line(
    cx: &mut TestAppContext,
    language: Language,
    name: &str,
    code: &str,
    expect: &str,
) {
    if !available(language) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), name, code);
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    f.wait_until_done(cx);
    let output = f.output(cx);
    assert!(
        output.contains("boom") && output.contains(expect),
        "{output}"
    );
    assert!(output.contains("エラーで終了しました"), "{output}");
}

#[gpui::test]
fn a_javascript_error_shows_the_line_number(cx: &mut TestAppContext) {
    // node prints `<file>:<line>` above the message
    script_error_names_its_line(
        cx,
        Language::JavaScript,
        "err.js",
        "Editor.GoFileTop();\nthrow new Error('boom');\n",
        "err.js:2",
    );
}

#[gpui::test]
fn a_python_error_shows_the_line_number(cx: &mut TestAppContext) {
    script_error_names_its_line(
        cx,
        Language::Python,
        "err.py",
        "Editor.GoFileTop()\nraise RuntimeError('boom')\n",
        "line 2",
    );
}

fn script_can_be_aborted(cx: &mut TestAppContext, language: Language, name: &str, code: &str) {
    if !available(language) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), name, code);
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    assert!(f.running(cx));
    // the window stays responsive: keys work while the macro runs
    cx.simulate_input("z");
    assert_eq!(f.lines(cx), ["z"]);
    assert!(f.running(cx));
    // a second macro does not start over the first
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    assert!(f.output(cx).contains("実行中です"), "{}", f.output(cx));

    f.view.update(cx, |v, cx| v.cancel_macro(cx));
    f.wait_until_done(cx);
    assert!(f.output(cx).contains("中断しました"), "{}", f.output(cx));
}

#[gpui::test]
fn a_javascript_macro_can_be_aborted(cx: &mut TestAppContext) {
    script_can_be_aborted(
        cx,
        Language::JavaScript,
        "wait.js",
        "setInterval(() => {}, 1000);\n",
    );
}

#[gpui::test]
fn a_python_macro_can_be_aborted(cx: &mut TestAppContext) {
    script_can_be_aborted(
        cx,
        Language::Python,
        "wait.py",
        "import time\ntime.sleep(60)\n",
    );
}

#[gpui::test]
fn a_key_macro_waits_for_a_script_it_started(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let script = write(dir.path(), "mid.js", "Editor.InsText('-');\n");
    let key_macro = write(
        dir.path(),
        "all.mac",
        &format!(
            "Char(65);\nExecExternalMacro('{}');\nChar(66);\n",
            script.to_string_lossy().replace('\\', "\\\\")
        ),
    );
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&key_macro)]);
    f.wait_until_done(cx);
    assert_eq!(f.lines(cx), ["A-B"]);
}

#[gpui::test]
fn a_nested_macro_file_with_a_mistake_stops_the_outer_macro(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let bad = write(dir.path(), "bad.mac", "Char(66);\nNoSuchFunction();\n");
    let outer = write(
        dir.path(),
        "outer.mac",
        &format!(
            "Char(65);\nExecExternalMacro('{}');\nChar(67);\n",
            bad.to_string_lossy().replace('\\', "\\\\")
        ),
    );
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&outer)]);
    cx.run_until_parked();
    // the inner file is read as a whole first, so nothing of it ran, and the outer one stopped
    assert_eq!(f.lines(cx), ["A"]);
    let output = f.output(cx);
    assert!(
        output.contains("bad.mac") && output.contains("line 2"),
        "{output}"
    );
    assert!(output.contains("2 番目"), "{output}");
    assert!(!f.running(cx));
}

#[gpui::test]
fn run_function_says_whether_the_command_succeeded(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    // `Char` with a number is a typed character
    let ok = f.view.update_in(cx, |v, window, cx| {
        v.run_function(30200, &[Arg::Int(65)], window, cx)
    });
    assert!(ok);
    let failed = f
        .view
        .update_in(cx, |v, window, cx| v.run_function(30200, &[], window, cx));
    assert!(!failed);
    // a function that has no command here yet
    let missing = f
        .view
        .update_in(cx, |v, window, cx| v.run_function(30795, &[], window, cx));
    assert!(!missing);
    assert_eq!(f.lines(cx), ["A"]);
}

fn call_line(path: &Path) -> String {
    format!(
        "ExecExternalMacro('{}');\n",
        path.to_string_lossy().replace('\\', "\\\\")
    )
}

#[gpui::test]
fn a_macro_file_that_calls_itself_is_refused(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("loop.mac");
    std::fs::write(&path, format!("Char(65);\n{}Char(66);\n", call_line(&path))).unwrap();
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    cx.run_until_parked();
    // A ran, then the call to itself failed and stopped the macro
    assert_eq!(f.lines(cx), ["A"]);
    assert!(f.output(cx).contains("自分自身"), "{}", f.output(cx));
    assert!(!f.running(cx));
    // the guard is reset: the same file can be run again
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    assert_eq!(f.lines(cx), ["AA"]);
}

#[gpui::test]
fn two_macro_files_that_call_each_other_are_refused(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.mac");
    let b = dir.path().join("b.mac");
    std::fs::write(&a, format!("Char(65);\n{}", call_line(&b))).unwrap();
    std::fs::write(&b, format!("Char(66);\n{}", call_line(&a))).unwrap();
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&a)]);
    cx.run_until_parked();
    assert_eq!(f.lines(cx), ["AB"]);
    assert!(f.output(cx).contains("自分自身"), "{}", f.output(cx));
}

#[gpui::test]
fn the_same_file_may_be_called_twice_in_a_row(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let inner = write(dir.path(), "inner.mac", "Char(66);\n");
    let outer = write(
        dir.path(),
        "outer.mac",
        &format!("{}{}", call_line(&inner), call_line(&inner)),
    );
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&outer)]);
    cx.run_until_parked();
    assert_eq!(f.lines(cx), ["BB"]);
}

#[gpui::test]
fn nesting_has_a_limit(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let count = super::MAX_NESTING + 2;
    for i in 0..count {
        let text = if i + 1 < count {
            call_line(&dir.path().join(format!("n{}.mac", i + 1)))
        } else {
            "Char(65);\n".to_string()
        };
        std::fs::write(dir.path().join(format!("n{i}.mac")), text).unwrap();
    }
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&dir.path().join("n0.mac"))]);
    cx.run_until_parked();
    assert!(f.lines(cx).concat().is_empty());
    assert!(
        f.output(cx).contains("入れ子が深すぎます"),
        "{}",
        f.output(cx)
    );
}

fn failing_script_stops_the_key_macro(
    cx: &mut TestAppContext,
    language: Language,
    name: &str,
    code: &str,
) {
    if !available(language) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let script = write(dir.path(), name, code);
    let outer = write(
        dir.path(),
        "outer.mac",
        &format!("Char(65);\n{}Char(66);\n", call_line(&script)),
    );
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&outer)]);
    f.wait_until_done(cx);
    assert_eq!(f.lines(cx), ["A"]);
    assert!(
        f.output(cx).contains("エラーで終了しました"),
        "{}",
        f.output(cx)
    );
    assert!(f.output(cx).contains("2 番目"), "{}", f.output(cx));
    // nothing is left waiting: the next key macro runs
    f.run(
        cx,
        F_EXECEXTMACRO,
        &[text_arg(&write(dir.path(), "next.mac", "Char(67);\n"))],
    );
    assert_eq!(f.lines(cx), ["AC"]);
}

#[gpui::test]
fn a_failing_javascript_stops_the_key_macro_that_called_it(cx: &mut TestAppContext) {
    failing_script_stops_the_key_macro(
        cx,
        Language::JavaScript,
        "fail.js",
        "throw new Error('x');\n",
    );
}

#[gpui::test]
fn a_failing_python_stops_the_key_macro_that_called_it(cx: &mut TestAppContext) {
    failing_script_stops_the_key_macro(
        cx,
        Language::Python,
        "fail.py",
        "raise RuntimeError('x')\n",
    );
}

#[gpui::test]
fn a_call_from_a_script_ends_the_ime_composition_first(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler;
    use ume_plugin::session::HostEvent;
    let (f, cx) = setup(cx, "");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", None, window, cx);
        assert!(v.composing);
        let call = r#"{"jsonrpc":"2.0","id":1,"method":"InsText","params":["x"]}"#;
        v.on_host_event(HostEvent::Message(call.into()), window, cx);
        assert!(!v.composing && v.marked.is_none());
    });
    assert_eq!(f.lines(cx), ["にx"]);
}

#[gpui::test]
fn a_script_that_prints_a_lot_does_not_block_the_window(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let script = write(
        dir.path(),
        "flood.js",
        "for (let i = 0; i < 30000; i++) console.log('line ' + i);\nEditor.InsText('end');\n",
    );
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&script)]);
    f.wait_until_done(cx);
    assert_eq!(f.lines(cx), ["end"]);
    let output = f.output(cx);
    assert!(
        output.contains("line 29999") && output.contains("正常に終了"),
        "output ends"
    );
    assert!(output.lines().count() <= 5000);
}

#[gpui::test]
fn a_composition_that_stays_in_the_document_is_recorded(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler;
    let (f, cx) = setup(cx, "");
    // ended by the IME without a commit
    cx.simulate_keystrokes(REC);
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", None, window, cx);
        v.unmark_text(window, cx);
    });
    // ended by a key (Left): the text is recorded before the key
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "ほ", None, window, cx);
    });
    cx.simulate_keystrokes("left");
    cx.simulate_keystrokes(REC);
    assert_eq!(f.lines(cx), ["にほ"]);
    cx.simulate_keystrokes("secondary-end");
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["にほにほ"]);
}

#[gpui::test]
fn line_breaks_in_committed_text_are_recorded(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler;
    let (f, cx) = setup(cx, "");
    cx.simulate_keystrokes(REC);
    f.view.update_in(cx, |v, window, cx| {
        v.replace_text_in_range(None, "p\r\nq", window, cx);
    });
    cx.simulate_keystrokes(REC);
    let first = f.lines(cx);
    assert_eq!(first.len(), 2, "{first:?}");
    cx.simulate_keystrokes("secondary-end");
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["p", "qp", "q"]);
}
