//! Plugins whose handlers run on an interpreter of the user (Node.js and Python), under the
//! resident host (ADR-0026, PR 10). Each test skips (and says so) when the interpreter is missing;
//! the CI runner of Linux x64 has both and requires them (`MATSU_REQUIRE_INTERPRETERS`).

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use crate::api;
use crate::model::TextModel;
use crate::plugin_host::{HostSettings, Outcome, PluginApi, PluginHost, Report, respond};
use crate::protocol::RpcError;
use crate::session::{Language, Stream};
use crate::tests::available;

const PATIENT: Duration = Duration::from_secs(120);

/// What the editor answers to the objects of the plugin in these tests.
#[derive(Default)]
struct Api {
    options: BTreeMap<(String, String), String>,
    /// `Name(arguments)` of each call of an `Outline` object.
    outline: Vec<String>,
    added: Vec<String>,
}

impl PluginApi for Api {
    fn call(&mut self, name: &str, params: &[Value]) -> Result<Value, RpcError> {
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
                    .unwrap_or_else(|| "Hello".into())
            )),
            "Plugin.SetOption" => {
                self.options.insert((text(0), text(1)), text(2));
                Ok(json!(true))
            }
            "Plugin.GetString" => Ok(json!(format!("string {}", params[0]))),
            n if n.starts_with("Outline.") => {
                self.outline
                    .push(format!("{n}{}", Value::Array(params.to_vec())));
                Ok(json!(null))
            }
            "Complement.AddList" => {
                let word = text(0);
                if self.added.contains(&word) {
                    return Ok(json!(-1));
                }
                self.added.push(word);
                Ok(json!(self.added.len()))
            }
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

fn host(dir: &Path, language: Language, resident: bool) -> PluginHost {
    let settings = HostSettings::new("jp.example.test", dir, language, resident);
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

    fn call_jack(
        &mut self,
        host: &mut PluginHost,
        jack: &str,
        handler: &str,
        args: &Value,
        timeout: Duration,
    ) -> Report {
        let doc = &mut self.doc;
        let api_state = &mut self.api;
        host.run_blocking(jack, handler, args, timeout, |text| {
            respond(text, api_state, |method, params| {
                api::call(doc, method, params)
            })
        })
        .expect("the call begins")
    }

    fn call(&mut self, host: &mut PluginHost, handler: &str) -> Report {
        self.call_jack(host, "Command", handler, &json!({"n": 1}), PATIENT)
    }
}

fn stdout(report: &Report) -> Vec<&str> {
    report.lines(Stream::Stdout)
}

#[test]
fn a_node_plugin_runs_cjs_handlers_with_the_plugin_object() {
    if !available(Language::JavaScript) {
        return;
    }
    let code = "
const n = 1;
console.log(Plugin.jack, JSON.stringify(Plugin.args), Plugin.id);
console.log(Plugin.GetPluginDir(), Plugin.GetDef('Plugin', 'Name'));
Plugin.SetOption('Main', 'Level', '7');
console.log(Plugin.GetOption('Main', 'Level'), Plugin.GetString(2));
Editor.InsText('名前' + n);
";
    let dir = plugin_dir(&[("h.cjs", code)]);
    let mut host = host(dir.path(), Language::JavaScript, false);
    let mut world = World::new("");
    let report = world.call(&mut host, "h.cjs");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(
        stdout(&report),
        [
            "Command {\"n\":1} jp.example.test",
            "/plugins/テスト def:Plugin:Name",
            "7 string 2",
        ]
    );
    assert_eq!(world.doc.text(), "名前1");
    assert!(!host.is_running(), "not resident: the process ended");
}

#[test]
fn text_without_a_line_break_stays_whole_and_inside_its_call() {
    // A line cut by an editor call is joined; the text left at the end of a call belongs to it,
    // not to the next call.
    let cases = [
        (
            Language::JavaScript,
            "c.cjs",
            "globalThis.n = (globalThis.n || 0) + 1;
if (n === 1) { process.stdout.write('one '); process.stdout.write(Plugin.GetPluginDir()); } else { console.log('two'); }",
        ),
        (
            Language::Python,
            "c.py",
            "n = globals().get('n', 0) + 1
if n == 1:
    print('one ', end=''); print(Plugin.GetPluginDir(), end='')
else:
    print('two')",
        ),
    ];
    for (language, name, code) in cases {
        if !available(language) {
            continue;
        }
        let dir = plugin_dir(&[(name, code)]);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        let first = world.call(&mut host, name);
        let second = world.call(&mut host, name);
        assert_eq!(stdout(&first), ["one /plugins/テスト"], "{name}");
        assert_eq!(stdout(&second), ["two"], "{name}");
        host.shutdown();
    }
}

