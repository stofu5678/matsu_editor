//! End-to-end tests of the Lua engine: the built `ume-script` program runs as a macro process
//! under the editor's own session code (`ume_plugin::session`), against a plain text model.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ume_plugin::api;
use ume_plugin::model::TextModel;
use ume_plugin::session::{self, EndReason, HostEvent, Language, MacroConfig, Report, Stream};

const DOCUMENT_PATH: &str = "/work/テスト.txt";

fn config(dir: &tempfile::TempDir, file: &str, code: &str, timeout: Duration) -> MacroConfig {
    let script = dir.path().join(file);
    std::fs::write(&script, code).unwrap();
    let mut config = MacroConfig::for_script(&script, Some(DOCUMENT_PATH.into())).unwrap();
    assert_eq!(config.language, Language::Lua);
    config.interpreter = env!("CARGO_BIN_EXE_ume-script").into();
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
    run_in("macro.lua", code, text, Duration::from_secs(30))
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
        "Editor.InsText('hello ')\nEditor.InsText('world')\nEditor.AddTail('!')",
        "",
    );
    assert_clean_exit(&report);
    assert_eq!(doc.text(), "hello world!");
}

#[test]
fn reads_a_line_and_the_counts() {
    let code = r#"
print(Editor.GetLineCount(0))
print(Editor.GetLineStr(2) == "two\n")
Editor.Jump(3)
print(Editor.GetLineStr(0))
print(Editor.IsTextSelected())
Editor.SelectAll()
print(Editor.IsTextSelected(), #Editor.GetSelectedString(0))
print(Editor.GetFilename())
"#;
    let (report, _) = run(code, "one\ntwo\nthree");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["3", "true", "three", "0", "1\t13", DOCUMENT_PATH]
    );
}

#[test]
fn japanese_text_goes_both_ways() {
    let code = r#"
local line = Editor.GetLineStr(1)
print(line)
Editor.GoFileEnd()
Editor.InsText("、絵文字😀と" .. "日本語")
print("標準出力の日本語")
"#;
    let (report, doc) = run(code, "こんにちは");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["こんにちは", "標準出力の日本語"]);
    assert_eq!(doc.text(), "こんにちは、絵文字😀と日本語");
}

#[test]
fn a_script_in_a_path_with_japanese_runs_and_names_it_in_errors() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("マクロ😀");
    std::fs::create_dir(&folder).unwrap();
    let script = folder.join("あ.lua");
    std::fs::write(&script, "print('ok')\nerror('boom')\n").unwrap();
    let mut config = MacroConfig::for_script(&script, None).unwrap();
    config.interpreter = env!("CARGO_BIN_EXE_ume-script").into();
    let mut doc = TextModel::new("", None);
    let report =
        session::run_blocking(config, |m| api::handle_message(&mut doc, m)).expect("starts");
    assert_eq!(stdout(&report), ["ok"]);
    assert_eq!(report.finished.exit_code, Some(1));
    assert!(
        stderr(&report).contains("あ.lua:2: boom"),
        "{}",
        stderr(&report)
    );
}

#[test]
fn a_runtime_error_shows_the_line_number_of_the_script() {
    let (report, _) = run("local a = 1\nlocal b = 2\nundefined_function()\n", "");
    assert_eq!(report.finished.reason, EndReason::Exited);
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("macro.lua:3:"), "{err}");
    assert!(err.contains("undefined_function"), "{err}");
}

#[test]
fn a_syntax_error_shows_the_line_number() {
    let (report, _) = run("local a = 1\nlocal b = 2\nlocal c = = 3\n", "");
    assert_eq!(report.finished.exit_code, Some(1));
    assert!(
        stderr(&report).contains("macro.lua:3:"),
        "{}",
        stderr(&report)
    );
}

