//! End-to-end tests of the QuickJS engine: the built `matsu-script` program runs as a macro process
//! under the editor's own session code (`matsu_plugin::session`), against a plain text model.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use matsu_plugin::api;
use matsu_plugin::model::TextModel;
use matsu_plugin::session::{
    self, EndReason, Engine, HostEvent, Language, MacroConfig, Report, Stream,
};

const DOCUMENT_PATH: &str = "/work/テスト.txt";

fn config(dir: &tempfile::TempDir, file: &str, code: &str, timeout: Duration) -> MacroConfig {
    let script = dir.path().join(file);
    std::fs::write(&script, code).unwrap();
    let mut config = MacroConfig::for_script(&script, Some(DOCUMENT_PATH.into())).unwrap();
    assert_eq!(config.language, Language::Bundled(Engine::QuickJs));
    config.interpreter = env!("CARGO_BIN_EXE_matsu-script").into();
    config.timeout = timeout;
    config
}

fn run_in(file: &str, code: &str, text: &str, timeout: Duration) -> (Report, TextModel) {
    let dir = tempfile::tempdir().unwrap();
    let config = config(&dir, file, code, timeout);
    let mut doc = TextModel::new(text, Some(DOCUMENT_PATH.into()));
    let report = session::run_blocking(config, |message| api::handle_message(&mut doc, message))
        .expect("the host starts");
    (report, doc)
}

fn run(code: &str, text: &str) -> (Report, TextModel) {
    run_in("macro.js", code, text, Duration::from_secs(30))
}

fn stdout(report: &Report) -> Vec<&str> {
    report.lines(Stream::Stdout)
}

fn stderr(report: &Report) -> String {
    report.lines(Stream::Stderr).join("\n")
}

fn assert_clean_exit(report: &Report) {
    assert_eq!(
        (report.finished.reason, report.finished.exit_code),
        (EndReason::Exited, Some(0)),
        "stderr: {}",
        stderr(report)
    );
}

#[test]
fn inserts_text() {
    let (report, doc) = run(
        "Editor.InsText('hello ');\nEditor.InsText('world');\nEditor.AddTail('!');",
        "",
    );
    assert_clean_exit(&report);
    assert_eq!(doc.text(), "hello world!");
}

#[test]
fn reads_a_line_and_the_counts() {
    let code = r#"
console.log(Editor.GetLineCount(0));
console.log(Editor.GetLineStr(2) === "two\n");
Editor.Jump(3);
console.log(Editor.GetLineStr(0));
console.log(Editor.IsTextSelected());
Editor.SelectAll();
console.log(Editor.IsTextSelected(), Editor.GetSelectedString(0).length);
console.log(Editor.GetFilename());
"#;
    let (report, _) = run(code, "one\ntwo\nthree");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["3", "true", "three", "0", "1 13", DOCUMENT_PATH]
    );
}

#[test]
fn japanese_text_goes_both_ways() {
    let code = r#"
const line = Editor.GetLineStr(1);
console.log(line);
Editor.GoFileEnd();
Editor.InsText("、絵文字😀と" + "日本語");
console.log("標準出力の日本語", "😀".length);
"#;
    let (report, doc) = run(code, "こんにちは");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["こんにちは", "標準出力の日本語 2"]);
    assert_eq!(doc.text(), "こんにちは、絵文字😀と日本語");
}

#[test]
fn a_script_in_a_path_with_japanese_runs_and_names_it_in_errors() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("マクロ😀");
    std::fs::create_dir(&folder).unwrap();
    let script = folder.join("あ.js");
    std::fs::write(&script, "console.log('ok');\nthrow new Error('boom');\n").unwrap();
    let mut config = MacroConfig::for_script(&script, None).unwrap();
    config.interpreter = env!("CARGO_BIN_EXE_matsu-script").into();
    let mut doc = TextModel::new("", None);
    let report =
        session::run_blocking(config, |m| api::handle_message(&mut doc, m)).expect("starts");
    assert_eq!(stdout(&report), ["ok"]);
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("Error: boom"), "{err}");
    assert!(err.contains("あ.js:2"), "{err}");
}

#[test]
fn a_runtime_error_shows_the_line_number_of_the_script() {
    let (report, _) = run("const a = 1;\nconst b = 2;\nundefinedFunction();\n", "");
    assert_eq!(report.finished.reason, EndReason::Exited);
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("macro.js:3"), "{err}");
    assert!(err.contains("undefinedFunction"), "{err}");
}

