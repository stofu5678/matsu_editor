//! End-to-end tests of the plugin host (ADR-0026): the built `matsu-script` program serves a
//! plugin under the editor's own host code (`matsu_plugin::plugin_host`), against a plain text
//! model.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use matsu_plugin::api;
use matsu_plugin::model::TextModel;
use matsu_plugin::plugin_host::{
    BeginError, HostSettings, MAX_FAILURES, Outcome, PluginApi, PluginHost, Progress, Report,
    respond,
};
use matsu_plugin::protocol::RpcError;
use matsu_plugin::session::{Engine, Language, Stream};
use serde_json::{Value, json};

const PATIENT: Duration = Duration::from_secs(30);

/// What the editor answers to the `Plugin` object in these tests.
#[derive(Default)]
struct Api {
    options: BTreeMap<(String, String), String>,
    calls: Vec<String>,
}

impl PluginApi for Api {
    fn call(&mut self, name: &str, params: &[Value]) -> Result<Value, RpcError> {
        self.calls.push(name.to_owned());
        let text = |i: usize| {
            params
                .get(i)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        match name {
            "Plugin.GetPluginDir" => Ok(json!("/plugins/テスト")),
            "Plugin.GetDef" => Ok(json!(format!("def:{}:{}", text(0), text(1)))),
            "Plugin.GetOption" => Ok(json!(
                self.options
                    .get(&(text(0), text(1)))
                    .cloned()
                    .unwrap_or_default()
            )),
            "Plugin.SetOption" => {
                self.options.insert((text(0), text(1)), text(2));
                Ok(json!(true))
            }
            "Plugin.GetString" => Ok(json!(format!("string {}", params[0]))),
            other => Err(RpcError::new(-32601, format!("no such call: {other}"))),
        }
    }
}

fn plugin_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, code) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, code).unwrap();
    }
    dir
}

fn host(dir: &Path, resident: bool) -> PluginHost {
    let mut settings = HostSettings::new(
        "jp.example.test",
        dir,
        Language::Bundled(Engine::QuickJs),
        resident,
    );
    settings.interpreter = env!("CARGO_BIN_EXE_matsu-script").into();
    PluginHost::new(settings)
}

struct World {
    doc: TextModel,
    api: Api,
}

impl World {
    fn new(text: &str) -> Self {
        Self {
            doc: TextModel::new(text, Some("/work/テスト.txt".into())),
            api: Api::default(),
        }
    }

    fn call(&mut self, host: &mut PluginHost, handler: &str, timeout: Duration) -> Report {
        let doc = &mut self.doc;
        let api_state = &mut self.api;
        host.run_blocking("Command", handler, json!({"n": 1}), timeout, |text| {
            respond(text, api_state, |method, params| {
                api::call(doc, method, params)
            })
        })
        .expect("the call begins")
    }
}

fn stdout(report: &Report) -> Vec<&str> {
    report.lines(Stream::Stdout)
}

#[test]
fn a_handler_edits_the_document_and_the_process_is_not_made_before_the_first_call() {
    let dir = plugin_dir(&[("insert.js", "Editor.InsText('こんにちは');")]);
    let mut host = host(dir.path(), false);
    assert!(
        !host.is_running(),
        "lazy: nothing runs before the first call"
    );
    let mut world = World::new("");
    let report = world.call(&mut host, "insert.js", PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.doc.text(), "こんにちは");
    // Not resident: the process ended with the call.
    assert!(!host.is_running());
}

#[test]
fn a_resident_host_keeps_its_process_and_its_globals_between_calls() {
    let dir = plugin_dir(&[(
        "count.js",
        "globalThis.n = (globalThis.n || 0) + 1; console.log('n=' + n);",
    )]);
    let mut host = host(dir.path(), true);
    let mut world = World::new("");
    let first = world.call(&mut host, "count.js", PATIENT);
    assert_eq!(first.outcome, Outcome::Ok);
    let pid = host.process_id().expect("it stays");
    let second = world.call(&mut host, "count.js", PATIENT);
    assert_eq!(host.process_id(), Some(pid), "the same process");
    assert_eq!(stdout(&first), ["n=1"]);
    assert_eq!(stdout(&second), ["n=2"]);
    host.shutdown();
    assert!(!host.is_running());
    // A new process starts the next time, from nothing.
    let third = world.call(&mut host, "count.js", PATIENT);
    assert_eq!(stdout(&third), ["n=1"]);
    assert_ne!(host.process_id(), Some(pid));
}