#[test]
fn an_error_response_becomes_an_editor_error() {
    let code = r#"
local ok, e = pcall(Editor.GetLineStr, 'x')
print(ok, getmetatable(e) == EditorError, e.name, e.code, e.where:find("macro.lua:2:", 1, true) ~= nil)
local ok, e = pcall(function() Editor.GetLineCount(5) end)
print(e.code, e.message)
print(tostring(e))
Editor.Jump(1, 2)
print('not reached')
"#;
    let (report, _) = run(code, "a");
    let lines = stdout(&report);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(lines[0], "false\ttrue\tEditorError\t-32602\ttrue");
    assert!(lines[1].starts_with("-32602\tGetLineCount: "), "{lines:?}");
    // A caught error shows where the script called the function.
    assert!(
        lines[2].contains("macro.lua:4: EditorError -32602: GetLineCount: "),
        "{lines:?}"
    );
    // The third call is not caught: the macro ends with an error that names the call and the line.
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("Jump: PL/SQL"), "{err}");
    assert!(err.contains("macro.lua:7:"), "{err}");
}

#[test]
fn what_cannot_be_sent_is_a_bad_argument() {
    let code = r#"
local ok, e = pcall(Editor.InsText, {})
print(ok, e.code, e.message)
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["false\t-32602\tInsText: argument \"text\" must be string, got table"]
    );
}

#[test]
fn a_string_that_is_not_utf8_is_refused_not_replaced() {
    let code = r#"
local ok, e = pcall(Editor.InsText, "a" .. string.char(255))
print(ok, e.code, e.message)
"#;
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "false\t-32602\tInsText: argument \"text\" must be string, got a string that is not valid UTF-8"
        ]
    );
    assert_eq!(doc.text(), "");
}

#[test]
fn a_trailing_nil_is_left_out_and_a_whole_float_is_an_integer() {
    // Jump(line, flag) without the flag means flag 1; the float is what `4 / 2` gives.
    let code = "local flag = nil\nEditor.Jump(4 / 2, flag)\nio.write(Editor.GetLineStr(0))";
    let (report, _) = run(code, "a\nb\nc");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["b"]);
}

#[test]
fn the_script_sees_the_initialize_message() {
    let code = "local i = Editor.info\nprint(i.protocolVersion, i.apiVersion, i.editor.name, i.document.path, i.processId > 0, i.script)";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [format!("1\t1\tUME Editor\t{DOCUMENT_PATH}\ttrue\tnil")]
    );
}

#[test]
fn output_stays_in_order_with_requests() {
    // `io.write` has no line break before the request; the host splits the line at the RS.
    let code = "io.write('abc')\nEditor.InsText('x')\nprint('def')\nio.write('tail')";
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["abc", "def", "tail"]);
    assert_eq!(doc.text(), "x");
}

#[test]
fn stderr_is_user_output() {
    let (report, _) = run("io.stderr:write('エラー出力\\n')\nprint('out')", "");
    assert_clean_exit(&report);
    assert_eq!(report.lines(Stream::Stderr), ["エラー出力"]);
    assert_eq!(stdout(&report), ["out"]);
}

#[test]
fn a_shebang_line_and_a_bom_are_fine_and_keep_the_line_numbers() {
    let (report, _) = run("#!/usr/bin/env lua\nprint(1)\nerror('x')\n", "");
    assert_eq!(stdout(&report), ["1"]);
    assert!(
        stderr(&report).contains("macro.lua:3:"),
        "{}",
        stderr(&report)
    );
    let (report, _) = run("\u{feff}print('bom')", "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["bom"]);
}

#[test]
fn an_infinite_loop_is_stopped_by_the_timeout() {
    let started = Instant::now();
    let (report, _) = run_in(
        "macro.lua",
        "print('start')\nwhile true do end",
        "",
        Duration::from_millis(500),
    );
    assert_eq!(report.finished.reason, EndReason::TimedOut);
    assert_eq!(stdout(&report), ["start"]);
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn the_macro_can_be_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let config = config(
        &dir,
        "macro.lua",
        "print('started')\nwhile true do end",
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
fn an_unknown_engine_and_a_missing_initialize_are_refused() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let host = env!("CARGO_BIN_EXE_ume-script");
    let output = Command::new(host)
        .args(["--engine", "cobol"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown engine"));

    let mut child = Command::new(host)
        .args(["--engine", "lua"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(child.stdin.take().unwrap(), "{{\"method\":\"other\"}}").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}
