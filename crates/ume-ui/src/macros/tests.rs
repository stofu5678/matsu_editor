//! Tests of key macros and macro files in GPUI's headless test window.
//!
//! The keys are Sakura's defaults (`sakura_core/func/CKeyBind.cpp`, key `M` and `L`): Ctrl+Shift+M
//! records, Ctrl+M saves, Ctrl+L loads, Ctrl+Shift+L plays. They are pressed with `ctrl-` on every
//! OS: Control is Sakura's Ctrl on macOS too (ADR-0008, plan A).
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
use ume_plugin::session::{Engine, Language};

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
    // A script macro runs in a real process, and the host's threads hand its events to the
    // window at any moment. GPUI's test scheduler takes a wake-up from another thread for
    // non-determinism unless parking has been allowed, so allow it before a macro can start:
    // allowing it only in `wait_until_done` failed now and then when an event came first. It
    // comes after `add_window_view`: allowed before it, the test of a script that prints a lot
    // timed out every time.
    vcx.executor().allow_parking();
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

const REC: &str = "ctrl-shift-m";
const EXEC: &str = "ctrl-shift-l";

#[test]
fn files_are_told_apart_by_their_extension() {
    assert_eq!(file_kind(Path::new("a.mac")), FileKind::KeyMacro);
    assert_eq!(file_kind(Path::new("A.MAC")), FileKind::KeyMacro);
    assert_eq!(
        file_kind(Path::new("a.cjs")),
        FileKind::Script(Language::JavaScript)
    );
    assert_eq!(
        file_kind(Path::new("a.js")),
        FileKind::Script(Language::Bundled(Engine::QuickJs))
    );
    assert_eq!(
        file_kind(Path::new("a.py")),
        FileKind::Script(Language::Python)
    );
    assert_eq!(
        file_kind(Path::new("a.mjs")),
        FileKind::Script(Language::JavaScript)
    );
    assert_eq!(
        file_kind(Path::new("a.rb")),
        FileKind::Script(Language::Ruby)
    );
    assert_eq!(
        file_kind(Path::new("a.PHP")),
        FileKind::Script(Language::Php)
    );
    assert_eq!(
        file_kind(Path::new("a.lua")),
        FileKind::Script(Language::Bundled(Engine::Lua))
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
    cx.simulate_keystrokes("ctrl-z");
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
    cx.simulate_keystrokes("ctrl-m");
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
    cx.simulate_keystrokes("ctrl-l");
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
    cx.simulate_keystrokes("ctrl-l");
    cx.simulate_path_prompt_response(|_| None);
    cx.run_until_parked();
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["xyz"]);
    assert_eq!(f.output(cx), "");
}

