//! End-to-end tests of the MicroPython engine: the built `ume-script` program runs as a macro
//! process under the editor's own session code (`ume_plugin::session`), against a plain text
//! model.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ume_plugin::api;
use ume_plugin::model::TextModel;
use ume_plugin::session::{
    self, EndReason, Engine, HostEvent, Language, MacroConfig, Report, Stream,
};

const DOCUMENT_PATH: &str = "/work/テスト.txt";

fn config(dir: &tempfile::TempDir, file: &str, code: &str, timeout: Duration) -> MacroConfig {
    let script = dir.path().join(file);
    std::fs::write(&script, code).unwrap();
    let mut config = MacroConfig::for_script(&script, Some(DOCUMENT_PATH.into())).unwrap();
    assert_eq!(config.language, Language::Bundled(Engine::MicroPython));
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
    run_in("macro.upy", code, text, Duration::from_secs(30))
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
print(Editor.IsTextSelected(), len(Editor.GetSelectedString(0)))
print(Editor.GetFilename())
"#;
    let (report, _) = run(code, "one\ntwo\nthree");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["3", "True", "three", "0", "1 13", DOCUMENT_PATH]
    );
}

#[test]
fn japanese_text_goes_both_ways() {
    let code = r#"
line = Editor.GetLineStr(1)
print(line)
Editor.GoFileEnd()
Editor.InsText("、絵文字😀と" + "日本語")
print("標準出力の日本語", len("😀"))
print(Editor.GetLineStr(1))
"#;
    let (report, doc) = run(code, "こんにちは");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "こんにちは",
            "標準出力の日本語 1",
            "こんにちは、絵文字😀と日本語"
        ]
    );
    assert_eq!(doc.text(), "こんにちは、絵文字😀と日本語");
}

#[test]
fn a_script_in_a_path_with_japanese_runs_and_names_it_in_errors() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("マクロ😀");
    std::fs::create_dir(&folder).unwrap();
    let script = folder.join("あ.upy");
    std::fs::write(&script, "print('ok')\nraise ValueError('boom')\n").unwrap();
    let mut config = MacroConfig::for_script(&script, None).unwrap();
    config.interpreter = env!("CARGO_BIN_EXE_ume-script").into();
    let mut doc = TextModel::new("", None);
    let report =
        session::run_blocking(config, |m| api::handle_message(&mut doc, m)).expect("starts");
    assert_eq!(stdout(&report), ["ok"]);
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("あ.upy\", line 2"), "{err}");
    assert!(err.contains("ValueError: boom"), "{err}");
}

#[test]
fn a_runtime_error_shows_the_line_number_of_the_script() {
    let (report, _) = run("a = 1\nb = 2\nundefined_function()\n", "");
    assert_eq!(report.finished.reason, EndReason::Exited);
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("Traceback"), "{err}");
    assert!(
        err.contains("File \"") && err.contains("macro.upy\", line 3"),
        "{err}"
    );
    assert!(err.contains("NameError"), "{err}");
    assert!(err.contains("undefined_function"), "{err}");
}

#[test]
fn an_error_in_a_function_shows_the_call_chain() {
    let (report, _) = run("def f():\n    return 1 / 0\n\n\nf()\n", "");
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("line 5, in <module>"), "{err}");
    assert!(err.contains("line 2, in f"), "{err}");
    assert!(err.contains("ZeroDivisionError"), "{err}");
}

#[test]
fn a_syntax_error_shows_the_line_number() {
    let (report, _) = run("a = 1\nb = 2\nc = = 3\n", "");
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("macro.upy\", line 3"), "{err}");
    assert!(err.contains("SyntaxError"), "{err}");
}

#[test]
fn an_error_response_becomes_an_editor_error() {
    let code = r#"
try:
    Editor.GetLineStr('x')
except EditorError as e:
    print(isinstance(e, Exception), type(e).__name__, e.code, e.message[:11])
try:
    Editor.GetLineCount(5)
except EditorError as e:
    print(e.code, e.message.startswith("GetLineCount: "), str(e) == e.message)
Editor.Jump(1, 2)
print('not reached')
"#;
    let (report, _) = run(code, "a");
    let lines = stdout(&report);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[0], "True EditorError -32602 GetLineStr:");
    assert_eq!(lines[1], "-32602 True True");
    // The third call is not caught: the macro ends with an error that names the call and the
    // line of the script.
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("EditorError: Jump: PL/SQL"), "{err}");
    assert!(err.contains("macro.upy\", line 10, in <module>"), "{err}");
}

#[test]
fn what_cannot_be_sent_is_a_bad_argument() {
    let code = r#"
for value in (object(), float("nan"), chr(0xD800)):
    try:
        Editor.InsText(value)
    except EditorError as e:
        print(e.code, e.message.split(":")[0])
"#;
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["-32602 InsText", "-32602 InsText", "-32602 InsText"]
    );
    assert_eq!(doc.text(), "");
}

