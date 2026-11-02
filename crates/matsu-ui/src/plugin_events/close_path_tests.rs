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
    // The key is pressed again while the handler runs.
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
    // The window stays with both tabs; the first one is told that the editor starts again.
    wait(cx, "EditorStart did not come", |cx| {
        told(&first, cx).len() >= 3 && !running(&first, cx)
    });
    assert_eq!(
        told(&first, cx),
        ["DocumentClose", "EditorEnd", "EditorStart"]
    );
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