#[test]
fn the_boot_refuses_a_handler_that_is_not_a_node_module() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = plugin_dir(&[("a.txt", "Editor.InsText('x');")]);
    let mut host = host(dir.path(), Language::JavaScript, false);
    let mut world = World::new("");
    let report = world.call(&mut host, "a.txt");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains(".cjs、.mjs、.ts"), "{text}");
    assert_eq!(world.doc.text(), "");
}

#[test]
fn a_resident_node_host_keeps_its_process_and_globals_and_runs_each_call_again() {
    if !available(Language::JavaScript) {
        return;
    }
    let code = "globalThis.n = (globalThis.n || 0) + 1; console.log('n=' + n);";
    let dir = plugin_dir(&[("c.cjs", code)]);
    let name = "c.cjs";
    let mut host = host(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    let first = world.call(&mut host, name);
    let pid = host.process_id().expect("it stays");
    let second = world.call(&mut host, name);
    assert_eq!(host.process_id(), Some(pid), "{name}: the same process");
    assert_eq!(stdout(&first), ["n=1"], "{name}");
    assert_eq!(stdout(&second), ["n=2"], "{name}");
    host.shutdown();
}

#[test]
fn a_node_handler_may_use_the_modules_of_its_folder() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = plugin_dir(&[
        ("req.cjs", "Editor.InsText(require('./lib/word.cjs').word);"),
        ("lib/word.cjs", "exports.word = 'こと';"),
    ]);
    let mut host = host(dir.path(), Language::JavaScript, false);
    let mut world = World::new("");
    assert_eq!(world.call(&mut host, "req.cjs").outcome, Outcome::Ok);
    assert_eq!(world.doc.text(), "こと");
}

#[test]
fn a_node_handler_that_throws_or_rejects_fails_the_call_with_its_place() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = plugin_dir(&[
        ("bad.cjs", "const a = 1;\nthrow new Error('だめ');"),
        ("reject.cjs", "Promise.reject(new Error('はじかれた'));"),
        ("editor.cjs", "Editor.GetLineStr('x');"),
        ("ok.cjs", "Editor.InsText('ok');"),
    ]);
    let mut host = host(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    for (name, needle) in [
        ("bad.cjs", "bad.cjs:2"),
        ("reject.cjs", "はじかれた"),
        ("editor.cjs", "EditorError"),
    ] {
        let report = world.call(&mut host, name);
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{name}: {:?}", report.outcome);
        };
        assert!(text.contains(needle), "{name}: {text}");
        host.reset();
    }
    let next = world.call(&mut host, "ok.cjs");
    assert_eq!(next.outcome, Outcome::Ok, "{:?}", next.output);
    assert_eq!(world.doc.text(), "ok");
}

#[test]
fn a_node_handler_that_never_ends_is_killed_at_the_limit_and_three_failures_stop_the_plugin() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = plugin_dir(&[("loop.cjs", "for (;;) {}"), ("ok.cjs", "1;")]);
    let mut host = host(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    for _ in 0..3 {
        let report = world.call_jack(
            &mut host,
            "Command",
            "loop.cjs",
            &Value::Null,
            Duration::from_secs(2),
        );
        assert_eq!(report.outcome, Outcome::TimedOut);
        assert!(!host.is_running(), "the process was killed");
    }
    assert!(host.is_stopped());
    assert!(
        host.begin("Command", "ok.cjs", &Value::Null, PATIENT)
            .is_err()
    );
}

