//! Tests of the calls of a plugin on a window: what the plugin asks, and a plugin that runs end
//! to end on the real `matsu-script` (skipped when that program is not built next to the tests).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_config::Profile;
use matsu_config::plugins::consent::{ConsentFile, fingerprint};
use matsu_config::plugins::{PluginRec, PluginSettings, PluginState, SearchDirs};
use matsu_core::buffer::Document;
use matsu_core::model::DocumentModel;
use matsu_plugin::plugin_host::PluginApi;
use matsu_plugin::{Value, json};

use super::*;
use crate::document::SharedDocument;
use crate::settings::UserSettings;

const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../matsu-plugin/samples/plugins"
);

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// The settings file in a temporary folder, with the sample plugin `HelloJs` in `plugins`.
struct Place {
    dir: tempfile::TempDir,
}

impl Place {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        copy_dir(
            &Path::new(SAMPLES).join("HelloJs"),
            &dir.path().join("plugins").join("HelloJs"),
        );
        Self { dir }
    }

    fn settings(&self) -> PathBuf {
        self.dir.path().join("matsu.ini")
    }

    fn dirs(&self) -> SearchDirs {
        SearchDirs::new(&self.settings(), None)
    }
}

fn def_api(place: &Place) -> DefApi {
    let dirs = place.dirs();
    let def = dirs.load_def("HelloJs", DEFAULT_LANGUAGE).unwrap();
    DefApi::new(&dirs, "HelloJs", def, 1)
}

fn ask(api: &mut DefApi, name: &str, params: &[Value]) -> Result<Value, RpcError> {
    api.call(name, params)
}

#[test]
fn the_plugin_object_reads_the_definition_and_the_options_of_its_plugin() {
    let place = Place::new();
    let mut api = def_api(&place);
    let dir = ask(&mut api, "Plugin.GetPluginDir", &[]).unwrap();
    assert!(dir.as_str().unwrap().ends_with("HelloJs"));
    assert_eq!(
        ask(
            &mut api,
            "Plugin.GetDef",
            &[json!("Plugin"), json!("Author")]
        ),
        Ok(json!("MATSU Editor"))
    );
    // Nothing to read is an empty string, as in Sakura.
    assert_eq!(
        ask(
            &mut api,
            "Plugin.GetDef",
            &[json!("Plugin"), json!("Nothing")]
        ),
        Ok(json!(""))
    );
    // An option that the file does not have gives the default of `plugin.def`.
    assert_eq!(
        ask(
            &mut api,
            "Plugin.GetOption",
            &[json!("Hello"), json!("Greeting")]
        ),
        Ok(json!("Hello"))
    );
    assert_eq!(
        ask(
            &mut api,
            "Plugin.GetOption",
            &[json!("Hello"), json!("Other")]
        ),
        Ok(json!(""))
    );
    assert_eq!(ask(&mut api, "Plugin.GetCommandNo", &[]), Ok(json!(1)));
    assert_eq!(
        ask(&mut api, "Plugin.GetString", &[json!(0)]),
        Ok(json!("ja_JP"))
    );
    assert_eq!(
        ask(&mut api, "Plugin.GetString", &[json!(1)]),
        Ok(json!(""))
    );
    assert_eq!(
        ask(&mut api, "Plugin.GetString", &[json!(100)]),
        Ok(json!(""))
    );
}

#[test]
fn setting_an_option_writes_the_options_file_of_the_plugin_at_once() {
    let place = Place::new();
    let mut api = def_api(&place);
    ask(
        &mut api,
        "Plugin.SetOption",
        &[json!("Hello"), json!("Greeting"), json!("こんにちは")],
    )
    .unwrap();
    let file = place.dirs().option_file("HelloJs");
    assert_eq!(file, place.dir.path().join("plugins").join("HelloJs.ini"));
    let written = Profile::load(&file).unwrap();
    assert_eq!(written.get("Hello", "Greeting"), Some("こんにちは"));
    // A new reading sees it, and the default is not used any more.
    let mut again = def_api(&place);
    assert_eq!(
        ask(
            &mut again,
            "Plugin.GetOption",
            &[json!("Hello"), json!("Greeting")]
        ),
        Ok(json!("こんにちは"))
    );
    // A number is written as text.
    ask(
        &mut api,
        "Plugin.SetOption",
        &[json!("S"), json!("N"), json!(5)],
    )
    .unwrap();
    assert_eq!(Profile::load(&file).unwrap().get("S", "N"), Some("5"));
}

