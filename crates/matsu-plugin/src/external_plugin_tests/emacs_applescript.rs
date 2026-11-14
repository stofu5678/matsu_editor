//! Emacs Lisp and AppleScript plugins (ADR-0026, PR 10, the last two languages). The objects of the
//! plugin in each language's own spelling: `(plugin-get-option ...)` in Emacs Lisp,
//! `editorCall("Plugin.GetOption", {...})` in AppleScript. Each test skips when the interpreter is
//! missing: Emacs is on the Linux x64 CI runner (`MATSU_REQUIRE_INTERPRETERS` makes a missing one a
//! failure there), `osascript` is part of macOS and runs on the macOS runners only. The handlers
//! end their lines with a line break unless a test is about that.

use super::*;

const EL: Language = Language::EmacsLisp;
const AS: Language = Language::AppleScript;

/// Starting an AppleScript program and compiling the handler take a moment on a loaded runner.
const SLOW: Duration = Duration::from_secs(15);

#[test]
fn emacs_lisp_and_applescript_are_served_as_plugins() {
    assert!(EL.serves_plugins());
    assert!(AS.serves_plugins());
    assert_eq!(AS.runs_here(), cfg!(target_os = "macos"));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn an_applescript_plugin_does_not_start_off_macos_and_says_why() {
    use crate::plugin_host::BeginError;
    use crate::session::StartError;

    let dir = plugin_dir(&[("h.applescript", "return\n")]);
    let mut host = host(dir.path(), AS, false);
    let error = host
        .begin("Command", "h.applescript", &json!({}), PATIENT)
        .expect_err("it cannot start here");
    let BeginError::Start(StartError::NotSupportedHere { language }) = &error else {
        panic!("{error:?}");
    };
    assert_eq!(*language, AS);
    assert!(error.to_string().contains("macOS"), "{error}");
}

#[test]
fn an_emacs_plugin_runs_with_the_plugin_variables_and_keeps_its_state() {
    if !available(EL) {
        return;
    }
    // `hello-n` is a global variable of the resident process: it stays for the next call.
    let code = r#"
(defvar hello-n 0)
(setq hello-n (1+ hello-n))
(princ (format "%s %s %s %d\n" plugin-jack (alist-get 'n plugin-args) plugin-id hello-n))
(princ (format "%s %s\n" (plugin-get-plugin-dir) (plugin-get-def "Plugin" "Name")))
(plugin-set-option "Main" "Level" "7")
(princ (format "%s %s\n" (plugin-get-option "Main" "Level") (plugin-get-string 2)))
(editor-ins-text "名前")
"#;
    let dir = plugin_dir(&[("h.el", code)]);
    let mut host = host(dir.path(), EL, true);
    let mut world = World::new("");
    let first = world.call(&mut host, "h.el");
    assert_eq!(first.outcome, Outcome::Ok, "{:?}", first.output);
    assert_eq!(
        stdout(&first),
        [
            "Command 1 jp.example.test 1",
            "/plugins/テスト def:Plugin:Name",
            "7 string 2",
        ]
    );
    let pid = host.process_id().expect("it stays");
    let second = world.call(&mut host, "h.el");
    assert_eq!(host.process_id(), Some(pid));
    assert!(stdout(&second)[0].ends_with(" 2"), "{:?}", second.output);
    assert_eq!(world.doc.text(), "名前名前");
    host.shutdown();
}

#[test]
fn an_emacs_handler_keeps_its_let_variables_to_itself_and_its_global_ones_stay() {
    if !available(EL) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "set.el",
            "(let ((hello-local 5))\n  (setq hello-kept (+ hello-local 1)))\n(editor-ins-text \"s\")\n",
        ),
        (
            "get.el",
            "(princ (format \"%s %s\\n\" (boundp 'hello-local) hello-kept))\n",
        ),
    ]);
    let mut host = host(dir.path(), EL, true);
    let mut world = World::new("");
    assert_eq!(world.call(&mut host, "set.el").outcome, Outcome::Ok);
    let report = world.call(&mut host, "get.el");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(stdout(&report), ["nil 6"], "{:?}", report.output);
    host.shutdown();
}

#[test]
fn an_emacs_handler_cannot_see_or_overwrite_the_locals_of_the_host() {
    if !available(EL) {
        return;
    }
    // The names are those of the locals of the host's functions (`plugin--serve`, `plugin--invoke`,
    // `plugin--run-handler`, `editor--eval-forms`, `plugin--respond`). A handler that sets them
    // sets global variables, and the host goes on.
    let clobber = "(setq line 1 text 2 message 3 method 4 params 5 name 6 source 7 failure 8 id 9
      identity 10 original 11 found 12 reader 13 work 14 rest 15 answer 16 info 17 spec 18
      default 19 key 20 value 21 args 22 form 23)
(princ (format \"%s\\n\" (list (boundp 'plugin-jack) (stringp plugin-jack))))
(editor-ins-text \"c\")
";
    let dir = plugin_dir(&[
        ("clobber.el", clobber),
        ("ok.el", "(editor-ins-text \"o\")\n"),
    ]);
    let mut host = host(dir.path(), EL, true);
    let mut world = World::new("");
    for name in ["clobber.el", "ok.el", "clobber.el"] {
        let report = world.call(&mut host, name);
        assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
    }
    assert_eq!(world.doc.text(), "coc");
    host.shutdown();
}

#[test]
fn an_emacs_handler_may_load_the_files_of_its_folder_again_and_again() {
    if !available(EL) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "req.el",
            "(load (expand-file-name \"plugin_lib/word.el\" plugin-dir) nil t t)\n(require 'plugin-required)\n(editor-ins-text (concat (hello-word) (hello-required)))\n",
        ),
        (
            "plugin_lib/word.el",
            ";;; word.el  -*- lexical-binding: t; -*-\n(defun hello-word () \"word\")\n",
        ),
        (
            "plugin-required.el",
            ";;; plugin-required.el  -*- lexical-binding: t; -*-\n(defun hello-required () \"+req\")\n(provide 'plugin-required)\n",
        ),
    ]);
    let mut host = host(dir.path(), EL, true);
    let mut world = World::new("");
    for _ in 0..2 {
        let report = world.call(&mut host, "req.el");
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    }
    assert_eq!(world.doc.text(), "word+reqword+req");
    host.shutdown();
}

