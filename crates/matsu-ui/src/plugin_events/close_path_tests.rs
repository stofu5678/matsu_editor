//! The events of the ways a document or a window goes away, and of a window that begins with a Grep, on
//! windows of GPUI's test platform with the real `matsu-script` (skipped when that program is not built next
//! to the tests). Every wait is for the end of a flow, never for a number of turns.
//!
//! The output panel of a view shows which jacks ran for it (each handler writes `[<jack>]`), so a view that is
//! still held after its window went tells what its plugins were told.

use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_commands::sakura::FUNCTIONS;

use super::tests::{BUSY, ID, Place, begin};
use crate::document::SharedDocument;
use crate::file_ops::close::quits;
use crate::file_ops::windows::{editors, open_editor_window, workspace_of};
use crate::macros::plugin_call::{set_test_interpreter, shutdown};
use crate::tabs::settings::{TabOptions, TabSettings};
use crate::view::EditorView;

type Window = (AnyWindowHandle, Entity<EditorView>);

const SKIP: &str = "SKIP: matsu-script is not built next to the tests";

fn finish(cx: &mut TestAppContext) {
    set_test_interpreter(None);
    cx.update(|app| shutdown(app, ID));
}

/// Lets the windows and the plugin process run until `done` says yes.
fn wait(cx: &mut TestAppContext, what: &str, mut done: impl FnMut(&mut TestAppContext) -> bool) {
    for _ in 0..3000 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    // What the windows were told, for the one who reads the failure.
    let views = cx.update(editors);
    for (handle, view) in views {
        eprintln!(
            "{handle:?}: {:?}\n{}",
            told(&view, cx),
            view.read_with(cx, |v, _| v.macro_output_text())
        );
    }
    panic!("{what}");
}

/// What the plugins were told for `view`, in order.
fn told(view: &Entity<EditorView>, cx: &mut TestAppContext) -> Vec<String> {
    view.read_with(cx, |v, _| v.macro_output_text())
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix('[')?.strip_suffix(']').map(str::to_owned)
        })
        .collect()
}

fn running(view: &Entity<EditorView>, cx: &mut TestAppContext) -> bool {
    view.read_with(cx, |v, _| v.plugin_call_running())
}

fn window_is_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update(|app| app.windows().contains(&handle))
}

fn view_of(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    cx.update(|app| {
        editors(app)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, view)| view)
            .unwrap()
    })
}

/// `count` untitled windows. The plugins are turned on afterwards, so that the windows have no event yet.
fn windows(cx: &mut TestAppContext, place: &Place, count: usize) -> Vec<Window> {
    cx.update(gpui_component::init);
    let made: Vec<Window> = (0..count)
        .map(|_| {
            let handle = cx
                .update(|app| open_editor_window(app, SharedDocument::empty(), None))
                .unwrap();
            cx.run_until_parked();
            (handle, view_of(cx, handle))
        })
        .collect();
    place.enable(VisualTestContext::from_window(made[0].0, cx).into_mut());
    made
}

/// The tab mode of Sakura, where the last tab closes the window (`m_bTab_RetainEmptyWin` off).
fn tab_mode(cx: &mut TestAppContext) {
    cx.update(|app| {
        app.set_global(TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        });
        app.set_global(TabOptions {
            retain_empty: false,
            ..Default::default()
        });
    });
}