#[test]
fn a_call_that_is_not_resident_starts_a_new_process_each_time() {
    let dir = plugin_dir(&[(
        "count.js",
        "globalThis.n = (globalThis.n || 0) + 1; console.log('n=' + n);",
    )]);
    let mut host = host(dir.path(), false);
    let mut world = World::new("");
    for _ in 0..2 {
        let report = world.call(&mut host, "count.js", PATIENT);
        assert_eq!(stdout(&report), ["n=1"]);
        assert!(!host.is_running());
    }
}

#[test]
fn the_plugin_object_asks_the_editor_and_the_jack_and_arguments_are_there() {
    let code = r#"
console.log(Plugin.jack, JSON.stringify(Plugin.args), Plugin.id);
console.log(Plugin.GetPluginDir());
console.log(Plugin.GetDef('Plugin', 'Name'));
Plugin.SetOption('Main', 'Level', '7');
console.log(Plugin.GetOption('Main', 'Level'), '/', Plugin.GetOption('Main', 'None') === '');
console.log(Plugin.GetString(2));
"#;
    let dir = plugin_dir(&[("object.js", code)]);
    let mut host = host(dir.path(), false);
    let mut world = World::new("");
    let report = world.call(&mut host, "object.js", PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(
        stdout(&report),
        [
            "Command {\"n\":1} jp.example.test",
            "/plugins/テスト",
            "def:Plugin:Name",
            "7 / true",
            "string 2",
        ]
    );
    assert_eq!(world.api.calls.len(), 6);
}

#[test]
fn a_handler_that_throws_fails_with_its_line_and_the_host_goes_on() {
    let dir = plugin_dir(&[
        ("bad.js", "const a = 1;\nthrow new Error('だめ');"),
        ("ok.js", "Editor.InsText('ok');"),
    ]);
    let mut host = host(dir.path(), true);
    let mut world = World::new("");
    let report = world.call(&mut host, "bad.js", PATIENT);
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains("だめ") && text.contains("bad.js:2"), "{text}");
    // A resident host that had an error: the process was ended (a failure is not a good state),
    // and the next call starts a new one.
    let next = world.call(&mut host, "ok.js", PATIENT);
    assert_eq!(next.outcome, Outcome::Ok);
    assert_eq!(world.doc.text(), "ok");
    assert_eq!(host.failures(), 0, "a success starts the count over");
}

#[test]
fn an_error_of_the_editor_is_an_exception_in_the_handler() {
    let code = "try { Editor.GetLineStr('not a number'); } catch (e) { console.log(e.name); }";
    let dir = plugin_dir(&[("e.js", code)]);
    let mut host = host(dir.path(), false);
    let mut world = World::new("");
    let report = world.call(&mut host, "e.js", PATIENT);
    assert_eq!(report.outcome, Outcome::Ok);
    assert_eq!(stdout(&report), ["EditorError"]);
}