#[test]
fn a_node_plugin_serves_the_outline_complement_and_smart_indent_objects() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "outline.cjs",
            "Outline.SetTitle('見出し'); Outline.AddFuncInfo2(3, 1, 'a', 0);",
        ),
        (
            "complement.cjs",
            "const w = Complement.GetCurrentWord();\nconsole.log(Complement.GetOption());\nComplement.AddList(w + '1'); console.log(Complement.AddList(w + '1'));",
        ),
        (
            "indent.cjs",
            "console.log(JSON.stringify(Indent.GetChar()));",
        ),
        ("noindent.cjs", "console.log(typeof Indent);"),
    ]);
    let mut host = host(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    let report = world.call_jack(&mut host, "Outline", "outline.cjs", &json!({}), PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(
        world.api.outline,
        [
            "Outline.SetTitle[\"見出し\"]",
            "Outline.AddFuncInfo2[3,1,\"a\",0]"
        ]
    );
    let args = json!({"word": "あ", "option": 3});
    let report = world.call_jack(&mut host, "Complement", "complement.cjs", &args, PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(stdout(&report), ["3", "-1"]);
    assert_eq!(world.api.added, ["あ1"]);
    let args = json!({"char": "\r"});
    let report = world.call_jack(&mut host, "SmartIndent", "indent.cjs", &args, PATIENT);
    assert_eq!(stdout(&report), ["\"\\r\""], "{:?}", report.output);
    let report = world.call_jack(&mut host, "Command", "noindent.cjs", &json!({}), PATIENT);
    assert_eq!(stdout(&report), ["undefined"]);
}

#[test]
fn a_python_plugin_runs_with_the_plugin_object_and_keeps_its_namespace() {
    if !available(Language::Python) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "h.py",
            "import sys\nglobal_count = globals().get('global_count', 0) + 1\nprint(Plugin.jack, Plugin.args, Plugin.id, global_count)\nprint(Plugin.GetPluginDir(), Plugin.GetDef('Plugin', 'Name'))\nPlugin.SetOption('Main', 'Level', '7')\nprint(Plugin.GetOption('Main', 'Level'), Plugin.GetString(2))\nEditor.InsText('名前')\n",
        ),
        ("日本語/ハンドラ.py", "Editor.InsText('か')"),
    ]);
    let mut host = host(dir.path(), Language::Python, true);
    let mut world = World::new("");
    let first = world.call(&mut host, "h.py");
    assert_eq!(first.outcome, Outcome::Ok, "{:?}", first.output);
    assert_eq!(
        stdout(&first),
        [
            "Command {'n': 1} jp.example.test 1",
            "/plugins/テスト def:Plugin:Name",
            "7 string 2",
        ]
    );
    let pid = host.process_id().expect("it stays");
    let second = world.call(&mut host, "h.py");
    assert_eq!(host.process_id(), Some(pid));
    assert!(stdout(&second)[0].ends_with(" 2"), "{:?}", second.output);
    let third = world.call(&mut host, "日本語/ハンドラ.py");
    assert_eq!(third.outcome, Outcome::Ok, "{:?}", third.output);
    assert_eq!(world.doc.text(), "名前名前か");
    host.shutdown();
}

#[test]
fn a_python_handler_that_raises_fails_with_its_line_and_sys_exit_is_not_a_failure() {
    if !available(Language::Python) {
        return;
    }
    let dir = plugin_dir(&[
        ("bad.py", "a = 1\nraise ValueError('だめ')\n"),
        ("syntax.py", "def (:\n"),
        ("editor.py", "Editor.GetLineStr('x')\n"),
        ("exit.py", "import sys\nsys.exit(3)\n"),
        ("ok.py", "Editor.InsText('ok')\n"),
    ]);
    let mut host = host(dir.path(), Language::Python, true);
    let mut world = World::new("");
    for (name, needle) in [
        ("bad.py", "bad.py\", line 2"),
        ("syntax.py", "SyntaxError"),
        ("editor.py", "EditorError"),
    ] {
        let report = world.call(&mut host, name);
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{name}: {:?}", report.outcome);
        };
        assert!(text.contains(needle), "{name}: {text}");
        host.reset();
    }
    assert_eq!(world.call(&mut host, "exit.py").outcome, Outcome::Ok);
    assert_eq!(world.call(&mut host, "ok.py").outcome, Outcome::Ok);
    assert_eq!(world.doc.text(), "ok");
}

