//! The shell tab (ADR-0028, `crate::terminal`) in a window: opening it, the keys, the size, the end of the
//! shell, and the questions before a running shell is closed.
//!
//! The shells are real: a PTY (ConPTY on Windows) with a small program of the test's own, never the user's
//! shell. The tests wait for what the program prints, with a time limit and no fixed sleeps. On Windows a
//! shell is closed only after it has printed something: a ConPTY closed while its client is still starting
//! can leave the client running (`matsu-terminal`, PR 1).

use std::time::{Duration, Instant};

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_terminal::ShellCommand;

use super::content::{CloseCheck, MenuKind, TabContent};
use super::icon::TabIcon;
use super::tests::{active_tab, answer, run, tab_count, visual, window, workspace};
use crate::app::Workspace;
use crate::terminal::TerminalView;

const YES: &str = "はい(Y)";
const NO: &str = "いいえ(N)";

/// A window for a test with a real PTY. The events of the shell come from a thread of their own and wake
/// tasks of the window from there, which GPUI's test scheduler refuses unless parking is allowed.
fn terminal_window(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.executor().allow_parking();
    window(cx, true)
}

/// A shell that prints `hello` and then stays, echoing what is typed (`cat`; on Windows, cmd.exe itself).
fn staying() -> ShellCommand {
    if cfg!(windows) {
        ShellCommand::new("cmd.exe", ["/Q", "/K", "echo hello"])
    } else {
        ShellCommand::new("/bin/sh", ["-c", "printf 'hello\\n'; exec cat"])
    }
}

/// A shell that prints `bye` and ends with `code`.
fn ending(code: i32) -> ShellCommand {
    if cfg!(windows) {
        // The output right before the end can be lost by ConPTY: the shell waits a moment (about a second,
        // `ping -n 2`) after it printed, and the tests look at the screen before they wait for the end.
        ShellCommand::new(
            "cmd.exe",
            [
                "/C".to_owned(),
                format!("echo bye & ping -n 2 127.0.0.1 >nul & exit {code}"),
            ],
        )
    } else {
        ShellCommand::new(
            "/bin/sh",
            ["-c".to_owned(), format!("echo bye; exit {code}")],
        )
    }
}

fn open(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    shell: ShellCommand,
) -> Entity<TerminalView> {
    let ws = workspace(cx, handle);
    let view = handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.open_shell_with(shell, window, cx))
        })
        .unwrap()
        .expect("the shell starts");
    cx.run_until_parked();
    view
}

/// Runs the events until `done` is true, or fails after a while.
fn wait_until(
    cx: &mut TestAppContext,
    what: &str,
    mut done: impl FnMut(&mut TestAppContext) -> bool,
) {
    cx.executor().allow_parking();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn screen(cx: &mut TestAppContext, view: &Entity<TerminalView>) -> String {
    view.read_with(cx, |view, _| view.screen_text())
}

fn wait_for_text(cx: &mut TestAppContext, view: &Entity<TerminalView>, text: &str) {
    let view = view.clone();
    wait_until(cx, &format!("{text:?} on the screen"), move |cx| {
        screen(cx, &view).contains(text)
    });
}

fn wait_for_end(cx: &mut TestAppContext, view: &Entity<TerminalView>) {
    let view = view.clone();
    wait_until(cx, "the shell to end", move |cx| {
        view.read_with(cx, |view, _| view.ended().is_some())
    });
}

fn running(cx: &mut TestAppContext, view: &Entity<TerminalView>) -> bool {
    view.read_with(cx, |view, _| view.is_running())
}

fn close_tab(cx: &mut TestAppContext, handle: AnyWindowHandle, index: usize) {
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.close_own_tab(index, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn has_prompt(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    visual(cx, handle).has_pending_prompt()
}

#[gpui::test]
fn opening_a_shell_shows_its_output_in_a_tab_with_the_contract_of_a_tab(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, cx| {
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.active_tab(), 1, "the new tab is shown");
        let panel = ws.tab_panel(1).unwrap();
        assert_eq!(panel.menu_kind(), MenuKind::Terminal);
        let caption = panel.caption(cx);
        assert!(!caption.is_empty() && !caption.contains('/'), "{caption}");
        assert!(panel.tooltip(cx).contains(&caption));
        assert_eq!(panel.icon(cx), TabIcon::Terminal);
        assert!(panel.accessible_name(cx).contains(&caption));
        // A running shell asks before it is closed.
        assert_eq!(panel.close_check(cx), CloseCheck::RunningProcess);
        let status = panel.status(cx).expect("the shell has a status bar");
        assert!(status.message_only);
        assert!(status.message().contains('×'), "{}", status.message());
        // Not one of the window's editors.
        assert_eq!(ws.tab_editors().len(), 1);
        assert_eq!(ws.all_views().len(), 1);
        assert!(ws.tab_editor(1).is_none());
        assert!(ws.tab_content(1).and_then(TabContent::terminal).is_some());
    });
}

#[gpui::test]
fn the_function_opens_a_shell_next_to_the_document(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    // The function starts a shell of the test's own (`use_shell_in_tests`), as the function always does.
    super::terminal_tab::use_shell_in_tests(staying());
    run(cx, handle, "F_SHELL_TAB");
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 2);
        assert!(ws.tab_content(1).and_then(TabContent::terminal).is_some());
    });
    let views = ws.read_with(cx, |ws, _| ws.shell_views());
    wait_for_text(cx, &views[0], "hello");
    for view in views {
        view.update(cx, |view, _| view.shutdown());
    }
    assert!(matsu_terminal::wait_for_closing(Duration::from_secs(60)));
}