#[gpui::test]
fn only_a_key_macro_can_be_saved(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let script = write(dir.path(), "m.cjs", "// nothing\n");
    let target = dir.path().join("out.mac");
    let (f, cx) = setup(cx, "");
    // load a script as "the key macro", then try to save it (Sakura: STR_ERR_CEDITVIEW_CMD26)
    cx.simulate_keystrokes("ctrl-l");
    cx.simulate_path_prompt_response(move |_| Some(vec![script]));
    cx.run_until_parked();
    cx.simulate_keystrokes("ctrl-m");
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
fn the_macro_dialogs_start_in_the_macro_folder(cx: &mut TestAppContext) {
    use crate::file_dialog::tests::asked;
    use ume_platform::dialog::Kind;
    let dir = tempfile::tempdir().unwrap();
    let macros = dir.path().join("macros");
    std::fs::create_dir(&macros).unwrap();
    let ini = dir.path().join("ume.ini");
    // a relative folder is taken from the folder of the settings file
    let profile = ume_config::Profile::from_bytes(b"[Macro]\nszMACROFOLDER=macros\n");
    cx.update(|cx| {
        cx.set_global(crate::settings::UserSettings::from_profile(
            profile,
            Some(ini),
        ));
    });
    asked();
    let (f, cx) = setup(cx, "xyz");
    f.run(cx, F_EXECEXTMACRO, &[]);
    cx.run_until_parked();
    let run = asked();
    assert_eq!(run.len(), 1, "{run:?}");
    assert_eq!(run[0].kind, Kind::OpenFile { multiple: false });
    assert_eq!(run[0].start.as_deref(), Some(macros.as_path()));
    cx.simulate_path_prompt_response(|_| None);
    cx.run_until_parked();
    // 保存 and 読み込み of the key macro
    cx.simulate_keystrokes(REC);
    cx.simulate_keystrokes("ctrl-m");
    let save = asked();
    assert_eq!(save.len(), 1, "{save:?}");
    assert_eq!(save[0].kind, Kind::SaveFile);
    assert_eq!(save[0].start.as_deref(), Some(macros.as_path()));
    assert_eq!(save[0].name.as_deref(), Some("RecKey.mac"));
}

#[gpui::test]
fn running_a_key_macro_file_asks_for_the_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "a.mac", "Right();\nChar(66);\n");
    let (f, cx) = setup(cx, "xyz");
    f.run(cx, F_EXECEXTMACRO, &[]);
    cx.run_until_parked();
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
    cx.simulate_keystrokes("ctrl-m");
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
        // On a CI runner that has every interpreter, a missing one is a failure, not a skip:
        // otherwise the tests of that language would pass without running.
        if std::env::var_os("UME_REQUIRE_INTERPRETERS").is_some_and(|v| !v.is_empty()) {
            panic!(
                "{} is required here (UME_REQUIRE_INTERPRETERS)",
                language.display_name()
            );
        }
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
    script_edits_the_document(cx, Language::JavaScript, "edit.cjs", JS_EDIT);
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
        "err.cjs",
        "Editor.GoFileTop();\nthrow new Error('boom');\n",
        "err.cjs:2",
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
        "wait.cjs",
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
    let script = write(dir.path(), "mid.cjs", "Editor.InsText('-');\n");
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
    // a function that has no command here yet (F_FILEOPEN_DROPDOWN: there is no tool bar)
    let missing = f
        .view
        .update_in(cx, |v, window, cx| v.run_function(30108, &[], window, cx));
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
        "fail.cjs",
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
    // The deadline this test can miss is the script's own timeout (`DEFAULT_TIMEOUT`, 30 s): the
    // time the host's reader waits for the window counts against it, and the window takes a
    // line in about 130 us in a debug build (4 s for 30 000 lines when the machine is idle),
    // which a busy machine made eight times as long. The output keeps 5000 lines, so 12 000 lines
    // are enough to show that the flood is capped and that the window is not blocked; a real
    // slowdown of the window (a tenfold cost per line) would still end the script by the timeout.
    const LINES: usize = 12_000;
    let dir = tempfile::tempdir().unwrap();
    let script = write(
        dir.path(),
        "flood.cjs",
        &format!(
            "for (let i = 0; i < {LINES}; i++) console.log('line ' + i);\nEditor.InsText('end');\n"
        ),
    );
    let (f, cx) = setup(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&script)]);
    f.wait_until_done(cx);
    assert_eq!(f.lines(cx), ["end"]);
    let output = f.output(cx);
    assert!(
        output.contains(&format!("line {}", LINES - 1)) && output.contains("正常に終了"),
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
    cx.simulate_keystrokes("ctrl-end");
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
    cx.simulate_keystrokes("ctrl-end");
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["p", "qp", "q"]);
}

// ---- functions that need the window: the view, cookies, the dialogs, Sleep ----

