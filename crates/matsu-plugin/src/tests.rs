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

/// Whether this Emacs has the JSON functions (built in from Emacs 30; before, only when built
/// with JSON support). Without them the Emacs Lisp tests cannot run.
fn emacs_has_json(interpreter: &std::ffi::OsStr) -> bool {
    Command::new(interpreter)
        .args([
            "-Q",
            "--batch",
            "--eval",
            "(kill-emacs (if (and (fboundp 'json-parse-string) (fboundp 'json-serialize)) 0 1))",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

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
    }) && (language != Language::EmacsLisp || emacs_has_json(&interpreter));
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
        Language::PowerShell => "macro.ps1",
        Language::Perl => "macro.pl",
        Language::EmacsLisp => "macro.el",
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
const PS: Language = Language::PowerShell;
const PL: Language = Language::Perl;
const EL: Language = Language::EmacsLisp;
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
            Language::Bundled(_) | Language::PowerShell | Language::Perl | Language::EmacsLisp => {
                unreachable!()
            }
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
            Language::Bundled(_) | Language::PowerShell | Language::Perl | Language::EmacsLisp => {
                unreachable!()
            }
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
            Language::Bundled(_) | Language::PowerShell | Language::Perl | Language::EmacsLisp => {
                unreachable!()
            }
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
        (PS, "# nothing"),
        (PL, "# nothing"),
        (EL, ";; nothing"),
        (JS, ""),
        (PY, ""),
        (RB, ""),
        (PHP, ""),
        (PS, ""),
        (PL, ""),
        (EL, ""),
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
            Language::Bundled(_) | Language::PowerShell | Language::Perl | Language::EmacsLisp => {
                unreachable!()
            }
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
    for extension in [
        ".mjs", ".js", ".ts", ".py", ".rb", ".php", ".ps1", ".pl", ".el",
    ] {
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
        (PS, "sample.ps1"),
        (PL, "sample.pl"),
        (EL, "sample.el"),
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

/// The sample macros that run the SQL of the document on a database (ADR-0023) print the rows and
/// the error with the line the statement starts at. They use SQLite, which each language has (Python
/// built in; Node.js `node:sqlite` from 22.5; PHP `pdo_sqlite`): a language without it is skipped,
/// the others must run.
#[test]
fn the_database_sample_macros_run() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
    let sql = "create table t(a, b);\ninsert into t values (1, 'x'),\n  (2, 'y');\nselect * from t;\nselec 1;\nselect\n  a\nfrom t where a = 2;\n";
    let mut ran = 0;
    for (language, name) in [
        (PY, "sample_db.py"),
        (JS, "sample_db.mjs"),
        (PHP, "sample_db.php"),
    ] {
        if !available(language) {
            continue;
        }
        let mut config = MacroConfig::for_script(&dir.join(name), None).unwrap();
        let temp = tempfile::tempdir().unwrap();
        config.temp_dir = Some(temp.path().to_path_buf());
        let mut doc = TextModel::new(sql, None);
        let report = session::run_blocking(config, |m| api::handle_message(&mut doc, m)).unwrap();
        let stderr = report.lines(Stream::Stderr).join("\n");
        if report.finished.exit_code != Some(0)
            && (stderr.contains("sqlite") || stderr.contains("driver"))
        {
            // the language has no SQLite here
            continue;
        }
        assert_clean_exit(&report);
        let lines = stdout(&report);
        assert!(lines.contains(&"a\tb"), "{name}: {lines:?}");
        assert!(
            lines.contains(&"1\tx") && lines.contains(&"2\ty"),
            "{name}: {lines:?}"
        );
        // the error of the fifth line, in the form that a tag jump reads
        assert!(
            lines.iter().any(|l| l.starts_with("(無題):5: ERROR:")),
            "{name}: {lines:?}"
        );
        // the statement after it, which starts at the sixth line
        assert!(
            lines.contains(&"a") && lines.contains(&"2"),
            "{name}: {lines:?}"
        );
        // the macro changes nothing in the document
        assert_eq!(doc.text(), sql, "{name}");
        ran += 1;
    }
    // Python is on every machine that runs these tests
    assert!(ran >= 1 || !available(PY));
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
        (PS, ""),
        (PL, ""),
        (EL, ""),
        (JS, "console.log(1)"),
        (PY, "print(1)"),
        (RB, "puts 1"),
        (PHP, "echo 1;"),
        (PS, "Write-Output 1"),
        (PL, "print 1;"),
        (EL, "(princ 1)"),
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
        Language::of_file(std::path::Path::new("a.ts")),
        Some(Language::TypeScript)
    );
    // Node.js strips types only from `.ts` in lower case.
    assert_eq!(Language::of_file(std::path::Path::new("a.TS")), None);
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

// ---- PowerShell (ADR-0025): the script goes in `initialize` and is compiled by the boot ----

#[test]
fn ps1_files_are_powershell() {
    assert_eq!(
        Language::of_file(std::path::Path::new("a.PS1")),
        Some(Language::PowerShell)
    );
    // PowerShell 7 first; on Windows without it, the Windows PowerShell 5.1 that is part of the
    // system.
    let interpreter = PS.default_interpreter();
    assert!(
        interpreter == "pwsh" || (cfg!(windows) && interpreter == "powershell"),
        "{interpreter:?}"
    );
}

#[test]
fn a_powershell_macro_round_trips_japanese_and_emoji() {
    let code = "$t = 'あいう😀 \"q\" \\ end'\n\
        $Editor.InsText($t)\n\
        Write-Output $Editor.GetLineStr(1)\n\
        Write-Output $t\n";
    let Some((report, doc)) = run(PS, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(doc.text(), "あいう😀 \"q\" \\ end");
    assert_eq!(
        stdout(&report),
        ["あいう😀 \"q\" \\ end", "あいう😀 \"q\" \\ end"]
    );
}

#[test]
fn a_powershell_string_that_looks_like_a_date_stays_a_string() {
    // ConvertFrom-Json would turn it into a DateTime in some versions.
    let code = "$Editor.InsText('2020-01-02T03:04:05Z')\n\
        $s = $Editor.GetLineStr(1)\n\
        Write-Output \"$($s.GetType().Name) $s\"\n";
    let Some((report, _)) = run(PS, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["String 2020-01-02T03:04:05Z"]);
}

#[test]
fn powershell_sends_whole_numbers_as_integers_and_keeps_the_order_of_the_output() {
    let code = "Write-Output 'before'\n\
        $Editor.InsText('abc')\n\
        $Editor.MoveCursor(4 / 4, 4 / 2, 0)\n\
        Write-Output \"after $($Editor.GetLineCount(0))\"\n";
    let Some((report, _)) = run(PS, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["before", "after 1"]);
}

#[test]
fn a_powershell_error_shows_the_line_number_of_the_users_file() {
    let code = "$a = 1\n$b = 2\nundefinedFunction\n";
    let Some((report, _)) = run(PS, code, "") else {
        return;
    };
    assert_ne!(report.finished.exit_code, Some(0));
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(stderr.contains("macro.ps1:3:"), "{stderr}");
}

#[test]
fn powershell_refuses_nan_and_infinity_before_sending() {
    let code = "foreach ($v in @([double]::NaN, [double]::PositiveInfinity, 1)) {\n\
        try { $Editor.MoveCursor($v, 1, 0); Write-Output 'sent' } catch { Write-Output \"refused $(Get-EditorErrorCode $_)\" }\n\
        }\n";
    let Some((report, _)) = run(PS, code, "a") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["refused -32602", "refused -32602", "sent"]
    );
}

#[test]
fn powershell_keeps_ignorable_characters_and_formatted_output() {
    // U+FEFF, U+00AD and NUL must not be sent as a backspace; a table that PowerShell holds back
    // to format must be written before the process exits.
    let code = "$Editor.InsText([string][char]0xfeff + [string][char]0xad + 'x')\n\
        Write-Output $Editor.GetLineStr(1).Length\n\
        [pscustomobject]@{Result='hello'}\n\
        Write-Output 'after'\n";
    let Some((report, doc)) = run(PS, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(doc.text(), "\u{feff}\u{ad}x");
    let out = stdout(&report).join("\n");
    assert!(out.starts_with("3\n"), "{out}");
    assert!(
        out.contains("hello") && out.trim_end().ends_with("after"),
        "{out}"
    );
}

#[test]
fn a_powershell_macro_knows_where_it_is() {
    // `$PSScriptRoot` is empty for text that is compiled without a file; `$MacroDir` stands in.
    let code = "Write-Output ([IO.Path]::GetFileName($MacroPath))\n\
        Write-Output (Test-Path -LiteralPath ([IO.Path]::Combine($MacroDir, 'macro.ps1')))\n";
    let Some((report, _)) = run(PS, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["macro.ps1", "True"]);
}

#[test]
fn a_powershell_syntax_error_shows_its_line() {
    let code = "$a = 1\nif ($a {\n}\n";
    let Some((report, _)) = run(PS, code, "") else {
        return;
    };
    assert_ne!(report.finished.exit_code, Some(0));
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(stderr.contains("macro.ps1:2:"), "{stderr}");
}

#[test]
fn an_error_response_becomes_a_powershell_error_with_the_line_of_the_call() {
    let code = "try { $Editor.GetLineStr('x') } catch {\n\
        Write-Output \"caught $(Get-EditorErrorCode $_)\"\n\
        }\n\
        function Fail { $Editor.GetLineCount(5) }\n\
        Fail\n\
        Write-Output 'not reached'\n";
    let Some((report, _)) = run(PS, code, "a") else {
        return;
    };
    assert_eq!(stdout(&report), ["caught -32602"]);
    assert_eq!(report.finished.exit_code, Some(1));
    // The line is the one of the call in the user's own code (the function's body).
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(
        stderr.contains("macro.ps1:4: EditorError -32602"),
        "{stderr}"
    );
}

#[test]
fn a_powershell_macro_sees_the_initialize_message() {
    let code = "$i = $Editor.Info\n\
        Write-Output \"$($i['protocolVersion']) $($i['apiVersion']) $($i['editor']['name']) $($i['document']['path']) $($i['processId'] -gt 0)\"\n";
    let Some((report, _)) = run(PS, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["1 1 MATSU Editor /work/テスト.txt True"]);
}

#[test]
fn a_powershell_macro_can_call_a_name_that_starts_with_a_digit() {
    // `Call` sends any name as it is; the editor of this test knows only the basic API.
    let code = "$Editor.Call('InsText', 'x')\n\
        try { $Editor.Call('1PageUp') } catch { Write-Output \"code $(Get-EditorErrorCode $_)\" }\n\
        Write-Output (Get-EditorErrorCode 'not an error record')\n";
    let Some((report, doc)) = run(PS, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(doc.text(), "x");
    assert_eq!(stdout(&report), ["code -32601"]);
}

#[test]
fn a_long_line_goes_both_ways_in_powershell() {
    let code =
        "$t = 'あ' * 200000\n$Editor.InsText($t)\nWrite-Output $Editor.GetLineStr(1).Length\n";
    let Some((report, doc)) = run(PS, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(doc.text().chars().count(), 200_000);
    assert_eq!(stdout(&report), ["200000"]);
}

#[test]
fn an_infinite_powershell_macro_is_stopped_by_the_timeout() {
    let Some((report, _)) = run_with(
        PS,
        "while ($true) { }\n",
        "",
        Duration::from_millis(1500),
        |doc, message| api::handle_message(doc, message),
    ) else {
        return;
    };
    assert_eq!(report.finished.reason, EndReason::TimedOut);
}

// ---- Perl (ADR-0025): the program, the API and the script go on the standard input of `perl -` ----

#[test]
fn pl_files_are_perl() {
    assert_eq!(
        Language::of_file(std::path::Path::new("a.pl")),
        Some(Language::Perl)
    );
    assert_eq!(PL.default_interpreter(), "perl");
}

#[test]
fn a_perl_macro_round_trips_japanese_and_emoji() {
    // The script is characters: the literal is one character long, and the output is UTF-8.
    let code = "my $t = 'あいう😀 \"q\" \\\\ end';\n\
        Editor->InsText($t);\n\
        print(Editor->GetLineStr(1), \"\\n\");\n\
        print length($t), \"\\n\";\n";
    let Some((report, doc)) = run(PL, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(doc.text(), "あいう😀 \"q\" \\ end");
    assert_eq!(stdout(&report), ["あいう😀 \"q\" \\ end", "14"]);
}

#[test]
fn perl_tells_numbers_from_strings() {
    // A number that was printed still goes as a number; "$x" is a string.
    let code = "my $n = 2;\nprint \"n=$n\\n\";\nEditor->MoveCursor($n, 1, 0);\n\
        my $s = '7';\nEditor->InsText($s);\n\
        eval { Editor->InsText(8 / 4); 1 } or print 'number ', $@->code, \"\\n\";\n\
        Editor->InsText(\"$n\");\n\
        print Editor->GetLineCount(0), \"\\n\";\n";
    let Some((report, doc)) = run(PL, code, "a\nb\nc") else {
        return;
    };
    assert_clean_exit(&report);
    // MoveCursor(2, 1) puts the cursor on line 2; "7" goes in, the number 2 is refused as the
    // text, and "2" goes in.
    assert_eq!(doc.text(), "a\n72b\nc");
    assert_eq!(stdout(&report), ["n=2", "number -32602", "3"]);
}

#[test]
fn perl_refuses_references_and_nan_before_sending() {
    let code = "for my $v ([1], 9**9**9, -sin(9**9**9)) {\n\
        eval { Editor->MoveCursor($v, 1, 0); print \"sent\\n\"; 1 } or print 'refused ', $@->code, \"\\n\";\n\
        }\n";
    let Some((report, _)) = run(PL, code, "a") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["refused -32602", "refused -32602", "refused -32602"]
    );
}

#[test]
fn a_perl_error_shows_the_line_number_of_the_users_file() {
    let code = "my $a = 1;\nmy $b = 2;\nundefined_function();\n";
    let Some((report, _)) = run(PL, code, "") else {
        return;
    };
    assert_ne!(report.finished.exit_code, Some(0));
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(stderr.contains("macro.pl line 3"), "{stderr}");
}

#[test]
fn a_perl_syntax_error_shows_its_line() {
    let code = "my $a = 1;\nmy $b = ;\n";
    let Some((report, _)) = run(PL, code, "") else {
        return;
    };
    assert_ne!(report.finished.exit_code, Some(0));
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(stderr.contains("macro.pl line 2"), "{stderr}");
}

#[test]
fn an_error_response_becomes_a_perl_exception_with_the_line_of_the_call() {
    let code = "eval { Editor->GetLineStr('x'); 1 } or do {\n\
        print ref($@), ' ', $@->code, ' ', ($@->isa('EditorError') ? 'yes' : 'no'), \"\\n\";\n\
        };\n\
        sub fail { Editor->GetLineCount(5) }\n\
        fail();\n\
        print \"not reached\\n\";\n";
    let Some((report, _)) = run(PL, code, "a") else {
        return;
    };
    assert_eq!(stdout(&report), ["EditorError -32602 yes"]);
    assert_eq!(report.finished.exit_code, Some(1));
    // The line is the one of the call in the user's own code (the sub's body).
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(
        stderr.contains("EditorError -32602") && stderr.contains("macro.pl line 4."),
        "{stderr}"
    );
}

#[test]
fn a_perl_macro_sees_the_initialize_message_and_calls_any_name() {
    let code = "my $i = Editor->info;\n\
        print join(' ', $i->{protocolVersion}, $i->{apiVersion}, $i->{editor}{name}, $i->{document}{path}), \"\\n\";\n\
        eval { Editor->Call('1PageUp'); 1 } or print 'code ', $@->code, \"\\n\";\n\
        eval { Editor->NoSuchMacro(); 1 } or print 'code ', $@->code, \"\\n\";\n\
        eval { Editor->lower_case(); 1 } or print 'no method', \"\\n\";\n";
    let Some((report, _)) = run(PL, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "1 1 MATSU Editor /work/テスト.txt",
            "code -32601",
            "code -32601",
            "no method"
        ]
    );
}

#[test]
fn a_perl_script_is_not_under_strict_and_warnings_of_the_boot() {
    // As under `perl script.pl`: a global without `my` is fine, and `use strict` in the script
    // works as usual.
    let Some((report, _)) = run(PL, "$x = 5;\nprint \"$x\\n\";\n", "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["5"]);
    let Some((report, _)) = run(PL, "use strict;\n$y = 1;\n", "") else {
        return;
    };
    assert_ne!(report.finished.exit_code, Some(0));
    assert!(
        report
            .lines(Stream::Stderr)
            .join("\n")
            .contains("macro.pl line 2")
    );
}

#[test]
fn a_perl_script_cannot_break_the_boot_by_its_own_variables_and_settings() {
    // `$json` is a name of the boot; the script's own is another variable. `local $/;` (slurp
    // mode) and the output separators `$,` and `$\` must not reach the protocol.
    let code = "$json = '{}'; $next_id = 'x'; $RS = 1;\n\
        $, = '-'; $\\ = \"!\\n\";\n\
        { local $/; print Editor->GetLineCount(0); }\n\
        $\\ = undef; $, = undef;\n\
        $name = 'v'; print \"$main::name\\n\";\n\
        print $0 =~ /macro\\.pl\\z/ ? \"0 ok\\n\" : \"0 bad\\n\";\n\
        print \"done\\n\";\n";
    let Some((report, _)) = run(PL, code, "a\nb") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["2!", "v", "0 ok", "done"]);
}

#[test]
fn a_perl_script_in_a_folder_with_a_japanese_name_shows_the_name_right() {
    // Perl holds file names as bytes: the name in the messages must not be encoded twice.
    if !available(PL) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("テスト😀");
    std::fs::create_dir(&folder).unwrap();
    let script = folder.join("マクロ.pl");
    let code = "warn \"warned\\n\" if 0;\n\
        eval { Editor->GetLineCount(5); 1 } or print \"caught: $@\";\n\
        eval { die 'plain' }; print \"plain: $@\";\n\
        warn 'careful';\n\
        Editor->GetLineCount(5);\n";
    std::fs::write(&script, code).unwrap();
    let mut config = MacroConfig::for_script(&script, None).unwrap();
    config.temp_dir = Some(dir.path().to_path_buf());
    let mut doc = TextModel::new("a", None);
    let report = session::run_blocking(config, |m| api::handle_message(&mut doc, m)).unwrap();
    let name = script.display().to_string();
    let out = stdout(&report).join("\n");
    assert!(
        out.contains("EditorError -32602: GetLineCount: ")
            && out.contains(&format!("at {name} line 2.")),
        "{out}"
    );
    assert!(out.contains(&format!("plain at {name} line 3")), "{out}");
    let err = report.lines(Stream::Stderr).join("\n");
    assert!(err.contains(&format!("careful at {name} line 4")), "{err}");
    assert!(err.contains(&format!("at {name} line 5")), "{err}");
    assert!(
        !err.contains('\u{e3}') && !out.contains('\u{e3}'),
        "{err} {out}"
    );
}

#[test]
fn a_long_line_goes_both_ways_in_perl() {
    let code = "my $t = 'あ' x 200000;\nEditor->InsText($t);\nprint length(Editor->GetLineStr(1)), \"\\n\";\n";
    let Some((report, doc)) = run(PL, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(doc.text().chars().count(), 200_000);
    assert_eq!(stdout(&report), ["200000"]);
}

#[test]
fn an_infinite_perl_macro_is_stopped_by_the_timeout() {
    let Some((report, _)) = run_with(
        PL,
        "1 while 1;\n",
        "",
        Duration::from_millis(1500),
        |doc, message| api::handle_message(doc, message),
    ) else {
        return;
    };
    assert_eq!(report.finished.reason, EndReason::TimedOut);
}

// ---- Emacs Lisp (ADR-0025): `emacs -Q --batch --load boot.el`, the script in `initialize` ----

#[test]
fn el_files_are_emacs_lisp() {
    assert_eq!(
        Language::of_file(std::path::Path::new("a.el")),
        Some(Language::EmacsLisp)
    );
    assert_eq!(EL.default_interpreter(), "emacs");
}

#[test]
fn an_emacs_lisp_macro_round_trips_japanese_and_emoji() {
    // The script is characters: the string "あ😀" is two characters, and the output is UTF-8.
    let code = "(editor-ins-text \"あいう😀 \\\"q\\\" \\\\ end\")\n\
        (princ (editor-get-line-str 1))\n\
        (terpri)\n\
        (princ (length \"あ😀\"))\n\
        (terpri)\n";
    let Some((report, doc)) = run(EL, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(doc.text(), "あいう😀 \"q\" \\ end");
    assert_eq!(stdout(&report), ["あいう😀 \"q\" \\ end", "2"]);
}

#[test]
fn emacs_lisp_sends_whole_floats_as_integers_and_refuses_nan_and_infinity() {
    let code = "(editor-ins-text \"abc\")\n\
        (editor-move-cursor (/ 4.0 4) (/ 4.0 2) 0)\n\
        (dolist (v (list 0.0e+NaN 1.0e+INF))\n\
          (condition-case err (editor-move-cursor v 1 0)\n\
            (editor-error (princ (format \"refused %s\\n\" (cadr err))))))\n\
        (princ (format \"%s\\n\" (editor-get-line-count 0)))\n";
    let Some((report, _)) = run(EL, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["refused -32602", "refused -32602", "1"]);
}

#[test]
fn an_emacs_lisp_error_shows_the_line_where_its_form_began() {
    let code = "(setq a 1)\n;; a comment\n\n(setq b\n      (undefined-function a))\n";
    let Some((report, _)) = run(EL, code, "") else {
        return;
    };
    assert_ne!(report.finished.exit_code, Some(0));
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(stderr.contains("macro.el:4:"), "{stderr}");
}

#[test]
fn an_emacs_lisp_syntax_error_shows_its_line() {
    let code = "(setq a 1)\n(setq b 2))\n";
    let Some((report, _)) = run(EL, code, "") else {
        return;
    };
    assert_ne!(report.finished.exit_code, Some(0));
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(stderr.contains("macro.el:2:"), "{stderr}");
}

#[test]
fn an_error_response_becomes_an_editor_error_with_the_line_of_the_call() {
    let code = "(condition-case err (editor-get-line-str \"x\")\n\
          (editor-error (princ (format \"caught %s %s\\n\" (car err) (cadr err)))))\n\
        (defun fail () (editor-get-line-count 5))\n\
        (fail)\n\
        (princ \"not reached\\n\")\n";
    let Some((report, _)) = run(EL, code, "a") else {
        return;
    };
    assert_eq!(stdout(&report), ["caught editor-error -32602"]);
    assert_eq!(report.finished.exit_code, Some(1));
    // The line is the one where the failing form began.
    let stderr = report.lines(Stream::Stderr).join("\n");
    assert!(
        stderr.contains("macro.el:4: EditorError -32602"),
        "{stderr}"
    );
}

#[test]
fn an_emacs_lisp_macro_sees_the_initialize_message_and_calls_any_name() {
    let code = "(princ (format \"%s %s %s\\n\" (alist-get 'protocolVersion editor-info)\n\
                       (alist-get 'name (alist-get 'editor editor-info))\n\
                       (alist-get 'path (alist-get 'document editor-info))))\n\
        (condition-case err (editor-call \"1PageUp\")\n\
          (editor-error (princ (format \"code %s\\n\" (cadr err)))))\n";
    let Some((report, _)) = run(EL, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["1 MATSU Editor /work/テスト.txt", "code -32601"]
    );
}

#[test]
fn an_emacs_lisp_macro_is_not_under_the_users_init_file_and_ends_cleanly() {
    // -Q: no init file; the process ends by itself with the exit code 0.
    let Some((report, _)) = run(EL, ";; nothing\n", "") else {
        return;
    };
    assert_clean_exit(&report);
    assert!(stdout(&report).is_empty());
}

#[test]
fn what_an_emacs_lisp_script_does_to_its_buffer_does_not_disturb_the_reading() {
    // `goto-char` to the start would make the script read itself again.
    let code = "(insert \"x\")\n(goto-char (point-min))\n(princ \"one\\n\")\n\
        (set-buffer (get-buffer-create \"other\"))\n(erase-buffer)\n(princ \"two\\n\")\n";
    let Some((report, _)) = run(EL, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["one", "two"]);
}

#[test]
fn an_emacs_lisp_script_may_capture_its_output_around_an_api_call() {
    let code = "(let ((captured (with-output-to-string\n\
          (princ (editor-get-line-count 0)))))\n\
          (princ (format \"captured %s\\n\" captured)))\n";
    let Some((report, _)) = run(EL, code, "a\nb") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["captured 2"]);
}

#[test]
fn a_long_line_goes_both_ways_in_emacs_lisp() {
    let code = "(editor-ins-text (make-string 50000 ?あ))\n\
        (princ (length (editor-get-line-str 1)))\n\
        (terpri)\n";
    let Some((report, doc)) = run(EL, code, "") else {
        return;
    };
    assert_clean_exit(&report);
    assert_eq!(doc.text().chars().count(), 50_000);
    assert_eq!(stdout(&report), ["50000"]);
}

#[test]
fn an_infinite_emacs_lisp_macro_is_stopped_by_the_timeout() {
    let Some((report, _)) = run_with(
        EL,
        "(while t)\n",
        "",
        Duration::from_millis(1500),
        |doc, message| api::handle_message(doc, message),
    ) else {
        return;
    };
    assert_eq!(report.finished.reason, EndReason::TimedOut);
}