fn run(cx: &mut TestAppContext, (handle, view): &Window, function: &str) {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

/// The × of the tab of `view`.
fn close_button(cx: &mut TestAppContext, (handle, view): &Window) {
    let (handle, view) = (*handle, view.clone());
    handle
        .update(cx, |_, window, app| {
            window
                .spawn(app, async move |cx| {
                    crate::tabs::bar::close_document(cx, handle, &view).await;
                })
                .detach();
        })
        .unwrap();
}

fn tab_views(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<Entity<EditorView>> {
    let ws = cx.update(|app| workspace_of(app, handle)).unwrap();
    ws.read_with(cx, |ws, _| ws.tab_editors())
}

const TOLD_CLOSE_AND_END: [&str; 2] = ["DocumentClose", "EditorEnd"];

#[gpui::test]
fn the_button_of_the_last_tab_tells_close_and_then_editor_end(cx: &mut TestAppContext) {
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    let wins = windows(cx, &place, 1);
    close_button(cx, &wins[0]);
    let (handle, view) = wins[0].clone();
    wait(cx, "the window did not close", |cx| {
        !window_is_open(cx, handle)
    });
    assert_eq!(told(&view, cx), TOLD_CLOSE_AND_END);
    finish(cx);
}

#[gpui::test]
fn the_button_of_a_tab_in_a_window_with_more_tabs_tells_close_only(cx: &mut TestAppContext) {
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    tab_mode(cx);
    let wins = windows(cx, &place, 1);
    run(cx, &wins[0], "F_FILENEW");
    let (handle, first) = wins[0].clone();
    let views = tab_views(cx, handle);
    assert_eq!(views.len(), 2);
    // The new document is told `DocumentOpen`; that is over before the button is pressed.
    wait(cx, "the new tab was not told it opened", |cx| {
        told(&views[1], cx) == ["DocumentOpen"] && !running(&views[1], cx)
    });
    close_button(cx, &(handle, first.clone()));
    wait(cx, "the tab did not close", |cx| {
        tab_views(cx, handle).len() == 1
    });
    assert_eq!(told(&first, cx), ["DocumentClose"]);
    assert_eq!(
        told(&views[1], cx),
        ["DocumentOpen"],
        "the other tab hears nothing more"
    );
    assert!(window_is_open(cx, handle));
    finish(cx);
}

#[gpui::test]
fn a_tag_jump_that_closes_the_tab_it_came_from_tells_close_and_then_editor_end(
    cx: &mut TestAppContext,
) {
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.txt");
    std::fs::write(&target, "one\ntwo\n").unwrap();
    let out = dir.path().join("output.txt");
    std::fs::write(&out, format!("{}(2,1): two\n", target.display())).unwrap();
    cx.update(gpui_component::init);
    let handle = cx
        .update(|app| {
            open_editor_window(
                app,
                SharedDocument::empty(),
                Some(crate::file_ops::windows::LoadRequest {
                    path: out.clone(),
                    encoding: None,
                }),
            )
        })
        .unwrap();
    wait(cx, "the file was not read", |cx| {
        let view = view_of(cx, handle);
        view.read_with(cx, |v, app| v.document().read(app).path().is_some())
    });
    let view = view_of(cx, handle);
    place.enable(VisualTestContext::from_window(handle, cx).into_mut());
    run(cx, &(handle, view.clone()), "F_TAGJUMP_CLOSE");
    wait(cx, "the window did not close", |cx| {
        !window_is_open(cx, handle)
    });
    // The window had its `DocumentOpen` before the plugins were turned on; now it is told the close.
    assert_eq!(told(&view, cx), TOLD_CLOSE_AND_END);
    finish(cx);
}

#[gpui::test]
fn quitting_tells_close_and_then_editor_end_for_each_window(cx: &mut TestAppContext) {
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    let wins = windows(cx, &place, 2);
    cx.update(crate::file_ops::close::quit);
    wait(cx, "the application did not quit", |_| quits() > 0);
    for (_, view) in &wins {
        assert_eq!(told(view, cx), TOLD_CLOSE_AND_END);
    }
    finish(cx);
}

#[gpui::test]
fn a_second_quit_while_the_first_waits_for_a_plugin_does_not_tell_the_close_again(
    cx: &mut TestAppContext,
) {
    let script = format!("console.log('[DocumentClose]'); {BUSY}");
    let Some((place, _files)) = begin(cx, &[("DocumentClose", &script)]) else {
        eprintln!("{SKIP}");
        return;
    };
    let wins = windows(cx, &place, 1);
    let (_, view) = wins[0].clone();
    cx.update(crate::file_ops::close::quit);
    wait(cx, "the handler did not start", |cx| running(&view, cx));
    // The key is pressed again, twice, while the handler runs: a refused ask must not let the first go.
    cx.update(crate::file_ops::close::quit);
    cx.update(crate::file_ops::close::quit);
    wait(cx, "the application did not quit", |_| quits() > 0);
    assert_eq!(quits(), 1);
    assert_eq!(told(&view, cx), TOLD_CLOSE_AND_END);
    finish(cx);
}

/// A window, `DocumentClose` slow; an exit begins, the user types while the handler runs, and the question
/// that comes back is answered with `answer`.
fn exit_with_typing(cx: &mut TestAppContext, answer: &str) -> Option<(Place, Window)> {
    let script = format!("console.log('[DocumentClose]'); {BUSY}");
    let Some((place, _files)) = begin(cx, &[("DocumentClose", &script)]) else {
        eprintln!("{SKIP}");
        return None;
    };
    let wins = windows(cx, &place, 1);
    let window = wins[0].clone();
    let (handle, view) = window.clone();
    run(cx, &window, "F_EXITALL");
    wait(cx, "the handler did not start", |cx| running(&view, cx));
    VisualTestContext::from_window(handle, cx)
        .into_mut()
        .simulate_input("z");
    wait(cx, "the question did not come back", |cx| {
        VisualTestContext::from_window(handle, cx)
            .into_mut()
            .has_pending_prompt()
    });
    VisualTestContext::from_window(handle, cx)
        .into_mut()
        .simulate_prompt_answer(answer);
    // The folder of the plugin stays as long as the caller keeps `place`.
    Some((place, window))
}

#[gpui::test]
fn a_round_that_starts_over_tells_the_close_once(cx: &mut TestAppContext) {
    let Some((_place, (_, view))) = exit_with_typing(cx, "保存しない(N)") else {
        return;
    };
    wait(cx, "the application did not quit", |_| quits() > 0);
    assert_eq!(told(&view, cx), TOLD_CLOSE_AND_END);
    finish(cx);
}

#[gpui::test]
fn an_exit_that_is_given_up_after_editor_end_tells_editor_start_again(cx: &mut TestAppContext) {
    let Some((_place, (handle, view))) = exit_with_typing(cx, "キャンセル") else {
        return;
    };
    wait(
        cx,
        "the plugins were not told that the editor goes on",
        |cx| told(&view, cx).len() >= 3 && !running(&view, cx),
    );
    assert_eq!(
        told(&view, cx),
        ["DocumentClose", "EditorEnd", "EditorStart"]
    );
    assert!(window_is_open(cx, handle));
    assert_eq!(quits(), 0);
    finish(cx);
}

/// A window of the tab mode with one tab whose `EditorEnd` handler is slow; `close` asks the window to close.
/// While the handler runs, a tab opens in the window.
fn tab_opens_while_editor_end_runs(
    cx: &mut TestAppContext,
    close: impl FnOnce(&mut TestAppContext, &Window),
) -> Option<(Place, Window)> {
    let script = format!("console.log('[EditorEnd]'); {BUSY}");
    let Some((place, _files)) = begin(cx, &[("EditorEnd", &script)]) else {
        eprintln!("{SKIP}");
        return None;
    };
    tab_mode(cx);
    let wins = windows(cx, &place, 1);
    let window = wins[0].clone();
    close(cx, &window);
    let (handle, view) = window.clone();
    wait(cx, "EditorEnd did not start", |cx| {
        told(&view, cx).contains(&"EditorEnd".to_owned()) && running(&view, cx)
    });
    handle
        .update(cx, |_, window, app| {
            crate::tabs::flows::open_document(
                app,
                Some(window),
                handle,
                false,
                SharedDocument::empty(),
                None,
            )
            .unwrap();
        })
        .unwrap();
    cx.run_until_parked();
    Some((place, window))
}

#[gpui::test]
fn a_tab_that_opens_while_editor_end_runs_is_told_editor_start_after_the_close_command(
    cx: &mut TestAppContext,
) {
    let Some((_place, (handle, first))) =
        tab_opens_while_editor_end_runs(cx, |cx, window| run(cx, window, "F_WINCLOSE"))
    else {
        return;
    };
    // The first tab closes, the window goes on with the new tab, and its plugins are told it starts again.
    wait(cx, "the first tab did not close", |cx| {
        let tabs = tab_views(cx, handle);
        tabs.len() == 1 && tabs[0] != first
    });
    let rest = tab_views(cx, handle).remove(0);
    wait(cx, "EditorStart did not come", |cx| {
        told(&rest, cx).contains(&"EditorStart".to_owned())
    });
    assert_eq!(told(&first, cx), TOLD_CLOSE_AND_END);
    assert_eq!(told(&rest, cx), ["EditorStart"]);
    assert!(window_is_open(cx, handle));
    finish(cx);
}

#[gpui::test]
fn a_tab_that_opens_while_editor_end_runs_is_told_editor_start_after_the_close_button(
    cx: &mut TestAppContext,
) {
    let Some((_place, (handle, first))) = tab_opens_while_editor_end_runs(cx, |cx, (handle, _)| {
        let vcx = VisualTestContext::from_window(*handle, cx).into_mut();
        assert!(!vcx.simulate_close(), "the window waits for the plugins");
    }) else {
        return;
    };
    // The window stays with both tabs; the tab that shows (the new one) is told that the editor starts again.
    let shown = tab_views(cx, handle).remove(1);
    wait(cx, "EditorStart did not come", |cx| {
        told(&shown, cx).contains(&"EditorStart".to_owned()) && !running(&shown, cx)
    });
    assert_eq!(told(&first, cx), TOLD_CLOSE_AND_END);
    assert_eq!(told(&shown, cx), ["EditorStart"]);
    assert_eq!(tab_views(cx, handle).len(), 2);
    assert!(window_is_open(cx, handle));
    finish(cx);
}

#[gpui::test]
fn a_grep_launch_tells_editor_start_and_document_open_after_the_search(cx: &mut TestAppContext) {
    use matsu_platform::instance::Request;
    // The handler of EditorStart says how many lines the window has when it runs.
    let script = "console.log('[EditorStart]'); console.log('lines=' + Editor.GetLineCount(0));";
    let Some((place, _files)) = begin(cx, &[("EditorStart", script)]) else {
        eprintln!("{SKIP}");
        return;
    };
    cx.update(gpui_component::init);
    cx.executor().allow_parking();
    // A window of the test platform turns the plugins on; the launch opens its own window.
    let user = {
        let (_, vcx) = super::tests::setup(cx);
        place.enable(vcx);
        vcx.update(|_, app| app.global::<crate::settings::UserSettings>().clone())
    };
    cx.update(|app| app.set_global(user));
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), "hello world\nneedle here\n").unwrap();
    std::fs::write(src.join("b.txt"), "needle again\n").unwrap();
    let request = crate::launch::OpenRequest::from_launch(&Request {
        cwd: dir.path().to_owned(),
        args: [
            "-GREPMODE",
            "-GKEY=needle",
            "-GFILE=*.txt",
            "-GFOLDER=src",
            "-GOPT=S",
        ]
        .iter()
        .map(std::ffi::OsString::from)
        .collect(),
    });
    let options = crate::file_ops::start_options::LaunchOptions::from_request(&request);
    cx.update(|app| crate::file_ops::windows::open_from_outside_with(app, request.paths, &options))
        .unwrap();
    let view = cx.update(|app| editors(app).pop().map(|(_, v)| v)).unwrap();
    wait(cx, "the events did not come", |cx| {
        told(&view, cx).len() >= 2
    });
    assert_eq!(told(&view, cx), ["EditorStart", "DocumentOpen"]);
    // EditorStart saw the finished result: the lines of the whole search, not the empty window.
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    let seen: usize = output
        .lines()
        .find_map(|line| line.trim().strip_prefix("lines=")?.parse().ok())
        .expect("the handler printed the number of lines");
    let all = view.read_with(cx, |v, app| v.document().read(app).doc().line_count());
    assert!(seen >= 4, "{output}");
    assert!(seen + 1 >= all, "seen {seen} of {all}\n{output}");
    finish(cx);
}

