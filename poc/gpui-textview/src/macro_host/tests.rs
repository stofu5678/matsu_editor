//! Integration tests: they run the real interpreters (Node.js and Python) as macros against a
//! plain text model. A test whose interpreter is not installed prints "SKIP" and passes. (Run
//! with `--nocapture` to see which ones were skipped.)

use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::api::{self, MacroDocument};
use super::model::TextModel;
use super::session::{self, EndReason, HostEvent, Language, MacroConfig, Report, Stream};

/// Whether the interpreter starts. Prints why not if it does not.
fn available(language: Language) -> bool {
    let interpreter = language.interpreter();
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
            "SKIP: {} ({}) is not available; set {} to run this test",
            language.display_name(),
            interpreter.to_string_lossy(),
            language.env_var()
        );
    }
    ok
}

fn config_for(
    dir: &tempfile::TempDir,
    language: Language,
    code: &str,
    timeout: Duration,
) -> MacroConfig {
    let name = match language {
        Language::JavaScript => "macro.js",
        Language::Python => "macro.py",
    };
    let script = dir.path().join(name);
    std::fs::write(&script, code).unwrap();
    let mut config = MacroConfig::for_script(&script, Some("/work/テスト.txt".into())).unwrap();
    config.timeout = timeout;
    config
}

/// Runs the macro on a text. `None` when the interpreter is missing.
fn run(language: Language, code: &str, text: &str) -> Option<(Report, TextModel)> {
    run_with(
        language,
        code,
        text,
        Duration::from_secs(30),
        |doc, message| api::handle_message(doc, message),
    )
}