#[test]
fn a_handler_that_never_ends_is_killed_at_the_limit_and_the_next_call_works() {
    let dir = plugin_dir(&[
        ("loop.js", "while (true) {}"),
        ("ok.js", "console.log('alive');"),
    ]);
    let mut host = host(dir.path(), true);
    let mut world = World::new("");
    let started = Instant::now();
    let report = world.call(&mut host, "loop.js", Duration::from_millis(400));
    assert_eq!(report.outcome, Outcome::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(!host.is_running(), "the process was killed");
    assert_eq!(host.failures(), 1);
    let next = world.call(&mut host, "ok.js", PATIENT);
    assert_eq!(stdout(&next), ["alive"]);
    assert_eq!(host.failures(), 0);
}

#[test]
fn a_process_that_dies_in_a_call_is_reported_and_restarted_by_the_next_one() {
    let dir = plugin_dir(&[
        // A crash is made from outside, by killing the process while the handler runs.
        ("wait.js", "while (true) {}"),
        ("ok.js", "console.log('again');"),
    ]);
    let mut host = host(dir.path(), true);
    let mut world = World::new("");
    host.begin("Command", "wait.js", Value::Null, PATIENT)
        .unwrap();
    let pid = host.process_id().unwrap();
    kill_process(pid);
    let outcome = loop {
        if let Some(Progress::Done(outcome)) = host.poll(Duration::from_millis(100)) {
            break outcome;
        }
    };
    assert!(matches!(outcome, Outcome::Died(_)), "{outcome:?}");
    assert!(!host.is_running());
    assert_eq!(host.failures(), 1);
    let next = world.call(&mut host, "ok.js", PATIENT);
    assert_eq!(stdout(&next), ["again"]);
}

fn kill_process(pid: u32) {
    #[cfg(unix)]
    let status = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status();
    #[cfg(windows)]
    let status = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .status();
    assert!(status.unwrap().success());
}

#[test]
fn three_failures_in_a_row_stop_the_plugin() {
    let dir = plugin_dir(&[("bad.js", "throw new Error('x');"), ("ok.js", "1;")]);
    let mut host = host(dir.path(), false);
    let mut world = World::new("");
    for _ in 0..MAX_FAILURES {
        assert!(!host.is_stopped());
        let report = world.call(&mut host, "bad.js", PATIENT);
        assert!(matches!(report.outcome, Outcome::Failed(_)));
    }
    assert!(host.is_stopped());
    assert!(
        host.begin("Command", "ok.js", Value::Null, PATIENT)
            .is_err()
    );
    assert!(!host.is_running(), "a stopped plugin starts nothing");
    host.reset();
    assert_eq!(world.call(&mut host, "ok.js", PATIENT).outcome, Outcome::Ok);
}

#[test]
fn the_user_can_stop_a_call_and_it_is_not_a_failure() {
    let dir = plugin_dir(&[("loop.js", "while (true) {}")]);
    let mut host = host(dir.path(), true);
    host.begin("Command", "loop.js", Value::Null, PATIENT)
        .unwrap();
    assert!(host.is_busy());
    // The loop is running; a second call is refused.
    assert!(matches!(
        host.begin("Command", "loop.js", Value::Null, PATIENT),
        Err(BeginError::Busy)
    ));
    assert_eq!(host.cancel(), Some(Outcome::Cancelled));
    assert!(!host.is_running() && !host.is_busy());
    assert_eq!(host.failures(), 0);
}

#[test]
fn a_japanese_plugin_folder_and_handler_name_work() {
    let dir = plugin_dir(&[("日本語/スクリプト.js", "Editor.InsText('名前');")]);
    let mut host = host(dir.path(), false);
    let mut world = World::new("");
    let report = world.call(&mut host, "日本語/スクリプト.js", PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.doc.text(), "名前");
}

#[test]
fn a_language_that_cannot_serve_a_plugin_yet_says_so() {
    let dir = plugin_dir(&[("a.lua", "print(1)")]);
    let mut settings = HostSettings::new("p", dir.path(), Language::Bundled(Engine::Lua), false);
    settings.interpreter = env!("CARGO_BIN_EXE_matsu-script").into();
    let mut host = PluginHost::new(settings);
    let error = host
        .begin("Command", "a.lua", Value::Null, PATIENT)
        .unwrap_err();
    assert!(error.to_string().contains("まだ動かせません"), "{error}");
}

#[test]
fn a_handler_that_prints_without_end_still_meets_its_limit() {
    let dir = plugin_dir(&[("flood.js", "while (true) console.log('x');")]);
    let mut host = host(dir.path(), false);
    let mut world = World::new("");
    let started = Instant::now();
    let report = world.call(&mut host, "flood.js", Duration::from_millis(500));
    assert_eq!(report.outcome, Outcome::TimedOut);
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
    assert!(!host.is_running());
}

#[test]
fn shutting_a_host_down_in_a_call_ends_the_call_for_whoever_follows_it() {
    let dir = plugin_dir(&[("loop.js", "while (true) {}")]);
    let mut host = host(dir.path(), true);
    host.begin("Command", "loop.js", Value::Null, PATIENT)
        .unwrap();
    host.shutdown();
    assert!(!host.is_running() && !host.is_busy());
    // The one who polls is told once that the call is over, and it is not a failure.
    assert!(matches!(
        host.poll(Duration::ZERO),
        Some(Progress::Done(Outcome::Cancelled))
    ));
    assert!(host.poll(Duration::ZERO).is_none());
    assert_eq!(host.failures(), 0);
}

#[test]
fn a_host_that_was_shut_down_in_a_call_serves_the_next_call_normally() {
    let dir = plugin_dir(&[
        ("loop.js", "while (true) {}"),
        ("ok.js", "console.log('fine');"),
    ]);
    let mut host = host(dir.path(), true);
    host.begin("Command", "loop.js", Value::Null, PATIENT)
        .unwrap();
    host.shutdown();
    // Nobody polled for the end of the first call; the second is not taken for it.
    let mut world = World::new("");
    let report = world.call(&mut host, "ok.js", PATIENT);
    assert_eq!(report.outcome, Outcome::Ok);
    assert_eq!(stdout(&report), ["fine"]);
}

#[test]
fn each_handler_file_has_its_own_global_scope_and_may_declare_what_another_declares() {
    let dir = plugin_dir(&[
        (
            "a.js",
            "const config = 'a'; globalThis.seen = typeof other; globalThis.fromA = (globalThis.fromA || 0) + 1; console.log('a', config, seen, fromA);",
        ),
        (
            "b.js",
            "const config = 'b'; class Thing {} console.log('b', config, typeof fromA, typeof seen);",
        ),
    ]);
    let mut host = host(dir.path(), true);
    let mut world = World::new("");
    let a = world.call(&mut host, "a.js", PATIENT);
    assert_eq!(a.outcome, Outcome::Ok, "{:?}", a.output);
    // The second file declares the same `const` without trouble, and does not see the first's.
    let b = world.call(&mut host, "b.js", PATIENT);
    assert_eq!(b.outcome, Outcome::Ok, "{:?}", b.output);
    assert_eq!(stdout(&b), ["b b undefined undefined"]);
    // The first file again: its own `globalThis` value is still there.
    let again = world.call(&mut host, "a.js", PATIENT);
    assert_eq!(again.outcome, Outcome::Ok, "{:?}", again.output);
    assert_eq!(stdout(&again), ["a a undefined 2"]);
}

#[test]
fn a_resident_handler_with_a_let_or_a_const_can_be_called_again_and_keeps_global_this_state() {
    let code = "let local = 1; const fixed = 2; class K {} var mine = 1; function f() { return globalThis.kept; } globalThis.kept = (typeof globalThis.kept === 'number' ? globalThis.kept : 0) + 1; console.log(local + fixed, f(), typeof mine);";
    let dir = plugin_dir(&[("again.js", code)]);
    let mut host = host(dir.path(), true);
    let mut world = World::new("");
    for expected in ["3 1 number", "3 2 number", "3 3 number"] {
        let report = world.call(&mut host, "again.js", PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        assert_eq!(stdout(&report), [expected]);
    }
}

#[test]
fn the_line_of_an_error_is_the_line_of_the_file_also_inside_the_block() {
    let dir = plugin_dir(&[("e.js", "let a = 1;\nlet b = 2;\nthrow new Error('三行目');")]);
    let mut host = host(dir.path(), false);
    let mut world = World::new("");
    let report = world.call(&mut host, "e.js", PATIENT);
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains("e.js:3"), "{text}");
}

#[test]
fn a_handler_outside_the_folder_is_not_run() {
    let outer = tempfile::tempdir().unwrap();
    std::fs::write(outer.path().join("outside.js"), "Editor.InsText('no');").unwrap();
    let plugin = outer.path().join("plugin");
    std::fs::create_dir_all(&plugin).unwrap();
    let mut host = host(&plugin, false);
    let error = host
        .begin("Command", "../outside.js", Value::Null, PATIENT)
        .unwrap_err();
    assert!(matches!(error, BeginError::Outside { .. }), "{error}");
    assert!(!host.is_running());
}

/// Whether a process with this id is still there (unix only; elsewhere the check is skipped).
#[cfg(unix)]
fn process_is_there(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Waits until the process is gone, up to two seconds.
#[cfg(unix)]
fn gone_soon(pid: u32) -> bool {
    let end = Instant::now() + Duration::from_secs(2);
    while Instant::now() < end {
        if !process_is_there(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[cfg(unix)]
#[test]
fn dropping_a_host_that_loops_kills_its_process_before_returning() {
    let dir = plugin_dir(&[("loop.js", "while (true) {}")]);
    let mut host = host(dir.path(), true);
    host.begin("Command", "loop.js", Value::Null, PATIENT)
        .unwrap();
    let pid = host.process_id().unwrap();
    assert!(process_is_there(pid));
    // The test ends right after the drop, as the editor does when it quits: the process must not
    // depend on a thread that may never run again.
    drop(host);
    assert!(gone_soon(pid), "the process {pid} is still there");
}

#[cfg(unix)]
#[test]
fn a_call_that_times_out_leaves_no_process_behind() {
    let dir = plugin_dir(&[("loop.js", "while (true) {}")]);
    let mut host = host(dir.path(), false);
    host.begin(
        "Command",
        "loop.js",
        Value::Null,
        Duration::from_millis(200),
    )
    .unwrap();
    let pid = host.process_id().unwrap();
    let outcome = loop {
        if let Some(Progress::Done(outcome)) = host.poll(Duration::from_millis(50)) {
            break outcome;
        }
    };
    assert_eq!(outcome, Outcome::TimedOut);
    assert!(gone_soon(pid), "the process {pid} is still there");
}

#[cfg(unix)]
#[test]
fn shutting_down_and_cancelling_a_looping_call_kill_the_process() {
    let dir = plugin_dir(&[("loop.js", "while (true) {}")]);
    for how in ["shutdown", "cancel"] {
        let mut host = host(dir.path(), true);
        host.begin("Command", "loop.js", Value::Null, PATIENT)
            .unwrap();
        let pid = host.process_id().unwrap();
        if how == "shutdown" {
            host.shutdown();
        } else {
            host.cancel();
        }
        assert!(gone_soon(pid), "{how}: the process {pid} is still there");
    }
}

#[test]
fn a_handler_that_asks_for_strict_mode_gets_it_in_every_form_and_one_that_does_not_runs_loose() {
    let dir = plugin_dir(&[
        ("plain.js", "'use strict';\nundeclared = 1;"),
        (
            "comment_first.js",
            "// a comment first\n\"use strict\"; undeclared = 1;",
        ),
        (
            "second_directive.js",
            "\"custom\"; \"use strict\"; undeclared = 1;",
        ),
        (
            "comment_after.js",
            "\"use strict\" /* c */; undeclared = 1;",
        ),
        ("comment_next.js", "\"use strict\"\n// c\nundeclared = 1;"),
        // Not directives: the string is part of an expression, and the handler is loose.
        (
            "expression.js",
            "\"use strict\"\n+ 1; undeclared = 1; console.log(typeof undeclared);",
        ),
        (
            "after_code.js",
            "var a = 1; \"use strict\"; undeclared = 1; console.log(typeof undeclared);",
        ),
        (
            "loose.js",
            "undeclared3 = 1; console.log(typeof undeclared3);",
        ),
        (
            "strict_this.js",
            "'use strict'; function f() { return this === undefined; } console.log(f());",
        ),
    ]);
    let mut host = host(dir.path(), true);
    let mut world = World::new("");
    for file in [
        "plain.js",
        "comment_first.js",
        "second_directive.js",
        "comment_after.js",
        "comment_next.js",
    ] {
        let report = world.call(&mut host, file, PATIENT);
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{file}: {:?}", report.outcome);
        };
        assert!(text.contains("is not defined"), "{file}: {text}");
        host.reset();
    }
    for file in ["loose.js", "after_code.js"] {
        let report = world.call(&mut host, file, PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{file}: {:?}", report.output);
        assert_eq!(stdout(&report), ["number"], "{file}");
    }
    let this = world.call(&mut host, "strict_this.js", PATIENT);
    assert_eq!(stdout(&this), ["true"]);
}

#[test]
fn a_flood_of_meaningless_messages_does_not_keep_a_poll_from_returning() {
    let dir = plugin_dir(&[("junk.js", "for (;;) console.log('\\x1e{}');")]);
    let mut host = host(dir.path(), false);
    host.begin("Command", "junk.js", Value::Null, PATIENT)
        .unwrap();
    for _ in 0..50 {
        let started = Instant::now();
        let _ = host.poll(Duration::ZERO);
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "{:?}",
            started.elapsed()
        );
    }
    host.shutdown();
}