#[gpui::test]
fn a_grep_to_the_standard_output_ends_after_the_events_of_the_start(cx: &mut TestAppContext) {
    use matsu_platform::instance::Request;
    let Some((place, _files)) = begin(cx, &[]) else {
        eprintln!("{SKIP}");
        return;
    };
    cx.update(gpui_component::init);
    cx.executor().allow_parking();
    let user = {
        let (_, vcx) = super::tests::setup(cx);
        place.enable(vcx);
        vcx.update(|_, app| app.global::<crate::settings::UserSettings>().clone())
    };
    cx.update(|app| app.set_global(user));
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "needle\n").unwrap();
    let request = crate::launch::OpenRequest::from_launch(&Request {
        cwd: dir.path().to_owned(),
        args: [
            "-GREPMODE",
            "-GKEY=needle",
            "-GFILE=*.txt",
            "-GFOLDER=.",
            "-GOPT=SU",
        ]
        .iter()
        .map(std::ffi::OsString::from)
        .collect(),
    });
    let options = crate::file_ops::start_options::LaunchOptions::from_request(&request);
    let before = quits();
    cx.update(|app| crate::file_ops::windows::open_from_outside_with(app, request.paths, &options))
        .unwrap();
    let view = cx.update(|app| editors(app).pop().map(|(_, v)| v)).unwrap();
    wait(cx, "the process did not end", |_| quits() > before);
    // The plugins were told before the process ended, not skipped by it: the events of the start, and
    // the close that Sakura sends with `MYWM_CLOSE`.
    assert_eq!(
        told(&view, cx),
        ["EditorStart", "DocumentOpen", "DocumentClose", "EditorEnd"]
    );
    finish(cx);
}