fn run_with(
    language: Language,
    code: &str,
    text: &str,
    timeout: Duration,
    mut handler: impl FnMut(&mut TextModel, &str) -> Option<String>,
) -> Option<(Report, TextModel)> {
    if !available(language) {
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let config = config_for(&dir, language, code, timeout);
    let mut doc = TextModel::new(text, Some("/work/テスト.txt".into()));
    let report = session::run_blocking(config, |message| handler(&mut doc, message)).unwrap();
    Some((report, doc))
}

fn stdout(report: &Report) -> Vec<&str> {
    report.lines(Stream::Stdout)
}

fn assert_clean_exit(report: &Report) {
    assert_eq!(
        (report.finished.reason, report.finished.exit_code),
        (EndReason::Exited, Some(0)),
        "stderr: {:?}",
        report.lines(Stream::Stderr)
    );
}

const JS: Language = Language::JavaScript;
const PY: Language = Language::Python;

#[test]
fn japanese_and_emoji_round_trip() {
    let js = r#"
const line = Editor.GetLineStr(1);
Editor.AddTail('追記: ' + line.trim() + '😀\n');
console.log('行数', Editor.GetLineCount(0), Editor.GetFilename());
"#;
    let py = r#"
line = Editor.GetLineStr(1)
Editor.AddTail('追記: ' + line.strip() + '😀\n')
print('行数', Editor.GetLineCount(0), Editor.GetFilename())
"#;
    for (language, code) in [(JS, js), (PY, py)] {
        let Some((report, doc)) = run(language, code, "日本語のテキスト👨‍👩‍👧\n")
        else {
            continue;
        };
        assert_clean_exit(&report);
        assert_eq!(
            doc.text(),
            "日本語のテキスト👨‍👩‍👧\n追記: 日本語のテキスト👨‍👩‍👧😀\n"
        );
        assert_eq!(stdout(&report), ["行数 2 /work/テスト.txt"], "{language:?}");
    }
}

#[test]
fn a_slow_answer_is_waited_for() {
    let js = "Editor.InsText('x'); console.log(Editor.GetLineStr(1));";
    let py = "Editor.InsText('x'); print(Editor.GetLineStr(1))";
    for (language, code) in [(JS, js), (PY, py)] {
        let started = Instant::now();
        let Some((report, _)) = run_with(
            language,
            code,
            "abc",
            Duration::from_secs(30),
            |doc, message| {
                // The editor takes its time with the first call.
                if message.contains("InsText") {
                    std::thread::sleep(Duration::from_millis(1500));
                }
                api::handle_message(doc, message)
            },
        ) else {
            continue;
        };
        assert_clean_exit(&report);
        assert_eq!(stdout(&report), ["xabc"]);
        assert!(started.elapsed() >= Duration::from_millis(1500));
    }
}

#[test]
fn a_one_megabyte_line_goes_both_ways() {
    // 350,000 characters of 3 bytes: more than 1 MB in UTF-8 and 2 MB as \uXXXX on the wire.
    let line = "あ".repeat(350_000);
    let js = "const s = Editor.GetLineStr(1); Editor.AddTail(s); console.log(s.length, Editor.GetLineCount(0));";
    let py = "s = Editor.GetLineStr(1); Editor.AddTail(s); print(len(s), Editor.GetLineCount(0))";
    for (language, code) in [(JS, js), (PY, py)] {
        let Some((report, doc)) = run(language, code, &format!("{line}\n")) else {
            continue;
        };
        assert_clean_exit(&report);
        // The macro read the line and its line break, and wrote it back at the end.
        assert_eq!(stdout(&report), ["350001 2"], "{language:?}");
        assert_eq!(doc.text(), format!("{line}\n{line}\n"));
    }
}

#[test]
fn output_without_a_line_break_before_a_request() {
    let js = "process.stdout.write('abc'); Editor.GoFileTop(); console.log('def'); process.stdout.write('tail');";
    let py = "print('abc', end=''); Editor.GoFileTop(); print('def'); print('tail', end='')";
    for (language, code) in [(JS, js), (PY, py)] {
        let mut requests = 0;
        let Some((report, _)) = run_with(
            language,
            code,
            "x",
            Duration::from_secs(30),
            |doc, message| {
                requests += 1;
                api::handle_message(doc, message)
            },
        ) else {
            continue;
        };
        assert_clean_exit(&report);
        assert_eq!(requests, 1, "{language:?}");
        // "abc" came before the request on the same line; the last output has no line break.
        assert_eq!(stdout(&report), ["abc", "def", "tail"], "{language:?}");
    }
}

#[test]
fn a_syntax_error_shows_the_line_number() {
    let js = "const a = 1;\nconst b = 2;\nconst c = ;\n";
    let py = "a = 1\nb = 2\nc = (\n";
    for (language, code) in [(JS, js), (PY, py)] {
        let Some((report, _)) = run(language, code, "") else {
            continue;
        };
        assert_eq!(report.finished.reason, EndReason::Exited);
        assert_ne!(report.finished.exit_code, Some(0));
        let stderr = report.lines(Stream::Stderr).join("\n");
        let (file, line) = match language {
            Language::JavaScript => ("macro.js:3", ""),
            Language::Python => ("macro.py", "line 3"),
        };
        assert!(
            stderr.contains(file) && stderr.contains(line),
            "{language:?}: {stderr}"
        );
    }
}

#[test]
fn a_runtime_error_shows_the_line_number_of_the_users_file() {
    let js = "const a = 1;\nconst b = 2;\nundefinedFunction();\n";
    let py = "a = 1\nb = 2\nundefined_function()\n";
    for (language, code) in [(JS, js), (PY, py)] {
        let Some((report, _)) = run(language, code, "") else {
            continue;
        };
        assert_ne!(report.finished.exit_code, Some(0));
        let stderr = report.lines(Stream::Stderr).join("\n");
        let want = match language {
            Language::JavaScript => "macro.js:3",
            Language::Python => "macro.py\", line 3",
        };
        assert!(stderr.contains(want), "{language:?}: {stderr}");
    }
}

#[test]
fn an_error_response_becomes_an_exception() {
    let js = r#"
try { Editor.GetLineStr('x'); } catch (e) { console.log('caught', e.name, e.code, e instanceof Error, e instanceof EditorError); }
try { Editor.GetLineCount(5); } catch (e) { console.log('caught', e.code, e.message); }
Editor.Jump(1, 2);
console.log('not reached');
"#;
    let py = r#"
try:
    Editor.GetLineStr('x')
except EditorError as e:
    print('caught', type(e).__name__, e.code, isinstance(e, Exception))
try:
    Editor.GetLineCount(5)
except EditorError as e:
    print('caught', e.code, e.message)
Editor.Jump(1, 2)
print('not reached')
"#;
    for (language, code) in [(JS, js), (PY, py)] {
        let Some((report, _)) = run(language, code, "a") else {
            continue;
        };
        let lines = stdout(&report);
        assert_eq!(lines.len(), 2, "{language:?}: {lines:?}");
        match language {
            Language::JavaScript => assert_eq!(lines[0], "caught EditorError -32602 true true"),
            Language::Python => assert_eq!(lines[0], "caught EditorError -32602 True"),
        }
        assert!(
            lines[1].starts_with("caught -32602 GetLineCount: "),
            "{lines:?}"
        );
        // The third call is not caught: the macro ends with an error that names the call.
        assert_ne!(report.finished.exit_code, Some(0));
        let stderr = report.lines(Stream::Stderr).join("\n");
        assert!(stderr.contains("Jump: PL/SQL"), "{language:?}: {stderr}");
        assert!(stderr.contains("macro."), "{language:?}: {stderr}");
    }
}

#[test]
fn a_trailing_undefined_or_unset_argument_is_left_out() {
    // Jump(line, flag) with the flag missing means flag 1; an undefined flag must not be null.
    let js =
        "const flag = undefined; Editor.Jump(2, flag); process.stdout.write(Editor.GetLineStr(0));";
    let py = "Editor.Jump(2); print(Editor.GetLineStr(0), end='')";
    for (language, code) in [(JS, js), (PY, py)] {
        let Some((report, _)) = run(language, code, "a\nb\nc") else {
            continue;
        };
        assert_clean_exit(&report);
        assert_eq!(stdout(&report), ["b"], "{language:?}");
    }
}

#[test]
fn an_infinite_loop_is_stopped_by_the_timeout() {
    for (language, code) in [(JS, "while (true) {}"), (PY, "while True:\n    pass")] {
        let started = Instant::now();
        let Some((report, _)) =
            run_with(language, code, "", Duration::from_millis(700), |doc, m| {
                api::handle_message(doc, m)
            })
        else {
            continue;
        };
        assert_eq!(report.finished.reason, EndReason::TimedOut, "{language:?}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}

#[test]
fn a_blocked_macro_is_stopped_by_the_timeout() {
    // Waiting for an answer that never comes is also a hang.
    let js = "Editor.GoFileTop();";
    let py = "Editor.GoFileTop()";
    for (language, code) in [(JS, js), (PY, py)] {
        let Some((report, _)) =
            run_with(language, code, "", Duration::from_millis(700), |_, _| None)
        else {
            continue;
        };
        assert_eq!(report.finished.reason, EndReason::TimedOut, "{language:?}");
    }
}

#[test]
fn a_macro_that_exits_at_once_ends_cleanly() {
    for (language, code) in [(JS, "// nothing"), (PY, "pass"), (JS, ""), (PY, "")] {
        let mut requests = 0;
        let Some((report, _)) = run_with(language, code, "", Duration::from_secs(30), |doc, m| {
            requests += 1;
            api::handle_message(doc, m)
        }) else {
            continue;
        };
        assert_clean_exit(&report);
        assert_eq!(requests, 0);
        assert!(report.output.is_empty(), "{:?}", report.output);
    }
}

#[test]
fn the_exit_code_is_reported() {
    for (language, code) in [
        (JS, "console.log('bye'); process.exit(3);"),
        (PY, "print('bye')\nimport sys\nsys.exit(3)"),
    ] {
        let Some((report, _)) = run(language, code, "") else {
            continue;
        };
        assert_eq!(report.finished.exit_code, Some(3), "{language:?}");
        assert_eq!(stdout(&report), ["bye"]);
    }
}

#[test]
fn python_prints_japanese_in_utf8() {
    let Some((report, _)) = run(
        PY,
        "print('日本語😀'); import sys; print(sys.stdout.encoding, sys.flags.utf8_mode)",
        "",
    ) else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["日本語😀", "utf-8 1"]);
}

#[test]
fn stderr_is_user_output() {
    let js = "console.error('エラー出力'); console.log('out');";
    let py = "import sys\nprint('エラー出力', file=sys.stderr)\nprint('out')";
    for (language, code) in [(JS, js), (PY, py)] {
        let Some((report, _)) = run(language, code, "") else {
            continue;
        };
        assert_clean_exit(&report);
        assert_eq!(report.lines(Stream::Stderr), ["エラー出力"]);
        assert_eq!(stdout(&report), ["out"]);
    }
}

#[test]
fn the_macro_sees_the_initialize_message() {
    let js = "const i = Editor.info; console.log(i.protocolVersion, i.apiVersion, i.editor.name, i.document.path, i.processId > 0);";
    let py = "i = Editor.info; print(i['protocolVersion'], i['apiVersion'], i['editor']['name'], i['document']['path'], i['processId'] > 0)";
    for (language, code) in [(JS, js), (PY, py)] {
        let Some((report, _)) = run(language, code, "") else {
            continue;
        };
        assert_clean_exit(&report);
        let want = match language {
            Language::JavaScript => "1 1 UME Editor /work/テスト.txt true",
            Language::Python => "1 1 UME Editor /work/テスト.txt True",
        };
        assert_eq!(stdout(&report), [want]);
    }
}

#[test]
fn malformed_requests_get_json_rpc_errors() {
    // The macro writes raw lines (Python, without the generated API) and prints the error codes.
    let py = r#"
import sys, json
def raw(text):
    sys.stdout.write("\x1e" + text + "\n")
    sys.stdout.flush()
    return json.loads(sys.stdin.readline())
print(raw('{"jsonrpc":"2.0","id":1,"method":"NoSuchMethod","params":[]}')["error"]["code"])
print(raw('{oops')["error"]["code"])
print(raw('[1]')["error"]["code"])
print(raw('{"jsonrpc":"2.0","id":2,"method":"GetLineCount","params":["x"]}')["error"]["code"])
print(raw('{"jsonrpc":"2.0","id":3,"method":"GetLineCount","params":[0]}')["result"])
"#;
    let Some((report, _)) = run(PY, py, "a\nb\n") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["-32601", "-32700", "-32600", "-32602", "2"]
    );
}

#[test]
fn cancel_kills_the_macro() {
    if !available(JS) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let config = config_for(
        &dir,
        JS,
        "console.log('started'); while (true) {}",
        Duration::from_secs(60),
    );
    let (tx, rx) = mpsc::channel();
    let sink: session::Sink = Arc::new(move |event| {
        let _ = tx.send(event);
    });
    let session = session::start(config, sink).unwrap();
    let started = Instant::now();
    let mut cancelled = false;
    loop {
        match rx.recv_timeout(Duration::from_secs(20)).expect("no event") {
            HostEvent::Output { text, .. } if text == "started" && !cancelled => {
                session.cancel();
                cancelled = true;
            }
            HostEvent::Finished(finished) => {
                assert_eq!(finished.reason, EndReason::Cancelled);
                break;
            }
            _ => {}
        }
    }
    assert!(cancelled && started.elapsed() < Duration::from_secs(10));
}

#[test]
fn a_missing_interpreter_is_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = config_for(&dir, JS, "", Duration::from_secs(5));
    config.interpreter = "ume-no-such-interpreter".into();
    let err = session::run_blocking(config, |_| None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("Node.js") && err.contains("POC_NODE"), "{err}");
    let err = MacroConfig::for_script(&dir.path().join("a.txt"), None)
        .err()
        .unwrap();
    assert!(err.to_string().contains(".js"), "{err}");
}

#[test]
fn the_sample_macros_run() {
    // The samples in macros/ change the text and print what they did.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("macros");
    for (language, name) in [(JS, "sample.js"), (PY, "sample.py")] {
        if !available(language) {
            continue;
        }
        let config = MacroConfig::for_script(&dir.join(name), None).unwrap();
        let mut doc = TextModel::new("りんご\nみかん😀\nぶどう\n", None);
        let report = session::run_blocking(config, |m| api::handle_message(&mut doc, m)).unwrap();
        assert_clean_exit(&report);
        let text = doc.text();
        assert!(text.starts_with("# サンプルマクロ"), "{name}: {text}");
        assert!(text.contains("みかん😀"), "{name}: {text}");
        assert!(!stdout(&report).is_empty());
    }
}

#[test]
fn generated_code_has_every_function() {
    let js = super::codegen::generate_js();
    let py = super::codegen::generate_py();
    for func in api::FUNCS {
        assert!(js.contains(&format!("{}(", func.name)), "{}", func.name);
        assert!(py.contains(&format!("def {}(", func.name)), "{}", func.name);
    }
    // The model answers every function of the table (no function without code).
    let mut doc = TextModel::new("a", None);
    for func in api::FUNCS {
        let params: Vec<Value> = func
            .params
            .iter()
            .map(|p| match p.ty {
                api::Ty::Int => Value::from(if func.name == "Jump" { 1 } else { 0 }),
                api::Ty::Str => Value::from("x"),
                api::Ty::Null => Value::Null,
            })
            .collect();
        assert!(
            api::call(&mut doc as &mut dyn MacroDocument, func.name, &params).is_ok(),
            "{}",
            func.name
        );
    }
}
