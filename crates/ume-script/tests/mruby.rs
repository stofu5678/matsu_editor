//! End-to-end tests of the mruby engine: the built `ume-script` program runs as a macro process
//! under the editor's own session code (`ume_plugin::session`), against a plain text model.

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
    assert_eq!(config.language, Language::Bundled(Engine::Mruby));
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
    run_in("macro.mruby", code, text, Duration::from_secs(30))
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
    let code = r##"
puts Editor.GetLineCount(0)
puts Editor.GetLineStr(2) == "two\n"
Editor.Jump(3)
puts Editor.GetLineStr(0)
puts Editor.IsTextSelected
Editor.SelectAll
puts "#{Editor.IsTextSelected} #{Editor.GetSelectedString(0).size}"
puts Editor.GetFilename
"##;
    let (report, _) = run(code, "one\ntwo\nthree");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["3", "true", "three", "0", "1 13", DOCUMENT_PATH]
    );
}

#[test]
fn japanese_text_goes_both_ways() {
    let code = r##"
line = Editor.GetLineStr(1)
puts line
puts line.length
Editor.GoFileEnd
Editor.InsText("、絵文字😀と" + "日本語")
puts "標準出力の日本語"
"##;
    let (report, doc) = run(code, "こんにちは");
    assert_clean_exit(&report);
    // `length` counts characters: the build has MRB_UTF8_STRING.
    assert_eq!(stdout(&report), ["こんにちは", "5", "標準出力の日本語"]);
    assert_eq!(doc.text(), "こんにちは、絵文字😀と日本語");
}

#[test]
fn a_script_in_a_path_with_japanese_runs_and_names_it_in_errors() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("マクロ😀");
    std::fs::create_dir(&folder).unwrap();
    let script = folder.join("あ.mruby");
    std::fs::write(&script, "puts 'ok'\nraise 'boom'\n").unwrap();
    let mut config = MacroConfig::for_script(&script, None).unwrap();
    config.interpreter = env!("CARGO_BIN_EXE_ume-script").into();
    let mut doc = TextModel::new("", None);
    let report =
        session::run_blocking(config, |m| api::handle_message(&mut doc, m)).expect("starts");
    assert_eq!(stdout(&report), ["ok"]);
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("あ.mruby:2: boom (RuntimeError)"), "{err}");
}

#[test]
fn a_runtime_error_shows_the_line_number_of_the_script() {
    let (report, _) = run("a = 1\nb = 2\nundefined_function\n", "");
    assert_eq!(report.finished.reason, EndReason::Exited);
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("macro.mruby:3:"), "{err}");
    assert!(err.contains("undefined_function"), "{err}");
    assert!(err.ends_with("(NoMethodError)"), "{err}");
}

#[test]
fn an_error_inside_a_method_names_the_line_of_the_raise() {
    let (report, _) = run("def f\n  raise ArgumentError, 'bad'\nend\n\nf\n", "");
    assert_eq!(report.finished.exit_code, Some(1));
    assert!(
        stderr(&report).ends_with("macro.mruby:2: bad (ArgumentError)"),
        "{}",
        stderr(&report)
    );
}

#[test]
fn a_syntax_error_shows_the_line_number() {
    let (report, _) = run("a = 1\nb = 2\nc = = 3\nputs 'never'\n", "");
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("macro.mruby:3: "), "{err}");
    assert!(err.ends_with("(SyntaxError)"), "{err}");
    // Nothing ran.
    assert_eq!(stdout(&report), Vec::<&str>::new());
}

#[test]
fn an_error_response_becomes_an_editor_error() {
    let code = r##"
begin
  Editor.GetLineStr('x')
rescue EditorError => e
  puts "#{e.class} #{e.code} #{e.is_a?(StandardError)} #{e.backtrace.first.include?('macro.mruby:3')}"
end
begin
  Editor.GetLineCount(5)
rescue => e
  puts e.code
  puts e.message
end
Editor.Jump(1, 2)
puts 'not reached'
"##;
    let (report, _) = run(code, "a");
    let lines = stdout(&report);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(lines[0], "EditorError -32602 true true");
    assert_eq!(lines[1], "-32602");
    assert!(lines[2].starts_with("GetLineCount: "), "{lines:?}");
    // The third call is not rescued: the macro ends with an error that names the call and the
    // line.
    assert_eq!(report.finished.exit_code, Some(1));
    let err = stderr(&report);
    assert!(err.contains("macro.mruby:13: Jump: "), "{err}");
    assert!(err.ends_with("(EditorError)"), "{err}");
}

#[test]
fn what_cannot_be_sent_is_a_bad_argument() {
    let code = r##"
begin
  Editor.InsText(1..2)
rescue EditorError => e
  puts "#{e.code} #{e.message}"
end
"##;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["-32602 InsText: argument \"text\" must be string, got Range"]
    );
}

#[test]
fn a_string_that_is_not_utf8_is_refused_not_replaced() {
    let code = r##"
begin
  Editor.InsText("a" + 255.chr)
rescue EditorError => e
  puts "#{e.code} #{e.message}"
end
"##;
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        ["-32602 InsText: argument \"text\" must be string, got a string that is not valid UTF-8"]
    );
    assert_eq!(doc.text(), "");
}

