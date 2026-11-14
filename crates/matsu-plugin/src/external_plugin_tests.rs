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
fn the_boot_refuses_a_handler_that_is_not_cjs() {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = plugin_dir(&[("a.mjs", "Editor.InsText('x');")]);
    let mut host = host(dir.path(), Language::JavaScript, false);
    let mut world = World::new("");
    let report = world.call(&mut host, "a.mjs");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains(".cjs だけ"), "{text}");
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
