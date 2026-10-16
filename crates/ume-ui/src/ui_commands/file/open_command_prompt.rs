//! `F_OPEN_COMMAND_PROMPT`: opens a command prompt in the folder of the file.
//!
//! Sakura: `Command_OPEN_COMMAND_PROMPT(FALSE)` (`sakura_core/cmd/CViewCommander_File.cpp`) starts
//! `%COMSPEC%` with `ShellExecuteEx` in the folder of the file. A document without a file and a
//! folder on the network (UNC) beep and do nothing. On macOS and Linux the default terminal opens
//! in the folder (`ume_platform::shell`).

use crate::file_ops::external::open_shell_here;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_platform::shell::Shell;

#[derive(Default)]
pub struct OpenCommandPrompt;

impl UiCommand for OpenCommandPrompt {
    const ID: &'static str = "open_command_prompt";
    const SAKURA: &'static str = "F_OPEN_COMMAND_PROMPT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        open_shell_here(view, Shell::CommandPrompt, false, cx);
        Ok(())
    }
}

register_ui_command!(OpenCommandPrompt);
