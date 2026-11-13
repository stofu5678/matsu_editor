//! The settings of the shell tab (`[Terminal]`, ADR-0028 point 11) in a window: a tab that starts takes the
//! shell, the way its command line is handed over, and the length of the scrollback; a tab that is open keeps
//! its own.
//!
//! The shells are real, as in `terminal_tests`: a PTY (ConPTY on Windows) with a small program of the test's
//! own, written as the command line of the setting.

use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_config::terminal::TerminalSettings;

use super::terminal_tests::{screen, terminal_window, wait_for_text};
use super::tests::workspace;
use crate::terminal::TerminalView;

fn configure(cx: &mut TestAppContext, shell: &str, scrollback_lines: i64) {
    cx.update(|app| {
        crate::terminal::settings::set(
            app,
            TerminalSettings::from_values(shell, scrollback_lines, false),
        );
    });
}

/// Opens a shell tab the way `F_SHELL_TAB` does, with what the settings say.
fn open_configured(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<TerminalView> {
    let ws = workspace(cx, handle);
    let view = handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.open_shell_from_settings(window, cx))
        })
        .unwrap()
        .expect("the shell starts");
    cx.run_until_parked();
    view
}

fn close_all(cx: &mut TestAppContext, views: &[Entity<TerminalView>]) {
    for view in views {
        view.update(cx, |view, _| view.shutdown());
    }
    assert!(matsu_terminal::wait_for_closing(Duration::from_secs(60)));
}

/// The shell of the setting is started with its command line as it is written: the two spaces in the quotes
/// reach the program (on Windows the quotes and the `&` reach cmd.exe, which reads them by its own rules).
#[gpui::test]
fn a_new_tab_starts_the_shell_of_the_setting_with_its_command_line_as_written(
    cx: &mut TestAppContext,
) {
    let handle = terminal_window(cx);
    let line = if cfg!(windows) {
        r#"cmd.exe /Q /K echo "two  spaces" & echo configured"#
    } else {
        r#"/bin/sh -c 'printf "two  spaces\nconfigured\n"; exec cat'"#
    };
    configure(cx, line, 5000);
    let view = open_configured(cx, handle);
    wait_for_text(cx, &view, "configured");
    let text = screen(cx, &view);
    assert!(text.contains("two  spaces"), "the screen was:\n{text}");
    // (A backslash can be in the prompt of cmd.exe: only the escaped quote would be the C runtime's.)
    assert!(
        !text.contains("\\\""),
        "the quotes must not be escaped:\n{text}"
    );
    close_all(cx, &[view]);
}

/// The scrollback of the setting is the one the tab starts with, and a tab that is open keeps its own when the
/// setting changes. The output is longer than any screen the test window can give (a taller tab takes lines
/// back from the scrollback, so a short output would leave it empty). (Unix: ConPTY repaints the screen and puts lines into the scrollback by itself.)
#[cfg(unix)]
#[gpui::test]
fn a_new_tab_takes_the_scrollback_of_the_setting_and_an_open_tab_keeps_its_own(
    cx: &mut TestAppContext,
) {
    let handle = terminal_window(cx);
    let line = "/bin/sh -c 'i=0; while [ $i -lt 1000 ]; do echo line$i; i=$((i+1)); done; echo done-printing; exec cat'";
    configure(cx, line, 7);
    let first = open_configured(cx, handle);
    wait_for_text(cx, &first, "done-printing");
    assert_eq!(first.read_with(cx, |view, _| view.history_size()), 7);

    configure(cx, line, 12);
    let second = open_configured(cx, handle);
    wait_for_text(cx, &second, "done-printing");
    assert_eq!(second.read_with(cx, |view, _| view.history_size()), 12);
    assert_eq!(
        first.read_with(cx, |view, _| view.history_size()),
        7,
        "the tab that was open keeps its scrollback"
    );
    close_all(cx, &[first, second]);
}