#[test]
fn a_syntax_error_shows_the_line_number() {
    let (report, _) = run("const a = 1;\nconst b = 2;\nconst c = = 3;\n", "");
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("SyntaxError"), "{err}");
    assert!(err.contains("macro.js:3"), "{err}");
}

#[test]
fn a_thrown_value_that_is_not_an_error_is_shown() {
    let (report, _) = run("throw 'plain text';", "");
    assert_eq!(report.finished.exit_code, Some(1));
    assert!(
        stderr(&report).contains("Uncaught plain text"),
        "{}",
        stderr(&report)
    );
}

#[test]
fn an_error_response_becomes_an_editor_error() {
    let code = r#"
try {
  Editor.GetLineStr('x');
} catch (e) {
  console.log(e instanceof EditorError, e instanceof Error, e.name, e.code);
  console.log(e.stack.split('\n')[0].includes('macro.js:3'));
}
try {
  Editor.GetLineCount(5);
} catch (e) {
  console.log(e.code, e.message);
  console.log(String(e));
}
Editor.Jump(1, 2);
console.log('not reached');
"#;
    let (report, _) = run(code, "a");
    let lines = stdout(&report);
    assert_eq!(lines.len(), 4, "{lines:?}");
    assert_eq!(lines[0], "true true EditorError -32602");
    assert_eq!(lines[1], "true");
    assert!(lines[2].starts_with("-32602 GetLineCount: "), "{lines:?}");
    assert!(
        lines[3].starts_with("EditorError: GetLineCount: "),
        "{lines:?}"
    );
    // The third call is not caught: the macro ends with an error that names the call and the line.
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("EditorError -32602: Jump: PL/SQL"), "{err}");
    assert!(err.contains("macro.js:14"), "{err}");
    // The frames of the boot are not shown.
    assert!(!err.contains("matsu-script:boot"), "{err}");
}

#[test]
fn what_cannot_be_sent_is_a_bad_argument() {
    let code = r#"
for (const bad of [() => 1, Symbol('s'), 10n]) {
  try {
    Editor.InsText(bad);
  } catch (e) {
    console.log(e.code, e.message);
  }
}
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "-32602 InsText: argument \"text\" must be string, got function",
            "-32602 InsText: argument \"text\" must be string, got symbol",
            "-32602 InsText: argument \"text\" must be string, got bigint",
        ]
    );
}

#[test]
fn a_string_with_a_lone_surrogate_is_refused_not_replaced() {
    let code = r#"
try {
  Editor.InsText("a" + String.fromCharCode(0xd800));
} catch (e) {
  console.log(e.code, e.message);
}
"#;
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    let lines = stdout(&report);
    assert!(
        lines[0].starts_with("-32602 InsText: argument \"text\" must be string, got a string that is not valid UTF-16"),
        "{lines:?}"
    );
    assert_eq!(doc.text(), "");
}

#[test]
fn a_trailing_undefined_is_left_out_and_a_whole_float_is_an_integer() {
    // Jump(line, flag) without the flag means flag 1; `4 / 2` is a float in QuickJS.
    let code = "let flag;\nEditor.Jump(4 / 2, flag);\nconsole.log(Editor.GetLineStr(0).trim());";
    let (report, _) = run(code, "a\nb\nc");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["b"]);
}

#[test]
fn the_script_sees_the_initialize_message() {
    let code = "const i = Editor.info;\nconsole.log(i.protocolVersion, i.apiVersion, i.editor.name, i.document.path, i.processId > 0, i.script);";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [format!("1 1 MATSU Editor {DOCUMENT_PATH} true undefined")]
    );
}

#[test]
fn output_stays_in_order_with_requests() {
    let code =
        "console.log('abc');\nEditor.InsText('x');\nconsole.log('def');\nconsole.log('tail');";
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["abc", "def", "tail"]);
    assert_eq!(doc.text(), "x");
}

#[test]
fn console_formats_values_and_error_goes_to_stderr() {
    let code = r#"
console.log({ a: 1, b: [1, 2] }, [null, 'x'], undefined, null, 1.5, true);
console.error('エラー出力');
console.warn('warn');
console.info('info');
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            r#"{"a":1,"b":[1,2]} [null,"x"] undefined null 1.5 true"#,
            "info"
        ]
    );
    assert_eq!(report.lines(Stream::Stderr), ["エラー出力", "warn"]);
}

#[test]
fn a_script_is_a_classic_script_not_a_module() {
    // `var` at the top level is a property of the global object; `this` is the global object.
    let code = "var a = 1;\nconsole.log(globalThis.a, this === globalThis, typeof require, typeof module);";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["1 true undefined undefined"]);
}