#[test]
fn an_emacs_handler_that_fails_keeps_the_kind_and_the_place_and_not_its_text() {
    if !available(EL) {
        return;
    }
    let cases = [
        (
            "bad.el",
            "(setq a 1)\n(error \"だめ\")\n",
            ["error", "bad.el:2", "だめ"],
        ),
        (
            "runtime.el",
            "(setq a 1)\n(car 1)\n",
            ["wrong-type-argument", "runtime.el:2", "listp"],
        ),
        (
            "void.el",
            "(setq a 1)\n(princ no-such-variable)\n",
            ["void-variable", "void.el:2", "no-such-variable"],
        ),
        (
            "syntax.el",
            "(setq a 1)\n(setq b (\n",
            ["end-of-file", "syntax.el:2", "End of file"],
        ),
        (
            "throw.el",
            "(setq a 1)\n(throw 'nothing 1)\n",
            ["no-catch", "throw.el:2", "nothing"],
        ),
        (
            "editor.el",
            "(setq a 1)\n(editor-get-line-str \"x\")\n",
            ["EditorError", "editor.el:2", "EditorError"],
        ),
        (
            "quit.el",
            "(setq a 1)\n(keyboard-quit)\n",
            ["quit", "quit.el:2", "Quit"],
        ),
    ];
    for (name, code, needles) in cases {
        let dir = plugin_dir(&[(name, code), ("ok.el", "(editor-ins-text \"ok\")\n")]);
        let mut host = host(dir.path(), EL, true);
        let mut world = World::new("");
        let report = world.call(&mut host, name);
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{name}: {:?} {:?}", report.outcome, report.output);
        };
        for needle in needles {
            assert!(text.contains(needle), "{name}: {needle} not in {text}");
        }
        // The text of the handler is never the failure (`(setq a 1)` is its first line).
        assert!(!text.contains("(setq a 1)"), "{name}: {text}");
        // A failed call ends its process (the host does that for every language); the next call
        // starts a new one and is served.
        assert_eq!(
            world.call(&mut host, "ok.el").outcome,
            Outcome::Ok,
            "{name}"
        );
        assert_eq!(world.doc.text(), "ok", "{name}");
        host.shutdown();
    }
}

