//! End-to-end tests of the plugin host with the bundled Lua, MicroPython and mruby (ADR-0026): the
//! built `matsu-script` program serves a plugin under the editor's own host code. The same
//! scenarios run for each engine, with a handler written in its language (QuickJS has its own file,
//! `plugin_host.rs`).

mod plugin_support;

use std::time::{Duration, Instant};

use matsu_plugin::plugin_host::{MAX_FAILURES, Outcome};
use matsu_plugin::session::{Engine, Stream};
use plugin_support::{PATIENT, World, host_of, plugin_dir, stdout};
use serde_json::json;

/// The handlers of one engine, one for each scenario.
struct Lang {
    engine: Engine,
    /// The extension of its handler files.
    ext: &'static str,
    /// Counts in a state that the file keeps: prints `n=1`, `n=2`...
    count: &'static str,
    /// Prints `other=none` when it does not see the state of `count`.
    other: &'static str,
    /// Inserts a text and prints the jack, `Plugin.args.n` and `Plugin.id`.
    command: &'static str,
    /// Uses every call of the `Plugin` object and prints the answers.
    plugin_object: &'static str,
    /// Uses every call of the `Outline` object.
    outline: &'static str,
    /// Uses the `Complement` object for the word `fo`, option 3.
    complement: &'static str,
    /// Writes the line `e<n>` (`n` of `Plugin.args`) to the standard error.
    stderr: &'static str,
    /// Fails on its second line with the text `だめ`; the error shows `place`.
    failing: &'static str,
    place: &'static str,
    /// Prints `enter` when `Indent.GetChar()` is the Enter key.
    indent: &'static str,
    /// Inserts `a`, leaves the handler early and would insert `b`.
    early_exit: &'static str,
    /// Never ends.
    spin: &'static str,
}

/// Lua and MicroPython can share a handler where the syntax is the same.
const LUA: Lang = Lang {
    engine: Engine::Lua,
    ext: "lua",
    count: "n = (n or 0) + 1\nprint('n=' .. n)",
    other: "print(n == nil and 'other=none' or 'other=seen')",
    command: "Editor.InsText('こんにちは')\nprint(Plugin.jack .. ' ' .. Plugin.args.n .. ' ' .. Plugin.id)",
    plugin_object: "print(Plugin.GetPluginDir())\nprint(Plugin.GetDef('Plugin', 'Name'))\nPlugin.SetOption('Main', 'Level', '7')\nprint(Plugin.GetOption('Main', 'Level'))\nprint(Plugin.GetString(2))",
    outline: OUTLINE,
    complement: "local word = Complement.GetCurrentWord()
print(word .. ':' .. Complement.GetOption())
print(Complement.AddList(word .. '1') .. ' ' .. Complement.AddList(word .. '1') .. ' ' .. Complement.AddList(2))
local ok, e = pcall(Complement.AddList, '')
print(tostring(ok) .. ' ' .. e.name)
ok, e = pcall(Complement.AddList)
print(tostring(ok) .. ' ' .. e.name)",
    stderr: "io.stderr:write('e' .. Plugin.args.n .. '\\n')",
    failing: "local a = 1\nerror('だめ')",
    place: "bad.lua:2",
    indent: "print(Indent.GetChar() == '\\r' and 'enter' or 'other')",
    early_exit: "Editor.InsText('a')\ndo return end\nEditor.InsText('b')",
    spin: "while true do end",
};

const MICROPYTHON: Lang = Lang {
    engine: Engine::MicroPython,
    ext: "upy",
    count: "n = globals().get('n', 0) + 1\nprint('n=%d' % n)",
    other: "print('other=none' if 'n' not in globals() else 'other=seen')",
    command: "Editor.InsText('こんにちは')\nprint(Plugin.jack, Plugin.args['n'], Plugin.id)",
    plugin_object: "print(Plugin.GetPluginDir())\nprint(Plugin.GetDef('Plugin', 'Name'))\nPlugin.SetOption('Main', 'Level', '7')\nprint(Plugin.GetOption('Main', 'Level'))\nprint(Plugin.GetString(2))",
    outline: OUTLINE,
    complement: "word = Complement.GetCurrentWord()
print(word + ':' + str(Complement.GetOption()))
print(Complement.AddList(word + '1'), Complement.AddList(word + '1'), Complement.AddList(2))
for call in (lambda: Complement.AddList(''), lambda: Complement.AddList()):
    try:
        call()
    except EditorError:
        print('false EditorError')",
    stderr: "import sys\nprint('e%d' % Plugin.args['n'], file=sys.stderr)",
    failing: "a = 1\nraise ValueError('だめ')",
    place: "bad.upy\", line 2",
    indent: "print('enter' if Indent.GetChar() == '\\r' else 'other')",
    early_exit: "import sys\nEditor.InsText('a')\nsys.exit()\nEditor.InsText('b')",
    spin: "while True:\n    pass",
};