#[test]
fn bad_calls_of_the_plugin_object_are_errors_not_panics() {
    let place = Place::new();
    let mut api = def_api(&place);
    assert!(ask(&mut api, "Plugin.GetDef", &[json!("only one")]).is_err());
    assert!(ask(&mut api, "Plugin.GetString", &[json!("x")]).is_err());
    assert!(ask(&mut api, "Plugin.SetOption", &[json!("S"), json!("K")]).is_err());
    assert!(ask(&mut api, "Plugin.Unknown", &[]).is_err());
}

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Entity<EditorView>, &'a mut VisualTestContext) {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.executor().allow_parking();
    (view, vcx)
}

fn text_of(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, cx| {
        let doc = view.document().read(cx).doc();
        (0..doc.line_count())
            .map(|i| doc.line(i).unwrap().unwrap().text)
            .collect::<Vec<_>>()
            .concat()
    })
}

#[gpui::test]
fn a_request_of_a_plugin_runs_on_the_document_and_the_plugin_calls_on_the_definition(
    cx: &mut TestAppContext,
) {
    let place = Place::new();
    let (view, cx) = setup(cx, "");
    let mut api = def_api(&place);
    let insert = r#"{"jsonrpc":"2.0","id":1,"method":"InsText","params":["あいう"]}"#;
    let reply = view
        .update(cx, |view, cx| {
            view.answer_plugin_request(insert, &mut api, cx)
        })
        .unwrap();
    let reply: Value = reply.parse().unwrap();
    assert_eq!(reply["id"], 1);
    assert!(reply.get("error").is_none(), "{reply}");
    assert_eq!(text_of(&view, cx), "あいう");

    let option =
        r#"{"jsonrpc":"2.0","id":2,"method":"Plugin.GetOption","params":["Hello","Greeting"]}"#;
    let reply = view
        .update(cx, |view, cx| {
            view.answer_plugin_request(option, &mut api, cx)
        })
        .unwrap();
    let reply: Value = reply.parse().unwrap();
    assert_eq!(reply["result"], "Hello");

    // A call that does not exist is an error answer, and a notification gets none.
    let bad = r#"{"jsonrpc":"2.0","id":3,"method":"NoSuchCall","params":[]}"#;
    let reply = view
        .update(cx, |view, cx| view.answer_plugin_request(bad, &mut api, cx))
        .unwrap();
    assert!(reply.contains("error"), "{reply}");
    let note = r#"{"jsonrpc":"2.0","method":"InsText","params":["x"]}"#;
    assert_eq!(
        view.update(cx, |view, cx| view
            .answer_plugin_request(note, &mut api, cx)),
        None
    );
}

/// The program that serves the plugins, if the workspace build made it.
fn script_host() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let name = if cfg!(windows) {
        "matsu-script.exe"
    } else {
        "matsu-script"
    };
    // target/debug/deps/<test> -> target/debug/matsu-script
    let host = exe.parent()?.parent()?.join(name);
    host.is_file().then_some(host)
}

/// The settings and the agreement of a user who enabled the sample plugin.
fn enable_sample(cx: &mut VisualTestContext, place: &Place, enabled: bool, agree: bool) {
    let dirs = place.dirs();
    let def = dirs.load_def("HelloJs", DEFAULT_LANGUAGE).unwrap();
    let mut settings = PluginSettings {
        enabled,
        ..PluginSettings::default()
    };
    settings.table[0] = PluginRec {
        id: def.id.clone(),
        name: "HelloJs".into(),
        cmd_num: 1,
        state: PluginState::Stopped,
    };
    let mut profile = Profile::new();
    settings.store(&mut profile).unwrap();
    profile.save(&place.settings()).unwrap();
    if !agree {
        let _ = fs::remove_file(dirs.consent_file());
    } else {
        let files = fingerprint(&dirs.plugin_dir("HelloJs").unwrap()).unwrap();
        let mut consent = ConsentFile::default();
        consent
            .enable(&def.id, "HelloJs", &files, "2026-10-26")
            .unwrap();
        consent.save(&dirs.consent_file()).unwrap();
    }
    let user = UserSettings::from_profile(profile, Some(place.settings()));
    cx.update(|_, app| app.set_global(user));
}

fn wait_for(view: &Entity<EditorView>, cx: &mut VisualTestContext, needle: &str) -> String {
    for _ in 0..600 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        let output = view.read_with(cx, |v, _| v.macro_output_text());
        if output.contains(needle) {
            return output;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "no \"{needle}\": {}",
        view.read_with(cx, |v, _| v.macro_output_text())
    );
}