#[test]
fn an_emacs_handler_keeps_the_output_it_wrote_before_it_failed() {
    if !available(EL) {
        return;
    }
    let dir = plugin_dir(&[("before.el", "(princ \"before\\n\")\n(error \"oops\")\n")]);
    let mut host = host(dir.path(), EL, true);
    let mut world = World::new("");
    let report = world.call(&mut host, "before.el");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(
        text.contains("oops") && text.contains("before.el:2"),
        "{text}"
    );
    assert!(stdout(&report).contains(&"before"), "{:?}", report.output);
    // The standard error is not checked here: the host reads it on its own thread, and a line
    // written right before the answer can arrive after the call has ended.
    host.shutdown();
}

#[test]
fn emacs_output_without_a_line_break_before_an_editor_call_stays_one_line() {
    if !available(EL) {
        return;
    }
    let dir = plugin_dir(&[(
        "partial.el",
        "(princ \"partial \")\n(editor-ins-text \"x\")\n(princ \"rest\\n\")\n",
    )]);
    let mut host = host(dir.path(), EL, true);
    let mut world = World::new("");
    let report = world.call(&mut host, "partial.el");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(stdout(&report), ["partial rest"], "{:?}", report.output);
    host.shutdown();
}

#[test]
fn an_emacs_exit_ends_only_the_handler() {
    if !available(EL) {
        return;
    }
    let cases = [
        (
            "exit.el",
            "(editor-ins-text \"a\")\n(kill-emacs 3)\n(editor-ins-text \"b\")\n",
            "a",
        ),
        (
            "exit-in-function.el",
            "(defun hello-stop () (kill-emacs 4))\n(editor-ins-text \"a\")\n(hello-stop)\n(editor-ins-text \"b\")\n",
            "a",
        ),
        (
            "exit-in-condition-case.el",
            "(editor-ins-text \"a\")\n(condition-case nil (kill-emacs 2) (error (editor-ins-text \"e\")))\n(editor-ins-text \"b\")\n",
            "a",
        ),
        (
            "exit-unwind.el",
            "(editor-ins-text \"a\")\n(unwind-protect (kill-emacs 1) (editor-ins-text \"u\"))\n(editor-ins-text \"b\")\n",
            "au",
        ),
        (
            "throw.el",
            "(editor-ins-text \"a\")\n(throw 'plugin-exit nil)\n(editor-ins-text \"b\")\n",
            "a",
        ),
    ];
    for (name, code, expected) in cases {
        let dir = plugin_dir(&[(name, code), ("ok.el", "(editor-ins-text \"o\")\n")]);
        let mut host = host(dir.path(), EL, true);
        let mut world = World::new("");
        let report = world.call(&mut host, name);
        assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
        let pid = host.process_id().expect("it stays");
        assert_eq!(
            world.call(&mut host, "ok.el").outcome,
            Outcome::Ok,
            "{name}"
        );
        assert_eq!(host.process_id(), Some(pid), "{name}: the host did not end");
        assert_eq!(world.doc.text(), format!("{expected}o"), "{name}");
        host.shutdown();
    }
}

#[test]
fn an_emacs_handler_that_never_ends_is_killed_at_the_limit() {
    if !available(EL) {
        return;
    }
    let dir = plugin_dir(&[("loop.el", "(while t)\n")]);
    let mut host = host(dir.path(), EL, true);
    let mut world = World::new("");
    let limit = Duration::from_secs(8);
    let report = world.call_jack(&mut host, "Command", "loop.el", &Value::Null, limit);
    assert_eq!(report.outcome, Outcome::TimedOut);
    assert!(!host.is_running());
}

