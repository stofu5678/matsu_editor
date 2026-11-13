//! `F_SHELL_TAB`: opens a shell in a tab (ADR-0028). MATSU Editor's own function; Sakura has nothing like it.
//! The window does the work (`tabs/terminal_tab.rs`) after the command ends, because it changes the tabs.
//!
//! Opening starts the shell and sends it nothing. No function sends text to a terminal (ADR-0028 point 2):
//! only the user's keys do.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ShellTab;

impl UiCommand for ShellTab {
    const ID: &'static str = "shell_tab";
    const SAKURA: &'static str = "F_SHELL_TAB";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.with_workspace(window, cx, |workspace, window, cx| {
            workspace.open_shell(window, cx);
        })
    }
}

register_ui_command!(ShellTab);