#[test]
fn a_cyclic_argument_is_a_catchable_error() {
    let code = "a = []\na.append(a)\ntry:\n    Editor.InsText(a)\nexcept RuntimeError:\n    print('too deep')\nEditor.InsText('ok')";
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["too deep"]);
    assert_eq!(doc.text(), "ok");
}

#[test]
fn a_wrong_type_is_the_editors_answer() {
    let code = r#"
try:
    Editor.InsText(5)
except EditorError as e:
    print(e.code, e.message)
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    let lines = stdout(&report);
    assert!(lines[0].starts_with("-32602 InsText: "), "{lines:?}");
}

#[test]
fn a_trailing_none_is_left_out_and_a_whole_float_is_an_integer() {
    // Jump(line, flag) without the flag means flag 1; the float is what `4 / 2` gives.
    let code = "flag = None\nEditor.Jump(4 / 2, flag)\nprint(Editor.GetLineStr(0), end='')";
    let (report, _) = run(code, "a\nb\nc");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["b"]);
}

#[test]
fn the_script_sees_the_initialize_message() {
    let code = "i = Editor.info\nprint(i['protocolVersion'], i['apiVersion'], i['editor']['name'], i['document']['path'], i['processId'] > 0, 'script' in i)";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [format!("1 1 UME Editor {DOCUMENT_PATH} True False")]
    );
}

#[test]
fn output_stays_in_order_with_requests() {
    // No line break before the request: the host splits the line at the RS.
    let code = "import sys\nsys.stdout.write('abc')\nEditor.InsText('x')\nprint('def')\nprint('tail', end='')";
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["abc", "def", "tail"]);
    assert_eq!(doc.text(), "x");
}

#[test]
fn stderr_is_user_output() {
    let (report, _) = run(
        "import sys\nsys.stderr.write('エラー出力\\n')\nprint('out')\nprint('also', file=sys.stderr)",
        "",
    );
    assert_clean_exit(&report);
    assert_eq!(report.lines(Stream::Stderr), ["エラー出力", "also"]);
    assert_eq!(stdout(&report), ["out"]);
}

#[test]
fn the_built_in_modules_are_there_and_files_are_not() {
    let code = r#"
import json, math, struct, collections, binascii, heapq, array, gc, sys, re
print(json.dumps({"a": [1, 2.5, None, "日本語"]}), math.floor(2.5), 2 ** 100)
print(sys.platform, len(sys.version) > 0)
try:
    open("x.txt")
except OSError:
    print("no files")
print(sys.stdin.read() == "")
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "{\"a\": [1, 2.5, null, \"日本語\"]} 2 1267650600228229401496703205376",
            "ume True",
            "no files",
            "True",
        ]
    );
}

#[test]
fn sys_exit_ends_the_macro() {
    let (report, _) = run("print('a')\nimport sys\nsys.exit(0)\nprint('b')", "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["a"]);
    let (report, _) = run("import sys\nsys.exit(3)", "");
    assert_eq!(report.finished.exit_code, Some(1));
}

#[test]
fn deep_recursion_is_an_exception() {
    let code = "def f(n):\n    return f(n + 1) + 1\n\n\ntry:\n    f(0)\nexcept RuntimeError as e:\n    print('caught', e)\n";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report).len(), 1, "{}", stderr(&report));
    assert!(
        stdout(&report)[0].starts_with("caught"),
        "{:?}",
        stdout(&report)
    );
}

#[test]
fn a_large_document_fits_in_the_heap() {
    // 3 MB of text, selected and read as one string, and written back.
    let text = "あいうえお0123456789\n".repeat(150_000);
    let code = "Editor.SelectAll()\ns = Editor.GetSelectedString(0)\nprint(len(s), s.count('\\n'))\nEditor.InsText(s.upper())";
    let (report, doc) = run(code, &text);
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["2400000 150000"]);
    assert_eq!(doc.text(), text.to_uppercase());
}

#[test]
fn the_sample_macro_runs() {
    let sample = include_str!("../../ume-plugin/samples/sample.upy");
    let (report, doc) = run(sample, "りんご\nバナナ\nぶどう\nもも\n");
    assert_clean_exit(&report);
    let lines = stdout(&report);
    assert_eq!(lines[0], format!("行数: 5 ファイル: {DOCUMENT_PATH}"));
    assert!(
        lines.contains(&"選択: バナナ / IsTextSelected = 1"),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("エラー: -32602 GetLineStr")),
        "{lines:?}"
    );
    assert_eq!(
        doc.text(),
        "# サンプルマクロ 😀\nりんご\nみかん😀 (置き換えました)\nぶどう\nもも\n-- マクロの終わり --\n"
    );
}

#[test]
fn running_out_of_heap_is_a_memory_error() {
    let code = "try:\n    x = 'a' * (200 * 1024 * 1024)\nexcept MemoryError:\n    print('memory error')\nprint(len('b' * 1000))";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["memory error", "1000"]);
}