/// A launch with `-GREPMODE -GREPDLG` (the request only). The temporary folder lives as long as the test
/// needs it: the search of an OK test is over before `wait` returns.
fn request_grep_dialog(cx: &mut TestAppContext) {
    use matsu_platform::instance::Request;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), "needle here\n").unwrap();
    let request = crate::launch::OpenRequest::from_launch(&Request {
        cwd: dir.path().to_owned(),
        args: [
            "-GREPMODE",
            "-GREPDLG",
            "-GKEY=needle",
            "-GFILE=*.txt",
            "-GFOLDER=src",
            "-GOPT=S",
        ]
        .iter()
        .map(std::ffi::OsString::from)
        .collect(),
    });
    let options = crate::file_ops::start_options::LaunchOptions::from_request(&request);
    cx.update(|app| crate::file_ops::windows::open_from_outside_with(app, request.paths, &options))
        .unwrap();
    std::mem::forget(dir);
}

/// A launch with `-GREPMODE -GREPDLG`: the window and its dialog. The handlers write `[<jack>]`.
fn grep_dialog_launch(
    cx: &mut TestAppContext,
    place: &Place,
) -> (AnyWindowHandle, Entity<EditorView>) {
    cx.update(gpui_component::init);
    cx.executor().allow_parking();
    let user = {
        let (_, vcx) = super::tests::setup(cx);
        place.enable(vcx);
        vcx.update(|_, app| app.global::<crate::settings::UserSettings>().clone())
    };
    cx.update(|app| app.set_global(user));
    request_grep_dialog(cx);
    cx.update(|app| editors(app).pop()).unwrap()
}