#[gpui::test]
fn a_plugin_command_runs_end_to_end_and_the_checks_before_it_hold(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "");

    // Plugins off: refused, and the words say what to do.
    enable_sample(cx, &place, false, true);
    let error = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .err()
        .unwrap();
    assert!(error.contains("プラグインを有効にする"), "{error}");
    // On, but the user did not agree to this plugin.
    enable_sample(cx, &place, true, false);
    let error = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .err()
        .unwrap();
    assert!(error.contains("有効にしていません"), "{error}");
    // Agreed, and then a file changed.
    enable_sample(cx, &place, true, true);
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    let original = fs::read_to_string(&script).unwrap();
    fs::write(&script, format!("{original}\n// changed")).unwrap();
    let error = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .err()
        .unwrap();
    assert!(error.contains("ファイルが変わった"), "{error}");
    fs::write(&script, &original).unwrap();
    // No such command.
    let error = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 2))
        .err()
        .unwrap();
    assert!(error.contains("コマンドがありません"), "{error}");

    // The checks pass: the command runs, writes the default greeting, and ends.
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    let output = wait_for(&view, cx, "正常に終了しました");
    assert!(
        output.contains("Hello (JavaScript)") || output.contains("ハロー"),
        "{output}"
    );
    assert_eq!(text_of(&view, cx), "Hello");

    // The plugin wrote nothing to the options file; the greeting set there is used next time.
    let mut options = PluginOptions::load(&dirs.option_file("HelloJs"));
    options.set("Hello", "Greeting", "どうも").unwrap();
    options.save().unwrap();
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    for _ in 0..600 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if text_of(&view, cx).contains("どうも") {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(text_of(&view, cx), "Helloどうも");
    // Both calls were one undo step each.
    view.update(cx, |view, cx| {
        let depth = view.document().read(cx).model().history().depth();
        assert!(depth <= 2, "{depth}");
    });
    set_test_interpreter(None);
    cx.update(|_, app| shutdown(app, "io.github.stofu5678.matsu-editor.sample.hello-js"));
}

#[gpui::test]
fn a_plugin_that_fails_three_times_is_stopped_and_shows_it(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "");
    // The handler of the command throws.
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    fs::write(&script, "throw new Error('壊れています');").unwrap();
    enable_sample(cx, &place, true, true);
    let id = "io.github.stofu5678.matsu-editor.sample.hello-js";
    for round in 1..=3 {
        let prepared = cx
            .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
            .unwrap();
        let editor = view.clone();
        cx.update(|_, app| run(editor, prepared, app));
        let output = wait_for(&view, cx, "エラーで終了しました");
        assert!(output.contains("壊れています"), "{output}");
        assert_eq!(is_stopped(id), round == 3, "after {round}");
    }
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(output.contains("止めました"), "{output}");
    let error = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .err()
        .unwrap();
    assert!(error.contains("止めています"), "{error}");
    // Enabling again starts over.
    cx.update(|_, app| reset(app, id));
    assert!(!is_stopped(id));
    fs::write(&script, "Editor.InsText('戻りました');").unwrap();
    // The file changed, so the user agrees again (the agreement is of the files).
    enable_sample(cx, &place, true, true);
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    for _ in 0..600 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if text_of(&view, cx).contains("戻りました") {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(text_of(&view, cx), "戻りました");
    set_test_interpreter(None);
    cx.update(|_, app| shutdown(app, id));
}

#[gpui::test]
fn everything_a_plugin_does_in_a_call_is_undone_by_one_undo(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "元");
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    fs::write(
        &script,
        "Editor.GoFileEnd(); Editor.InsText('a'); Editor.InsText('b'); Editor.InsText('c');",
    )
    .unwrap();
    enable_sample(cx, &place, true, true);
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    wait_for(&view, cx, "正常に終了しました");
    assert_eq!(text_of(&view, cx), "元abc");
    let undo = super::super::function_code("F_UNDO").unwrap();
    view.update_in(cx, |v, window, cx| v.run_function(undo, &[], window, cx));
    assert_eq!(
        text_of(&view, cx),
        "元",
        "one undo takes back the whole call"
    );
    set_test_interpreter(None);
    cx.update(|_, app| shutdown(app, "io.github.stofu5678.matsu-editor.sample.hello-js"));
}

#[gpui::test]
fn a_plugin_that_asks_without_end_leaves_the_window_its_turn(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "");
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    // Hundreds of requests and lines of output in a row.
    fs::write(
        &script,
        "for (let i = 0; i < 500; i++) { Editor.GetLineCount(0); console.log(i); } Editor.InsText('done');",
    )
    .unwrap();
    enable_sample(cx, &place, true, true);
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    wait_for(&view, cx, "正常に終了しました");
    assert_eq!(text_of(&view, cx), "done");
    set_test_interpreter(None);
    cx.update(|_, app| shutdown(app, "io.github.stofu5678.matsu-editor.sample.hello-js"));
}
