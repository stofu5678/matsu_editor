//! Tests of the bridge: calls by Sakura's macro name run on a real `CommandContext`.
//!
//! Most tests send JSON-RPC messages or calls directly. Two run a JavaScript and a Python macro
//! through the real session (they print "SKIP" and pass when the interpreter is missing, like the
//! tests of the host).

use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use ume_commands::context::CommandContext;
use ume_commands::layout::PlainLayout;
use ume_commands::registry::Registry;
use ume_core::buffer::{Document, Pos};
use ume_core::cursor::Cursor;
use ume_core::model::DocumentModel;

use super::*;
use crate::protocol::INVALID_PARAMS;
use crate::session::{self, Language, MacroConfig, Stream};

/// A document with a cursor, the way the editor holds them.
struct Fixture {
    model: DocumentModel,
    cursor: Cursor,
    layout: PlainLayout,
    path: Option<PathBuf>,
}

impl Fixture {
    fn new(text: &str) -> Self {
        Self {
            model: DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap()),
            cursor: Cursor::default(),
            layout: PlainLayout::default(),
            path: None,
        }
    }

    fn with_path(mut self, path: &str) -> Self {
        self.path = Some(PathBuf::from(path));
        self
    }

    fn cx(&mut self) -> CommandContext<'_> {
        CommandContext::new(
            &mut self.model,
            &mut self.cursor,
            &mut self.layout,
            self.path.as_deref(),
        )
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, RpcError> {
        let params = params.as_array().expect("params are an array").clone();
        call(Registry::global(), &mut self.cx(), method, &params)
    }

    /// The whole text, with the line endings as they are.
    fn text(&self) -> String {
        let doc = self.model.doc();
        let end = doc.end_pos().unwrap();
        doc.text_between(Pos::new(0, 0), end).unwrap()
    }

    fn caret(&self) -> (usize, usize) {
        (self.cursor.caret().line, self.cursor.caret().col)
    }
}

// ---- commands ----

#[test]
fn a_name_runs_the_registered_command() {
    let mut f = Fixture::new("one\ntwo\nthree");
    // F_DOWN has the command cursor_down, F_WCHAR has input_char, F_GOFILEEND has go_file_end.
    f.call("Down", json!([])).unwrap();
    assert_eq!(f.caret(), (1, 0));
    f.call("GoFileEnd", json!([])).unwrap();
    assert_eq!(f.caret(), (2, 5));
    f.call("Char", json!([0x65E5])).unwrap();
    assert_eq!(f.text(), "one\ntwo\nthree日");
    f.call("Up", json!([])).unwrap();
    assert_eq!(f.caret().0, 1);
    f.call("GoFileTop", json!([])).unwrap();
    assert_eq!(f.caret(), (0, 0));
}

#[test]
fn the_old_s_prefix_is_accepted() {
    let mut f = Fixture::new("a\nb");
    f.call("S_Down", json!([])).unwrap();
    assert_eq!(f.caret(), (1, 0));
}

#[test]
fn each_command_is_one_undo_step() {
    let mut f = Fixture::new("");
    f.call("Char", json!([97])).unwrap();
    f.call("Char", json!([98])).unwrap();
    assert_eq!(f.text(), "ab");
    f.call("Undo", json!([])).unwrap();
    assert_eq!(f.text(), "a");
    f.call("Redo", json!([])).unwrap();
    assert_eq!(f.text(), "ab");
}

#[test]
fn a_command_that_beeps_does_not_fail() {
    // Up on the first line does nothing; Sakura's ErrorBeep() is not an error for the macro.
    let mut f = Fixture::new("a");
    assert_eq!(f.call("Up", json!([])), Ok(Value::Null));
    assert_eq!(f.caret(), (0, 0));
}

