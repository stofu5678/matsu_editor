//! Tests of the event jacks on a window, with the real `matsu-script` (skipped when that program is not
//! built next to the tests). A plugin with one handler for each jack writes its name when it runs, so the
//! output panel shows which jacks ran and in which order.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::Profile;
use matsu_config::plugins::consent::{ConsentFile, fingerprint};
use matsu_config::plugins::def::Jack;
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginRec, PluginSettings, PluginState, SearchDirs};

use crate::document::SharedDocument;
use crate::macros::plugin_call::tests::script_host;
use crate::macros::plugin_call::{set_test_interpreter, shutdown};
use crate::settings::UserSettings;
use crate::view::EditorView;

const ID: &str = "test.events";

/// A folder with the settings file and a plugin `Events` that has the six jacks. Each handler writes
/// `[<jack>]`; `script` can replace a handler.
struct Place {
    dir: tempfile::TempDir,
}

impl Place {
    fn new(scripts: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let plugin = dir.path().join("plugins").join("Events");
        fs::create_dir_all(&plugin).unwrap();
        let jacks = [
            "EditorStart",
            "EditorEnd",
            "DocumentOpen",
            "DocumentClose",
            "DocumentBeforeSave",
            "DocumentAfterSave",
        ];
        let mut def = format!(
            "[Plugin]\nId={ID}\nName=Events\nType=wsh\n\n[Plug]\n{}\n\n[Wsh]\nUseCache=1\n",
            jacks
                .iter()
                .map(|jack| format!("{jack}={jack}.js"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        def.push('\n');
        fs::write(plugin.join("plugin.def"), def).unwrap();
        for jack in jacks {
            let script = scripts.iter().find(|(name, _)| *name == jack).map_or_else(
                || format!("console.log('[{jack}]');"),
                |(_, s)| (*s).to_owned(),
            );
            fs::write(plugin.join(format!("{jack}.js")), script).unwrap();
        }
        Self { dir }
    }

    fn settings(&self) -> PathBuf {
        self.dir.path().join("matsu.ini")
    }

    fn dirs(&self) -> SearchDirs {
        SearchDirs::new(&self.settings(), None)
    }

    /// The user agrees to the files as they are now.
    fn agree(&self) {
        let dirs = self.dirs();
        let def = dirs.load_def("Events", DEFAULT_LANGUAGE).unwrap();
        let files = fingerprint(&dirs.plugin_dir("Events").unwrap()).unwrap();
        let mut consent = ConsentFile::default();
        consent
            .enable(&def.id, "Events", &files, "2026-11-02")
            .unwrap();
        consent.save(&dirs.consent_file()).unwrap();
    }

    /// The user turned plugins on and agreed to this one.
    fn enable(&self, cx: &mut VisualTestContext) {
        let dirs = self.dirs();
        let def = dirs.load_def("Events", DEFAULT_LANGUAGE).unwrap();
        assert_eq!(def.plugs.len(), 6);
        let mut settings = PluginSettings {
            enabled: true,
            ..PluginSettings::default()
        };
        settings.table[0] = PluginRec {
            id: def.id.clone(),
            name: "Events".into(),
            cmd_num: 0,
            state: PluginState::Stopped,
        };
        let mut profile = Profile::new();
        settings.store(&mut profile).unwrap();
        profile.save(&self.settings()).unwrap();
        self.agree();
        let user = UserSettings::from_profile(profile, Some(self.settings()));
        cx.update(|_, app| app.set_global(user));
    }
}

struct Files(PathBuf);

impl Files {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap().keep();
        Self(dir)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn setup(cx: &mut TestAppContext) -> (Entity<EditorView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    let shared = cx.new(|_| SharedDocument::empty());
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.executor().allow_parking();
    (view, vcx)
}

fn run(view: &Entity<EditorView>, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, args, window, cx)
    });
    cx.run_until_parked();
}

/// The jacks that have run, in order, from the lines `[<jack>]` in the output panel.
fn jacks(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> Vec<String> {
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix('[')?.strip_suffix(']').map(str::to_owned)
        })
        .collect()
}