impl Fixture {
    /// Runs the script at `path` and lets the macro and the window talk until it ends. `answer`
    /// is called each time round, to answer a dialog that the macro opened; the clock moves on
    /// by 100 ms each time round for `Sleep`.
    fn run_script(
        &self,
        cx: &mut VisualTestContext,
        path: &Path,
        mut answer: impl FnMut(&mut VisualTestContext),
    ) {
        self.run(cx, F_EXECEXTMACRO, &[text_arg(path)]);
        cx.executor().allow_parking();
        for _ in 0..1500 {
            cx.run_until_parked();
            answer(cx);
            if !self.running(cx) {
                return;
            }
            cx.executor().advance_clock(Duration::from_millis(100));
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("the macro did not end: {}", self.output(cx));
    }

    fn script(&self, cx: &mut VisualTestContext, code: &str) -> String {
        self.script_answering(cx, code, |_| {})
    }

    fn script_answering(
        &self,
        cx: &mut VisualTestContext,
        code: &str,
        answer: impl FnMut(&mut VisualTestContext),
    ) -> String {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "fn.cjs", code);
        self.run_script(cx, &path, answer);
        self.output(cx)
    }
}

/// The values that a macro printed with `console.log('r=' + ...)`, one per line.
fn results(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix("r="))
        .collect()
}

#[gpui::test]
fn a_macro_reads_the_view_and_the_file(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (f, cx) = setup(cx, "日本語\nabc");
    let output = f.script(
        cx,
        "console.log('r=' + [Editor.GetViewLines(), Editor.GetViewColumns(), Editor.GetViewTop(), \
         Editor.GetCharCode(), Editor.GetLineCode(), Editor.IsShownStatus(), Editor.IsInsMode(), \
         Editor.GetDrawSwitch()].join(','));",
    );
    // `CTextArea::UpdateViewColRowNums`: (size - 1) / pitch, at least 1.
    let (rows, columns) = f.view.read_with(cx, |v, _| {
        let m = v.metrics();
        (
            (((m.view_height - 1.0) / m.line_height).floor() as i64).max(1),
            (((m.view_width - 1.0) / m.cell_width).floor() as i64).max(1),
        )
    });
    let line = results(&output);
    let [view] = line.as_slice() else {
        panic!("{output}");
    };
    let parts: Vec<&str> = view.split(',').collect();
    assert_eq!(
        parts[..4],
        [
            rows.to_string().as_str(),
            columns.to_string().as_str(),
            "1",
            "4"
        ],
        "{output}"
    );
    // UTF-8, with the line ending of a new document of this platform
    assert!(parts[4] == "0" || parts[4] == "2", "{output}");
    assert_eq!(parts[6..], ["1", "1"], "{output}");
}

#[gpui::test]
fn cookies_stay_in_the_window_between_macros(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (f, cx) = setup(cx, "");
    f.script(
        cx,
        "Editor.SetCookie('window', 'count', '41'); Editor.SetCookie('document', 'name', '日本語');\
         Editor.SetDrawSwitch(0);",
    );
    let output = f.script(
        cx,
        "console.log('r=' + [Editor.GetCookie('window', 'count'), Editor.GetCookie('document', 'name'), \
         Editor.GetCookie('window', 'name'), Editor.GetCookieNames('window'), \
         Editor.GetDrawSwitch()].join('|'));",
    );
    assert!(results(&output).contains(&"41|日本語||count|0"), "{output}");
}

#[gpui::test]
fn the_tab_width_and_the_wrap_column_change_for_the_window(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (f, cx) = setup(cx, "abcdefghijklmnopqrstuvwxyz");
    let output = f.script(
        cx,
        "const t = Editor.ChangeTabWidth(8);\n\
         const w = Editor.ChangeWrapColumn(10);\n\
         console.log('r=' + [t, w, Editor.ChangeTabWidth(0), Editor.GetStrWidth('\\t'), \
         Editor.LineColumnToIndex(2, 3), Editor.LogicToLayoutLineNum(1, 25), \
         Editor.LayoutToLogicLineNum(3)].join(','));",
    );
    // 4 and no wrapping (10240) before; 8 and 10 after. The second row starts at index 11 (from 1):
    // its third cell is index 13. Index 25 is on the third row.
    assert!(results(&output).contains(&"4,10240,8,8,13,3,1"), "{output}");
}

