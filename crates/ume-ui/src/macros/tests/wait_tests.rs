//! Tests of the wait for a UI command that finishes later (`macros/wait.rs`): a dialog or a panel
//! that is open, a file that is read in the background, a file that goes into a new window.

use super::*;

/// The panel of the type list (`F_TYPE_LIST`) stands for any dialog: it is open until the user
/// closes it (with OK or Cancel; both set `doc_type.list` to `None`).
fn type_list_open(f: &Fixture, cx: &mut VisualTestContext) -> bool {
    f.view.read_with(cx, |v, _| v.doc_type.list.is_some())
}

fn close_type_list(f: &Fixture, cx: &mut VisualTestContext) {
    f.view.update(cx, |v, cx| {
        v.doc_type.list = None;
        cx.notify();
    });
}

fn waiting(f: &Fixture, cx: &mut VisualTestContext) -> bool {
    f.view.read_with(cx, |v, _| v.macros.waiting.is_some())
}

fn held(f: &Fixture, cx: &mut VisualTestContext) -> bool {
    f.view.read_with(cx, |v, _| {
        v.macros
            .running
            .as_ref()
            .is_some_and(|running| running.session.is_held())
    })
}

/// Lets the window and the macro's process talk for a while, and the clock reach the next look of
/// the wait. `rounds` is how long a script is given to answer.
fn settle(cx: &mut VisualTestContext, rounds: usize) {
    cx.executor().allow_parking();
    for _ in 0..rounds {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Like [`settle`], until `ready` says yes: a script's process may take seconds to start and to
/// make its first call on a slow machine.
fn settle_until(cx: &mut VisualTestContext, mut ready: impl FnMut(&mut VisualTestContext) -> bool) {
    cx.executor().allow_parking();
    for _ in 0..1500 {
        settle(cx, 1);
        if ready(cx) {
            return;
        }
    }
    panic!("the macro never got there");
}

fn setup_with_ui<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    setup(cx, text)
}

fn run_file(f: &Fixture, cx: &mut VisualTestContext, path: &Path) {
    f.run(cx, F_EXECEXTMACRO, &[text_arg(path)]);
    settle(cx, 3);
}

// ---- key macros ----

#[gpui::test]
fn a_key_macro_goes_on_after_the_dialog_is_closed(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "w.mac", "TypeList();\nChar(66);\n");
    let (f, cx) = setup_with_ui(cx, "");
    run_file(&f, cx, &path);
    assert!(type_list_open(&f, cx));
    assert!(f.running(cx) && waiting(&f, cx));
    // It does not go on by itself, however long it takes.
    settle(cx, 20);
    assert!(f.lines(cx).is_empty());
    close_type_list(&f, cx);
    settle(cx, 3);
    assert_eq!(f.lines(cx), ["B"]);
    assert!(!f.running(cx) && !waiting(&f, cx));
    // The calls are one undo step again, and a next macro can start.
    run_file(&f, cx, &path);
    assert!(type_list_open(&f, cx));
}

#[gpui::test]
fn the_cancel_button_ends_the_wait_of_a_key_macro(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "w.mac", "TypeList();\nChar(66);\n");
    let (f, cx) = setup_with_ui(cx, "");
    run_file(&f, cx, &path);
    assert!(f.running(cx));
    f.view.update(cx, |v, cx| v.cancel_macro(cx));
    assert!(!f.running(cx));
    // The dialog is the user's to close; when they do, the macro does not come back.
    close_type_list(&f, cx);
    settle(cx, 5);
    assert!(f.lines(cx).is_empty());
    // The next macro starts at once.
    let second = write(dir.path(), "x.mac", "Char(67);\n");
    run_file(&f, cx, &second);
    assert_eq!(f.lines(cx), ["C"]);
}

#[gpui::test]
fn a_call_that_fails_stops_a_key_macro_and_waits_for_nothing(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    // `Char()` without its argument fails, so the macro stops before the dialog
    let path = write(dir.path(), "w.mac", "Char();\nTypeList();\n");
    let (f, cx) = setup_with_ui(cx, "");
    run_file(&f, cx, &path);
    assert!(!f.running(cx) && !type_list_open(&f, cx));
}