const START_AND_OPEN: &[(&str, &str)] = &[
    ("EditorStart", "console.log('[EditorStart]');"),
    ("DocumentOpen", "console.log('[DocumentOpen]');"),
];

#[gpui::test]
fn a_grep_dialog_launch_tells_the_events_only_after_the_dialog_is_gone(cx: &mut TestAppContext) {
    let Some((place, _files)) = begin(cx, START_AND_OPEN) else {
        eprintln!("{SKIP}");
        return;
    };
    let (handle, view) = grep_dialog_launch(cx, &place);
    // The dialog is up and nothing has been told: Sakura waits in `DoModal` before the plugins.
    for _ in 0..50 {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(view.read_with(cx, |v, _| v.grep_panel_open()));
    assert!(told(&view, cx).is_empty(), "{:?}", told(&view, cx));
    // Cancel: the plugins are told all the same (`CNormalProcess` calls them after `DoModal` either way).
    let panel = view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.cancel(window, cx))
        })
        .unwrap();
    wait(cx, "the events did not come after the cancel", |cx| {
        told(&view, cx).len() >= 2
    });
    assert_eq!(told(&view, cx), ["EditorStart", "DocumentOpen"]);
    finish(cx);
}

#[gpui::test]
fn a_grep_dialog_launch_closed_with_ok_tells_the_grep_then_the_start(cx: &mut TestAppContext) {
    let Some((place, _files)) = begin(cx, START_AND_OPEN) else {
        eprintln!("{SKIP}");
        return;
    };
    let (handle, view) = grep_dialog_launch(cx, &place);
    let panel = view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.ok(window, cx))
        })
        .unwrap();
    wait(cx, "the events did not come after the search", |cx| {
        told(&view, cx).len() >= 3
    });
    // `Command_GREP` (`DoGrep`, `DocumentOpen`), then `CNormalProcess` (`EditorStart`, `DocumentOpen`).
    assert_eq!(
        told(&view, cx),
        ["DocumentOpen", "EditorStart", "DocumentOpen"]
    );
    finish(cx);
}