#[test]
fn an_emacs_plugin_serves_the_outline_complement_and_smart_indent_objects() {
    if !available(EL) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "outline.el",
            "(plugin-outline-set-title \"見出し\")\n(plugin-outline-add-func-info2 3 1 \"a\" 0)\n",
        ),
        (
            "complement.el",
            "(let ((w (plugin-complement-get-current-word)))\n  (princ (format \"%s\\n\" (plugin-complement-get-option)))\n  (plugin-complement-add-list (concat w \"1\"))\n  (princ (format \"%s\\n\" (plugin-complement-add-list (concat w \"1\")))))\n",
        ),
        (
            "indent.el",
            "(princ (format \"%d\\n\" (aref (plugin-indent-get-char) 0)))\n",
        ),
        (
            "noindent.el",
            "(princ (format \"[%s]\\n\" (plugin-indent-get-char)))\n",
        ),
    ]);
    let mut host = host(dir.path(), EL, true);
    let mut world = World::new("");
    let report = world.call_jack(&mut host, "Outline", "outline.el", &json!({}), PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(
        world.api.outline,
        [
            "Outline.SetTitle[\"見出し\"]",
            "Outline.AddFuncInfo2[3,1,\"a\",0]"
        ]
    );
    let args = json!({"word": "あ", "option": 3});
    let report = world.call_jack(&mut host, "Complement", "complement.el", &args, PATIENT);
    assert_eq!(stdout(&report), ["3", "-1"], "{:?}", report.output);
    assert_eq!(world.api.added, ["あ1"]);
    let args = json!({"char": "\r"});
    let report = world.call_jack(&mut host, "SmartIndent", "indent.el", &args, PATIENT);
    assert_eq!(stdout(&report), ["13"], "{:?}", report.output);
    let report = world.call_jack(&mut host, "Command", "noindent.el", &json!({}), PATIENT);
    assert_eq!(stdout(&report), ["[]"], "{:?}", report.output);
    host.shutdown();
}