#[gpui::test]
fn a_key_macro_waits_for_a_file_in_a_new_window_and_stays_in_its_window(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let opened = write(dir.path(), "opened.txt", "from file");
    let mac = write(
        dir.path(),
        "w.mac",
        &format!(
            "Char(65);\nFileOpen('{}');\nChar(66);\n",
            opened.to_string_lossy().replace('\\', "\\\\")
        ),
    );
    let (f, cx) = setup_with_ui(cx, "");
    run_file(&f, cx, &mac);
    settle(cx, 10);
    // This window is modified by the first call, so the file goes into a new window; the call
    // after it still goes to this window.
    assert_eq!(f.lines(cx), ["AB"]);
    assert!(!f.running(cx));
    let loaded = cx.update(|_, app| {
        let editors = crate::file_ops::windows::editors(app);
        assert_eq!(editors.len(), 1, "one new window");
        let (handle, view) = &editors[0];
        assert!(!crate::file_ops::windows::has_pending(app, *handle));
        let text = view.read(app).document().read(app).doc().line(0);
        text.unwrap().unwrap().text
    });
    assert_eq!(loaded, "from file");
}

// ---- scripts ----

#[gpui::test]
fn a_script_gets_its_answer_when_the_dialog_is_closed(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        dir.path(),
        "w.cjs",
        "Editor.InsText('a'); Editor.TypeList(); Editor.InsText('b'); console.log('r=end');",
    );
    let (f, cx) = setup_with_ui(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    settle_until(cx, |cx| type_list_open(&f, cx));
    settle(cx, 20);
    assert!(f.running(cx));
    assert_eq!(f.lines(cx), ["a"]);
    // The clock of the script stands still while the dialog is open (no timeout).
    assert!(held(&f, cx));
    close_type_list(&f, cx);
    f.wait_until_done_advancing(cx);
    assert_eq!(f.lines(cx), ["ab"]);
    assert_eq!(results(&f.output(cx)), ["end"], "{}", f.output(cx));
    assert!(!waiting(&f, cx));
}

#[gpui::test]
fn stopping_a_script_that_waits_leaves_nothing_behind(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        dir.path(),
        "w.cjs",
        "Editor.TypeList(); Editor.InsText('late');",
    );
    let (f, cx) = setup_with_ui(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    settle_until(cx, |cx| type_list_open(&f, cx));
    settle(cx, 5);
    assert!(f.running(cx) && waiting(&f, cx));
    f.view.update(cx, |v, cx| v.cancel_macro(cx));
    f.wait_until_done_advancing(cx);
    assert!(!waiting(&f, cx));
    // No answer after the end: closing the dialog now does nothing.
    close_type_list(&f, cx);
    settle(cx, 5);
    assert!(f.lines(cx).is_empty());
    // The next macro can start.
    let second = write(dir.path(), "x.mac", "Char(67);\n");
    run_file(&f, cx, &second);
    assert_eq!(f.lines(cx), ["C"]);
}

#[gpui::test]
fn a_script_waits_for_a_file_in_a_new_window(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let opened = write(dir.path(), "opened.txt", "from file");
    let script = write(
        dir.path(),
        "w.cjs",
        &format!(
            "Editor.InsText('A'); Editor.FileOpen({:?}); Editor.InsText('B');",
            opened.to_string_lossy()
        ),
    );
    let (f, cx) = setup_with_ui(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&script)]);
    cx.executor().allow_parking();
    for _ in 0..1500 {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        // The call after FileOpen never runs while the new window still reads its file.
        let reading = cx.update(|_, app| {
            crate::file_ops::windows::editors(app)
                .iter()
                .any(|(handle, _)| crate::file_ops::windows::has_pending(app, *handle))
        });
        if reading {
            assert!(
                !f.lines(cx)[0].contains('B'),
                "B came before the load ended"
            );
        }
        if !f.running(cx) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(f.lines(cx), ["AB"], "{}", f.output(cx));
    let loaded = cx.update(|_, app| {
        let editors = crate::file_ops::windows::editors(app);
        assert_eq!(editors.len(), 1);
        let text = editors[0].1.read(app).document().read(app).doc().line(0);
        text.unwrap().unwrap().text
    });
    assert_eq!(loaded, "from file");
}

/// `FileOpen('')` shows the file dialog and the script waits for it. `choose` answers it with a
/// file; else the dialog is cancelled. `with_folder` gives the 4th argument (the folder the dialog
/// starts in).
fn file_open_dialog(cx: &mut TestAppContext, choose: bool, with_folder: bool) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let opened = write(dir.path(), "opened.txt", "chosen");
    let call = if with_folder {
        format!(
            "Editor.FileOpen('', 99, 0, {:?})",
            dir.path().to_string_lossy()
        )
    } else {
        "Editor.FileOpen('')".to_owned()
    };
    let script = write(
        dir.path(),
        "w.cjs",
        &format!("{call}; console.log('r=after'); Editor.InsText('x');"),
    );
    let (f, cx) = setup_with_ui(cx, "");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&script)]);
    settle_until(cx, |cx| cx.did_prompt_for_paths());
    settle(cx, 10);
    assert!(cx.did_prompt_for_paths(), "the dialog is open");
    assert!(f.running(cx));
    assert!(results(&f.output(cx)).is_empty(), "the script waits");
    cx.simulate_path_prompt_response(move |_| choose.then(|| vec![opened.clone()]));
    f.wait_until_done_advancing(cx);
    // OK: the file is read into this window before the next call. Cancel: nothing opens and the
    // macro goes on all the same (`HandleCommand` returns true).
    let expected = if choose { "xchosen" } else { "x" };
    assert_eq!(f.lines(cx), [expected], "{}", f.output(cx));
    assert_eq!(results(&f.output(cx)), ["after"]);
}