#[gpui::test]
fn typed_keys_reach_the_program(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let shell = if cfg!(windows) {
        ShellCommand::new("cmd.exe", ["/Q", "/K", "echo hello"])
    } else {
        ShellCommand::new("/bin/sh", ["-c", "read word; echo got-$word; exec cat"])
    };
    let view = open(cx, handle, shell);
    if cfg!(windows) {
        wait_for_text(cx, &view, "hello");
    }
    // The terminal has the focus: the keys go to it, as text and as keys.
    let ws = workspace(cx, handle);
    let focused = handle
        .update(cx, |_, window, app| {
            ws.read(app).active_focus(app).is_focused(window)
        })
        .unwrap();
    assert!(focused);
    let typed = visual(cx, handle);
    if cfg!(windows) {
        typed.simulate_input("echo xyz");
    } else {
        typed.simulate_input("abc");
    }
    typed.simulate_keystrokes("enter");
    cx.run_until_parked();
    let expected = if cfg!(windows) { "xyz" } else { "got-abc" };
    let v = view.clone();
    wait_until(cx, "the program to answer", move |cx| {
        // a line of its own: the echo of the typed text is another line
        screen(cx, &v).lines().any(|line| line == expected)
    });
}

#[gpui::test]
fn the_keys_of_the_window_stay_with_the_window(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    let ws = workspace(cx, handle);
    assert_eq!(active_tab(cx, handle), 1);
    // Ctrl+Tab is Sakura's next window: the terminal does not take it.
    visual(cx, handle).simulate_keystrokes("ctrl-tab");
    cx.run_until_parked();
    assert_eq!(ws.read_with(cx, |ws, _| ws.active_tab()), 0);
}

#[test]
fn the_window_functions_of_the_terminal_are_the_window_group_without_the_splits_and_redraw() {
    use crate::terminal::is_window_function;
    let code = |name: &str| {
        matsu_commands::sakura::FUNCTIONS
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.code)
            .unwrap()
    };
    for name in [
        "F_NEXTWINDOW",
        "F_PREVWINDOW",
        "F_NEXTGROUP",
        "F_PREVGROUP",
        "F_WINCLOSE",
        "F_GROUPCLOSE",
        "F_TAB_MOVELEFT",
        "F_WINLIST",
        "F_SHELL_TAB",
        "F_PREVIEW_TAB",
    ] {
        assert!(is_window_function(code(name)), "{name}");
    }
    for name in [
        "F_SPLIT_V",
        "F_SPLIT_H",
        "F_SPLIT_VH",
        "F_FILESAVE",
        "F_UNDO",
    ] {
        assert!(!is_window_function(code(name)), "{name}");
    }
}

