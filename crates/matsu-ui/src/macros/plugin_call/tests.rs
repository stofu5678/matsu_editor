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

#[gpui::test]
fn a_plugin_that_was_replaced_by_another_folder_gets_a_host_of_that_folder(
    cx: &mut TestAppContext,
) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let first = Place::new();
    let second = Place::new();
    let (_view, cx) = setup(cx, "");
    enable_sample(cx, &first, true, true);
    let one = cx
        .update(|_, app| prepare_command(app, &first.dirs(), "HelloJs", 1))
        .unwrap();
    let dir_one = one.host.borrow().settings().dir.clone();
    enable_sample(cx, &second, true, true);
    let two = cx
        .update(|_, app| prepare_command(app, &second.dirs(), "HelloJs", 1))
        .unwrap();
    let dir_two = two.host.borrow().settings().dir.clone();
    assert_ne!(dir_one, dir_two);
    assert!(dir_two.starts_with(second.dir.path()), "{dir_two:?}");
    set_test_interpreter(None);
    cx.update(|_, app| shutdown(app, &two.id));
}

#[gpui::test]
fn a_second_plugin_call_waits_until_the_first_is_over(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "");
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    fs::write(&script, "while (true) {}").unwrap();
    enable_sample(cx, &place, true, true);
    let first = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let id = first.id.clone();
    let editor = view.clone();
    cx.update(|_, app| run(editor, first, app));
    assert!(view.read_with(cx, |v, _| v.macros.plugin_running));
    // Another call on the same window is refused at once, and says why.
    let second = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, second, app));
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(
        output.contains("ほかのプラグインかマクロが動いている間"),
        "{output}"
    );
    // Disabling the plugin ends the loop at once (no wait for a polite end), and frees the window.
    let started = std::time::Instant::now();
    cx.update(|_, app| shutdown(app, &id));
    assert!(
        started.elapsed() < Duration::from_millis(1000),
        "{:?}",
        started.elapsed()
    );
    wait_for(&view, cx, "中断しました");
    assert!(!view.read_with(cx, |v, _| v.macros.plugin_running));
    set_test_interpreter(None);
}

#[gpui::test]
fn a_macro_cannot_start_while_a_plugin_is_in_a_call(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "元");
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    fs::write(&script, "while (true) {}").unwrap();
    enable_sample(cx, &place, true, true);
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let id = prepared.id.clone();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    assert!(view.read_with(cx, |v, _| v.macros.busy()));
    // Neither a macro text nor a macro file starts, and the document is not touched.
    let started = view.update_in(cx, |v, window, cx| {
        v.exec_macro_text("Editor.InsText('x');", "js", window, cx)
    });
    assert!(started.is_err());
    let file = place.dir.path().join("m.js");
    fs::write(&file, "Editor.InsText('y');").unwrap();
    let started = view.update_in(cx, |v, window, cx| v.exec_ext_macro(&file, window, cx));
    assert!(started.is_err());
    assert_eq!(text_of(&view, cx), "元");
    // When the plugin has ended, a macro may start again.
    cx.update(|_, app| shutdown(app, &id));
    wait_for(&view, cx, "中断しました");
    assert!(!view.read_with(cx, |v, _| v.macros.busy()));
    set_test_interpreter(None);
}

#[gpui::test]
fn a_plugin_does_not_edit_a_document_that_replaced_the_one_it_began_on(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "");
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    fs::write(&script, "while (true) { Editor.InsText('x'); }").unwrap();
    enable_sample(cx, &place, true, true);
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    for _ in 0..600 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if text_of(&view, cx).contains('x') {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        text_of(&view, cx).contains('x'),
        "the plugin edits its document"
    );
    // The window loads another document in place, as opening a file in it does.
    let doc = Document::from_utf8("新".as_bytes().to_vec()).unwrap();
    view.update(cx, |view, cx| {
        view.load_document(SharedDocument::new(DocumentModel::new(doc), None), None, cx);
    });
    wait_for(&view, cx, "中断しました");
    assert_eq!(text_of(&view, cx), "新", "the new document was not touched");
    assert!(!view.read_with(cx, |v, _| v.macros.plugin_running));
    set_test_interpreter(None);
}

#[gpui::test]
fn two_panes_of_one_document_do_not_run_plugins_or_macros_at_once(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (first, cx) = setup(cx, "元");
    let shared = first.read_with(cx, |v, _| v.document().clone());
    let (second, cx2) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    fs::write(&script, "while (true) {}").unwrap();
    enable_sample(cx2, &place, true, true);
    let prepared = cx2
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let id = prepared.id.clone();
    let editor = first.clone();
    cx2.update(|_, app| run(editor, prepared, app));
    // The other pane shares the undo history: it is refused, for a plugin and for a macro.
    let again = cx2
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = second.clone();
    cx2.update(|_, app| run(editor, again, app));
    let output = second.read_with(cx2, |v, _| v.macro_output_text());
    assert!(
        output.contains("ほかのプラグインかマクロが動いている間"),
        "{output}"
    );
    let started = second.update_in(cx2, |v, window, cx| {
        v.exec_macro_text("Editor.InsText('x');", "js", window, cx)
    });
    assert!(started.is_err());
    cx2.update(|_, app| shutdown(app, &id));
    set_test_interpreter(None);
}