#[test]
fn the_emacs_sample_plugin_runs() {
    if !available(EL) {
        return;
    }
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/plugins");
    let text = "# 見出し\n本文\n## 小見出し\n";
    let mut host = host(&samples.join("HelloEmacsLisp"), EL, true);
    let mut world = World::new(text);
    for _ in 0..2 {
        let report = world.call_jack(&mut host, "DocumentOpen", "opened.el", &json!({}), PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    }
    let report = world.call(&mut host, "command.el");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert!(world.doc.text().starts_with("Hello (Command)"));
    let report = world.call_jack(&mut host, "Outline", "outline.el", &json!({}), PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert!(
        world
            .api
            .outline
            .iter()
            .any(|c| c.contains("\"小見出し\",1")),
        "{:?}",
        world.api.outline
    );
    let args = json!({"word": "ab", "option": 0});
    let report = world.call_jack(&mut host, "Complement", "complement.el", &args, PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.api.added, ["ab_hello", "ab_world"]);
    host.shutdown();
}

// ---- AppleScript (macOS only) -------------------------------------------------------------------

#[test]
fn an_applescript_plugin_runs_with_the_plugin_handlers() {
    if !available(AS) {
        return;
    }
    let code = "
editorPrint(pluginJack() & \" \" & pluginArg(\"n\") & \" \" & pluginId())
editorPrint(editorCall(\"Plugin.GetPluginDir\", {}) & \" \" & editorCall(\"Plugin.GetDef\", {\"Plugin\", \"Name\"}))
editorCall(\"Plugin.SetOption\", {\"Main\", \"Level\", \"7\"})
editorPrint(editorCall(\"Plugin.GetOption\", {\"Main\", \"Level\"}) & \" \" & editorCall(\"Plugin.GetString\", {2}))
editorCall(\"InsText\", {\"名前\"})
";
    let dir = plugin_dir(&[("h.applescript", code)]);
    let mut host = host(dir.path(), AS, true);
    let mut world = World::new("");
    let first = world.call_jack(
        &mut host,
        "Command",
        "h.applescript",
        &json!({"n": 1}),
        SLOW,
    );
    assert_eq!(first.outcome, Outcome::Ok, "{:?}", first.output);
    assert_eq!(
        stdout(&first),
        [
            "Command 1 jp.example.test",
            "/plugins/テスト def:Plugin:Name",
            "7 string 2",
        ]
    );
    let pid = host.process_id().expect("it stays");
    let second = world.call_jack(
        &mut host,
        "Command",
        "h.applescript",
        &json!({"n": 1}),
        SLOW,
    );
    assert_eq!(second.outcome, Outcome::Ok, "{:?}", second.output);
    assert_eq!(host.process_id(), Some(pid));
    assert_eq!(world.doc.text(), "名前名前");
    host.shutdown();
}

#[test]
fn an_applescript_handler_keeps_its_properties_while_the_process_stays() {
    if !available(AS) {
        return;
    }
    // The compiled handler is run again for each call, so `keptValue` goes on from where it was;
    // the host's own names (set in the handler as its own variables) are never touched.
    let code = "property keptValue : 5
set keptValue to keptValue + 1
set theLine to \"x\"
set theMessage to \"y\"
set handlerName to \"z\"
editorPrint(keptValue as text)
editorCall(\"InsText\", {\"c\"})
";
    let dir = plugin_dir(&[("first.applescript", code)]);
    let mut host = host(dir.path(), AS, true);
    let mut world = World::new("");
    let mut seen = Vec::new();
    for _ in 0..3 {
        let report = world.call_jack(&mut host, "Command", "first.applescript", &json!({}), SLOW);
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        seen.extend(stdout(&report).iter().map(|l| (*l).to_owned()));
    }
    assert_eq!(seen, ["6", "7", "8"], "properties must stay between calls");
    assert_eq!(world.doc.text(), "ccc");
    host.shutdown();
}

#[test]
fn an_applescript_handler_with_a_new_text_is_compiled_again() {
    if !available(AS) {
        return;
    }
    let dir = plugin_dir(&[(
        "h.applescript",
        "property n : 0\nset n to n + 1\neditorPrint(\"v1 \" & n)\n",
    )]);
    let mut host = host(dir.path(), AS, true);
    let mut world = World::new("");
    let first = world.call_jack(&mut host, "Command", "h.applescript", &json!({}), SLOW);
    assert_eq!(
        stdout(&first),
        ["v1 1"],
        "{:?} {:?}",
        first.outcome,
        first.output
    );
    // The file changes; the kept program is for the old text, so the handler starts afresh.
    std::fs::write(
        dir.path().join("h.applescript"),
        "property n : 0\nset n to n + 10\neditorPrint(\"v2 \" & n)\n",
    )
    .unwrap();
    let second = world.call_jack(&mut host, "Command", "h.applescript", &json!({}), SLOW);
    assert_eq!(
        stdout(&second),
        ["v2 10"],
        "{:?} {:?}",
        second.outcome,
        second.output
    );
    // A change of case only is a change too: the handler is compiled again (n starts from 0).
    std::fs::write(
        dir.path().join("h.applescript"),
        "property n : 0\nset n to n + 10\neditorPrint(\"V2 \" & n)\n",
    )
    .unwrap();
    let third = world.call_jack(&mut host, "Command", "h.applescript", &json!({}), SLOW);
    assert_eq!(
        stdout(&third),
        ["V2 10"],
        "a handler that only changed case must run its new text: {:?} {:?}",
        third.outcome,
        third.output
    );
    host.shutdown();
}

#[test]
fn an_applescript_handler_may_run_a_file_of_its_folder() {
    if !available(AS) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "req.applescript",
            "set theWord to run script (read (POSIX file (pluginDir() & \"/plugin_lib/word.applescript\")) as \u{ab}class utf8\u{bb})\neditorCall(\"InsText\", {theWord})\n",
        ),
        ("plugin_lib/word.applescript", "return \"word\"\n"),
    ]);
    let mut host = host(dir.path(), AS, true);
    let mut world = World::new("");
    // The plugin folder in the context is the one the editor gave (the handler's own folder).
    let report = world.call_jack(&mut host, "Command", "req.applescript", &json!({}), SLOW);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.doc.text(), "word");
    host.shutdown();
}

#[test]
fn an_applescript_handler_that_fails_keeps_the_kind_and_the_place_and_not_its_text() {
    if !available(AS) {
        return;
    }
    let cases = [
        (
            "bad.applescript",
            "set a to 1\nerror \"だめ\"\n",
            vec!["execution error", "bad.applescript:2", "だめ"],
        ),
        (
            "runtime.applescript",
            "set a to 1\nset b to 1 / 0\n",
            vec!["execution error", "runtime.applescript:2", "zero"],
        ),
        (
            "undefined.applescript",
            "set a to 1\nset b to undefinedThing\n",
            vec![
                "execution error",
                "undefined.applescript:2",
                "undefinedThing",
            ],
        ),
        (
            "syntax.applescript",
            "set a to 1\nset b to to 2\n",
            vec!["script error", "syntax.applescript:2"],
        ),
        (
            "editor.applescript",
            "set a to 1\neditorCall(\"GetLineStr\", {\"x\"})\n",
            // The error is raised inside the boot's `editorCall`, and AppleScript has no call stack:
            // the line of the call cannot be known, and the failure says it came from an editor call.
            vec!["EditorError", "editor.applescript (in an editor call)"],
        ),
    ];
    for (name, code, needles) in cases {
        let dir = plugin_dir(&[
            (name, code),
            ("ok.applescript", "editorCall(\"InsText\", {\"ok\"})\n"),
        ]);
        let mut host = host(dir.path(), AS, true);
        let mut world = World::new("");
        let report = world.call_jack(&mut host, "Command", name, &json!({}), SLOW);
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{name}: {:?} {:?}", report.outcome, report.output);
        };
        for needle in needles {
            assert!(text.contains(needle), "{name}: {needle} not in {text}");
        }
        // The text of the handler is never the failure (`set a to 1` is its first line).
        assert!(!text.contains("set a to 1"), "{name}: {text}");
        let report = world.call_jack(&mut host, "Command", "ok.applescript", &json!({}), SLOW);
        assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
        assert_eq!(world.doc.text(), "ok", "{name}");
        host.shutdown();
    }
}

#[test]
fn an_applescript_handler_keeps_the_output_it_wrote_before_it_failed() {
    if !available(AS) {
        return;
    }
    let dir = plugin_dir(&[(
        "before.applescript",
        "editorPrint(\"before\")\nerror \"oops\"\n",
    )]);
    let mut host = host(dir.path(), AS, true);
    let mut world = World::new("");
    let report = world.call_jack(&mut host, "Command", "before.applescript", &json!({}), SLOW);
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(
        text.contains("oops") && text.contains("before.applescript:2"),
        "{text}"
    );
    assert!(stdout(&report).contains(&"before"), "{:?}", report.output);
    host.shutdown();
}

#[test]
fn applescript_output_without_a_line_break_before_an_editor_call_stays_one_line() {
    if !available(AS) {
        return;
    }
    let dir = plugin_dir(&[(
        "partial.applescript",
        "editorWrite(\"partial \")\neditorCall(\"InsText\", {\"x\"})\neditorPrint(\"rest\")\n",
    )]);
    let mut host = host(dir.path(), AS, true);
    let mut world = World::new("");
    let report = world.call_jack(
        &mut host,
        "Command",
        "partial.applescript",
        &json!({}),
        SLOW,
    );
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(stdout(&report), ["partial rest"], "{:?}", report.output);
    host.shutdown();
}

#[test]
fn an_applescript_return_or_a_quiet_error_ends_only_the_handler() {
    if !available(AS) {
        return;
    }
    let cases = [
        (
            "early.applescript",
            "editorCall(\"InsText\", {\"a\"})\nreturn\neditorCall(\"InsText\", {\"b\"})\n",
        ),
        (
            "quiet.applescript",
            "editorCall(\"InsText\", {\"a\"})\nerror number -128\neditorCall(\"InsText\", {\"b\"})\n",
        ),
        (
            "in-handler.applescript",
            "on stopHere()\n\treturn\nend stopHere\neditorCall(\"InsText\", {\"a\"})\nmy stopHere()\nreturn\neditorCall(\"InsText\", {\"b\"})\n",
        ),
    ];
    for (name, code) in cases {
        let dir = plugin_dir(&[
            (name, code),
            ("ok.applescript", "editorCall(\"InsText\", {\"o\"})\n"),
        ]);
        let mut host = host(dir.path(), AS, true);
        let mut world = World::new("");
        let report = world.call_jack(&mut host, "Command", name, &json!({}), SLOW);
        assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
        let pid = host.process_id().expect("it stays");
        let report = world.call_jack(&mut host, "Command", "ok.applescript", &json!({}), SLOW);
        assert_eq!(report.outcome, Outcome::Ok, "{name}");
        assert_eq!(host.process_id(), Some(pid), "{name}: the host did not end");
        assert_eq!(world.doc.text(), "ao", "{name}");
        host.shutdown();
    }
}

#[test]
fn an_applescript_handler_that_never_ends_is_killed_at_the_limit() {
    if !available(AS) {
        return;
    }
    let dir = plugin_dir(&[("loop.applescript", "repeat\nend repeat\n")]);
    let mut host = host(dir.path(), AS, true);
    let mut world = World::new("");
    let report = world.call_jack(
        &mut host,
        "Command",
        "loop.applescript",
        &Value::Null,
        Duration::from_secs(15),
    );
    assert_eq!(report.outcome, Outcome::TimedOut);
    assert!(!host.is_running());
}

#[test]
fn an_applescript_plugin_serves_the_outline_complement_and_smart_indent_handlers() {
    if !available(AS) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "outline.applescript",
            "editorCall(\"Outline.SetTitle\", {\"見出し\"})\neditorCall(\"Outline.AddFuncInfo2\", {3, 1, \"a\", 0})\n",
        ),
        (
            "complement.applescript",
            "set theWord to complementGetCurrentWord()\neditorPrint(complementGetOption() as text)\neditorCall(\"Complement.AddList\", {theWord & \"1\"})\neditorPrint(editorCall(\"Complement.AddList\", {theWord & \"1\"}) as text)\n",
        ),
        (
            "indent.applescript",
            "set typedChar to indentGetChar()\nif typedChar is \"\" then\n\teditorPrint(\"empty\")\nelse\n\teditorPrint((ASCII number typedChar) as text)\nend if\n",
        ),
        (
            "noindent.applescript",
            "editorPrint(\"[\" & indentGetChar() & \"]\")\n",
        ),
    ]);
    let mut host = host(dir.path(), AS, true);
    let mut world = World::new("");
    let report = world.call_jack(
        &mut host,
        "Outline",
        "outline.applescript",
        &json!({}),
        SLOW,
    );
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(
        world.api.outline,
        [
            "Outline.SetTitle[\"見出し\"]",
            "Outline.AddFuncInfo2[3,1,\"a\",0]"
        ]
    );
    let args = json!({"word": "あ", "option": 3});
    let report = world.call_jack(
        &mut host,
        "Complement",
        "complement.applescript",
        &args,
        SLOW,
    );
    assert_eq!(stdout(&report), ["3", "-1"], "{:?}", report.output);
    assert_eq!(world.api.added, ["あ1"]);
    let args = json!({"char": "\r"});
    let report = world.call_jack(&mut host, "SmartIndent", "indent.applescript", &args, SLOW);
    assert_eq!(
        stdout(&report),
        ["13"],
        "the typed char (CR) as indentGetChar() gave it: outcome {:?}, output {:?}",
        report.outcome,
        report.output
    );
    let report = world.call_jack(
        &mut host,
        "Command",
        "noindent.applescript",
        &json!({}),
        SLOW,
    );
    assert_eq!(stdout(&report), ["[]"], "{:?}", report.output);
    host.shutdown();
}

#[test]
fn the_applescript_sample_plugin_runs() {
    if !available(AS) {
        return;
    }
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/plugins");
    let text = "# 見出し\n本文\n## 小見出し\n";
    let mut host = host(&samples.join("HelloAppleScript"), AS, true);
    let mut world = World::new(text);
    let report = world.call_jack(
        &mut host,
        "DocumentOpen",
        "opened.applescript",
        &json!({}),
        SLOW,
    );
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    let report = world.call_jack(
        &mut host,
        "Command",
        "command.applescript",
        &json!({}),
        SLOW,
    );
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert!(world.doc.text().starts_with("Hello (Command)"));
    let report = world.call_jack(
        &mut host,
        "Outline",
        "outline.applescript",
        &json!({}),
        SLOW,
    );
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert!(
        world
            .api
            .outline
            .iter()
            .any(|c| c.contains("\"小見出し\",1")),
        "{:?}",
        world.api.outline
    );
    let args = json!({"word": "ab", "option": 0});
    let report = world.call_jack(
        &mut host,
        "Complement",
        "complement.applescript",
        &args,
        SLOW,
    );
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.api.added, ["ab_hello", "ab_world"]);
    host.shutdown();
}