#[test]
fn a_python_handler_that_never_ends_is_killed_at_the_limit() {
    if !available(Language::Python) {
        return;
    }
    let dir = plugin_dir(&[("loop.py", "while True:\n    pass\n"), ("ok.py", "pass\n")]);
    let mut host = host(dir.path(), Language::Python, true);
    let mut world = World::new("");
    let report = world.call_jack(
        &mut host,
        "Command",
        "loop.py",
        &Value::Null,
        Duration::from_secs(2),
    );
    assert_eq!(report.outcome, Outcome::TimedOut);
    assert!(!host.is_running());
    // The next call starts a new process and works.
    assert_eq!(world.call(&mut host, "ok.py").outcome, Outcome::Ok);
}

#[test]
fn a_python_plugin_serves_the_outline_complement_and_smart_indent_objects() {
    if !available(Language::Python) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "outline.py",
            "Outline.SetTitle('見出し')\nOutline.AddFuncInfo2(3, 1, 'a', 0)\n",
        ),
        (
            "complement.py",
            "w = Complement.GetCurrentWord()\nprint(Complement.GetOption())\nComplement.AddList(w + '1')\nprint(Complement.AddList(w + '1'))\n",
        ),
        ("indent.py", "print(repr(Indent.GetChar()))\n"),
        ("noindent.py", "print('Indent' in globals())\n"),
    ]);
    let mut host = host(dir.path(), Language::Python, true);
    let mut world = World::new("");
    let report = world.call_jack(&mut host, "Outline", "outline.py", &json!({}), PATIENT);
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(
        world.api.outline,
        [
            "Outline.SetTitle[\"見出し\"]",
            "Outline.AddFuncInfo2[3,1,\"a\",0]"
        ]
    );
    let args = json!({"word": "あ", "option": 3});
    let report = world.call_jack(&mut host, "Complement", "complement.py", &args, PATIENT);
    assert_eq!(stdout(&report), ["3", "-1"], "{:?}", report.output);
    assert_eq!(world.api.added, ["あ1"]);
    let args = json!({"char": "\r"});
    let report = world.call_jack(&mut host, "SmartIndent", "indent.py", &args, PATIENT);
    assert_eq!(stdout(&report), ["'\\r'"], "{:?}", report.output);
    let report = world.call_jack(&mut host, "Command", "noindent.py", &json!({}), PATIENT);
    assert_eq!(stdout(&report), ["False"]);
}