#[test]
fn arguments_are_checked_against_sakura_s_table() {
    let mut f = Fixture::new("a");
    // CSMacroMgr.cpp: Char takes one number (VT_I4).
    for params in [json!(["a"]), json!([1.5]), json!([null]), json!([1, 2])] {
        let err = f.call("Char", params.clone()).unwrap_err();
        assert_eq!(err.code, INVALID_PARAMS, "{params}");
    }
    // Down takes none.
    assert_eq!(f.call("Down", json!([1])).unwrap_err().code, INVALID_PARAMS);
    // A number that does not fit an i64 is not turned into another number.
    let err = f
        .call("Char", json!([18446744073709551615u64]))
        .unwrap_err();
    assert_eq!(err.code, INVALID_PARAMS);
    // A missing argument is passed on; the command says what it needs.
    let err = f.call("Char", json!([])).unwrap_err();
    assert_eq!(err.code, INVALID_PARAMS);
    assert_eq!(f.text(), "a");
}

#[test]
fn a_command_without_what_it_needs_says_so() {
    // The fixture has no clipboard (as when a macro runs without a window), so Copy cannot run.
    let mut f = Fixture::new("abc");
    let err = f.call("Copy", json!([])).unwrap_err();
    assert_eq!(err.code, UNAVAILABLE);
    assert!(err.message.contains("clipboard"), "{}", err.message);
    assert_eq!(f.text(), "abc");
}

#[test]
fn a_name_without_a_command_says_so() {
    let mut f = Fixture::new("a");
    // F_FILESAVE is a command of Sakura's table, and this crate has no command for it yet.
    let err = f.call("FileSave", json!([])).unwrap_err();
    assert_eq!(err.code, NOT_IMPLEMENTED);
    assert!(err.message.contains("FileSave"), "{}", err.message);
    assert!(err.message.contains("F_FILESAVE"), "{}", err.message);
    // A function that returns a value and that the editor cannot answer yet.
    let err = f.call("GetViewTop", json!([])).unwrap_err();
    assert_eq!(err.code, NOT_IMPLEMENTED);
    assert!(err.message.contains("GetViewTop"), "{}", err.message);
    // A name that Sakura does not have.
    let err = f.call("NoSuchMacro", json!([])).unwrap_err();
    assert_eq!(err.code, METHOD_NOT_FOUND);
    assert_eq!(
        f.call("down", json!([])).unwrap_err().code,
        METHOD_NOT_FOUND
    );
}

#[test]
fn every_name_gets_an_answer_and_none_panics() {
    // Each function of Sakura's tables, called with no argument and with wrong ones: the result
    // is a value or an error, never a panic (the host would answer a panic with -32603, and
    // that is a bug here).
    use ume_commands::sakura::macro_functions::{MACRO_COMMANDS, MACRO_FUNCTIONS};
    for row in MACRO_COMMANDS.iter().chain(MACRO_FUNCTIONS) {
        let mut f = Fixture::new("one\ntwo");
        let outcome = f.call(row.name, json!([]));
        if let Err(e) = outcome {
            assert_ne!(e.code, INTERNAL_ERROR, "{}: {}", row.name, e.message);
        }
    }
}

// ---- functions that return a value ----

#[test]
fn the_selected_text_has_only_lf_line_breaks() {
    // CR LF, CR, NEL, LS and PS are line endings of the document.
    let mut f = Fixture::new("a\r\nb\rc\u{85}d\u{2028}e\u{2029}f");
    f.call("SelectAll", json!([])).unwrap();
    assert_eq!(
        f.call("GetSelectedString", json!([0])),
        Ok(json!("a\nb\nc\nd\ne\nf"))
    );
}