/// The default keys that stay with the window while a terminal has the focus (listed in the pull request): the
/// index is the place in Sakura's table of modifiers (2 Ctrl, 3 Ctrl+Shift, 6 Ctrl+Alt, 7 Ctrl+Alt+Shift).
#[test]
fn the_default_keys_that_stay_with_the_window_are_these() {
    use crate::terminal::is_window_function;
    let mut taken: Vec<(&str, usize)> = Vec::new();
    for binding in matsu_commands::sakura::KEY_BINDINGS {
        for (modifiers, &function) in binding.functions.iter().enumerate() {
            if function != 0 && is_window_function(function) {
                taken.push((binding.key_name, modifiers));
            }
        }
    }
    taken.sort_unstable();
    let mut expected = vec![
        ("Tab", 2),
        ("Tab", 3),
        ("E", 6),
        ("H", 6),
        ("T", 6),
        ("↑", 7),
        ("→", 7),
        ("↓", 7),
    ];
    expected.sort_unstable();
    assert_eq!(taken, expected);
}

#[gpui::test]
fn a_change_of_the_size_of_the_tab_resizes_the_terminal(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    let grid = |cx: &mut TestAppContext| {
        view.read_with(cx, |view, _| {
            let size = view.size();
            // the terminal and the view agree
            assert_eq!(
                view.grid(),
                (usize::from(size.columns), usize::from(size.rows))
            );
            (size.columns, size.rows)
        })
    };
    visual(cx, handle).simulate_resize(gpui::size(gpui::px(1000.), gpui::px(700.)));
    cx.run_until_parked();
    let (wide, tall) = grid(cx);
    assert!(wide >= 2 && tall >= 1);
    visual(cx, handle).simulate_resize(gpui::size(gpui::px(500.), gpui::px(300.)));
    cx.run_until_parked();
    let (narrow, short) = grid(cx);
    assert!(narrow < wide, "{narrow} < {wide}");
    assert!(short < tall, "{short} < {tall}");
    // the status bar says so
    let text = view.read_with(cx, |view, _| view.size_text());
    assert_eq!(text, format!("{narrow}×{short}"));
}

#[gpui::test]
fn closing_a_running_shell_asks_first(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    close_tab(cx, handle, 1);
    assert!(has_prompt(cx, handle), "a running shell is asked about");
    // "No": the tab and the shell stay.
    answer(cx, handle, NO);
    assert_eq!(tab_count(cx, handle), 2);
    assert!(running(cx, &view));
    // "Yes": the tab goes, and the shell with it.
    close_tab(cx, handle, 1);
    answer(cx, handle, YES);
    assert_eq!(tab_count(cx, handle), 1);
    drop(view);
    assert!(matsu_terminal::wait_for_closing(Duration::from_secs(60)));
}

#[gpui::test]
fn the_close_button_of_the_bar_asks_about_a_running_shell(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    let visual = visual(cx, handle);
    visual.run_until_parked();
    let at = visual
        .debug_bounds("tab-bar-close")
        .expect("painted")
        .center();
    visual.simulate_mouse_down(at, gpui::MouseButton::Left, gpui::Modifiers::default());
    visual.simulate_mouse_up(at, gpui::MouseButton::Left, gpui::Modifiers::default());
    visual.run_until_parked();
    assert!(visual.has_pending_prompt());
    visual.simulate_prompt_answer(YES);
    visual.run_until_parked();
    assert_eq!(tab_count(cx, handle), 1);
}

#[gpui::test]
fn closing_a_shell_that_has_ended_asks_nothing(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, ending(3));
    wait_for_text(cx, &view, "bye");
    wait_for_end(cx, &view);
    assert_eq!(tab_count(cx, handle), 2, "a failed shell keeps its tab");
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, cx| {
        assert_eq!(ws.tab_panel(1).unwrap().close_check(cx), CloseCheck::None);
    });
    close_tab(cx, handle, 1);
    assert!(!has_prompt(cx, handle));
    assert_eq!(tab_count(cx, handle), 1);
}