#[test]
fn the_sample_plugins_run() {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/plugins");
    let text = "# 見出し\n本文\n## 小見出し\n";

    if available(Language::JavaScript) {
        let mut host = host(&samples.join("HelloNode"), Language::JavaScript, true);
        let mut world = World::new(text);
        let opened = world.call_jack(&mut host, "DocumentOpen", "opened.cjs", &json!({}), PATIENT);
        assert_eq!(opened.outcome, Outcome::Ok, "{:?}", opened.output);
        let report = world.call(&mut host, "command.cjs");
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        assert!(
            world
                .doc
                .text()
                .starts_with("Hello (Command, opened 1 times)")
        );
        let report = world.call_jack(&mut host, "Outline", "outline.cjs", &json!({}), PATIENT);
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
    }

    if available(Language::Python) {
        let mut host = host(&samples.join("HelloPython"), Language::Python, true);
        let mut world = World::new(text);
        for handler in ["opened.py", "opened.py"] {
            let report = world.call_jack(&mut host, "DocumentOpen", handler, &json!({}), PATIENT);
            assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        }
        let report = world.call(&mut host, "command.py");
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        assert!(world.doc.text().starts_with("Hello (Command)"));
        let report = world.call_jack(&mut host, "Outline", "outline.py", &json!({}), PATIENT);
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
        let report = world.call_jack(&mut host, "Complement", "complement.py", &args, PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        assert_eq!(world.api.added, ["ab_hello", "ab_world"]);
    }
}

#[test]
fn the_module_sample_plugin_runs_its_mjs_and_ts_handlers() {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/plugins/HelloModule");
    let text = "# 見出し\n本文\n## 小見出し\n";
    if available(Language::JavaScript) {
        let mut host = host_with_agreement(&samples, Language::JavaScript, true);
        let mut world = World::new(text);
        for _ in 0..2 {
            let opened =
                world.call_jack(&mut host, "DocumentOpen", "opened.mjs", &json!({}), PATIENT);
            assert_eq!(opened.outcome, Outcome::Ok, "{:?}", opened.output);
        }
        let report = world.call(&mut host, "command.mjs");
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        // The command and the jack share the module `opened.mjs`: it was loaded once.
        assert!(
            world
                .doc
                .text()
                .starts_with("Hello (Command, opened 2 times, ES module)"),
            "{}",
            world.doc.text()
        );
        host.shutdown();
    }
    if available(Language::TypeScript) {
        let mut host = host_with_agreement(&samples, Language::TypeScript, true);
        let mut world = World::new(text);
        let report = world.call_jack(&mut host, "Outline", "outline.ts", &json!({}), PATIENT);
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
        host.shutdown();
    }
}

#[test]
fn a_node_timer_of_a_handler_finishes_inside_its_own_call() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "timer.cjs",
            "setTimeout(() => Editor.InsText('x'), 100);\nsetTimeout(() => setTimeout(() => Editor.InsText('y'), 30), 30);",
        ),
        (
            "late_error.cjs",
            "setTimeout(() => { throw new Error('遅い'); }, 20);",
        ),
        ("forever.cjs", "setInterval(() => {}, 1000);"),
        ("next.cjs", "Editor.InsText('n');"),
    ]);
    let mut host = host(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    let report = world.call(&mut host, "timer.cjs");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    // Both timers, also the one that starts another, ran before the call ended: the chain of
    // 60 ms came first, the timer of 100 ms second.
    assert_eq!(world.doc.text(), "yx");
    // An error in a timer fails that call.
    let report = world.call(&mut host, "late_error.cjs");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains("遅い"), "{text}");
    host.reset();
    // A timer that never ends is the time limit's business.
    let report = world.call_jack(
        &mut host,
        "Command",
        "forever.cjs",
        &Value::Null,
        Duration::from_secs(2),
    );
    assert_eq!(report.outcome, Outcome::TimedOut);
    host.reset();
    let before = world.doc.text();
    assert_eq!(world.call(&mut host, "next.cjs").outcome, Outcome::Ok);
    assert_eq!(world.doc.text(), format!("{before}n"));
}

#[test]
fn a_python_namespace_is_a_main_module_while_the_handler_runs() {
    if !available(Language::Python) {
        return;
    }
    let first = "from __future__ import annotations
import dataclasses, pickle, sys

class Point:
    def __init__(self, x):
        self.x = x

@dataclasses.dataclass
class Box:
    point: Point
    n: int = 1

copy = pickle.loads(pickle.dumps(Point(7)))
print(copy.x, sys.modules['__main__'].Point is Point, Box(Point(1)).n)
";
    let second = "print(Point)
";
    let dir = plugin_dir(&[("first.py", first), ("second.py", second)]);
    let mut host = host(dir.path(), Language::Python, true);
    let mut world = World::new("");
    let report = world.call(&mut host, "first.py");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(stdout(&report), ["7 True 1"]);
    // The boot's own `__main__` is back between the calls: the next file has a module of its own,
    // and does not see the names of the first.
    let report = world.call(&mut host, "second.py");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?} {:?}", report.outcome, report.output);
    };
    assert!(text.contains("NameError"), "{text}");
}

#[test]
fn an_error_while_timers_run_ends_the_call_at_once_and_cancels_the_rest() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "reject.cjs",
            "setInterval(() => Editor.InsText('i'), 10);\nPromise.reject(new Error('はじかれた'));",
        ),
        (
            "throw.cjs",
            "setInterval(() => Editor.InsText('i'), 5);\nsetTimeout(() => { throw new Error('遅い'); }, 40);",
        ),
    ]);
    for (name, needle) in [("reject.cjs", "はじかれた"), ("throw.cjs", "遅い")] {
        let mut host = host(dir.path(), Language::JavaScript, true);
        let mut world = World::new("");
        let started = std::time::Instant::now();
        let report = world.call_jack(
            &mut host,
            "Command",
            name,
            &Value::Null,
            Duration::from_secs(60),
        );
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{name}: {:?}", report.outcome);
        };
        assert!(text.contains(needle), "{name}: {text}");
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "{name}: not at the deadline"
        );
        // Nothing edits the document after the call has ended.
        let after = world.doc.text();
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(world.doc.text(), after, "{name}");
    }
}