#[gpui::test]
fn a_message_box_waits_for_the_answer(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (f, cx) = setup(cx, "");
    let mut answers = vec!["キャンセル", "OK", "いいえ"];
    let output = f.script_answering(
        cx,
        "console.log('r=' + Editor.MessageBox('三択', 3));\n\
         console.log('r=' + Editor.InfoMsg('お知らせ'));\n\
         console.log('r=' + Editor.YesNoBox('いいですか'));\n",
        |cx| {
            if cx.has_pending_prompt() && !answers.is_empty() {
                // The macro waits for this answer; the answers come in the order of the calls.
                let button = answers.remove(0);
                cx.simulate_prompt_answer(button);
            }
        },
    );
    // Yes/No/Cancel: Cancel is 2. Info: OK is 1. Yes/No: No is 7.
    assert_eq!(results(&output), ["2", "1", "7"], "{output}");
    assert!(answers.is_empty());
}

/// Answers a "choose a file to open / a folder" dialog when the macro has opened it.
fn answer_paths(cx: &mut VisualTestContext, paths: Option<Vec<PathBuf>>) {
    if cx.did_prompt_for_paths() {
        cx.simulate_path_prompt_response(move |_| paths.clone());
    }
}

#[gpui::test]
fn a_file_dialog_gives_the_path_and_a_cancel_gives_an_empty_string(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let chosen = dir.path().join("選んだ.txt");
    let (f, cx) = setup(cx, "");
    let to_open = chosen.clone();
    let output = f.script_answering(
        cx,
        "console.log('r=' + Editor.FileOpenDialog('a.txt', '*.txt'));\n\
         console.log('r=' + Editor.FolderDialog('message', ''));\n",
        |cx| answer_paths(cx, Some(vec![to_open.clone()])),
    );
    assert_eq!(
        results(&output),
        [
            chosen.to_string_lossy().as_ref(),
            chosen.to_string_lossy().as_ref()
        ],
        "{output}"
    );
    // Cancelled
    let output = f.script_answering(
        cx,
        "console.log('r=[' + Editor.FolderDialog('message', '') + ']');\n",
        |cx| answer_paths(cx, None),
    );
    assert_eq!(results(&output), ["[]"], "{output}");
}

#[gpui::test]
fn the_save_dialog_gives_the_path_and_a_cancel_gives_an_empty_string(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("保存.txt");
    let (f, cx) = setup(cx, "");
    // The platform of the tests tells no one when a save dialog opens, so the answer is tried
    // until the dialog is there.
    let mut answers = vec![Some(target.clone()), None];
    let output = f.script_answering(
        cx,
        "console.log('r=[' + Editor.FileSaveDialog('x.txt') + ']');\n\
         console.log('r=[' + Editor.FileSaveDialog() + ']');\n",
        |cx| {
            if let Some(answer) = answers.first().cloned() {
                let tried = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    cx.simulate_new_path_selection(move |_| answer.clone());
                }));
                if tried.is_ok() {
                    answers.remove(0);
                }
            }
        },
    );
    assert_eq!(
        results(&output),
        [format!("[{}]", target.display()).as_str(), "[]"],
        "{output}"
    );
}

#[gpui::test]
fn sleep_waits_for_the_clock_and_the_macro_goes_on(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (f, cx) = setup(cx, "");
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        dir.path(),
        "sleep.cjs",
        "Editor.InsText('a'); console.log('r=' + Editor.Sleep(1000)); Editor.InsText('b');",
    );
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    cx.executor().allow_parking();
    // Nothing moves the clock: the macro is stuck in Sleep after the first insert.
    for _ in 0..60 {
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(f.running(cx), "{}", f.output(cx));
    assert_eq!(f.lines(cx), ["a"]);
    // The clock reaches the time.
    f.wait_until_done_advancing(cx);
    assert_eq!(f.lines(cx), ["ab"]);
    assert_eq!(results(&f.output(cx)), ["0"]);
}