/// Bound tabs, one window that was there before the plugins: the launch makes a tab in it, which has no
/// `EditorStart` of its own. Sakura's sequence for the document stays: the `DocumentOpen` of `Command_GREP`
/// (OK only), then the one of `CNormalProcess` (in Sakura the launch is a process of its own, which also tells
/// `EditorStart`; here the window has had it).
fn grep_dialog_launch_in_a_tab(cx: &mut TestAppContext) -> Option<(Window, Place)> {
    let (place, _files) = begin(cx, START_AND_OPEN)?;
    tab_mode(cx);
    let first = windows(cx, &place, 1).remove(0);
    cx.executor().allow_parking();
    request_grep_dialog(cx);
    let tabs = tab_views(cx, first.0);
    assert_eq!(tabs.len(), 2, "the launch made a tab");
    let shown = tabs
        .into_iter()
        .find(|tab| tab.read_with(cx, |v, _| v.grep_panel_open()))
        .expect("a tab has the dialog");
    Some(((first.0, shown), place))
}

#[gpui::test]
fn a_grep_dialog_launch_in_a_tab_closed_with_ok_tells_two_document_opens(cx: &mut TestAppContext) {
    let Some(((handle, view), _place)) = grep_dialog_launch_in_a_tab(cx) else {
        eprintln!("{SKIP}");
        return;
    };
    let panel = view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.ok(window, cx))
        })
        .unwrap();
    wait(cx, "the events did not come", |cx| {
        told(&view, cx).len() >= 2
    });
    // Time to show that no third one comes.
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(told(&view, cx), ["DocumentOpen", "DocumentOpen"]);
    finish(cx);
}

#[gpui::test]
fn a_grep_dialog_launch_in_a_tab_cancelled_tells_one_document_open(cx: &mut TestAppContext) {
    let Some(((handle, view), _place)) = grep_dialog_launch_in_a_tab(cx) else {
        eprintln!("{SKIP}");
        return;
    };
    let panel = view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.cancel(window, cx))
        })
        .unwrap();
    wait(cx, "the event did not come", |cx| {
        !told(&view, cx).is_empty()
    });
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(told(&view, cx), ["DocumentOpen"]);
    finish(cx);
}