#[test]
fn value_functions_are_answered_from_the_document() {
    let mut f = Fixture::new("あ😀\r\nb\nlast").with_path("/work/テスト.txt");
    assert_eq!(f.call("GetLineCount", json!([0])), Ok(json!(3)));
    // The line break is "\n" for the macro whatever the document has.
    assert_eq!(f.call("GetLineStr", json!([1])), Ok(json!("あ😀\n")));
    assert_eq!(f.call("GetLineStr", json!([3])), Ok(json!("last")));
    assert_eq!(f.call("GetLineStr", json!([4])), Ok(json!("")));
    assert_eq!(
        f.call("GetFilename", json!([])),
        Ok(json!("/work/テスト.txt"))
    );
    assert_eq!(f.call("IsTextSelected", json!([])), Ok(json!(0)));
    assert_eq!(f.call("GetSelectedString", json!([0])), Ok(json!("")));
    f.call("SelectAll", json!([])).unwrap();
    assert_eq!(f.call("IsTextSelected", json!([])), Ok(json!(1)));
    assert_eq!(
        f.call("GetSelectedString", json!([0])),
        Ok(json!("あ😀\nb\nlast"))
    );
    // Line 0 is the line of the caret (at the end of the text after SelectAll).
    assert_eq!(f.call("GetLineStr", json!([0])), Ok(json!("last")));
    // A document without a file.
    assert_eq!(
        Fixture::new("").call("GetFilename", json!([])),
        Ok(json!(""))
    );
}

#[test]
fn value_functions_check_their_arguments() {
    let mut f = Fixture::new("a");
    assert_eq!(
        f.call("GetLineStr", json!(["1"])).unwrap_err().code,
        INVALID_PARAMS
    );
    // The reserved argument of GetLineCount must be 0 (CMacro::HandleFunction).
    assert_eq!(
        f.call("GetLineCount", json!([1])).unwrap_err().code,
        INVALID_PARAMS
    );
    assert_eq!(
        f.call("GetFilename", json!([1])).unwrap_err().code,
        INVALID_PARAMS
    );
}

// ---- the basic API where there is no command yet ----

#[test]
fn the_basic_api_runs_where_no_command_is_registered() {
    let registry = Registry::global();
    // If one of these gets a command, the test must move to the section above.
    for name in ["InsText", "AddTail", "MoveCursor", "Jump"] {
        let code = ume_commands::macro_names::by_name(name).unwrap().code;
        assert!(
            registry.get_by_code(code).is_none(),
            "{name} has a command now"
        );
    }
    let mut f = Fixture::new("abc\ndef");
    f.call("MoveCursor", json!([1, 2, 0])).unwrap();
    f.call("InsText", json!(["X\nY"])).unwrap();
    assert_eq!(f.text(), "aX\nYbc\ndef");
    assert_eq!(f.caret(), (1, 1));
    f.call("AddTail", json!(["!"])).unwrap();
    assert_eq!(f.text(), "aX\nYbc\ndef!");
    f.call("Jump", json!([2])).unwrap();
    assert_eq!(f.caret(), (1, 0));
    // MoveCursor with the select option, then InsText replaces the selection.
    f.call("MoveCursor", json!([1, 1, 0])).unwrap();
    f.call("MoveCursor", json!([1, 3, 1])).unwrap();
    assert_eq!(f.call("GetSelectedString", json!([0])), Ok(json!("aX")));
    f.call("InsText", json!(["-"])).unwrap();
    assert_eq!(f.text(), "-\nYbc\ndef!");
    // The edits are undoable like any other.
    f.call("Undo", json!([])).unwrap();
    assert_eq!(f.text(), "aX\nYbc\ndef!");
}

#[test]
fn column_counts_are_utf16_units_in_move_cursor() {
    let mut f = Fixture::new("a😀b");
    f.call("MoveCursor", json!([1, 4, 0])).unwrap();
    assert_eq!(f.caret(), (0, "a😀".len()));
}

// ---- messages ----

