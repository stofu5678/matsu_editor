//! The events that a macro's save or read raises (`Turn`, `MacroState::owns_window`): Sakura calls the plugins
//! from inside the command (`CDocFileOperation::DoSaveFlow`, `FileLoad`), so a key macro and a script get them,
//! in the order of a save of the user's, and go on after them. The plugin of these tests writes `[<jack>]`
//! when a handler runs; the text of the document shows which calls of the macro ran after the events.
//!
//! Paths: the tests write temporary files and put their names into macros (`save_as`, `open_it`), and the
//! files are `.cjs`, `.mac` and `.dat`, which no built-in type colors as URLs.

use std::path::Path;
use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_plugin::session::Language;

use super::tests::{Files, begin, finish, jacks, setup, spin, text_of};
use crate::macros::plugin_call::set_test_limit;
use crate::view::EditorView;

const SKIP: &str = "SKIP: matsu-script is not built next to the tests";

/// A path as the text of a macro file, in single quotes (`\` doubled).
fn quoted(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\\', "\\\\"))
}

/// Starts the macro file `path` and lets the window run until the macro has ended.
fn play(view: &Entity<EditorView>, cx: &mut VisualTestContext, path: &Path) {
    let code = FUNCTIONS
        .iter()
        .find(|f| f.name == "F_EXECEXTMACRO")
        .unwrap()
        .code;
    let arg = Arg::Text(path.to_string_lossy().into_owned());
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[arg], window, cx)
    });
    spin(cx, "the macro did not end", |cx| {
        !view.read_with(cx, |v, _| v.is_macro_running())
    });
    // Nothing else may follow the end of the macro.
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn text_in(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |v, app| {
        v.document()
            .read(app)
            .doc()
            .line(0)
            .ok()
            .flatten()
            .map_or_else(String::new, |line| line.text)
    })
}

const SAVE_AS_ORDER: [&str; 4] = [
    "DocumentBeforeSave",
    "DocumentClose",
    "DocumentAfterSave",
    "DocumentOpen",
];