#[gpui::test]
fn a_shell_that_ends_with_zero_closes_its_tab_and_one_that_fails_keeps_it(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let failing = open(cx, handle, ending(3));
    wait_for_text(cx, &failing, "bye");
    wait_for_end(cx, &failing);
    // The last screen stays, and the tab and the status bar say what happened.
    assert!(screen(cx, &failing).contains("bye"));
    assert_eq!(
        failing.read_with(cx, |v, _| v.ended().and_then(|e| e.code)),
        Some(3)
    );
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, cx| {
        assert_eq!(ws.tab_count(), 2);
        let panel = ws.tab_panel(1).unwrap();
        assert!(
            panel.caption(cx).ends_with("(終了)"),
            "{}",
            panel.caption(cx)
        );
        let message = panel.status(cx).unwrap().message();
        assert!(message.contains("終了コード 3"), "{message}");
    });
    // Keys do nothing, and nothing is shown for it.
    visual(cx, handle).simulate_input("x");
    visual(cx, handle).simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(tab_count(cx, handle), 2);

    // A shell that ends well takes its tab with it.
    let well = open(cx, handle, ending(0));
    drop(well);
    wait_until(cx, "the tab of the shell that ended well to go", |cx| {
        tab_count(cx, handle) == 2
    });
    assert_eq!(
        tab_count(cx, handle),
        2,
        "only the failed shell's tab is left besides the document"
    );
}

#[gpui::test]
fn closing_the_window_asks_once_for_all_the_shells(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let first = open(cx, handle, staying());
    let second = open(cx, handle, staying());
    wait_for_text(cx, &first, "hello");
    wait_for_text(cx, &second, "hello");
    assert!(
        !visual(cx, handle).simulate_close(),
        "the window waits for the answer"
    );
    cx.run_until_parked();
    assert!(has_prompt(cx, handle));
    answer(cx, handle, NO);
    assert!(
        !has_prompt(cx, handle),
        "one question, not one for each shell"
    );
    assert!(cx.windows().contains(&handle));
    assert!(running(cx, &first) && running(cx, &second));
    // Again, and "yes": the window goes.
    assert!(!visual(cx, handle).simulate_close());
    cx.run_until_parked();
    answer(cx, handle, YES);
    assert!(!has_prompt(cx, handle), "no second question");
    assert!(!cx.windows().contains(&handle));
    drop((first, second));
    assert!(matsu_terminal::wait_for_closing(Duration::from_secs(60)));
}

#[gpui::test]
fn closing_a_window_with_only_a_shell_that_ended_asks_nothing(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, ending(3));
    wait_for_text(cx, &view, "bye");
    wait_for_end(cx, &view);
    assert!(visual(cx, handle).simulate_close());
}

#[gpui::test]
fn exiting_the_editor_asks_once_for_all_the_shells(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let first = open(cx, handle, staying());
    let second = open(cx, handle, staying());
    wait_for_text(cx, &first, "hello");
    wait_for_text(cx, &second, "hello");
    let quits = crate::file_ops::close::quits();
    run(cx, handle, "F_EXITALL");
    cx.run_until_parked();
    assert!(has_prompt(cx, handle));
    answer(cx, handle, NO);
    assert!(!has_prompt(cx, handle));
    assert_eq!(crate::file_ops::close::quits(), quits, "the editor stays");
    assert!(running(cx, &first) && running(cx, &second));
    run(cx, handle, "F_EXITALL");
    cx.run_until_parked();
    answer(cx, handle, YES);
    assert!(!has_prompt(cx, handle), "no second question");
    assert_eq!(crate::file_ops::close::quits(), quits + 1);
    cx.update(crate::terminal::shutdown_all);
    assert!(matsu_terminal::wait_for_closing(Duration::from_secs(60)));
}