#[test]
fn handle_message_answers_with_json_rpc() {
    let mut f = Fixture::new("日本語😀");
    let registry = Registry::global();
    let mut send = |text: &str| -> Option<Value> {
        handle_message(registry, &mut f.cx(), text).map(|l| {
            assert!(l.is_ascii(), "{l}");
            serde_json::from_str(&l).unwrap()
        })
    };
    let ok = send(r#"{"jsonrpc":"2.0","id":3,"method":"GetLineStr","params":[1]}"#).unwrap();
    assert_eq!((&ok["id"], &ok["result"]), (&json!(3), &json!("日本語😀")));
    let missing = send(r#"{"jsonrpc":"2.0","id":"x","method":"Nope","params":[]}"#).unwrap();
    assert_eq!(missing["error"]["code"], METHOD_NOT_FOUND);
    let todo = send(r#"{"jsonrpc":"2.0","id":4,"method":"FileSave","params":[]}"#).unwrap();
    assert_eq!(todo["error"]["code"], NOT_IMPLEMENTED);
    assert_eq!(send("not json").unwrap()["error"]["code"], -32700);
    // A notification gets no response.
    assert_eq!(send(r#"{"jsonrpc":"2.0","method":"GoFileEnd"}"#), None);
}

#[test]
fn a_panic_in_a_command_becomes_an_error_response() {
    // The bridge answers through `api::respond`, which catches a panic. A command that panics
    // is simulated by a call that the closure turns into a panic.
    let line = api::respond(
        r#"{"jsonrpc":"2.0","id":1,"method":"Down","params":[]}"#,
        |_, _| panic!("boom"),
    )
    .unwrap();
    let value: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(value["error"]["code"], INTERNAL_ERROR);
}

#[test]
fn context_document_reports_the_first_error() {
    let mut f = Fixture::new("a");
    let mut cx = f.cx();
    let mut doc = ContextDocument::new(&mut cx);
    // Positions are clamped by the context, so a move cannot fail; an edit that cannot be done
    // would set `error`.
    doc.move_cursor((9, 9), false);
    assert!(doc.error.is_none());
    assert_eq!(doc.cursor(), (0, 1));
    assert_eq!(doc.line_len(5), 0);
    assert_eq!(doc.line_text(5), "");
}

// ---- real macros ----

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
        eprintln!("SKIP: {} is not available", language.display_name());
    }
    ok
}

/// Runs a macro on the bridge. `None` when the interpreter is missing.
fn run_macro(language: Language, code: &str, f: &mut Fixture) -> Option<crate::session::Report> {
    if !available(language) {
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let name = match language {
        Language::JavaScript => "macro.js",
        Language::Python => "macro.py",
        Language::Ruby => "macro.rb",
        Language::Php => "macro.php",
    };
    let script = dir.path().join(name);
    // A PHP file starts with `<?php`.
    let code = if language == Language::Php {
        format!("<?php {code}")
    } else {
        code.to_string()
    };
    std::fs::write(&script, code).unwrap();
    let mut config = MacroConfig::for_script(&script, None).unwrap();
    config.temp_dir = Some(dir.path().to_path_buf());
    let registry = Registry::global();
    let mut cx = f.cx();
    Some(session::run_blocking(config, |m| handle_message(registry, &mut cx, m)).unwrap())
}

#[test]
fn macros_call_commands_by_sakura_s_names() {
    // `Left`, `Char` and `DeleteBack` are not in the basic API table: the generated libraries
    // send any Sakura-like name as it is.
    let js = r#"
Editor.GoFileEnd();
Editor.Char(0x41);
Editor.Left();
Editor.DeleteBack();
Editor.Down();
console.log(Editor.GetLineCount(0), JSON.stringify(Editor.GetLineStr(1)));
try { Editor.FileSave(); } catch (e) { console.log('caught', e.code); }
try { Editor.NoSuchMacro(); } catch (e) { console.log('caught', e.code); }
console.log(typeof Editor.then, typeof Editor.toString);
"#;
    let py = r#"
Editor.GoFileEnd()
Editor.Char(0x41)
Editor.Left()
Editor.DeleteBack()
Editor.Down()
print(Editor.GetLineCount(0), Editor.GetLineStr(1).encode('unicode_escape').decode())
try:
    Editor.FileSave()
except EditorError as e:
    print('caught', e.code)
try:
    Editor.NoSuchMacro()
except EditorError as e:
    print('caught', e.code)
try:
    Editor.lower_case_name
except AttributeError:
    print('attribute error')
"#;
    let rb = r##"
Editor.GoFileEnd
Editor.Char(0x41)
Editor.Left
Editor.DeleteBack
Editor.Down
puts "#{Editor.GetLineCount(0)} #{Editor.GetLineStr(1).inspect}"
begin; Editor.FileSave; rescue EditorError => e; puts "caught #{e.code}"; end
begin; Editor.NoSuchMacro; rescue EditorError => e; puts "caught #{e.code}"; end
begin; Editor.lower_case_name; rescue NoMethodError; puts 'no method'; end
"##;
    let php = r#"
Editor::GoFileEnd();
Editor::Char(0x41);
Editor::Left();
Editor::DeleteBack();
Editor::Down();
echo Editor::GetLineCount(0) . ' ' . json_encode(Editor::GetLineStr(1)) . "\n";
try { Editor::FileSave(); } catch (EditorError $e) { echo 'caught ', $e->getCode(), "\n"; }
try { Editor::NoSuchMacro(); } catch (EditorError $e) { echo 'caught ', $e->getCode(), "\n"; }
try { Editor::lower_case_name(); } catch (BadMethodCallException $e) { echo "no method\n"; }
"#;
    let all = [
        (Language::JavaScript, js),
        (Language::Python, py),
        (Language::Ruby, rb),
        (Language::Php, php),
    ];
    for (language, code) in all {
        let mut f = Fixture::new("one\ntwo");
        let Some(report) = run_macro(language, code, &mut f) else {
            continue;
        };
        let stderr = report.lines(Stream::Stderr).join("\n");
        assert_eq!(report.finished.exit_code, Some(0), "{language:?}: {stderr}");
        // GoFileEnd, Char A ("twoA"), Left, DeleteBack removes "o" ("twA").
        assert_eq!(f.text(), "one\ntwA", "{language:?}");
        let out = report.lines(Stream::Stdout);
        match language {
            Language::JavaScript => assert_eq!(
                out,
                [
                    "2 \"one\\n\"",
                    "caught -32001",
                    "caught -32601",
                    "undefined function"
                ]
            ),
            Language::Python => assert_eq!(
                out,
                [
                    "2 one\\n",
                    "caught -32001",
                    "caught -32601",
                    "attribute error"
                ]
            ),
            Language::Ruby => assert_eq!(
                out,
                [
                    "2 \"one\\n\"",
                    "caught -32001",
                    "caught -32601",
                    "no method"
                ]
            ),
            Language::Php => assert_eq!(
                out,
                [
                    "2 \"one\\n\"",
                    "caught -32001",
                    "caught -32601",
                    "no method"
                ]
            ),
        }
    }
}

#[test]
fn the_sample_macros_run_on_the_commands() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
    for (language, name) in [
        (Language::JavaScript, "sample.js"),
        (Language::JavaScript, "sample.mjs"),
        (Language::Python, "sample.py"),
        (Language::Ruby, "sample.rb"),
        (Language::Php, "sample.php"),
    ] {
        if !available(language) {
            continue;
        }
        let mut config = MacroConfig::for_script(&dir.join(name), None).unwrap();
        let temp = tempfile::tempdir().unwrap();
        config.temp_dir = Some(temp.path().to_path_buf());
        let mut f = Fixture::new("りんご\nみかん😀\nぶどう\n");
        let registry = Registry::global();
        let report = {
            let mut cx = f.cx();
            session::run_blocking(config, |m| handle_message(registry, &mut cx, m)).unwrap()
        };
        let stderr = report.lines(Stream::Stderr).join("\n");
        assert_eq!(report.finished.exit_code, Some(0), "{name}: {stderr}");
        let text = f.text();
        assert!(text.starts_with("# サンプルマクロ"), "{name}: {text}");
        assert!(text.contains("みかん😀"), "{name}: {text}");
    }
}
