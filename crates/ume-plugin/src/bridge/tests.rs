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
    let err = f.call("InputBox", json!(["message"])).unwrap_err();
    assert_eq!(err.code, NOT_IMPLEMENTED);
    assert!(err.message.contains("InputBox"), "{}", err.message);
    // One that it can answer, but not without a window.
    let err = f.call("GetViewTop", json!([])).unwrap_err();
    assert_eq!(err.code, UNAVAILABLE);
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

/// Arguments that fit a row's types: 1 for a number and "x" for a string.
fn sample_arguments(row: &ume_commands::sakura::macro_functions::MacroFunction) -> Vec<Value> {
    use ume_commands::sakura::macro_functions::ArgType;
    row.args
        .iter()
        .map(|ty| match ty {
            ArgType::Int => json!(1),
            ArgType::Str => json!("x"),
        })
        .collect()
}

#[test]
fn every_command_of_the_table_runs_or_says_it_is_not_implemented() {
    use ume_commands::sakura::macro_functions::MACRO_COMMANDS;
    let registry = Registry::global();
    for row in MACRO_COMMANDS {
        let has_command = registry.get_by_code(row.code).is_some();
        let has_basic_api = api::FUNCS.iter().any(|f| f.name == row.name);
        for text in ["", "one\ntwo\nthree\n"] {
            // No argument at all (the last ones may be left out), and arguments that fit.
            for params in [Vec::new(), sample_arguments(row)] {
                let mut f = Fixture::new(text);
                let outcome = f.call(row.name, Value::Array(params.clone()));
                let context = format!("{} {params:?} on {text:?}", row.name);
                match outcome {
                    Ok(value) => {
                        assert_eq!(value, Value::Null, "{context}");
                        assert!(has_command || has_basic_api, "{context}: no command");
                    }
                    Err(e) if e.code == NOT_IMPLEMENTED => {
                        assert!(
                            !has_command && !has_basic_api,
                            "{context}: a command exists but the bridge says not implemented"
                        );
                        // The text names the function, as Sakura's table does.
                        assert!(e.message.contains(row.function), "{context}: {e:?}");
                    }
                    // A command that needs a window or a clipboard, or an argument.
                    Err(e) => {
                        assert!(has_command || has_basic_api, "{context}: unexpected {e:?}");
                        assert!(
                            [UNAVAILABLE, INVALID_PARAMS].contains(&e.code),
                            "{context}: {e:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn names_that_share_a_function_code_are_all_callable() {
    use ume_commands::sakura::macro_functions::MACRO_COMMANDS;
    let mut shared = 0;
    for row in MACRO_COMMANDS {
        for other in MACRO_COMMANDS
            .iter()
            .filter(|o| o.code == row.code && o.name != row.name)
        {
            shared += 1;
            let a = Fixture::new("a")
                .call(row.name, json!([]))
                .map_err(|e| e.code);
            let b = Fixture::new("a")
                .call(other.name, json!([]))
                .map_err(|e| e.code);
            assert_eq!(a, b, "{} and {}", row.name, other.name);
        }
    }
    assert!(
        shared > 0,
        "the table has old spellings (ViewMode, ReadOnly)"
    );
}

#[test]
fn too_many_arguments_and_wrong_types_are_invalid_params_for_every_name() {
    use ume_commands::sakura::macro_functions::{ArgType, MACRO_COMMANDS};
    for row in MACRO_COMMANDS {
        let mut f = Fixture::new("abc");
        // One argument more than the table has.
        let mut too_many = sample_arguments(row);
        too_many.push(json!(1));
        let err = f.call(row.name, Value::Array(too_many)).unwrap_err();
        assert_eq!(err.code, INVALID_PARAMS, "{}", row.name);
        // A string for a number, and a number for a string.
        for (i, ty) in row.args.iter().enumerate() {
            let mut params = sample_arguments(row);
            params[i] = match ty {
                ArgType::Int => json!("1"),
                ArgType::Str => json!(1),
            };
            let err = f.call(row.name, Value::Array(params)).unwrap_err();
            assert_eq!(err.code, INVALID_PARAMS, "{} argument {}", row.name, i + 1);
            assert!(err.message.contains(row.name), "{}", err.message);
        }
        assert_eq!(f.text(), "abc", "{}", row.name);
    }
}

#[test]
fn a_call_is_one_undo_step_whichever_way_it_runs() {
    // A registered command (Char) and the basic API (InsText) are both one undo block.
    let mut f = Fixture::new("");
    f.call("InsText", json!(["a\nb"])).unwrap();
    f.call("Char", json!([99])).unwrap();
    assert_eq!(f.text(), "a\nbc");
    f.call("Undo", json!([])).unwrap();
    assert_eq!(f.text(), "a\nb");
    f.call("Undo", json!([])).unwrap();
    assert_eq!(f.text(), "");
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
    for name in ["InsText", "AddTail"] {
        let code = ume_commands::macro_names::by_name(name).unwrap().code;
        assert!(
            registry.get_by_code(code).is_none(),
            "{name} has a command now"
        );
    }
    let mut f = Fixture::new("abc\ndef");
    // (`MoveCursor` has a command since the macro-only commands; it behaves the same.)
    f.call("MoveCursor", json!([1, 2, 0])).unwrap();
    f.call("InsText", json!(["X\nY"])).unwrap();
    assert_eq!(f.text(), "aX\nYbc\ndef");
    assert_eq!(f.caret(), (1, 1));
    f.call("AddTail", json!(["!"])).unwrap();
    assert_eq!(f.text(), "aX\nYbc\ndef!");
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
fn jump_is_a_command_with_sakuras_options() {
    let code = ume_commands::macro_names::by_name("Jump").unwrap().code;
    assert!(Registry::global().get_by_code(code).is_some());
    let mut f = Fixture::new("abc\ndef\nghi");
    f.call("Jump", json!([2])).unwrap();
    assert_eq!(f.caret(), (1, 0));
    // below 1 is the first line, past the end the last (option 1: lines up to line breaks)
    f.call("Jump", json!([99, 1])).unwrap();
    assert_eq!(f.caret(), (2, 0));
    f.call("Jump", json!([-3, 1])).unwrap();
    assert_eq!(f.caret(), (0, 0));
    // option 2 is PL/SQL: the second valid line
    f.call("Jump", json!([2, 3])).unwrap();
    assert_eq!(f.caret(), (1, 0));
}

#[test]
fn column_counts_are_utf16_units_in_move_cursor() {
    let mut f = Fixture::new("a😀b");
    f.call("MoveCursor", json!([1, 4, 0])).unwrap();
    assert_eq!(f.caret(), (0, "a😀".len()));
}

// ---- the commands that only macros call ----

#[test]
fn move_cursor_is_a_command_that_selects_and_selects_boxes() {
    let code = ume_commands::macro_names::by_name("MoveCursor")
        .unwrap()
        .code;
    assert!(Registry::global().get_by_code(code).is_some());
    let mut f = Fixture::new("abcd\nefgh\nijkl");
    f.call("MoveCursor", json!([2, 3, 1])).unwrap();
    assert_eq!(f.caret(), (1, 2));
    assert_eq!(f.call("IsTextSelected", json!([])), Ok(json!(1)));
    assert_eq!(
        f.call("GetSelectedString", json!([0])),
        Ok(json!("abcd\nef"))
    );
    // option 2 is a box now (the basic API refused it)
    f.call("MoveCursor", json!([1, 2, 0])).unwrap();
    f.call("MoveCursor", json!([3, 4, 2])).unwrap();
    assert_eq!(f.call("IsTextSelected", json!([])), Ok(json!(2)));
    assert_eq!(
        f.call("GetSelectedString", json!([0])),
        Ok(json!("bc\nfg\njk\n"))
    );
    // a number below 1 does nothing, and all three arguments are needed
    f.call("MoveCursor", json!([0, 1, 0])).unwrap();
    assert_eq!(f.call("IsTextSelected", json!([])), Ok(json!(2)));
    assert_eq!(
        f.call("MoveCursor", json!([1, 1])).unwrap_err().code,
        INVALID_PARAMS
    );
}

#[test]
fn move_cursor_layout_counts_the_cells_of_the_layout() {
    let mut f = Fixture::new("あいう\nabc");
    f.call("MoveCursorLayout", json!([1, 3, 0])).unwrap();
    assert_eq!(f.caret(), (0, "あ".len()));
    f.call("MoveCursorLayout", json!([2, 2, 1])).unwrap();
    assert_eq!(f.caret(), (1, 1));
    assert_eq!(
        f.call("GetSelectedString", json!([0])),
        Ok(json!("いう\na"))
    );
    assert_eq!(
        f.call("MoveCursorLayout", json!([1])).unwrap_err().code,
        INVALID_PARAMS
    );
}

#[test]
fn the_undo_buffer_functions_make_calls_one_step() {
    let mut f = Fixture::new("");
    f.call("AddRefUndoBuffer", json!([])).unwrap();
    f.call("Char", json!([97])).unwrap();
    f.call("Char", json!([98])).unwrap();
    f.call("SetUndoBuffer", json!([])).unwrap();
    f.call("Char", json!([99])).unwrap();
    assert_eq!(f.text(), "abc");
    f.call("Undo", json!([])).unwrap();
    assert_eq!(f.text(), "ab");
    f.call("Undo", json!([])).unwrap();
    assert_eq!(f.text(), "");
    // CommitUndoBuffer cuts the step in two
    f.call("AddRefUndoBuffer", json!([])).unwrap();
    f.call("Char", json!([120])).unwrap();
    f.call("CommitUndoBuffer", json!([])).unwrap();
    f.call("Char", json!([121])).unwrap();
    f.call("SetUndoBuffer", json!([])).unwrap();
    f.call("Undo", json!([])).unwrap();
    assert_eq!(f.text(), "x");
    f.call("Undo", json!([])).unwrap();
    assert_eq!(f.text(), "");
}

#[test]
fn append_undo_buffer_cursor_makes_a_step_of_the_caret() {
    let mut f = Fixture::new("ab\ncd");
    f.call("AppendUndoBufferCursor", json!([])).unwrap();
    f.call("GoFileEnd", json!([])).unwrap();
    assert_eq!(f.caret(), (1, 2));
    f.call("Undo", json!([])).unwrap();
    assert_eq!(f.caret(), (0, 0));
    assert_eq!(f.text(), "ab\ncd");
}

#[test]
fn the_quote_string_takes_one_text() {
    let mut f = Fixture::new("a");
    f.call("SetMsgQuoteStr", json!(["# "])).unwrap();
    for params in [json!([]), json!([1]), json!(["a", "b"])] {
        assert_eq!(
            f.call("SetMsgQuoteStr", params).unwrap_err().code,
            INVALID_PARAMS
        );
    }
}

#[test]
fn clipboard_empty_needs_a_clipboard() {
    // The macro runs without a window here, so there is no clipboard.
    let mut f = Fixture::new("a");
    assert_eq!(
        f.call("ClipboardEmpty", json!([])).unwrap_err().code,
        UNAVAILABLE
    );
    let code = ume_commands::macro_names::by_name("ClipboardEmpty")
        .unwrap()
        .code;
    assert!(Registry::global().get_by_code(code).is_some());
}

#[test]
fn the_functions_that_need_the_window_are_not_run_by_the_bridge() {
    // The window runs them (`ume-ui`, `macros/ui_calls.rs`); here the answer is the one for a
    // function that is not supported yet.
    let mut f = Fixture::new("a");
    for (name, params) in [
        ("TraceOut", json!(["x"])),
        ("StatusMsg", json!(["x"])),
        ("PutFile", json!(["x"])),
        ("InsFile", json!(["x"])),
        ("SetViewTop", json!([1])),
        ("WinMaximize", json!([])),
    ] {
        assert_eq!(
            f.call(name, params).unwrap_err().code,
            NOT_IMPLEMENTED,
            "{name}"
        );
    }
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

/// Runs a macro on the bridge. `None` when the interpreter is missing.
fn run_macro(language: Language, code: &str, f: &mut Fixture) -> Option<crate::session::Report> {
    if !available(language) {
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let name = match language {
        Language::JavaScript => "macro.cjs",
        Language::Python => "macro.py",
        Language::Ruby => "macro.rb",
        Language::Php => "macro.php",
        Language::Bundled(_) => unreachable!("the bundled engines are tested in ume-script"),
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
            Language::Bundled(_) => unreachable!(),
        }
    }
}

#[test]
fn the_sample_macros_run_on_the_commands() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
    for (language, name) in [
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

#[test]
fn a_macro_moves_selects_and_edits_with_commands_in_each_language() {
    // Commands that are not in the basic API table (WordRight_Sel, Delete, GoLineEnd,
    // DuplicateLine, Char) next to functions that are (GoFileTop, Down, GetLineStr).
    let js = r#"
Editor.GoFileTop();
Editor.WordRight_Sel();
Editor.Delete();
Editor.GoLineEnd(0);
Editor.Char(33);
Editor.Down();
Editor.DuplicateLine();
console.log(JSON.stringify(Editor.GetLineStr(1)), Editor.GetLineCount(0));
"#;
    let py = r#"
Editor.GoFileTop()
Editor.WordRight_Sel()
Editor.Delete()
Editor.GoLineEnd(0)
Editor.Char(33)
Editor.Down()
Editor.DuplicateLine()
print(repr(Editor.GetLineStr(1)), Editor.GetLineCount(0))
"#;
    let rb = r##"
Editor.GoFileTop
Editor.WordRight_Sel
Editor.Delete
Editor.GoLineEnd(0)
Editor.Char(33)
Editor.Down
Editor.DuplicateLine
puts "#{Editor.GetLineStr(1).inspect} #{Editor.GetLineCount(0)}"
"##;
    for (language, code, shown) in [
        (Language::JavaScript, js, "\"two!\\n\" 3"),
        (Language::Python, py, "'two!\\n' 3"),
        (Language::Ruby, rb, "\"two!\\n\" 3"),
    ] {
        let mut f = Fixture::new("one two\nthree\n");
        let Some(report) = run_macro(language, code, &mut f) else {
            continue;
        };
        let stderr = report.lines(Stream::Stderr).join("\n");
        assert_eq!(report.finished.exit_code, Some(0), "{language:?}: {stderr}");
        assert_eq!(f.text(), "two!\nthree\nthree\n", "{language:?}");
        assert_eq!(report.lines(Stream::Stdout), [shown], "{language:?}");
    }
}

/// Runs a macro on the bridge with an environment that lives for the whole run.
fn run_macro_with_env(
    language: Language,
    code: &str,
    f: &mut Fixture,
    env: &mut HostEnv,
) -> Option<crate::session::Report> {
    if !available(language) {
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let name = match language {
        Language::JavaScript => "macro.cjs",
        Language::Python => "macro.py",
        Language::Ruby => "macro.rb",
        Language::Php => "macro.php",
        Language::Bundled(_) => unreachable!("the bundled engines are tested in ume-script"),
    };
    let script = dir.path().join(name);
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
    Some(session::run_blocking(config, |m| handle_message_with(registry, &mut cx, env, m)).unwrap())
}

#[test]
fn functions_with_a_result_run_in_each_language() {
    // A cookie round trip (the environment), the width of a string and the column of an index
    // (the layout), a version comparison, and a format string (the document and the caret).
    let js = r#"
Editor.SetCookie('window', 'k', '値');
console.log([Editor.GetCookie('window', 'k'), Editor.GetStrWidth('日本\ta', 1),
  Editor.CompareVersion('1.2.3', '1.2.4'), Editor.LineIndexToColumn(1, 3),
  Editor.ExpandParameter('$y,$x|$$')].join('/'));
try { Editor.GetViewLines(); } catch (e) { console.log('no view', e.code); }
"#;
    let py = r#"
Editor.SetCookie('window', 'k', '値')
print('/'.join(str(v) for v in [Editor.GetCookie('window', 'k'), Editor.GetStrWidth('日本\ta', 1),
  Editor.CompareVersion('1.2.3', '1.2.4'), Editor.LineIndexToColumn(1, 3),
  Editor.ExpandParameter('$y,$x|$$')]))
try:
    Editor.GetViewLines()
except EditorError as e:
    print('no view', e.code)
"#;
    let rb = r##"
Editor.SetCookie('window', 'k', '値')
puts [Editor.GetCookie('window', 'k'), Editor.GetStrWidth("日本\ta", 1),
  Editor.CompareVersion('1.2.3', '1.2.4'), Editor.LineIndexToColumn(1, 3),
  Editor.ExpandParameter('$y,$x|$$')].join('/')
begin; Editor.GetViewLines; rescue EditorError => e; puts "no view #{e.code}"; end
"##;
    let php = r#"
Editor::SetCookie('window', 'k', '値');
echo implode('/', [Editor::GetCookie('window', 'k'), Editor::GetStrWidth("日本\ta", 1),
  Editor::CompareVersion('1.2.3', '1.2.4'), Editor::LineIndexToColumn(1, 3),
  Editor::ExpandParameter('$y,$x|$$')]), "\n";
try { Editor::GetViewLines(); } catch (EditorError $e) { echo 'no view ', $e->getCode(), "\n"; }
"#;
    for (language, code) in [
        (Language::JavaScript, js),
        (Language::Python, py),
        (Language::Ruby, rb),
        (Language::Php, php),
    ] {
        let mut f = Fixture::new("日本語");
        let mut env = HostEnv::detached();
        let Some(report) = run_macro_with_env(language, code, &mut f, &mut env) else {
            continue;
        };
        let stderr = report.lines(Stream::Stderr).join("\n");
        assert_eq!(report.finished.exit_code, Some(0), "{language:?}: {stderr}");
        assert_eq!(
            report.lines(Stream::Stdout),
            ["値/9/-256/5/1,1|$", "no view -32002"],
            "{language:?}"
        );
        // The cookie is in the environment that the window keeps.
        assert_eq!(env.state.cookies.get("window", "k"), "値", "{language:?}");
    }
}

#[test]
fn a_box_selection_is_2_and_its_text_is_the_rows_of_the_box() {
    let mut f = Fixture::new("abcdef\nabcdef\nabcdef");
    assert_eq!(f.call("IsTextSelected", json!([])).unwrap(), json!(0));
    f.cursor.move_to(Pos::new(0, 1), false);
    f.cursor.begin_box(1, false);
    f.cursor.move_box_to(Pos::new(2, 3), 3);
    assert_eq!(f.call("IsTextSelected", json!([])).unwrap(), json!(2));
    assert_eq!(
        f.call("GetSelectedString", json!([0])).unwrap(),
        json!("bc\nbc\nbc\n")
    );
    // A normal selection is 1.
    f.cursor.clear_selection();
    f.cursor.select(Pos::new(0, 0), Pos::new(0, 2));
    assert_eq!(f.call("IsTextSelected", json!([])).unwrap(), json!(1));
}