const MRUBY: Lang = Lang {
    engine: Engine::Mruby,
    ext: "mruby",
    count: "@n = (@n || 0) + 1\nputs \"n=#{@n}\"",
    other: "puts(@n.nil? ? 'other=none' : 'other=seen')",
    command: "Editor.InsText('こんにちは')\nputs \"#{Plugin.jack} #{Plugin.args['n']} #{Plugin.id}\"",
    plugin_object: "puts Plugin.GetPluginDir\nputs Plugin.GetDef('Plugin', 'Name')\nPlugin.SetOption('Main', 'Level', '7')\nputs Plugin.GetOption('Main', 'Level')\nputs Plugin.GetString(2)",
    outline: OUTLINE,
    complement: "word = Complement.GetCurrentWord
puts \"#{word}:#{Complement.GetOption}\"
puts [Complement.AddList(word + '1'), Complement.AddList(word + '1'), Complement.AddList(2)].join(' ')
[proc { Complement.AddList('') }, proc { Complement.AddList }].each do |call|
  begin
    call.call
  rescue EditorError
    puts 'false EditorError'
  end
end",
    stderr: "$stderr.puts \"e#{Plugin.args['n']}\"",
    failing: "a = 1\nraise 'だめ'",
    place: "bad.mruby:2",
    indent: "puts(Indent.GetChar == \"\\r\" ? 'enter' : 'other')",
    early_exit: "Editor.InsText('a')\nreturn\nEditor.InsText('b')",
    spin: "while true; end",
};

/// The same text in the three languages.
const OUTLINE: &str = "Outline.SetListType(100)
Outline.SetTitle('題')
Outline.AddFuncInfo(1, 2, 'a', 3)
Outline.AddFuncInfo2(2, 1, 'b', 1)
Outline.AddFuncInfo3(3, 1, 'c', 0, 'f.txt')
Outline.AddFuncInfo4(4, 1, 'd', 2, 'g.txt')
Outline.SetLabel(10, ' モジュール')";

fn file(lang: &Lang, stem: &str) -> String {
    format!("{stem}.{}", lang.ext)
}

fn state_is_kept_between_calls_and_not_shared_between_files(lang: &Lang) {
    let (a, b) = (file(lang, "a"), file(lang, "b"));
    let dir = plugin_dir(&[(&a, lang.count), (&b, lang.other)]);
    let mut host = host_of(lang.engine, dir.path(), true);
    let mut world = World::new("");
    let first = world.call(&mut host, &a, PATIENT);
    assert_eq!(first.outcome, Outcome::Ok, "{:?}", first.output);
    let pid = host.process_id().expect("the process stays");
    let other = world.call(&mut host, &b, PATIENT);
    let second = world.call(&mut host, &a, PATIENT);
    assert_eq!(host.process_id(), Some(pid), "the same process");
    assert_eq!(stdout(&first), ["n=1"]);
    assert_eq!(stdout(&other), ["other=none"], "{:?}", other.output);
    assert_eq!(stdout(&second), ["n=2"], "{:?}", second.output);
    host.shutdown();
    // A new process starts the next time, from nothing.
    let third = world.call(&mut host, &a, PATIENT);
    assert_eq!(stdout(&third), ["n=1"]);
    assert_ne!(host.process_id(), Some(pid));
}

fn a_call_that_is_not_resident_starts_over_each_time(lang: &Lang) {
    let a = file(lang, "a");
    let dir = plugin_dir(&[(&a, lang.count)]);
    let mut host = host_of(lang.engine, dir.path(), false);
    let mut world = World::new("");
    for _ in 0..2 {
        let report = world.call(&mut host, &a, PATIENT);
        assert_eq!(stdout(&report), ["n=1"], "{:?}", report.output);
        assert!(!host.is_running());
    }
}

