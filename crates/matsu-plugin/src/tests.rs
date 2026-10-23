//! Integration tests: they run the real interpreters (Node.js, Python, Ruby and PHP) as macros
//! against a plain text model. A test whose interpreter is not installed prints "SKIP" and passes. (Run
//! with `--nocapture` to see which ones were skipped.)

use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::api::{self, MacroDocument};
use crate::model::TextModel;
use crate::session::{self, EndReason, HostEvent, Language, MacroConfig, Report, Stream};

/// The time limit of a test macro that is meant to end by itself. The limit counts the time the
/// interpreter takes to start and the host to answer, so on a busy machine (a CI runner that
/// builds other jobs) 30 seconds was not enough for a megabyte line or a syntax error. It does
/// not slow a test down: a macro that ends stops the wait at once, and only a macro that hangs
/// waits this long. The tests of the timeout itself set their own short limits.
const PATIENT: Duration = Duration::from_secs(300);

/// Whether the interpreter starts. Prints why not if it does not.
fn available(language: Language) -> bool {
    let interpreter = language.default_interpreter();
    let output = Command::new(&interpreter)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success());
    // A Node.js that cannot strip types is as good as missing for the TypeScript tests.
    let ok = output.is_some_and(|output| {
        language != Language::TypeScript
            || session::parse_node_version(&String::from_utf8_lossy(&output.stdout))
                .is_some_and(session::node_strips_types)
    });
    if !ok {
        eprintln!(
            "SKIP: {} ({}) is not available",
            language.display_name(),
            interpreter.to_string_lossy()
        );
        // On a CI runner that has every interpreter, a missing one is a failure, not a skip:
        // otherwise the tests of that language would pass without running.
        if std::env::var_os("MATSU_REQUIRE_INTERPRETERS").is_some_and(|v| !v.is_empty()) {
            panic!(
                "{} is required here (MATSU_REQUIRE_INTERPRETERS)",
                language.display_name()
            );
        }
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
        Language::JavaScript => "macro.cjs",
        Language::TypeScript => "macro.ts",
        Language::Python => "macro.py",
        Language::Ruby => "macro.rb",
        Language::Php => "macro.php",
        Language::Bundled(_) => unreachable!("the bundled engines are tested in matsu-script"),
    };
    let script = dir.path().join(name);
    // A PHP file starts with `<?php`. It goes on the first line, so the line numbers of the
    // snippets stay the same as in the other languages.
    let code = if language == Language::Php && !code.starts_with("<?php") {
        format!("<?php {code}")
    } else {
        code.to_string()
    };
    std::fs::write(&script, code).unwrap();
    let mut config = MacroConfig::for_script(&script, Some("/work/テスト.txt".into())).unwrap();
    config.timeout = timeout;
    // The boot folder goes next to the script, so that a test can see whether it was removed.
    config.temp_dir = Some(dir.path().to_path_buf());
    config
}

/// The boot folders (`matsu-macro-*`) that a run left in `dir`.
fn boot_folders(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("matsu-macro-"))
        .collect()
}

/// Runs the macro on a text. `None` when the interpreter is missing.
fn run(language: Language, code: &str, text: &str) -> Option<(Report, TextModel)> {
    run_with(language, code, text, PATIENT, |doc, message| {
        api::handle_message(doc, message)
    })
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
    // Every run in these tests must leave no boot folder, whatever the macro did.
    assert_eq!(boot_folders(dir.path()), Vec::<String>::new());
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
const TS: Language = Language::TypeScript;
const PY: Language = Language::Python;
const RB: Language = Language::Ruby;
const PHP: Language = Language::Php;

/// The same macro in each language.
fn cases<'a>(js: &'a str, py: &'a str, rb: &'a str, php: &'a str) -> [(Language, &'a str); 4] {
    [(JS, js), (PY, py), (RB, rb), (PHP, php)]
}

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
    let rb = r##"