#[test]
fn a_timer_callback_gets_its_receiver_and_arguments() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = plugin_dir(&[(
        "this.cjs",
        "let n = 0;\nsetInterval(function (a, b) { n += 1; if (n === 3) { Editor.InsText(a + b + typeof this.refresh); clearInterval(this); } }, 5, 'p', 'q');",
    )]);
    let mut host = host(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    let report = world.call_jack(
        &mut host,
        "Command",
        "this.cjs",
        &Value::Null,
        Duration::from_secs(30),
    );
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.doc.text(), "pqfunction");
}

// ---- ES modules and TypeScript (ADR-0026, PR 10) ----

/// Node.js with `module.registerHooks` (22.15, 23.5): the plugin host serves `.mjs` and `.ts`, and
/// `require` of the plugin folder, only there.
fn module_hooks() -> bool {
    if !available(Language::JavaScript) {
        return false;
    }
    let output = std::process::Command::new("node")
        .args(["-p", "typeof require('module').registerHooks"])
        .output()
        .expect("node runs");
    let has = String::from_utf8_lossy(&output.stdout).trim() == "function";
    if !has {
        eprintln!("SKIP: this Node.js has no module.registerHooks");
        if std::env::var_os("MATSU_REQUIRE_INTERPRETERS").is_some_and(|v| !v.is_empty()) {
            panic!("module.registerHooks is required here (MATSU_REQUIRE_INTERPRETERS)");
        }
    }
    has
}

/// The agreement of the user for the files in `dir`, as `plugins.ini` keeps it.
fn agreed(dir: &Path) -> Vec<(String, String)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let hash = crate::plugin_host::sha256_hex(&std::fs::read(&path).unwrap());
                out.push((relative, hash));
            }
        }
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files);
    files
}

fn host_with_agreement(dir: &Path, language: Language, resident: bool) -> PluginHost {
    PluginHost::new(
        HostSettings::new("jp.example.test", dir, language, resident).with_verified(agreed(dir)),
    )
}

#[test]
fn an_mjs_handler_is_loaded_once_and_its_default_export_is_called_for_each_call() {
    if !module_hooks() {
        return;
    }
    let dir = plugin_dir(&[
        (
            "h.mjs",
            "import { word } from './lib/word.mjs';
import data from './lib/data.json' with { type: 'json' };
import { readFileSync } from 'node:fs';
globalThis.loads = (globalThis.loads ?? 0) + 1;
let calls = 0;
export default async function (args) {
  calls += 1;
  console.log(Plugin.jack, JSON.stringify(args), word, data.n, typeof readFileSync, 'loads=' + globalThis.loads, 'calls=' + calls);
  Editor.InsText(word);
}",
        ),
        ("lib/word.mjs", "export const word = 'こと';"),
        ("lib/data.json", "{\"n\": 7}"),
    ]);
    let mut host = host_with_agreement(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    let first = world.call(&mut host, "h.mjs");
    assert_eq!(first.outcome, Outcome::Ok, "{:?}", first.output);
    let pid = host.process_id().expect("it stays");
    let second = world.call(&mut host, "h.mjs");
    assert_eq!(second.outcome, Outcome::Ok, "{:?}", second.output);
    assert_eq!(host.process_id(), Some(pid));
    assert_eq!(
        stdout(&first),
        ["Command {\"n\":1} こと 7 function loads=1 calls=1"]
    );
    // The module ran once: the second call is the same module, and its state stays.
    assert_eq!(
        stdout(&second),
        ["Command {\"n\":1} こと 7 function loads=1 calls=2"]
    );
    assert_eq!(world.doc.text(), "ことこと");
    host.shutdown();
}

#[test]
fn a_ts_handler_runs_with_its_types_stripped_and_imports_other_ts_files() {
    if !available(Language::TypeScript) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "h.ts",
            "import { twice } from './util.ts';
import type { Shape } from './util.ts';
export default function (args: { n: number }): void {
  const shape: Shape = { w: args.n };
  Editor.InsText(String(twice(shape.w)));
  throw new Error('line 6');
}",
        ),
        (
            "util.ts",
            "export interface Shape { w: number }
export const twice = (n: number): number => n * 2;",
        ),
    ]);
    let mut host = host_with_agreement(dir.path(), Language::TypeScript, false);
    let mut world = World::new("");
    let report = world.call(&mut host, "h.ts");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    // The types became blanks: the error is on the same line of the file.
    assert!(text.contains("line 6") && text.contains("h.ts:6"), "{text}");
    assert_eq!(world.doc.text(), "2");
}