#[gpui::test]
fn file_open_with_no_name_waits_for_the_file_dialog_and_the_file(cx: &mut TestAppContext) {
    file_open_dialog(cx, true, false);
}

#[gpui::test]
fn file_open_with_a_start_folder_waits_for_the_file_dialog(cx: &mut TestAppContext) {
    file_open_dialog(cx, true, true);
}

#[gpui::test]
fn file_open_with_no_name_goes_on_when_the_file_dialog_is_cancelled(cx: &mut TestAppContext) {
    file_open_dialog(cx, false, false);
}

#[gpui::test]
fn a_script_is_told_when_a_function_of_a_menu_fails(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // `Grep` with a condition that is wrong does not start
    let script = write(
        dir.path(),
        "bad.cjs",
        "try { Editor.Grep('(', '*.txt', '.', 0, 0, 0, 0, 0); console.log('r=no error'); } \
         catch (e) { console.log('r=error'); }\n",
    );
    let (f, cx) = setup_with_ui(cx, "");
    f.run_script(cx, &script, |_| {});
    let output = f.output(cx);
    assert_eq!(results(&output), ["error"], "{output}");
}

#[gpui::test]
fn a_script_cannot_give_arguments_to_a_command_that_ignores_them(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let script = write(
        dir.path(),
        "args.cjs",
        "try { Editor.ReplaceAll('a', 'b', 0); console.log('r=ran'); } \
         catch (e) { console.log('r=refused'); }\n",
    );
    let (f, cx) = setup_with_ui(cx, "aaa");
    f.run_script(cx, &script, |_| {});
    assert_eq!(results(&f.output(cx)), ["refused"], "{}", f.output(cx));
    assert_eq!(f.lines(cx), ["aaa"]);
}

#[gpui::test]
fn work_that_the_user_starts_in_the_dialog_is_waited_for_too(cx: &mut TestAppContext) {
    // The OK of a dialog often starts a job (DIFF, Grep, a file read). The macro waits for it,
    // not only for the dialog.
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "w.mac", "TypeList();\nChar(66);\n");
    let (f, cx) = setup_with_ui(cx, "");
    run_file(&f, cx, &path);
    assert!(type_list_open(&f, cx));
    f.view.update_in(cx, |v, window, cx| {
        v.doc_type.list = None;
        crate::file_ops::start(window, cx, async move |_, cx| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
        });
    });
    settle(cx, 20);
    // the dialog is closed, the job runs for 5 s of the test's clock (100 ms a round)
    assert!(f.running(cx));
    assert!(f.lines(cx).is_empty());
    settle(cx, 40);
    assert_eq!(f.lines(cx), ["B"]);
    assert!(!f.running(cx));
}

#[gpui::test]
fn a_key_macro_waits_for_the_dialog_of_the_macro_folder(cx: &mut TestAppContext) {
    // `ExecExternalMacro()` without a name asks for the file in the macro folder; the next step
    // does not run before the dialog is closed.
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "w.mac", "ExecExternalMacro();\nChar(66);\n");
    let (f, cx) = setup_with_ui(cx, "");
    run_file(&f, cx, &path);
    assert!(cx.did_prompt_for_paths(), "the dialog is open");
    assert!(f.running(cx) && waiting(&f, cx));
    settle(cx, 10);
    assert!(f.lines(cx).is_empty());
    cx.simulate_path_prompt_response(|_| None);
    settle(cx, 3);
    assert_eq!(f.lines(cx), ["B"]);
    assert!(!f.running(cx));
}

#[test]
fn a_script_can_call_a_command_of_the_menus() {
    // `TypeList` is a menu command (kind `Command`), not one of the macro-only functions.
    let row = ume_commands::macro_names::by_name("TypeList").unwrap();
    assert!(
        crate::ui_command::UiRegistry::global()
            .get_by_code(row.code)
            .is_some()
    );
}