/// Lets the window and the plugin process run until `count` jacks have run.
fn wait_for_jacks(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
    count: usize,
) -> Vec<String> {
    for _ in 0..1000 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        let ran = jacks(view, cx);
        if ran.len() >= count {
            // A little more time: nothing else may follow.
            for _ in 0..10 {
                cx.executor().advance_clock(Duration::from_millis(20));
                cx.run_until_parked();
                std::thread::sleep(Duration::from_millis(5));
            }
            return jacks(view, cx);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "{count} jacks did not run: {:?}\n{}",
        jacks(view, cx),
        view.read_with(cx, |v, _| v.macro_output_text())
    );
}

fn text_of(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

fn begin(cx: &mut TestAppContext, scripts: &[(&str, &str)]) -> Option<(Place, Files)> {
    let host = script_host()?;
    set_test_interpreter(Some(host));
    let _ = cx;
    Some((Place::new(scripts), Files::new()))
}

fn finish(cx: &mut VisualTestContext) {
    set_test_interpreter(None);
    cx.update(|_, app| shutdown(app, ID));
}

#[gpui::test]
fn a_save_under_a_name_calls_the_jacks_in_the_order_of_sakura(cx: &mut TestAppContext) {
    let Some((place, files)) = begin(cx, &[]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    cx.simulate_input("a");
    let target = files.path("named.txt");
    run(
        &view,
        cx,
        "F_FILESAVEAS",
        &[Arg::Text(target.display().to_string())],
    );
    // `DoSaveFlow`: BeforeSave, then Close of the document that was, the write, AfterSave; `FileSaveAs`:
    // then Open of the new name.
    let ran = wait_for_jacks(&view, cx, 4);
    assert_eq!(
        ran,
        [
            "DocumentBeforeSave",
            "DocumentClose",
            "DocumentAfterSave",
            "DocumentOpen"
        ]
    );
    assert_eq!(text_of(&target), "a");

    // A plain save over the same file: only the two jacks around the write.
    cx.simulate_input("b");
    run(&view, cx, "F_FILESAVE", &[]);
    let ran = wait_for_jacks(&view, cx, 6);
    assert_eq!(&ran[4..], ["DocumentBeforeSave", "DocumentAfterSave"]);
    assert_eq!(ran.len(), 6, "{ran:?}");
    assert_eq!(text_of(&target), "ab");
    finish(cx);
}

#[gpui::test]
fn the_plugin_of_a_before_save_jack_can_edit_what_is_written(cx: &mut TestAppContext) {
    let Some((place, files)) = begin(
        cx,
        &[(
            "DocumentBeforeSave",
            "console.log('[DocumentBeforeSave]'); Editor.AddTail('!');",
        )],
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    cx.simulate_input("a");
    let target = files.path("edited.txt");
    run(
        &view,
        cx,
        "F_FILESAVEAS",
        &[Arg::Text(target.display().to_string())],
    );
    wait_for_jacks(&view, cx, 4);
    assert_eq!(text_of(&target), "a!");
    finish(cx);
}

#[gpui::test]
fn a_before_save_jack_that_fails_does_not_stop_the_save(cx: &mut TestAppContext) {
    let Some((place, files)) = begin(
        cx,
        &[("DocumentBeforeSave", "throw new Error('壊れています');")],
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    cx.simulate_input("data");
    let target = files.path("kept.txt");
    run(
        &view,
        cx,
        "F_FILESAVEAS",
        &[Arg::Text(target.display().to_string())],
    );
    // The jacks after the failing one still run, and the file is written.
    let ran = wait_for_jacks(&view, cx, 3);
    assert_eq!(
        ran,
        ["DocumentClose", "DocumentAfterSave", "DocumentOpen"],
        "the failing jack writes no name"
    );
    assert_eq!(text_of(&target), "data");
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(output.contains("壊れています"), "{output}");
    assert!(output.contains("DocumentBeforeSave"), "{output}");
    // The document is saved: the title has no mark of changes.
    assert!(!view.read_with(cx, |v, app| v.document().read(app).model().is_modified()));
    finish(cx);
}

#[gpui::test]
fn a_file_that_is_read_calls_open_and_a_reload_calls_close_then_open(cx: &mut TestAppContext) {
    let Some((place, files)) = begin(cx, &[]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let path = files.write("in.txt", b"hello");
    run(
        &view,
        cx,
        "F_FILEOPEN",
        &[Arg::Text(path.display().to_string())],
    );
    assert_eq!(wait_for_jacks(&view, cx, 1), ["DocumentOpen"]);
    run(&view, cx, "F_FILE_REOPEN", &[]);
    assert_eq!(
        wait_for_jacks(&view, cx, 3),
        ["DocumentOpen", "DocumentClose", "DocumentOpen"]
    );
    // Closing the file leaves an untitled document: Close, and no Open (`FileClose`).
    run(&view, cx, "F_FILECLOSE", &[]);
    assert_eq!(
        wait_for_jacks(&view, cx, 4),
        [
            "DocumentOpen",
            "DocumentClose",
            "DocumentOpen",
            "DocumentClose"
        ]
    );
    finish(cx);
}

#[gpui::test]
fn a_window_that_closes_calls_close_and_then_editor_end(cx: &mut TestAppContext) {
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let weak = view.downgrade();
    cx.update(|window, app| {
        window
            .spawn(app, async move |cx| {
                super::window_closing(&[weak], cx).await;
            })
            .detach();
    });
    assert_eq!(wait_for_jacks(&view, cx, 2), ["DocumentClose", "EditorEnd"]);
    finish(cx);
}

#[gpui::test]
fn a_new_window_calls_editor_start(cx: &mut TestAppContext) {
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.update(gpui_component::init);
    let user = {
        let (_, vcx) = setup(cx);
        place.enable(vcx);
        vcx.update(|_, app| app.global::<UserSettings>().clone())
    };
    cx.update(|app| app.set_global(user));
    let handle = cx
        .update(|app| {
            crate::file_ops::windows::open_editor_window(app, SharedDocument::empty(), None)
        })
        .unwrap();
    let view = cx.update(|app| {
        crate::file_ops::windows::editors(app)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, view)| view)
            .unwrap()
    });
    let mut vcx = VisualTestContext::from_window(handle, cx);
    assert_eq!(wait_for_jacks(&view, &mut vcx, 1), ["EditorStart"]);
    finish(&mut vcx);
}

#[gpui::test]
fn plugins_that_are_off_or_not_agreed_to_get_no_event(cx: &mut TestAppContext) {
    let Some((place, files)) = begin(cx, &[]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    // The agreement is gone: nothing runs.
    let _ = fs::remove_file(place.dirs().consent_file());
    let path = files.write("in.txt", b"hello");
    run(
        &view,
        cx,
        "F_FILEOPEN",
        &[Arg::Text(path.display().to_string())],
    );
    for _ in 0..50 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(jacks(&view, cx).is_empty());
    finish(cx);
}

#[gpui::test]
fn typing_calls_no_plugin(cx: &mut TestAppContext) {
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    cx.simulate_input("abc");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("def");
    for _ in 0..50 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(jacks(&view, cx).is_empty(), "{:?}", jacks(&view, cx));
    finish(cx);
}

#[gpui::test]
fn two_windows_that_fire_at_once_both_get_the_event_though_the_host_is_shared(
    cx: &mut TestAppContext,
) {
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (first, cx) = setup(cx);
    place.enable(cx);
    let shared = cx.new(|_| SharedDocument::empty());
    let second = cx.new(|cx| EditorView::new(shared, cx));
    // One host serves the plugin for both: the second call waits for the first and is not dropped.
    cx.update(|_, app| {
        super::fire(app, &first, Jack::DocumentOpen);
        super::fire(app, &second, Jack::DocumentOpen);
    });
    assert_eq!(wait_for_jacks(&first, cx, 1), ["DocumentOpen"]);
    assert_eq!(wait_for_jacks(&second, cx, 1), ["DocumentOpen"]);
    finish(cx);
}

const BUSY: &str = "var t = Date.now(); while (Date.now() - t < 700) {}";

/// Lets the window and the plugin process run until `done` says yes.
fn spin(
    cx: &mut VisualTestContext,
    what: &str,
    mut done: impl FnMut(&mut VisualTestContext) -> bool,
) {
    for _ in 0..1000 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("{what}");
}

fn running(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> bool {
    view.read_with(cx, |v, _| v.plugin_call_running())
}

/// Two windows share the host; the first holds it with a slow handler, the second waits for it.
/// `change` happens while the second waits.
fn second_waits(cx: &mut TestAppContext, change: impl FnOnce(&Place, &mut VisualTestContext)) {
    let Some((place, _files)) = begin(
        cx,
        &[(
            "EditorStart",
            &format!("console.log('[EditorStart]'); {BUSY}"),
        )],
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (first, cx) = setup(cx);
    place.enable(cx);
    let shared = cx.new(|_| SharedDocument::empty());
    let second = cx.new(|cx| EditorView::new(shared, cx));
    cx.update(|_, app| {
        super::fire(app, &first, Jack::EditorStart);
        super::fire(app, &second, Jack::DocumentOpen);
    });
    spin(cx, "the first handler did not start", |cx| {
        running(&first, cx)
    });
    change(&place, cx);
    // The first ends; the second would begin now.
    spin(cx, "the first handler did not end", |cx| {
        !running(&first, cx)
    });
    for _ in 0..60 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(jacks(&first, cx), ["EditorStart"]);
    assert!(jacks(&second, cx).is_empty(), "{:?}", jacks(&second, cx));
    finish(cx);
}

#[gpui::test]
fn a_call_that_waited_does_not_run_when_the_plugin_was_switched_off_meanwhile(
    cx: &mut TestAppContext,
) {
    second_waits(cx, |place, cx| {
        let mut profile = Profile::load(&place.settings()).unwrap();
        let mut settings = PluginSettings::from_profile(&profile);
        settings.enabled = false;
        settings.store(&mut profile).unwrap();
        profile.save(&place.settings()).unwrap();
        let user = UserSettings::from_profile(profile, Some(place.settings()));
        cx.update(|_, app| app.set_global(user));
    });
}

#[gpui::test]
fn a_call_that_waited_does_not_run_when_a_file_of_the_plugin_changed_meanwhile(
    cx: &mut TestAppContext,
) {
    second_waits(cx, |place, _| {
        let handler = place
            .dirs()
            .plugin_dir("Events")
            .unwrap()
            .join("DocumentOpen.js");
        fs::write(handler, "console.log('[DocumentOpen]'); // changed").unwrap();
    });
}

#[gpui::test]
fn a_handler_that_prints_shows_the_panel_and_a_failure_keeps_what_was_printed(
    cx: &mut TestAppContext,
) {
    let Some((place, files)) = begin(
        cx,
        &[(
            "DocumentClose",
            "console.log('[DocumentClose]'); throw new Error('boom');",
        )],
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    view.update(cx, |v, _| v.close_output_panel_for_test());
    let path = files.write("in.txt", b"hello");
    run(
        &view,
        cx,
        "F_FILEOPEN",
        &[Arg::Text(path.display().to_string())],
    );
    wait_for_jacks(&view, cx, 1);
    assert!(view.read_with(cx, |v, _| v.output_panel_is_open_for_test()));
    view.update(cx, |v, _| v.close_output_panel_for_test());
    run(&view, cx, "F_FILE_REOPEN", &[]);
    spin(cx, "no failure", |cx| {
        view.read_with(cx, |v, _| v.macro_output_text())
            .contains("boom")
    });
    let text = view.read_with(cx, |v, _| v.macro_output_text());
    // The earlier output stays beside the failure, and the panel is shown.
    assert!(text.matches("[DocumentOpen]").count() >= 1, "{text}");
    assert!(text.contains("[DocumentClose]"), "{text}");
    assert!(view.read_with(cx, |v, _| v.output_panel_is_open_for_test()));
    finish(cx);
}

/// A file is open, and the handler of `DocumentClose` is `script`; closes the file.
fn close_with(
    cx: &mut TestAppContext,
    script: &str,
    during: impl FnOnce(&mut VisualTestContext),
) -> Option<(bool, Option<String>)> {
    let Some((place, files)) = begin(cx, &[("DocumentClose", script)]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return None;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let path = files.write("in.txt", b"hello");
    run(
        &view,
        cx,
        "F_FILEOPEN",
        &[Arg::Text(path.display().to_string())],
    );
    spin(cx, "not open", |cx| {
        view.read_with(cx, |v, app| v.document().read(app).path().is_some())
    });
    run(&view, cx, "F_FILECLOSE", &[]);
    spin(cx, "the handler did not start", |cx| running(&view, cx));
    during(cx);
    spin(cx, "the handler did not end", |cx| !running(&view, cx));
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
    }
    let closed = view.read_with(cx, |v, app| v.document().read(app).path().is_none());
    let notice = view.read_with(cx, |v, _| v.notice.clone());
    finish(cx);
    Some((closed, notice))
}

#[gpui::test]
fn what_only_the_plugin_wrote_while_a_close_waits_does_not_stop_the_close(cx: &mut TestAppContext) {
    let Some((closed, _)) = close_with(cx, &format!("Editor.AddTail('x'); {BUSY}"), |_| {}) else {
        return;
    };
    assert!(closed);
}

#[gpui::test]
fn text_the_user_typed_while_a_close_waits_stops_the_close(cx: &mut TestAppContext) {
    let Some((closed, notice)) = close_with(cx, BUSY, |cx| cx.simulate_input("z")) else {
        return;
    };
    assert!(!closed);
    assert!(notice.unwrap_or_default().contains("閉じるのをやめた"));
}

/// `window_closing` on a window whose `DocumentClose` handler is `script`; `during` runs while it waits.
fn closing_window(
    cx: &mut TestAppContext,
    script: &str,
    during: impl FnOnce(&mut VisualTestContext),
) -> Option<bool> {
    let Some((place, _files)) = begin(cx, &[("DocumentClose", script)]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return None;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let result = std::rc::Rc::new(std::cell::Cell::new(None));
    let weak = view.downgrade();
    let slot = result.clone();
    cx.update(|window, app| {
        window
            .spawn(app, async move |cx| {
                let edited = super::window_closing(&[weak], cx).await;
                slot.set(Some(edited));
            })
            .detach();
    });
    spin(cx, "the handler did not start", |cx| running(&view, cx));
    during(cx);
    spin(cx, "the closing did not end", |_| result.get().is_some());
    finish(cx);
    result.get()
}

#[gpui::test]
fn the_exit_is_not_restarted_by_what_a_close_handler_wrote(cx: &mut TestAppContext) {
    let Some(edited) = closing_window(cx, &format!("Editor.AddTail('x'); {BUSY}"), |_| {}) else {
        return;
    };
    assert!(!edited);
}

#[gpui::test]
fn the_exit_is_restarted_by_what_the_user_typed_while_close_handlers_ran(cx: &mut TestAppContext) {
    let Some(edited) = closing_window(cx, BUSY, |cx| cx.simulate_input("z")) else {
        return;
    };
    assert!(edited);
}

#[gpui::test]
fn a_call_that_waited_does_not_run_when_the_plugin_no_longer_declares_the_jack(
    cx: &mut TestAppContext,
) {
    second_waits(cx, |place, _| {
        // The plugin is changed so that it does not use DocumentOpen, and the user agrees again; the
        // handler file stays in the folder.
        let def = place
            .dirs()
            .plugin_dir("Events")
            .unwrap()
            .join("plugin.def");
        let text = fs::read_to_string(&def).unwrap();
        let text: String = text
            .lines()
            .filter(|line| !line.starts_with("DocumentOpen="))
            .map(|line| format!("{line}\n"))
            .collect();
        fs::write(&def, text).unwrap();
        place.agree();
    });
}

#[gpui::test]
fn a_handler_that_opens_a_file_is_refused_at_once_and_is_not_made_to_wait(cx: &mut TestAppContext) {
    // The calls of a plugin go to the commands of the core: the functions that need the window (open,
    // close, save) are refused, so a handler cannot start a flow whose events would wait for the handler.
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("opened.txt");
    fs::write(&target, "hello").unwrap();
    let path = target.display().to_string().replace('\\', "\\\\");
    let script = format!(
        "try {{ Editor.FileOpen('{path}'); console.log('opened'); }} catch (e) {{ console.log('refused'); }} \
         console.log('[EditorStart]');"
    );
    let Some((place, _files)) = begin(cx, &[("EditorStart", &script)]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let started = std::time::Instant::now();
    cx.update(|_, app| {
        super::fire(app, &view, Jack::EditorStart);
    });
    spin(cx, "the handler did not start", |cx| running(&view, cx));
    spin(cx, "the handler did not end", |cx| !running(&view, cx));
    // The handler ended by itself, long before its limit of five seconds.
    assert!(started.elapsed() < Duration::from_secs(3));
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
    }
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(!output.contains("時間切れ"), "{output}");
    assert!(
        output.contains("refused") || output.contains("opened"),
        "{output}"
    );
    assert_eq!(jacks(&view, cx), ["EditorStart"], "{output}");
    assert!(
        view.read_with(cx, |v, app| v.document().read(app).path().is_none())
            || output.contains("opened")
    );
    finish(cx);
}

#[gpui::test]
fn a_window_that_begins_with_a_file_reads_it_before_editor_start(cx: &mut TestAppContext) {
    // `CNormalProcess`: the file is loaded, then EditorStart, then DocumentOpen.
    let script = "console.log('[EditorStart]'); console.log('file=' + Editor.GetFilename());";
    let Some((place, files)) = begin(cx, &[("EditorStart", script)]) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.update(gpui_component::init);
    let user = {
        let (_, vcx) = setup(cx);
        place.enable(vcx);
        vcx.update(|_, app| app.global::<UserSettings>().clone())
    };
    cx.update(|app| app.set_global(user));
    let path = files.write("in.txt", b"hello");
    let request = crate::file_ops::windows::LoadRequest {
        path: path.clone(),
        encoding: None,
    };
    let handle = cx
        .update(|app| {
            crate::file_ops::windows::open_editor_window(
                app,
                SharedDocument::empty(),
                Some(request),
            )
        })
        .unwrap();
    let view = cx.update(|app| {
        crate::file_ops::windows::editors(app)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, view)| view)
            .unwrap()
    });
    let mut vcx = VisualTestContext::from_window(handle, cx);
    assert_eq!(
        wait_for_jacks(&view, &mut vcx, 2),
        ["EditorStart", "DocumentOpen"]
    );
    let output = view.read_with(&vcx, |v, _| v.macro_output_text());
    assert!(output.contains("in.txt"), "{output}");
    finish(&mut vcx);
}