#[gpui::test]
fn closing_the_pane_that_began_a_call_frees_the_document_for_the_other_pane(
    cx: &mut TestAppContext,
) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (first, cx) = setup(cx, "元");
    let shared = first.read_with(cx, |v, _| v.document().clone());
    let (second, cx2) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    // The plugin computes without a word: only the closing of its pane can end it early.
    fs::write(&script, "while (true) {}").unwrap();
    enable_sample(cx2, &place, true, true);
    let prepared = cx2
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = second.clone();
    cx2.update(|_, app| run(editor, prepared, app));
    assert!(first.read_with(cx2, |v, cx| v.plugin_in_call(cx)));
    // The pane that began the call goes away.
    drop(second);
    cx2.update(|window, _| window.remove_window());
    for _ in 0..300 {
        cx2.executor().advance_clock(Duration::from_millis(20));
        cx2.run_until_parked();
        if !first.read_with(cx2, |v, cx| v.plugin_in_call(cx)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!first.read_with(cx2, |v, cx| v.plugin_in_call(cx)));
    set_test_interpreter(None);
}

#[gpui::test]
fn a_plugin_does_not_start_while_a_key_macro_waits_for_a_dialog(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "元");
    enable_sample(cx, &place, true, true);
    // A key macro waits for the settings dialog (the wait is on; no script session runs).
    view.update(cx, |v, _| v.macros.waiting = Some(1));
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let id = prepared.id.clone();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(
        output.contains("ほかのプラグインかマクロが動いている間"),
        "{output}"
    );
    assert_eq!(text_of(&view, cx), "元");
    assert!(!view.read_with(cx, |v, _| v.macros.plugin_running));
    // The same for a key macro that still has calls to play.
    view.update(cx, |v, _| {
        v.macros.waiting = None;
        v.macros.queue.push_back(super::super::Item::Leave);
    });
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    assert!(!view.read_with(cx, |v, _| v.macros.plugin_running));
    cx.update(|_, app| shutdown(app, &id));
    set_test_interpreter(None);
}

#[gpui::test]
fn the_end_of_a_call_does_not_free_the_busy_mark_of_a_document_that_replaced_its_own(
    cx: &mut TestAppContext,
) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "");
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    fs::write(&script, "while (true) { Editor.InsText('x'); }").unwrap();
    enable_sample(cx, &place, true, true);
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    // The window loads another document; a call of another pane begins on it (the mark is set)
    // before the first call has noticed.
    let doc = Document::from_utf8("新".as_bytes().to_vec()).unwrap();
    view.update(cx, |view, cx| {
        view.load_document(SharedDocument::new(DocumentModel::new(doc), None), None, cx);
        view.document
            .update(cx, |shared, _| shared.set_plugin_busy(true));
    });
    wait_for(&view, cx, "中断しました");
    for _ in 0..20 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
    }
    assert!(
        view.read_with(cx, |v, cx| v.document.read(cx).plugin_busy()),
        "the end of the old call freed the mark of the new document"
    );
    set_test_interpreter(None);
}

#[gpui::test]
fn a_silent_plugin_is_stopped_when_its_document_is_replaced(cx: &mut TestAppContext) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    set_test_interpreter(Some(host));
    let place = Place::new();
    let dirs = place.dirs();
    let (view, cx) = setup(cx, "");
    let script = dirs.plugin_dir("HelloJs").unwrap().join("command.js");
    // It asks the editor for nothing: only the window can notice that the document is another.
    fs::write(&script, "while (true) {}").unwrap();
    enable_sample(cx, &place, true, true);
    let prepared = cx
        .update(|_, app| prepare_command(app, &dirs, "HelloJs", 1))
        .unwrap();
    let editor = view.clone();
    cx.update(|_, app| run(editor, prepared, app));
    let doc = Document::from_utf8("新".as_bytes().to_vec()).unwrap();
    view.update(cx, |view, cx| {
        view.load_document(SharedDocument::new(DocumentModel::new(doc), None), None, cx);
    });
    wait_for(&view, cx, "中断しました");
    assert!(!view.read_with(cx, |v, _| v.macros.plugin_running));
    assert_eq!(text_of(&view, cx), "新");
    set_test_interpreter(None);
}