fn a_command_handler_edits_the_document_and_sees_the_jack(lang: &Lang) {
    let name = file(lang, "command");
    let dir = plugin_dir(&[(&name, lang.command)]);
    let mut host = host_of(lang.engine, dir.path(), false);
    assert!(
        !host.is_running(),
        "lazy: nothing runs before the first call"
    );
    let mut world = World::new("");
    let report = world.call(&mut host, &name, PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.doc.text(), "こんにちは");
    assert_eq!(stdout(&report), ["Command 1 jp.example.test"]);
}

fn the_plugin_object_asks_the_editor(lang: &Lang) {
    let name = file(lang, "object");
    let dir = plugin_dir(&[(&name, lang.plugin_object)]);
    let mut host = host_of(lang.engine, dir.path(), false);
    let mut world = World::new("");
    let report = world.call(&mut host, &name, PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(
        stdout(&report),
        ["/plugins/テスト", "def:Plugin:Name", "7", "string 2"]
    );
    assert_eq!(
        world.api.calls,
        [
            "Plugin.GetPluginDir",
            "Plugin.GetDef",
            "Plugin.SetOption",
            "Plugin.GetOption",
            "Plugin.GetString"
        ]
    );
}

fn the_outline_object_sends_sakuras_calls_with_their_arguments(lang: &Lang) {
    let name = file(lang, "outline");
    let dir = plugin_dir(&[(&name, lang.outline)]);
    let mut host = host_of(lang.engine, dir.path(), false);
    let mut world = World::new("");
    let report = world.call_jack(&mut host, "Outline", &name, &json!({}), PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(
        world.api.calls,
        [
            "Outline.SetListType",
            "Outline.SetTitle",
            "Outline.AddFuncInfo",
            "Outline.AddFuncInfo2",
            "Outline.AddFuncInfo3",
            "Outline.AddFuncInfo4",
            "Outline.SetLabel",
        ]
    );
}

fn the_complement_object_gives_the_word_and_the_option_and_adds_candidates(lang: &Lang) {
    let name = file(lang, "complement");
    let dir = plugin_dir(&[(&name, lang.complement)]);
    let mut host = host_of(lang.engine, dir.path(), true);
    let mut world = World::new("");
    let args = json!({"word": "fo", "option": 3});
    let report = world.call_jack(&mut host, "Complement", &name, &args, PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(
        stdout(&report),
        ["fo:3", "1 -1 2", "false EditorError", "false EditorError"]
    );
    assert_eq!(world.api.added, ["fo1", "2"]);
    // Without the arguments of the jack the word is empty and the option 0.
    let report = world.call_jack(&mut host, "Complement", &name, &json!({}), PATIENT);
    assert_eq!(stdout(&report)[0], ":0", "{:?}", report.output);
}

fn the_indent_object_answers_the_typed_char(lang: &Lang) {
    let name = file(lang, "indent");
    let dir = plugin_dir(&[(&name, lang.indent)]);
    let mut host = host_of(lang.engine, dir.path(), true);
    let mut world = World::new("");
    let args = json!({"char": "\r"});
    let report = world.call_jack(&mut host, "SmartIndent", &name, &args, PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(stdout(&report), ["enter"]);
    let args = json!({"char": "}"});
    let report = world.call_jack(&mut host, "SmartIndent", &name, &args, PATIENT);
    assert_eq!(stdout(&report), ["other"], "{:?}", report.output);
}

fn a_stderr_line_comes_before_the_end_of_its_call(lang: &Lang) {
    const ROUNDS: usize = 100;
    let name = file(lang, "stderr");
    let dir = plugin_dir(&[(&name, lang.stderr)]);
    let mut host = host_of(lang.engine, dir.path(), true);
    let mut world = World::new("");
    for round in 0..ROUNDS {
        let report = world.call_jack(&mut host, "Command", &name, &json!({"n": round}), PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        assert_eq!(
            report.lines(Stream::Stderr),
            [format!("e{round}")],
            "round {round}: {:?}",
            report.output
        );
    }
}

fn a_handler_that_fails_says_where_and_the_host_goes_on(lang: &Lang) {
    let (bad, syntax, ok) = (file(lang, "bad"), file(lang, "syntax"), file(lang, "ok"));
    let dir = plugin_dir(&[
        (&bad, lang.failing),
        (&syntax, "x = = 1"),
        (&ok, "Editor.InsText('ok')"),
    ]);
    let mut host = host_of(lang.engine, dir.path(), true);
    let mut world = World::new("");
    let report = world.call(&mut host, &bad, PATIENT);
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains("だめ") && text.contains(lang.place), "{text}");
    let report = world.call(&mut host, &syntax, PATIENT);
    assert!(
        matches!(report.outcome, Outcome::Failed(_)),
        "{:?}",
        report.outcome
    );
    // The host was told of the failures: a success starts the count over, as with QuickJS.
    assert_eq!(host.failures(), 2);
    let next = world.call(&mut host, &ok, PATIENT);
    assert_eq!(next.outcome, Outcome::Ok, "{:?}", next.output);
    assert_eq!(world.doc.text(), "ok");
    assert_eq!(host.failures(), 0);
}

fn three_failures_in_a_row_stop_the_plugin(lang: &Lang) {
    let (bad, ok) = (file(lang, "bad"), file(lang, "ok"));
    let dir = plugin_dir(&[(&bad, lang.failing), (&ok, "Editor.InsText('ok')")]);
    let mut host = host_of(lang.engine, dir.path(), false);
    let mut world = World::new("");
    for _ in 0..MAX_FAILURES {
        assert!(!host.is_stopped());
        let report = world.call(&mut host, &bad, PATIENT);
        assert!(matches!(report.outcome, Outcome::Failed(_)));
    }
    assert!(host.is_stopped());
    host.reset();
    assert_eq!(world.call(&mut host, &ok, PATIENT).outcome, Outcome::Ok);
}

fn a_handler_that_never_ends_is_killed_at_the_limit_and_the_next_call_works(lang: &Lang) {
    let (spin, ok) = (file(lang, "spin"), file(lang, "ok"));
    let dir = plugin_dir(&[(&spin, lang.spin), (&ok, lang.count)]);
    let mut host = host_of(lang.engine, dir.path(), true);
    let mut world = World::new("");
    let started = Instant::now();
    let report = world.call(&mut host, &spin, Duration::from_millis(400));
    assert_eq!(report.outcome, Outcome::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(!host.is_running(), "the process was killed");
    assert_eq!(host.failures(), 1);
    let next = world.call(&mut host, &ok, PATIENT);
    assert_eq!(stdout(&next), ["n=1"], "{:?}", next.output);
    assert_eq!(host.failures(), 0);
}

fn a_handler_with_crlf_lines_and_japanese_names_runs(lang: &Lang) {
    let name = format!("日本語/ハンドラ.{}", lang.ext);
    let dir = plugin_dir(&[(&name, "Editor.InsText('a')\r\nEditor.InsText('あ')\r\n")]);
    let mut host = host_of(lang.engine, dir.path(), false);
    let mut world = World::new("");
    let report = world.call(&mut host, &name, PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.doc.text(), "aあ");
}

fn leaving_the_handler_early_is_not_a_failure(lang: &Lang) {
    let name = file(lang, "early");
    let dir = plugin_dir(&[(&name, lang.early_exit)]);
    let mut host = host_of(lang.engine, dir.path(), true);
    let mut world = World::new("");
    let report = world.call(&mut host, &name, PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.doc.text(), "a");
    // The host is still serving.
    let again = world.call(&mut host, &name, PATIENT);
    assert_eq!(again.outcome, Outcome::Ok, "{:?}", again.output);
    assert_eq!(world.doc.text(), "aa");
}

/// A handler that raises the exit of its language after it counted: it ends the handler only.
fn exiting_the_handler_keeps_the_host_and_its_state(lang: &Lang, exit: &str) {
    let name = file(lang, "exit");
    let code = format!("{}\n{exit}", lang.count);
    let dir = plugin_dir(&[(&name, &code)]);
    let mut host = host_of(lang.engine, dir.path(), true);
    let mut world = World::new("");
    for expected in ["n=1", "n=2"] {
        let report = world.call(&mut host, &name, PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        assert_eq!(stdout(&report), [expected]);
    }
    assert!(host.is_running());
    assert_eq!(host.failures(), 0);
}

fn an_error_of_the_editor_is_an_editor_error_in_the_handler(lang: &Lang) {
    // The same text in the three languages: `Editor.GetLineStr('x')` is refused by the editor.
    let name = file(lang, "editor_error");
    let dir = plugin_dir(&[(&name, "Editor.GetLineStr('not a number')")]);
    let mut host = host_of(lang.engine, dir.path(), false);
    let mut world = World::new("");
    let report = world.call(&mut host, &name, PATIENT);
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains("GetLineStr"), "{text}");
}

macro_rules! per_engine {
    ($module:ident, $lang:expr) => {
        mod $module {
            use super::*;

            #[test]
            fn state_is_kept_between_calls_and_not_shared_between_files() {
                super::state_is_kept_between_calls_and_not_shared_between_files(&$lang);
            }
            #[test]
            fn a_call_that_is_not_resident_starts_over_each_time() {
                super::a_call_that_is_not_resident_starts_over_each_time(&$lang);
            }
            #[test]
            fn a_command_handler_edits_the_document_and_sees_the_jack() {
                super::a_command_handler_edits_the_document_and_sees_the_jack(&$lang);
            }
            #[test]
            fn the_plugin_object_asks_the_editor() {
                super::the_plugin_object_asks_the_editor(&$lang);
            }
            #[test]
            fn the_outline_object_sends_sakuras_calls_with_their_arguments() {
                super::the_outline_object_sends_sakuras_calls_with_their_arguments(&$lang);
            }
            #[test]
            fn the_complement_object_gives_the_word_and_adds_candidates() {
                super::the_complement_object_gives_the_word_and_the_option_and_adds_candidates(
                    &$lang,
                );
            }
            #[test]
            fn the_indent_object_answers_the_typed_char() {
                super::the_indent_object_answers_the_typed_char(&$lang);
            }
            #[test]
            fn a_stderr_line_comes_before_the_end_of_its_call() {
                super::a_stderr_line_comes_before_the_end_of_its_call(&$lang);
            }
            #[test]
            fn a_handler_that_fails_says_where_and_the_host_goes_on() {
                super::a_handler_that_fails_says_where_and_the_host_goes_on(&$lang);
            }
            #[test]
            fn three_failures_in_a_row_stop_the_plugin() {
                super::three_failures_in_a_row_stop_the_plugin(&$lang);
            }
            #[test]
            fn a_handler_that_never_ends_is_killed_at_the_limit() {
                super::a_handler_that_never_ends_is_killed_at_the_limit_and_the_next_call_works(
                    &$lang,
                );
            }
            #[test]
            fn a_handler_with_crlf_lines_and_japanese_names_runs() {
                super::a_handler_with_crlf_lines_and_japanese_names_runs(&$lang);
            }
            #[test]
            fn leaving_the_handler_early_is_not_a_failure() {
                super::leaving_the_handler_early_is_not_a_failure(&$lang);
            }
            #[test]
            fn an_error_of_the_editor_is_an_editor_error_in_the_handler() {
                super::an_error_of_the_editor_is_an_editor_error_in_the_handler(&$lang);
            }
        }
    };
}

per_engine!(lua, LUA);
per_engine!(micropython, MICROPYTHON);
per_engine!(mruby, MRUBY);

#[test]
fn lua_os_exit_ends_the_handler_only_with_any_code() {
    exiting_the_handler_keeps_the_host_and_its_state(&LUA, "os.exit()");
    exiting_the_handler_keeps_the_host_and_its_state(&LUA, "os.exit(3)");
    exiting_the_handler_keeps_the_host_and_its_state(&LUA, "os.exit(false, true)");
}

#[test]
fn micropython_sys_exit_ends_the_handler_only() {
    exiting_the_handler_keeps_the_host_and_its_state(&MICROPYTHON, "import sys\nsys.exit(2)");
}

/// The bundled mruby has no `exit` (no `mruby-exit` gem): calling it fails that call with
/// `NoMethodError` (the host does not end, and the next call works).
#[test]
fn mruby_has_no_exit_so_it_fails_the_call_with_no_method_error() {
    let name = file(&MRUBY, "exit");
    let code = format!("{}\nexit", MRUBY.count);
    let dir = plugin_dir(&[(&name, &code), (&file(&MRUBY, "n"), MRUBY.count)]);
    let mut host = host_of(Engine::Mruby, dir.path(), true);
    let mut world = World::new("");
    let report = world.call(&mut host, &name, PATIENT);
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains("NoMethodError"), "{text}");
    let next = world.call(&mut host, &file(&MRUBY, "n"), PATIENT);
    assert_eq!(stdout(&next), ["n=1"], "{:?}", next.output);
}