#[test]
fn a_shebang_line_and_a_bom_are_fine_and_keep_the_line_numbers() {
    let (report, _) = run(
        "#!/usr/bin/env python\nprint(1)\nraise ValueError('x')\n",
        "",
    );
    assert_eq!(stdout(&report), ["1"]);
    assert!(
        stderr(&report).contains("macro.upy\", line 3"),
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
        "macro.upy",
        "print('start')\nwhile True:\n    pass",
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
        "macro.upy",
        "print('started')\nwhile True:\n    pass",
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

// MicroPython's `re` (the re1.5 engine, byte based). What it has: compile, match, search, sub,
// and split on a compiled pattern; groups, span/start/end; no flags, no lookaround, no `{m,n}`.
#[test]
fn re_compile_search_match_and_groups() {
    let code = r#"
import re
m = re.search(r"(\d+)-(\d+)", "tel 03-1234 x")
print(m.group(0), m.group(1), m.group(2), m.groups(), m.span(1), m.start(2), m.end(0))
print(re.match("a", "xa"), re.match("x", "xa").group(0))
r = re.compile("a(b)?")
print(r.match("ac").group(1), r.search("xab").group(1))
print(re.search(r"(?:ab)+", "ababc").group(0), re.search("a+?", "aaa").group(0))
print(re.search("(a)|(b)", "b").groups())
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "03-1234 03 1234 ('03', '1234') (4, 6) 7 11",
            "None x",
            "None b",
            "abab a",
            "(None, 'b')",
        ]
    );
}

#[test]
fn re_sub_and_split() {
    let code = r#"
import re
print(re.sub("a", "-", "banana"), re.sub("a", "-", "banana", 1))
print(re.sub("(a)(b)", r"\2\1", "abab"), re.sub("a", lambda m: "X", "aa"))
r = re.compile(",")
print(r.split("a,b,,c"), r.split("a,b,c", 1))
print(re.compile("a").sub("-", "banana", 1))
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "b-n-n- b-nana",
            "baba XX",
            "['a', 'b', '', 'c'] ['a', 'b,c']",
            "b-nana",
        ]
    );
}

// re1.5 works on the bytes of the UTF-8 text: a Japanese literal, an alternative and a character
// range match whole characters, but `.` and a quantifier apply to one byte, so a match that ends
// inside a character is a UnicodeError when the text is taken out.
#[test]
fn re_handles_japanese_literals_and_ranges_but_not_single_characters() {
    let code = r#"
import re
print(re.search("日本", "あ日日本").group(0), re.search("東京|大阪", "行東京").group(0))
print(re.search("[あ-ん]+", "xひらがなx").group(0), re.search("(日本)+", "あ日本日本").group(0))
print(re.sub("日本", "ニッポン", "日本と日本"), re.compile("、").split("あ、い、う"))
try:
    re.search(".", "日本").group(0)
except UnicodeError:
    print("byte match")
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "日本 東京",
            "ひらがな 日本日本",
            "ニッポンとニッポン ['あ', 'い', 'う']",
            "byte match",
        ]
    );
}

#[test]
fn re_rejects_invalid_and_unsupported_patterns() {
    let code = r#"
import re
for p in ["(", ")", "[a", "*a", "x**", "(?<=a)b", "(?=a)b", "(?P<n>a)", "(?i)a"]:
    try:
        re.compile(p)
        print(p, "compiled")
    except ValueError as e:
        print(p, "ValueError")
print(hasattr(re, "I"), hasattr(re, "findall"), hasattr(re, "split"))
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    let lines = stdout(&report);
    assert_eq!(lines.len(), 10, "{lines:?}");
    for line in &lines[..9] {
        assert!(line.ends_with("ValueError"), "{line}");
    }
    assert_eq!(lines[9], "False False False");
}

// UME Editor's change to modre.c: each callback call gets its own match object.
#[test]
fn re_sub_callback_matches_stay_valid_after_sub() {
    let code = r#"
import re
saved = []
def cb(m):
    saved.append(m)
    return "X"
print(re.sub("(a)(b|c)", cb, "ab-ac-ab"))
print([(m.group(0), m.group(1), m.group(2), m.span(0)) for m in saved])
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "X-X-X",
            "[('ab', 'a', 'b', (0, 2)), ('ac', 'a', 'c', (3, 5)), ('ab', 'a', 'b', (6, 8))]",
        ]
    );
}

// UME Editor's change to compilecode.c: a class range cut off by the end of the pattern.
#[test]
fn re_unterminated_class_ranges_are_value_errors() {
    let code = r#"
import re
for p in ["[a-", "[\\"]:
    try:
        re.compile(p)
        print("compiled")
    except ValueError:
        print("ValueError")
"#;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["ValueError", "ValueError"]);
}

#[test]
fn re_works_on_the_text_of_the_document() {
    let code = r#"
import re
n = Editor.GetLineCount(0)
r = re.compile(r"(\d+)円")
total = 0
for i in range(1, n + 1):
    m = r.search(Editor.GetLineStr(i))
    if m:
        total += int(m.group(1))
print(total)
Editor.InsText(re.sub("りんご", "みかん", Editor.GetLineStr(1)))
"#;
    let (report, doc) = run(code, "りんご 120円\nバナナ 80円\nなし\n");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["200"]);
    assert!(doc.text().contains("みかん 120円"), "{}", doc.text());
}