#[test]
fn imports_come_from_the_text_that_was_checked_not_from_the_disk() {
    if !module_hooks() {
        return;
    }
    let dir = plugin_dir(&[
        ("a.mjs", "export default function () { console.log('a'); }"),
        (
            "b.mjs",
            "import { word } from './later.mjs';
export default function () { console.log(word); }",
        ),
        ("later.mjs", "export const word = 'checked';"),
    ]);
    let mut host = host_with_agreement(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    assert_eq!(world.call(&mut host, "a.mjs").outcome, Outcome::Ok);
    // The files change after the process got them (after the check, before b.mjs is loaded).
    std::fs::write(dir.path().join("later.mjs"), "export const word = 'EVIL';").unwrap();
    std::fs::write(
        dir.path().join("b.mjs"),
        "export default function () { console.log('EVIL'); }",
    )
    .unwrap();
    let report = world.call(&mut host, "b.mjs");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(stdout(&report), ["checked"]);
    host.shutdown();
}

#[test]
fn a_file_that_changed_since_the_agreement_is_refused_before_anything_runs() {
    let dir = plugin_dir(&[
        (
            "h.mjs",
            "import './dep.mjs'; export default function () { Editor.InsText('ran'); }",
        ),
        ("dep.mjs", "export {};"),
        ("old.cjs", "Editor.InsText('cjs');"),
    ]);
    let mut host = host_with_agreement(dir.path(), Language::JavaScript, false);
    std::fs::write(
        dir.path().join("dep.mjs"),
        "Editor.InsText('EVIL'); export {};",
    )
    .unwrap();
    let error = host
        .begin("Command", "h.mjs", &Value::Null, PATIENT)
        .unwrap_err();
    assert!(
        matches!(&error, crate::plugin_host::BeginError::Changed { path } if path == "dep.mjs"),
        "{error}"
    );
    assert!(!host.is_running(), "no process was started");
    // A handler of another kind is checked too, and a file that was added is a change.
    std::fs::write(dir.path().join("old.cjs"), "Editor.InsText('EVIL');").unwrap();
    assert!(matches!(
        host.begin("Command", "old.cjs", &Value::Null, PATIENT),
        Err(crate::plugin_host::BeginError::Changed { .. })
    ));
    std::fs::write(dir.path().join("dep.mjs"), "export {};").unwrap();
    std::fs::write(dir.path().join("added.mjs"), "export {};").unwrap();
    assert!(matches!(
        host.begin("Command", "h.mjs", &Value::Null, PATIENT),
        Err(crate::plugin_host::BeginError::Changed { path }) if path == "added.mjs"
    ));
    assert!(!host.is_running());
}

#[test]
fn imports_outside_the_folder_and_packages_are_refused() {
    if !module_hooks() {
        return;
    }
    let outside = plugin_dir(&[("secret.mjs", "export const secret = 'x';")]);
    let outside_url = format!(
        "file://{}/secret.mjs",
        outside.path().display().to_string().replace('\\', "/")
    );
    let url_handler =
        format!("export default async function () {{ await import('{outside_url}'); }}");
    let dir = plugin_dir(&[
        (
            "up.mjs",
            "export default async function () { await import('../secret.mjs'); }",
        ),
        ("url.mjs", &url_handler),
        (
            "pkg.mjs",
            "export default async function () { await import('left-pad'); }",
        ),
        (
            "none.mjs",
            "export default async function () { await import('./missing.mjs'); }",
        ),
        ("nofn.mjs", "export const x = 1;"),
        ("ok.mjs", "export default () => Editor.InsText('ok');"),
    ]);
    let mut host = host_with_agreement(dir.path(), Language::JavaScript, false);
    let mut world = World::new("");
    for (file, needle) in [
        ("up.mjs", "同意したプラグインのフォルダーにありません"),
        ("url.mjs", "同意したプラグインのフォルダーにありません"),
        ("pkg.mjs", "パッケージ"),
        ("none.mjs", "同意したプラグインのフォルダーにありません"),
        ("nofn.mjs", "default で export"),
    ] {
        let report = world.call(&mut host, file);
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{file}: {:?}", report.outcome);
        };
        assert!(text.contains(needle), "{file}: {text}");
        host.reset();
    }
    assert_eq!(world.call(&mut host, "ok.mjs").outcome, Outcome::Ok);
    assert_eq!(world.doc.text(), "ok");
}