#[test]
fn a_trailing_nil_is_left_out_and_a_whole_float_is_an_integer() {
    // Jump(line, flag) without the flag means flag 1; the float is what `4 / 2.0` gives.
    let code = "flag = nil\nEditor.Jump(4 / 2.0, flag)\nprint Editor.GetLineStr(0)";
    let (report, _) = run(code, "a\nb\nc");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["b"]);
}

#[test]
fn the_script_sees_the_initialize_message() {
    let code = "i = Editor.info\nputs [i['protocolVersion'], i['apiVersion'], i['editor']['name'], i['document']['path'], i['processId'] > 0, i['script'].inspect].join(' ')";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [format!("1 1 UME Editor {DOCUMENT_PATH} true nil")]
    );
}

#[test]
fn output_stays_in_order_with_requests() {
    // `print` has no line break before the request; the host splits the line at the RS.
    let code = "print 'abc'\nEditor.InsText('x')\nputs 'def'\nprint 'tail'";
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["abc", "def", "tail"]);
    assert_eq!(doc.text(), "x");
}

#[test]
fn p_and_inspect_work_and_line_breaks_are_bare() {
    let (report, _) = run("p [1, 'あ', nil, 2.5, {a: 1}]\nputs 'x'", "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["[1, \"あ\", nil, 2.5, {a: 1}]", "x"]);
}

#[test]
fn a_shebang_line_and_a_bom_are_fine_and_keep_the_line_numbers() {
    let (report, _) = run("#!/usr/bin/env mruby\nputs 1\nraise 'x'\n", "");
    assert_eq!(stdout(&report), ["1"]);
    assert!(
        stderr(&report).contains("macro.mruby:3: x"),
        "{}",
        stderr(&report)
    );
    let (report, _) = run("\u{feff}puts 'bom'", "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["bom"]);
}

#[test]
fn the_bundled_gems_are_there() {
    let code = r##"
puts format('%05.1f|%-3s|', 3.14159, 'a')
puts [3, 1, 2].sort.map { |x| x * 2 }.inspect
puts({ 'a' => 1 }.to_a.flatten.inspect)
puts 'あいう'.reverse
puts 2**70
puts Math.sqrt(16)
puts Time.at(0).utc.strftime('%Y-%m-%d')
puts (1..3).each_slice(2).to_a.inspect
puts eval('1 + 2')
S = Struct.new(:a)
puts S.new(5).a
"##;
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(
        stdout(&report),
        [
            "003.1|a  |",
            "[2, 4, 6]",
            "[\"a\", 1]",
            "ういあ",
            "1180591620717411303424",
            "4.0",
            "1970-01-01",
            "[[1, 2], [3]]",
            "3",
            "5",
        ]
    );
}

#[test]
fn stderr_is_user_output() {
    let code = "$stderr.puts 'エラー出力'\nwarn 'w', 'x'\n$stdout.print 'o'\nputs 'ut'";
    let (report, _) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(report.lines(Stream::Stderr), ["エラー出力", "w", "x"]);
    assert_eq!(stdout(&report), ["out"]);
}

#[test]
fn long_text_goes_both_ways() {
    let code = "text = 'あ😀' * 50000\nEditor.InsText(text)\nputs Editor.GetLineStr(1).length\nputs Editor.GetLineCount(0)";
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["100000", "1"]);
    assert_eq!(doc.text().chars().count(), 100_000);
}

#[test]
fn a_script_can_rescue_and_go_on_after_an_error_response() {
    let code = r##"
def try
  yield
rescue EditorError => e
  puts "rescued #{e.code}"
ensure
  puts 'ensure'
end
try { Editor.GetLineStr('x') }
Editor.InsText('after')
"##;
    let (report, doc) = run(code, "");
    assert_clean_exit(&report);
    assert_eq!(stdout(&report), ["rescued -32602", "ensure"]);
    assert_eq!(doc.text(), "after");
}

#[test]
fn an_infinite_loop_is_stopped_by_the_timeout() {
    let started = Instant::now();
    let (report, _) = run_in(
        "macro.mruby",
        "puts 'start'\nloop { }",
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
        "macro.mruby",
        "puts 'started'\nloop { }",
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
fn a_missing_initialize_is_refused() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_ume-script"))
        .args(["--engine", "mruby"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(child.stdin.take().unwrap(), "{{\"method\":\"other\"}}").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn the_sample_macro_runs() {
    let sample =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ume-plugin/samples/sample.mruby");
    let mut config = MacroConfig::for_script(&sample, None).unwrap();
    config.interpreter = env!("CARGO_BIN_EXE_ume-script").into();
    let mut doc = TextModel::new("りんご\nみかん😀\nぶどう\n", None);
    let report =
        session::run_blocking(config, |m| api::handle_message(&mut doc, m)).expect("starts");
    assert_clean_exit(&report);
    let text = doc.text();
    assert!(text.starts_with("# サンプルマクロ"), "{text}");
    assert!(text.contains("みかん😀"), "{text}");
    assert!(!stdout(&report).is_empty());
}