#[gpui::test]
fn a_key_macro_that_saves_gets_the_events_and_goes_on_after_them(cx: &mut TestAppContext) {
    let Some((place, files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let target = files.path("named.dat");
    let mac = files.write(
        "save.mac",
        format!(
            "Char(97);\nFileSaveAs({});\nChar(98);\nFileSave();\n",
            quoted(&target)
        )
        .as_bytes(),
    );
    play(&view, cx, &mac);
    // `FileSaveAs` as the user's: BeforeSave, Close of the document that was, AfterSave, Open of the new name;
    // the `Char` after it ran after all four, and `FileSave` wrote what it typed, with its two events.
    let mut want = SAVE_AS_ORDER.to_vec();
    want.extend(["DocumentBeforeSave", "DocumentAfterSave"]);
    assert_eq!(jacks(&view, cx), want);
    assert_eq!(text_of(&target), "ab");
    assert_eq!(text_in(&view, cx), "ab");
    finish(cx);
}

#[gpui::test]
fn a_script_that_saves_gets_the_events_and_goes_on_after_them(cx: &mut TestAppContext) {
    if !crate::macros::tests::available(Language::JavaScript) {
        return;
    }
    let Some((place, files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let target = files.path("named.dat");
    let script = files.write(
        "save.cjs",
        format!(
            "Editor.InsText('a'); Editor.FileSaveAs({:?}); Editor.InsText('b'); Editor.FileSave();",
            target.to_string_lossy()
        )
        .as_bytes(),
    );
    play(&view, cx, &script);
    let mut want = SAVE_AS_ORDER.to_vec();
    want.extend(["DocumentBeforeSave", "DocumentAfterSave"]);
    assert_eq!(jacks(&view, cx), want);
    assert_eq!(text_of(&target), "ab");
    finish(cx);
}

#[gpui::test]
fn a_macro_that_opens_and_closes_a_file_gets_open_and_close(cx: &mut TestAppContext) {
    let Some((place, files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let source = files.write("source.dat", b"hello");
    let mac = files.write(
        "open.mac",
        format!(
            "FileOpen({});\nChar(66);\nUndo();\nFileClose();\nChar(67);\n",
            quoted(&source)
        )
        .as_bytes(),
    );
    play(&view, cx, &mac);
    // The window was empty and unmodified: the file is read into it (as the user's open does). `DocumentOpen`
    // comes before the `Char` of the macro, and `DocumentClose` of `FileClose` before the last `Char`.
    assert_eq!(jacks(&view, cx), ["DocumentOpen", "DocumentClose"]);
    assert_eq!(text_in(&view, cx), "C");
    finish(cx);
}

#[gpui::test]
fn a_plugin_that_never_answers_ends_with_its_limit_and_the_macro_goes_on(cx: &mut TestAppContext) {
    let slow = "console.log('[DocumentBeforeSave]'); var t = Date.now(); while (Date.now() - t < 30000) {}";
    let Some((place, files)) = begin(cx, &[("DocumentBeforeSave", slow)]) else {
        eprintln!("{SKIP}");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    set_test_limit(Some(Duration::from_millis(300)));
    let target = files.path("slow.dat");
    let mac = files.write(
        "slow.mac",
        format!("Char(97);\nFileSaveAs({});\nChar(98);\n", quoted(&target)).as_bytes(),
    );
    play(&view, cx, &mac);
    set_test_limit(None);
    // The handler was cut off at its limit (the panel says so), the save went on, and so did the macro.
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(output.contains("時間切れ"), "{output}");
    assert_eq!(text_of(&target), "a");
    assert_eq!(text_in(&view, cx), "ab");
    assert!(jacks(&view, cx).contains(&"DocumentBeforeSave".to_owned()));
    finish(cx);
}

#[gpui::test]
fn an_event_that_comes_while_the_macro_is_in_a_call_is_not_given(cx: &mut TestAppContext) {
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    // The macro is in a call (not waiting for a flow): it has the window, as a command of a plugin has it.
    view.update(cx, |v, _| v.pretend_macro_call_for_test(true));
    let done =
        cx.update(|_, app| super::fire(app, &view, matsu_config::plugins::def::Jack::DocumentOpen));
    spin(cx, "the event call did not end", |_| done.get());
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
    }
    assert!(jacks(&view, cx).is_empty());
    view.update(cx, |v, _| v.pretend_macro_call_for_test(false));
    finish(cx);
}

#[gpui::test]
fn an_event_of_another_pane_does_not_run_while_a_macro_waits_in_this_one(cx: &mut TestAppContext) {
    use matsu_config::plugins::def::Jack;
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let shared = view.read_with(cx, |v, _| v.document().clone());
    let second = cx.new(|cx| EditorView::new(shared, cx));
    // The macro of the first pane waits for a flow (a dialog): the second pane's events would end where
    // that macro does not look, and the panes share the undo block and the plugin call of the document.
    view.update(cx, |v, _| v.pretend_macro_waits_for_test(true));
    let done = cx.update(|_, app| super::fire(app, &second, Jack::DocumentBeforeSave));
    spin(cx, "the event call did not end", |_| done.get());
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
    }
    assert!(jacks(&second, cx).is_empty());
    assert!(jacks(&view, cx).is_empty());
    // The pane of the macro gets its own events, and the macro is told when they end.
    let done = cx.update(|_, app| super::fire(app, &view, Jack::DocumentBeforeSave));
    spin(cx, "the event of the waiting pane did not end", |_| {
        done.get()
    });
    assert_eq!(jacks(&view, cx), ["DocumentBeforeSave"]);
    view.update(cx, |v, _| v.pretend_macro_waits_for_test(false));
    finish(cx);
}

/// A macro that is cancelled while a handler of `DocumentBeforeSave` edits (before and after the cancel):
/// the handler's edits are one step of undo, the undo depth is 0 when it ends, and what the user types next
/// is a step of its own. `start` writes the macro (a key macro or a script) that saves under a name.
fn cancelled_during_an_event(
    cx: &mut TestAppContext,
    start: impl FnOnce(&Files, &Path) -> std::path::PathBuf,
) {
    let handler = "Editor.InsText('X'); var t = Date.now(); while (Date.now() - t < 1500) {} Editor.InsText('Y');";
    let Some((place, files)) = begin(cx, &[("DocumentBeforeSave", handler)]) else {
        eprintln!("{SKIP}");
        return;
    };
    let (view, cx) = setup(cx);
    place.enable(cx);
    let target = files.path("undo.dat");
    let mac = start(&files, &target);
    let code = FUNCTIONS
        .iter()
        .find(|f| f.name == "F_EXECEXTMACRO")
        .unwrap()
        .code;
    let arg = Arg::Text(mac.to_string_lossy().into_owned());
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[arg], window, cx)
    });
    spin(cx, "the handler did not edit", |cx| {
        text_in(&view, cx).contains('X')
    });
    // The user stops the macro while the handler runs.
    view.update(cx, |v, cx| v.cancel_macro(cx));
    spin(cx, "the handler did not end", |cx| {
        !view.read_with(cx, |v, _| v.plugin_call_running() || v.is_macro_running())
    });
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(text_in(&view, cx), "aXY");
    let depth = view.read_with(cx, |v, app| {
        v.document().read(app).model().history().depth()
    });
    assert_eq!(depth, 0, "no undo block is left open");
    cx.simulate_input("z");
    let undo = FUNCTIONS.iter().find(|f| f.name == "F_UNDO").unwrap().code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(undo, &[], window, cx)
    });
    assert_eq!(text_in(&view, cx), "aXY", "the user's own edit is a step");
    view.update_in(cx, |view, window, cx| {
        view.run_function(undo, &[], window, cx)
    });
    assert_eq!(text_in(&view, cx), "a", "the handler's edits are one step");
    finish(cx);
}

#[gpui::test]
fn a_key_macro_cancelled_during_an_event_leaves_the_undo_blocks_whole(cx: &mut TestAppContext) {
    cancelled_during_an_event(cx, |files, target| {
        files.write(
            "cancel.mac",
            format!("Char(97);\nFileSaveAs({});\nChar(98);\n", quoted(target)).as_bytes(),
        )
    });
}

#[gpui::test]
fn a_script_that_ends_during_an_event_leaves_the_undo_blocks_whole(cx: &mut TestAppContext) {
    if !crate::macros::tests::available(Language::JavaScript) {
        return;
    }
    cancelled_during_an_event(cx, |files, target| {
        files.write(
            "cancel.cjs",
            format!(
                "Editor.InsText('a'); Editor.FileSaveAs({:?}); Editor.InsText('b');",
                target.to_string_lossy()
            )
            .as_bytes(),
        )
    });
}