/// A Grep that a macro or a plugin command starts is its own action: MATSU gives no event to what a macro or a
/// plugin does (`Turn::Taken`, `plugin_events`; `CJackManager::InvokePlugins` of Sakura has no guard, the rule
/// is MATSU's). The state is looked at when the Grep starts: the call may be over when the search ends, and a
/// handler of `DocumentOpen` that ran a Grep would otherwise call itself again and again.
#[gpui::test]
fn a_grep_that_a_call_starts_tells_no_document_open_even_when_the_call_is_over_first(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "needle second\n").unwrap();
    let folder = dir.path().to_string_lossy().replace('\\', "/");
    let Some((place, _files)) = begin(cx, START_AND_OPEN) else {
        eprintln!("{SKIP}");
        return;
    };
    let (handle, view) = windows(cx, &place, 1).remove(0);
    cx.executor().allow_parking();
    let args = crate::command_line::parse(&[
        "-GKEY=needle".into(),
        "-GFILE=*.txt".into(),
        format!("-GFOLDER={folder}").into(),
        "-GOPT=S".into(),
    ])
    .grep;
    let info = crate::file_ops::start_options::grep_info(&args, dir.path(), None);
    // The call has the window when the Grep starts, and is over before the search ends.
    view.update(cx, |view, _| view.pretend_plugin_call(true));
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| {
                view.start_grep(info.clone(), false, window, cx)
            })
        })
        .unwrap();
    view.update(cx, |view, _| view.pretend_plugin_call(false));
    wait(cx, "the search did not end", |cx| {
        view.read_with(cx, |v, _| v.grep_key().is_some() && !v.grep_busy())
    });
    for _ in 0..50 {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(told(&view, cx).is_empty(), "{:?}", told(&view, cx));
    // The same Grep of a user, with nothing else on the window, tells it once.
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.start_grep(info, false, window, cx))
        })
        .unwrap();
    wait(cx, "the event of the user's grep did not come", |cx| {
        !told(&view, cx).is_empty()
    });
    assert_eq!(told(&view, cx), ["DocumentOpen"]);
    finish(cx);
}

/// The conditions of a Grep of the files of `dir`, as a command (not the launch) gives them.
fn grep_of(dir: &std::path::Path) -> crate::grep::engine::GrepInfo {
    let folder = dir.to_string_lossy().replace('\\', "/");
    let args = crate::command_line::parse(&[
        "-GKEY=needle".into(),
        "-GFILE=*.txt".into(),
        format!("-GFOLDER={folder}").into(),
        "-GOPT=S".into(),
    ])
    .grep;
    crate::file_ops::start_options::grep_info(&args, dir, None)
}

/// The macro of タイプ変更後 that waits for a dialog (`TypeList`), and a type for `grepout`, added to the
/// settings that turned the plugins on.
fn type_changed_macro_waits(cx: &mut TestAppContext, place: &Place) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("t.mac"), "TypeList();\r\n").unwrap();
    let user = cx.update(|app| app.global::<crate::settings::UserSettings>().clone());
    let mut profile = user.profile().unwrap().clone();
    let folder = dir.path().to_string_lossy().into_owned();
    for (section, key, value) in [
        ("Folders", "szMACROFOLDER", folder.as_str()),
        ("Macro", "Name[001]", ""),
        ("Macro", "File[001]", "t.mac"),
        ("Macro", "nMacroOnTypeChanged", "1"),
        ("Other", "nTypesCount", "18"),
        ("Types(17)", "szTypeName", "Grep"),
        ("Types(17)", "szTypeExts", "grepout"),
    ] {
        profile.set(section, key, value).unwrap();
    }
    let user = crate::settings::UserSettings::from_profile(profile, Some(place.settings()));
    cx.update(|app| app.set_global(user));
    dir
}