#[test]
fn a_cjs_handler_requires_files_of_its_folder_from_the_checked_text_too() {
    if !module_hooks() {
        return;
    }
    let dir = plugin_dir(&[
        ("first.cjs", "Editor.InsText(require('./lib/word').word);"),
        (
            "second.cjs",
            "Editor.InsText(require('./lib2/word.cjs').word); require('left-pad');",
        ),
        ("lib/word.cjs", "exports.word = 'a';"),
        ("lib2/word.cjs", "exports.word = 'b';"),
    ]);
    let mut host = host_with_agreement(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    // No extension: the way of CommonJS.
    assert_eq!(world.call(&mut host, "first.cjs").outcome, Outcome::Ok);
    std::fs::write(dir.path().join("lib2/word.cjs"), "exports.word = 'EVIL';").unwrap();
    let report = world.call(&mut host, "second.cjs");
    // `b` came from the text that was checked; the package is refused.
    assert_eq!(world.doc.text(), "ab");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains("パッケージ"), "{text}");
}

#[test]
fn the_memory_of_a_resident_node_host_stays_flat_over_many_calls() {
    if !module_hooks() {
        return;
    }
    let dir = plugin_dir(&[(
        "h.mjs",
        "import { setFlagsFromString } from 'node:v8';
import { runInNewContext } from 'node:vm';
setFlagsFromString('--expose-gc');
const gc = runInNewContext('gc');
let calls = 0;
let base = 0;
export default function () {
  calls += 1;
  if (calls === 300) { gc(); base = process.memoryUsage().heapUsed; }
  if (calls === 3000) {
    gc();
    console.log('grown=' + (process.memoryUsage().heapUsed - base));
  }
}",
    )]);
    let mut host = host_with_agreement(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    let mut grown = None;
    for _ in 0..3000 {
        let report = world.call_jack(&mut host, "Command", "h.mjs", &Value::Null, PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        if let Some(line) = stdout(&report).first() {
            grown = line
                .strip_prefix("grown=")
                .and_then(|n| n.parse::<i64>().ok());
        }
    }
    // An import() for each call would keep about 11 KB for each: 30 MB over these calls.
    let grown = grown.expect("the last call reports");
    assert!(grown < 2 * 1024 * 1024, "the heap grew by {grown} bytes");
    host.shutdown();
}

#[test]
fn the_timer_rules_hold_for_module_handlers() {
    if !module_hooks() {
        return;
    }
    let dir = plugin_dir(&[
        (
            "timer.mjs",
            "export default function () { setTimeout(() => Editor.InsText('t'), 50); }",
        ),
        (
            "late.mjs",
            "export default function () { setTimeout(() => { throw new Error('遅い'); }, 20); }",
        ),
        (
            "reject.mjs",
            "export default async function () { throw new Error('はじかれた'); }",
        ),
    ]);
    let mut host = host_with_agreement(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    let report = world.call(&mut host, "timer.mjs");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(world.doc.text(), "t", "the call ended after its timer");
    for (file, needle) in [("late.mjs", "遅い"), ("reject.mjs", "はじかれた")] {
        let report = world.call(&mut host, file);
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{file}: {:?}", report.outcome);
        };
        assert!(text.contains(needle), "{file}: {text}");
        host.reset();
    }
}

mod ruby_php;

mod powershell_perl;

mod emacs_applescript;