#[test]
fn a_shebang_line_and_a_bom_are_fine_and_keep_the_line_numbers() {
    let (report, _) = run(
        "#!/usr/bin/env node\nconsole.log(1);\nthrow new Error('x');\n",
        "",
    );
    assert_eq!(stdout(&report), ["1"]);
    assert!(
        stderr(&report).contains("macro.js:3"),
        "{}",
        stderr(&report)
    );
    let (report, _) = run("\u{feff}console.log('bom');", "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["bom"]);
}

#[test]
fn promises_settle_after_the_script_and_an_unhandled_rejection_ends_the_macro() {
    let code = "Promise.resolve().then(() => console.log('later'));\nconsole.log('first');";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["first", "later"]);

    let (report, _) = run("(async () => { throw new Error('async boom'); })();", "");
    assert_eq!(report.finished.exit_code, Some(1));
    assert!(
        stderr(&report).contains("Error: async boom"),
        "{}",
        stderr(&report)
    );
}

#[test]
fn a_rejection_that_is_handled_does_not_hide_one_that_is_not() {
    let code = "Promise.reject(new Error('lost A'));\nconst b = Promise.reject(new Error('B'));\nb.catch(() => console.log('B handled'));";
    let (report, _) = run(code, "");
    assert_eq!(stdout(&report), ["B handled"]);
    assert_eq!(report.finished.exit_code, Some(1));
    assert!(
        stderr(&report).contains("Error: lost A"),
        "{}",
        stderr(&report)
    );

    let (report, _) = run(
        "queueMicrotask(() => { throw new Error('in a job'); });",
        "",
    );
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(
        err.contains("Error: in a job") && err.contains("macro.js:1"),
        "{err}"
    );

    let (report, _) = run("Promise.reject(new Error('B')).catch(() => {});", "");
    assert_clean_exit(&report);
}

#[test]
fn a_runaway_recursion_is_an_error_not_a_crash() {
    let code = "function f() { return f() + 1; }\ntry { f(); } catch (e) { console.log(e.name); }";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["RangeError"]);
}

#[test]
fn the_sample_macro_runs() {
    let sample =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../matsu-plugin/samples/sample.js");
    let mut config = MacroConfig::for_script(&sample, None).unwrap();
    assert_eq!(config.language, Language::Bundled(Engine::QuickJs));
    config.interpreter = env!("CARGO_BIN_EXE_matsu-script").into();
    let mut doc = TextModel::new("りんご\nみかん😀\nぶどう\n", None);
    let report =
        session::run_blocking(config, |m| api::handle_message(&mut doc, m)).expect("starts");
    assert_clean_exit(&report);
    assert!(
        doc.text().starts_with("# サンプルマクロ 😀\n"),
        "{}",
        doc.text()
    );
    assert!(
        doc.text().contains("みかん😀 (置き換えました)"),
        "{}",
        doc.text()
    );
    assert!(
        doc.text().ends_with("-- マクロの終わり --\n"),
        "{}",
        doc.text()
    );
    assert!(
        stdout(&report)
            .iter()
            .any(|l| l.starts_with("エラー: -32602")),
        "{:?}",
        stdout(&report)
    );
}

#[test]
fn an_infinite_loop_is_stopped_by_the_timeout() {
    let started = Instant::now();
    let (report, _) = run_in(
        "macro.js",
        "console.log('start');\nwhile (true) {}",
        "",
        Duration::from_secs(2),
    );
    assert_eq!(report.finished.reason, EndReason::TimedOut);
    assert_eq!(stdout(&report), ["start"]);
    // only a guard against a macro that is never stopped; the timeout above is 2 s
    assert!(started.elapsed() < Duration::from_secs(60));
}

#[test]
fn the_macro_can_be_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let config = config(
        &dir,
        "macro.js",
        "console.log('started');\nwhile (true) {}",
        Duration::from_secs(60),
    );
    let (tx, rx) = mpsc::channel();
    let session = session::start(
        config,
        Arc::new(move |event| {
            let _ = tx.send(event);
        }),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left).expect("no event") {
            HostEvent::Output { text, .. } if text == "started" => session.cancel(),
            HostEvent::Finished(finished) => {
                assert_eq!(finished.reason, EndReason::Cancelled);
                break;
            }
            _ => {}
        }
    }
}

#[test]
fn node_extensions_stay_on_node() {
    for name in ["a.mjs", "a.cjs"] {
        let config = MacroConfig::for_script(std::path::Path::new(name), None).unwrap();
        assert_eq!(config.language, Language::JavaScript, "{name}");
    }
}