#[gpui::test]
fn closing_the_group_asks_about_the_running_shells(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    // From the tab of the document.
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.activate_tab(0, window, cx))
        })
        .unwrap();
    run(cx, handle, "F_GROUPCLOSE");
    cx.run_until_parked();
    assert!(has_prompt(cx, handle));
    answer(cx, handle, NO);
    assert!(
        cx.windows().contains(&handle),
        "the window stays with its shell"
    );
    run(cx, handle, "F_GROUPCLOSE");
    cx.run_until_parked();
    answer(cx, handle, YES);
    assert!(!cx.windows().contains(&handle));
    drop(view);
    assert!(matsu_terminal::wait_for_closing(Duration::from_secs(60)));
}

#[gpui::test]
fn ending_the_editor_waits_for_the_shells_to_close(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    // What the end of the editor does (`app.rs`, `on_app_quit`): every shell is asked to end, and the
    // process waits, with a limit, for their closing to finish.
    cx.update(crate::terminal::shutdown_all);
    assert!(
        matsu_terminal::wait_for_closing(crate::terminal::QUIT_WAIT * 60),
        "the shell closes"
    );
    assert!(!running(cx, &view));
    // The limit leaves room in the five seconds that Windows gives the end of a session, next to the three
    // that the copies of the unsaved documents wait.
    assert!(crate::terminal::QUIT_WAIT + Duration::from_secs(3) < Duration::from_secs(5));
}

#[gpui::test]
fn the_menus_of_the_window_are_the_shells_while_it_is_active(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    cx.update(|cx| {
        crate::app::set_key_map(cx, matsu_commands::keymap::KeyMap::default());
        crate::app::refresh_menus(cx);
    });
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    let ws: Entity<Workspace> = workspace(cx, handle);
    let names = |cx: &mut TestAppContext| {
        let bar = ws.read_with(cx, |ws, _| ws.menu_bar.clone());
        bar.read_with(cx, |bar, cx| bar.menu_names(cx))
            .into_iter()
            .filter(|name| name != "MATSU Editor")
            .collect::<Vec<_>>()
    };
    assert_eq!(names(cx), vec!["シェル(S)".to_owned()]);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.activate_tab(0, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    assert!(
        names(cx).len() > 1,
        "Sakura's menus are back with the document"
    );
}

#[gpui::test]
fn a_character_of_the_layout_is_typed_and_not_taken_as_a_key_of_the_window(
    cx: &mut TestAppContext,
) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    // AltGr+E gives a euro sign on some layouts; GPUI reports Ctrl+Alt+E (Sakura's F_CASCADE) with the hint
    // that the character is wanted.
    let euro = gpui::Keystroke {
        modifiers: gpui::Modifiers {
            control: true,
            alt: true,
            ..Default::default()
        },
        key: "e".to_owned(),
        key_char: Some("\u{20ac}".to_owned()),
    };
    visual(cx, handle).simulate_event(gpui::KeyDownEvent {
        keystroke: euro,
        is_held: false,
        prefer_character_input: true,
    });
    cx.run_until_parked();
    wait_for_text(cx, &view, "\u{20ac}");
}

#[gpui::test]
fn a_shell_that_took_the_place_of_a_confirmed_one_is_not_confirmed(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let first = open(cx, handle, staying());
    wait_for_text(cx, &first, "hello");
    let ws = workspace(cx, handle);
    let confirmed = ws.read_with(cx, |ws, cx| ws.running_shell_ids(cx));
    assert_eq!(confirmed, vec![first.entity_id()]);
    assert!(!ws.read_with(cx, |ws, cx| ws.has_unconfirmed_shell(cx, &confirmed)));
    // The confirmed shell ends and another one starts: the number of running shells is the same.
    first.update(cx, |view, _| view.shutdown());
    let second = open(cx, handle, staying());
    wait_for_text(cx, &second, "hello");
    ws.read_with(cx, |ws, cx| {
        assert_eq!(ws.running_shells(cx), 1);
        assert!(
            ws.has_unconfirmed_shell(cx, &confirmed),
            "the new shell was not asked about"
        );
        let both = [first.entity_id(), second.entity_id()];
        assert!(!ws.has_unconfirmed_shell(cx, &both));
    });
    second.update(cx, |view, _| view.shutdown());
    assert!(matsu_terminal::wait_for_closing(Duration::from_secs(60)));
}