impl Fixture {
    fn wait_until_done_advancing(&self, cx: &mut VisualTestContext) {
        cx.executor().allow_parking();
        for _ in 0..1500 {
            cx.executor().advance_clock(Duration::from_millis(500));
            cx.run_until_parked();
            if !self.running(cx) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("the macro did not end: {}", self.output(cx));
    }
}

#[gpui::test]
fn expand_parameter_knows_the_macro_file(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (f, cx) = setup(cx, "abc");
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        dir.path(),
        "whoami.cjs",
        "console.log('r=' + Editor.ExpandParameter('$M|$f|$y,$x'));",
    );
    f.run_script(cx, &path, |_| {});
    let expected = format!("{}|(無題)|1,1", path.display());
    assert_eq!(
        results(&f.output(cx)),
        [expected.as_str()],
        "{}",
        f.output(cx)
    );
}

#[gpui::test]
fn functions_without_a_window_part_say_so_clearly(cx: &mut TestAppContext) {
    // InputBox has no dialog yet: the macro gets the "not implemented" error, and ends with it.
    if !available(Language::JavaScript) {
        return;
    }
    let (f, cx) = setup(cx, "");
    let output = f.script(
        cx,
        "try { Editor.InputBox('x'); } catch (e) { console.log('r=' + e.code + ' ' + e.message); }",
    );
    let line = results(&output);
    let [text] = line.as_slice() else {
        panic!("{output}");
    };
    assert!(text.starts_with("-32001 "), "{output}");
}

/// `IsCurTypeExt` and `IsSameTypeExt` compare the document types of the extensions
/// (`GetDocumentTypeOfExt`): the built-in types of Sakura, also without a settings file.
#[gpui::test]
fn the_extension_functions_use_the_document_types(cx: &mut TestAppContext) {
    let (f, vcx) = setup(cx, "");
    let types = f.view.update(vcx, |view, cx| {
        let env = view.host_env("IsCurTypeExt", cx);
        (
            env.current_type,
            env.type_of("cpp"),
            env.type_of(".h"),
            env.type_of("TXT"),
            env.type_of("zzz"),
        )
    });
    // The new document is the basic type (0); cpp and h are C/C++ (2), txt is テキスト (1).
    assert_eq!(types, (0, 2, 2, 1, 0));
}

mod wait_tests;

mod wait_tab_tests;

/// The warning of a lock that failed is a box of the command: the macro that opened the file
/// waits until it is closed. The other lock is held by the test, which `flock` honors on macOS
/// and Linux while the file stays readable.
#[cfg(unix)]
#[gpui::test]
fn a_macro_waits_for_the_warning_of_a_lock_that_failed(cx: &mut TestAppContext) {
    use ume_platform::file_lock::{FileLock, ShareMode};
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let target = write(dir.path(), "locked.txt", "abc");
    let _other = FileLock::acquire(&target, ShareMode::DenyReadWrite).unwrap();
    let (f, cx) = setup(cx, "");
    cx.update(|_, app| {
        app.set_global(crate::file_ops::prefs::FilePrefs {
            file_share_mode: 1,
            ..Default::default()
        })
    });
    let code = format!(
        "Editor.FileOpen({:?});\nEditor.InsText('Z');\n",
        target.to_string_lossy()
    );
    let script = write(dir.path(), "open.cjs", &code);
    let mut seen_while_boxed = None;
    f.run_script(cx, &script, |cx| {
        if cx.has_pending_prompt() && seen_while_boxed.is_none() {
            seen_while_boxed = Some(f.lines(cx));
            cx.simulate_prompt_answer("OK");
        }
    });
    assert_eq!(seen_while_boxed, Some(vec!["abc".to_owned()]));
    assert_eq!(f.lines(cx), ["Zabc"]);
}