/// The user's Grep into an empty window, where the macro of タイプ変更後 goes on beside the window: the
/// search begins when the macro has ended. The window is taken by the macro then, but the Grep is the user's:
/// `DocumentOpen` is decided when the Grep starts and told after the search.
#[gpui::test]
fn a_users_grep_that_waits_for_the_macro_of_the_type_tells_document_open_after_the_search(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "needle here\n").unwrap();
    let Some((place, _files)) = begin(cx, START_AND_OPEN) else {
        eprintln!("{SKIP}");
        return;
    };
    let (handle, view) = windows(cx, &place, 1).remove(0);
    let _macros = type_changed_macro_waits(cx, &place);
    cx.executor().allow_parking();
    let info = grep_of(dir.path());
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.start_grep(info, false, window, cx))
        })
        .unwrap();
    for _ in 0..10 {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
    }
    assert!(
        view.read_with(cx, |v, _| v.doc_type.list.is_some()),
        "the macro waits for the list of the types"
    );
    assert!(told(&view, cx).is_empty(), "nothing before the search");
    // The macro ends (the list is closed); the search runs; then the event.
    view.update(cx, |v, cx| {
        v.doc_type.list = None;
        cx.notify();
    });
    wait(cx, "the event did not come after the search", |cx| {
        !told(&view, cx).is_empty()
    });
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(told(&view, cx), ["DocumentOpen"]);
    finish(cx);
}

/// The dialog of `-GREPDLG` is cancelled and, before the launch looks again (it polls), a Grep of a key starts
/// in the window: that Grep tells its own `DocumentOpen`, and the launch does not tell one for a Grep of the
/// dialog that never started. Three in all (the Grep's, `EditorStart`, the launch's `DocumentOpen`), not four.
#[gpui::test]
fn a_grep_started_right_after_the_cancel_of_the_dialog_is_not_counted_for_the_dialog(
    cx: &mut TestAppContext,
) {
    let Some((place, _files)) = begin(cx, START_AND_OPEN) else {
        eprintln!("{SKIP}");
        return;
    };
    let (handle, view) = grep_dialog_launch(cx, &place);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "needle here\n").unwrap();
    let info = grep_of(dir.path());
    let panel = view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.cancel(window, cx));
            // No turn of the clock between: the launch has not looked yet.
            view.update(app, |view, cx| view.start_grep(info, false, window, cx))
        })
        .unwrap();
    wait(cx, "the events did not come", |cx| {
        told(&view, cx).len() >= 3
    });
    for _ in 0..30 {
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    let told = told(&view, cx);
    assert_eq!(told.len(), 3, "{told:?}");
    assert_eq!(told.iter().filter(|e| *e == "DocumentOpen").count(), 2);
    assert_eq!(told.iter().filter(|e| *e == "EditorStart").count(), 1);
    finish(cx);
}

/// The dialog of `-GREPDLG` is cancelled and another Grep dialog opens at once (before the launch looks again):
/// the launch does not wait for that one; its events come while it is still open.
#[gpui::test]
fn another_grep_dialog_opened_right_after_the_cancel_does_not_hold_back_the_start_events(
    cx: &mut TestAppContext,
) {
    let Some((place, _files)) = begin(cx, START_AND_OPEN) else {
        eprintln!("{SKIP}");
        return;
    };
    let (handle, view) = grep_dialog_launch(cx, &place);
    let panel = view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.cancel(window, cx));
            // No turn of the clock between: the launch has not looked yet.
            view.update(app, |view, cx| view.open_grep_panel(window, cx));
        })
        .unwrap();
    assert!(view.read_with(cx, |v, _| v.grep_panel_open()));
    wait(cx, "the start events waited for the other dialog", |cx| {
        told(&view, cx).len() >= 2
    });
    assert_eq!(told(&view, cx), ["EditorStart", "DocumentOpen"]);
    assert!(
        view.read_with(cx, |v, _| v.grep_panel_open()),
        "the other dialog is still open"
    );
    finish(cx);
}