line = Editor.GetLineStr(1)
Editor.AddTail('追記: ' + line.strip + "😀\n")
puts "行数 #{Editor.GetLineCount(0)} #{Editor.GetFilename}"
"##;
    let php = r#"
$line = Editor::GetLineStr(1);
Editor::AddTail('追記: ' . trim($line) . "😀\n");
echo '行数 ' . Editor::GetLineCount(0) . ' ' . Editor::GetFilename() . "\n";
"#;
    for (language, code) in cases(js, py, rb, php) {
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
    let rb = "Editor.InsText('x'); puts Editor.GetLineStr(1)";
    let php = "Editor::InsText('x'); echo Editor::GetLineStr(1), \"\\n\";";
    for (language, code) in cases(js, py, rb, php) {
        let started = Instant::now();
        let Some((report, _)) = run_with(language, code, "abc", PATIENT, |doc, message| {
            // The editor takes its time with the first call.
            if message.contains("InsText") {
                std::thread::sleep(Duration::from_millis(1500));
            }
            api::handle_message(doc, message)
        }) else {
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
    let rb = "s = Editor.GetLineStr(1); Editor.AddTail(s); puts \"#{s.length} #{Editor.GetLineCount(0)}\"";
    // PHP counts bytes: 350,001 characters, 350,000 of them 3 bytes.
    let php = "$s = Editor::GetLineStr(1); Editor::AddTail($s); echo strlen($s) . ' ' . Editor::GetLineCount(0) . \"\\n\";";
    for (language, code) in cases(js, py, rb, php) {
        let Some((report, doc)) = run(language, code, &format!("{line}\n")) else {
            continue;
        };
        assert_clean_exit(&report);
        // The macro read the line and its line break, and wrote it back at the end.
        let want = if language == PHP {
            "1050001 2"
        } else {
            "350001 2"
        };
        assert_eq!(stdout(&report), [want], "{language:?}");
        assert_eq!(doc.text(), format!("{line}\n{line}\n"));
    }
}

#[test]
fn output_without_a_line_break_before_a_request() {
    let js = "process.stdout.write('abc'); Editor.GoFileTop(); console.log('def'); process.stdout.write('tail');";
    let py = "print('abc', end=''); Editor.GoFileTop(); print('def'); print('tail', end='')";
    let rb = "$stdout.write('abc'); Editor.GoFileTop; puts 'def'; $stdout.write('tail')";
    let php = "echo 'abc'; Editor::GoFileTop(); echo \"def\\n\"; echo 'tail';";
    for (language, code) in cases(js, py, rb, php) {
        let mut requests = 0;
        let Some((report, _)) = run_with(language, code, "x", PATIENT, |doc, message| {
            requests += 1;
            api::handle_message(doc, message)
        }) else {
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
    let rb = "a = 1\nb = 2\nc = ;\n";
    let php = "$a = 1;\n$b = 2;\n$c = ;\n";
    for (language, code) in cases(js, py, rb, php) {
        let Some((report, _)) = run(language, code, "") else {
            continue;
        };
        assert_eq!(report.finished.reason, EndReason::Exited);
        assert_ne!(report.finished.exit_code, Some(0));
        let stderr = report.lines(Stream::Stderr).join("\n");
        let (file, line) = match language {
            Language::JavaScript => ("macro.cjs:3", ""),
            Language::TypeScript => ("macro.ts:3", ""),
            Language::Python => ("macro.py", "line 3"),
            Language::Ruby => ("macro.rb", ":3"),
            Language::Php => ("macro.php", "line 3"),
            Language::Bundled(_) => unreachable!(),
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
    let rb = "a = 1\nb = 2\nundefined_function\n";
    let php = "$a = 1;\n$b = 2;\nundefined_function();\n";
    for (language, code) in cases(js, py, rb, php) {
        let Some((report, _)) = run(language, code, "") else {
            continue;
        };
        assert_ne!(report.finished.exit_code, Some(0));
        let stderr = report.lines(Stream::Stderr).join("\n");
        let want = match language {
            Language::JavaScript => "macro.cjs:3",
            Language::TypeScript => "macro.ts:3",
            Language::Python => "macro.py\", line 3",
            Language::Ruby => "macro.rb:3",
            // "macro.php:3" or "macro.php on line 3" (Xdebug words it the second way).
            Language::Php => "macro.php",
            Language::Bundled(_) => unreachable!(),
        };
        assert!(stderr.contains(want), "{language:?}: {stderr}");
        if language == PHP {
            assert!(
                stderr.contains("line 3") || stderr.contains("macro.php:3"),
                "{stderr}"
            );
        }
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
    let rb = r##"
begin
  Editor.GetLineStr('x')
rescue EditorError => e
  puts "caught #{e.class} #{e.code} #{e.is_a?(StandardError)}"
end
begin
  Editor.GetLineCount(5)
rescue EditorError => e
  puts "caught #{e.code} #{e.message}"
end
Editor.Jump(1, 2)
puts 'not reached'
"##;
    let php = r#"
try { Editor::GetLineStr('x'); } catch (EditorError $e) {
    echo 'caught ', get_class($e), ' ', $e->getCode(), ' ', var_export($e instanceof Exception, true), "\n";
}
try { Editor::GetLineCount(5); } catch (EditorError $e) {
    echo 'caught ', $e->getCode(), ' ', $e->getMessage(), "\n";
}
Editor::Jump(1, 2);
echo "not reached\n";
"#;
    for (language, code) in cases(js, py, rb, php) {
        let Some((report, _)) = run(language, code, "a") else {
            continue;
        };
        let lines = stdout(&report);
        assert_eq!(lines.len(), 2, "{language:?}: {lines:?}");
        match language {
            Language::JavaScript | Language::TypeScript => {
                assert_eq!(lines[0], "caught EditorError -32602 true true");
            }
            Language::Python => assert_eq!(lines[0], "caught EditorError -32602 True"),
            Language::Ruby | Language::Php => {
                assert_eq!(lines[0], "caught EditorError -32602 true");
            }
            Language::Bundled(_) => unreachable!(),
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
    let rb = "Editor.Jump(2); print Editor.GetLineStr(0)";
    let php = "Editor::Jump(2); echo Editor::GetLineStr(0);";
    for (language, code) in cases(js, py, rb, php) {
        let Some((report, _)) = run(language, code, "a\nb\nc") else {
            continue;
        };
        assert_clean_exit(&report);
        assert_eq!(stdout(&report), ["b"], "{language:?}");
    }
}

#[test]
fn an_infinite_loop_is_stopped_by_the_timeout() {
    for (language, code) in cases(
        "while (true) {}",
        "while True:\n    pass",
        "while true; end",
        "while (true) {}",
    ) {
        let started = Instant::now();
        let Some((report, _)) =
            run_with(language, code, "", Duration::from_millis(700), |doc, m| {
                api::handle_message(doc, m)
            })
        else {
            continue;
        };
        assert_eq!(report.finished.reason, EndReason::TimedOut, "{language:?}");
        // only a guard against a macro that is never stopped (the timeout is 700 ms)
        assert!(started.elapsed() < Duration::from_secs(60));
    }
}

#[test]
fn a_blocked_macro_is_stopped_by_the_timeout() {
    // Waiting for an answer that never comes is also a hang.
    let js = "Editor.GoFileTop();";
    let py = "Editor.GoFileTop()";
    let rb = "Editor.GoFileTop";
    let php = "Editor::GoFileTop();";
    for (language, code) in cases(js, py, rb, php) {
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
    for (language, code) in [
        (JS, "// nothing"),
        (PY, "pass"),
        (RB, "# nothing"),
        (PHP, "// nothing"),
        (JS, ""),
        (PY, ""),
        (RB, ""),
        (PHP, ""),
    ] {
        let mut requests = 0;
        let Some((report, _)) = run_with(language, code, "", PATIENT, |doc, m| {
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
        (RB, "puts 'bye'; exit 3"),
        (PHP, "echo \"bye\\n\"; exit(3);"),
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
fn ruby_and_php_print_japanese_in_utf8() {
    let rb = "puts '日本語😀'; $stderr.puts 'エラー😀'; puts __FILE__ == $0";
    let php = "echo \"日本語😀\\n\"; fwrite(STDERR, \"エラー😀\\n\"); echo var_export(basename(__FILE__) == basename($argv[0]), true), \"\\n\";";
    for (language, code) in [(RB, rb), (PHP, php)] {
        let Some((report, _)) = run(language, code, "") else {
            continue;
        };
        assert_clean_exit(&report);
        assert_eq!(stdout(&report), ["日本語😀", "true"], "{language:?}");
        assert_eq!(report.lines(Stream::Stderr), ["エラー😀"], "{language:?}");
    }
}

#[test]
fn a_node_module_file_runs_with_top_level_await() {
    let code = "import { readFileSync } from 'node:fs';\nconst n = await Promise.resolve(Editor.GetLineCount(0));\nconsole.log(typeof readFileSync, n);\nundefinedFunction();\n";
    if !available(JS) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("macro.mjs");
    std::fs::write(&script, code).unwrap();
    let mut config = MacroConfig::for_script(&script, None).unwrap();
    assert_eq!(config.language, JS);
    config.temp_dir = Some(dir.path().to_path_buf());
    let mut doc = TextModel::new("a\nb\n", None);
    let report = session::run_blocking(config, |m| api::handle_message(&mut doc, m)).unwrap();
    assert_eq!(stdout(&report), ["function 2"]);
    assert_ne!(report.finished.exit_code, Some(0));
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(stderr.contains("macro.mjs:4"), "{stderr}");
}

#[test]
fn stderr_is_user_output() {
    let js = "console.error('エラー出力'); console.log('out');";
    let py = "import sys\nprint('エラー出力', file=sys.stderr)\nprint('out')";
    let rb = "$stderr.puts 'エラー出力'; puts 'out'";
    let php = "fwrite(STDERR, \"エラー出力\\n\"); echo \"out\\n\";";
    for (language, code) in cases(js, py, rb, php) {
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
    let rb = "i = Editor.info; puts [i['protocolVersion'], i['apiVersion'], i['editor']['name'], i['document']['path'], i['processId'] > 0].join(' ')";
    let php = "$i = Editor::$info; echo implode(' ', [$i['protocolVersion'], $i['apiVersion'], $i['editor']['name'], $i['document']['path'], var_export($i['processId'] > 0, true)]), \"\\n\";";
    for (language, code) in cases(js, py, rb, php) {
        let Some((report, _)) = run(language, code, "") else {
            continue;
        };
        assert_clean_exit(&report);
        let want = match language {
            Language::JavaScript | Language::TypeScript => "1 1 MATSU Editor /work/テスト.txt true",
            Language::Python => "1 1 MATSU Editor /work/テスト.txt True",
            Language::Ruby | Language::Php => "1 1 MATSU Editor /work/テスト.txt true",
            Language::Bundled(_) => unreachable!(),
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
    let rb = r#"
require 'json'
def raw(text)
  $stdout.write("\x1e" + text + "\n")
  JSON.parse($stdin.gets)
end
puts raw('{"jsonrpc":"2.0","id":1,"method":"NoSuchMethod","params":[]}')["error"]["code"]
puts raw('{oops')["error"]["code"]
puts raw('[1]')["error"]["code"]
puts raw('{"jsonrpc":"2.0","id":2,"method":"GetLineCount","params":["x"]}')["error"]["code"]
puts raw('{"jsonrpc":"2.0","id":3,"method":"GetLineCount","params":[0]}')["result"]
"#;
    let php = r#"
function raw($text) {
    fwrite(STDOUT, "\x1e" . $text . "\n");
    fflush(STDOUT);
    return json_decode(fgets(STDIN), true);
}
echo raw('{"jsonrpc":"2.0","id":1,"method":"NoSuchMethod","params":[]}')['error']['code'], "\n";
echo raw('{oops')['error']['code'], "\n";
echo raw('[1]')['error']['code'], "\n";
echo raw('{"jsonrpc":"2.0","id":2,"method":"GetLineCount","params":["x"]}')['error']['code'], "\n";
echo raw('{"jsonrpc":"2.0","id":3,"method":"GetLineCount","params":[0]}')['result'], "\n";
"#;
    for (language, code) in [(PY, py), (RB, rb), (PHP, php)] {
        let Some((report, _)) = run(language, code, "a\nb\n") else {
            continue;
        };
        assert_clean_exit(&report);
        assert_eq!(
            stdout(&report),
            ["-32601", "-32700", "-32600", "-32602", "2"],
            "{language:?}"
        );
    }
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
    // only a guard against a macro that is never stopped
    assert!(cancelled && started.elapsed() < Duration::from_secs(60));
    assert_eq!(boot_folders(dir.path()), Vec::<String>::new());
}

#[test]
fn a_missing_interpreter_is_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = config_for(&dir, JS, "", Duration::from_secs(5));
    config.interpreter = "matsu-no-such-interpreter".into();
    let err = session::run_blocking(config, |_| None)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("Node.js") && err.contains("matsu-no-such-interpreter"),
        "{err}"
    );
    // The folder that was made for the failed start is gone too.
    assert_eq!(boot_folders(dir.path()), Vec::<String>::new());
    let err = MacroConfig::for_script(&dir.path().join("a.txt"), None)
        .err()
        .unwrap();
    for extension in [".mjs", ".js", ".ts", ".py", ".rb", ".php"] {
        assert!(err.to_string().contains(extension), "{err}");
    }
}

#[test]
fn the_sample_macros_run() {
    // The samples in macros/ change the text and print what they did.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
    for (language, name) in [
        (JS, "sample.mjs"),
        (TS, "sample.ts"),
        (PY, "sample.py"),
        (RB, "sample.rb"),
        (PHP, "sample.php"),
    ] {
        if !available(language) {
            continue;
        }
        let mut config = MacroConfig::for_script(&dir.join(name), None).unwrap();
        let temp = tempfile::tempdir().unwrap();
        config.temp_dir = Some(temp.path().to_path_buf());
        let mut doc = TextModel::new("りんご\nみかん😀\nぶどう\n", None);
        let report = session::run_blocking(config, |m| api::handle_message(&mut doc, m)).unwrap();
        assert_clean_exit(&report);
        let text = doc.text();
        assert!(text.starts_with("# サンプルマクロ"), "{name}: {text}");
        assert!(text.contains("みかん😀"), "{name}: {text}");
        assert!(!stdout(&report).is_empty());
        assert_eq!(boot_folders(temp.path()), Vec::<String>::new());
    }
}

/// The boot folder is removed before `Finished` is reported, so a caller that exits at once
/// (the editor closing, a command line run) does not leave it behind. The PoC removed it after
/// the report and lost it on Windows (docs/poc/json-rpc.md).
#[test]
fn the_boot_folder_is_gone_when_finished_is_reported() {
    for (language, code) in [
        (JS, ""),
        (PY, ""),
        (RB, ""),
        (PHP, ""),
        (JS, "console.log(1)"),
        (PY, "print(1)"),
        (RB, "puts 1"),
        (PHP, "echo 1;"),
    ] {
        if !available(language) {
            continue;
        }
        for _ in 0..5 {
            let dir = tempfile::tempdir().unwrap();
            let config = config_for(&dir, language, code, PATIENT);
            let parent = dir.path().to_path_buf();
            let (tx, rx) = mpsc::channel();
            let sink: session::Sink = Arc::new(move |event| {
                if let HostEvent::Finished(_) = event {
                    // Looked at on the reader side, at the moment the end is reported.
                    let _ = tx.send(boot_folders(&parent));
                }
            });
            let _session = session::start(config, sink).unwrap();
            let left = rx.recv_timeout(PATIENT).unwrap();
            assert_eq!(left, Vec::<String>::new(), "{language:?}");
        }
    }
}

/// A timeout and a cancel kill the child; the folder goes then, too.
#[test]
fn the_boot_folder_is_removed_after_a_timeout() {
    for (language, code) in cases(
        "while (true) {}",
        "while True:\n    pass",
        "while true; end",
        "while (true) {}",
    ) {
        // `run_with` checks that no folder is left.
        let Some((report, _)) =
            run_with(language, code, "", Duration::from_millis(500), |_, _| None)
        else {
            continue;
        };
        assert_eq!(report.finished.reason, EndReason::TimedOut);
    }
}

#[test]
fn the_real_interpreter_path_can_be_asked_for() {
    for language in [JS, PY, RB, PHP] {
        if !available(language) {
            continue;
        }
        let name = language.default_interpreter();
        let path = language.real_interpreter_path(&name).unwrap();
        assert!(path.is_file(), "{path:?}");
    }
    let err = JS.real_interpreter_path("matsu-no-such-interpreter".as_ref());
    assert!(err.is_err());
}

#[test]
fn generated_code_has_every_function() {
    let js = crate::codegen::generate_js();
    let py = crate::codegen::generate_py();
    let rb = crate::codegen::generate_rb();
    let php = crate::codegen::generate_php();
    for func in api::FUNCS {
        assert!(js.contains(&format!("{}(", func.name)), "{}", func.name);
        assert!(py.contains(&format!("def {}(", func.name)), "{}", func.name);
        assert!(rb.contains(&format!("def {}", func.name)), "{}", func.name);
        assert!(
            php.contains(&format!("function {}(", func.name)),
            "{}",
            func.name
        );
    }
    // Every name of Sakura's macro tables is generated too, except the few that start with a
    // digit (`1PageUp`): they are reached through the by-name fallback.
    for entry in crate::catalog::entries() {
        if !entry.is_identifier() {
            assert!(entry.name.starts_with(|c: char| c.is_ascii_digit()));
            assert!(!js.contains(&format!("{}(", entry.name)), "{}", entry.name);
            continue;
        }
        assert!(
            js.contains(&format!("    {}(", entry.name)),
            "{}",
            entry.name
        );
        assert!(
            py.contains(&format!("def {}(", entry.name)),
            "{}",
            entry.name
        );
        assert!(
            rb.contains(&format!("def {}", entry.name)),
            "{}",
            entry.name
        );
        assert!(
            php.contains(&format!("function {}(", entry.name)),
            "{}",
            entry.name
        );
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

#[test]
fn the_time_spent_waiting_for_the_window_does_not_count_against_the_timeout() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // The timeout is shorter than the wait. The timeout is long enough for the interpreter to
    // start on a busy machine (its clock runs from the start, not from the first call).
    let config = config_for(
        &dir,
        Language::JavaScript,
        "Editor.GetFilename(); console.log('after the wait');",
        Duration::from_secs(2),
    );
    let (tx, rx) = mpsc::channel();
    let sink: session::Sink = Arc::new(move |event| {
        let _ = tx.send(event);
    });
    let session = Arc::new(session::start(config, sink).unwrap());
    let mut printed = Vec::new();
    for event in rx {
        match event {
            HostEvent::Output { text, .. } => printed.push(text),
            HostEvent::Message(text) => {
                // A dialog is open: the window holds the macro's clock, then answers.
                session.hold();
                let session = session.clone();
                let id = serde_json::from_str::<Value>(&text).unwrap()["id"].clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(5));
                    session.send(crate::protocol::response_ok(&id, Value::from("")));
                    session.release();
                });
            }
            HostEvent::Finished(finished) => {
                assert_eq!(finished.reason, EndReason::Exited, "{finished:?}");
                assert_eq!(finished.exit_code, Some(0));
                assert!(printed.iter().any(|l| l.contains("after the wait")));
                return;
            }
        }
    }
    panic!("no Finished event");
}

#[test]
fn without_a_hold_the_same_wait_times_out() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let config = config_for(
        &dir,
        Language::JavaScript,
        "Editor.GetFilename(); console.log('after the wait');",
        Duration::from_secs(2),
    );
    let (tx, rx) = mpsc::channel();
    let sink: session::Sink = Arc::new(move |event| {
        let _ = tx.send(event);
    });
    let session = Arc::new(session::start(config, sink).unwrap());
    for event in rx {
        match event {
            HostEvent::Output { .. } => {}
            HostEvent::Message(text) => {
                let id = serde_json::from_str::<Value>(&text).unwrap()["id"].clone();
                let session = session.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(5));
                    session.send(crate::protocol::response_ok(&id, Value::from("")));
                });
            }
            HostEvent::Finished(finished) => {
                assert_eq!(finished.reason, EndReason::TimedOut);
                return;
            }
        }
    }
    panic!("no Finished event");
}

// ---- TypeScript (ADR-0025): the JavaScript boot, run by a Node.js that strips the types ----

#[test]
fn ts_files_are_typescript() {
    assert_eq!(
        Language::of_file(std::path::Path::new("a.TS")),
        Some(Language::TypeScript)
    );
    assert_eq!(TS.default_interpreter(), JS.default_interpreter());
}

#[test]
fn node_versions_that_strip_types() {
    // https://nodejs.org/api/typescript.html: on by default from v22.18.0 and v23.6.0.
    for (version, want) in [
        ((22, 17, 9), false),
        ((22, 18, 0), true),
        ((22, 23, 3), true),
        ((23, 5, 0), false),
        ((23, 6, 0), true),
        ((24, 0, 0), true),
        ((20, 19, 0), false),
    ] {
        assert_eq!(session::node_strips_types(version), want, "{version:?}");
    }
    assert_eq!(session::parse_node_version("v22.23.3\n"), Some((22, 23, 3)));
    // Deno and Bun answer in other words: they are not judged.
    assert_eq!(session::parse_node_version("deno 2.9.8 (stable)\n"), None);
    assert_eq!(session::parse_node_version("1.4.4\n"), None);
    assert_eq!(session::parse_node_version("v22.x"), None);
}

#[test]
fn a_typescript_macro_uses_types_and_the_api() {
    let code = "interface Pos { line: number }\n\
        const p: Pos = { line: 1 };\n\
        const text: string = 'あ😀';\n\
        Editor.InsText(text + String(p.line as number));\n\
        console.log(Editor.GetLineStr(1) satisfies string);\n";
    let Some((report, doc)) = run(TS, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(doc.text(), "あ😀1");
    assert_eq!(stdout(&report), ["あ😀1"]);
}

#[test]
fn a_typescript_error_shows_the_line_number_of_the_users_file() {
    // The types become white space, so the line of the error is the line in the file.
    let code = "type A = { a: number };\nconst a: A = { a: 1 };\nundefinedFunction(a.a);\n";
    let Some((report, _)) = run(TS, code, "") else {
        return;
    };
    assert_ne!(report.finished.exit_code, Some(0));
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(stderr.contains("macro.ts:3"), "{stderr}");
}

#[test]
fn typescript_that_needs_a_transform_is_an_error_of_the_macro() {
    // Node.js only removes types; an enum has to be rewritten, so it is refused (the macro
    // author sees Node.js's message in the output panel).
    let Some((report, _)) = run(TS, "enum E { A }\nconsole.log(E.A);\n", "") else {
        return;
    };
    assert_ne!(report.finished.exit_code, Some(0));
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(stderr.contains("macro.ts"), "{stderr}");
}
