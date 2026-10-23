//! `F_OPEN_COMMAND_PROMPT_AS_ADMIN`: opens a command prompt as administrator in the folder of the
//! file.
//!
//! Sakura: `Command_OPEN_COMMAND_PROMPT(TRUE)` (`sakura_core/cmd/CViewCommander_File.cpp`), the same
//! with the `runas` verb of `ShellExecuteEx`. Only Windows can elevate: elsewhere the status bar
//! says it is not available (`matsu_platform::shell`).

use crate::file_ops::external::open_shell_here;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_platform::shell::Shell;

#[derive(Default)]
pub struct OpenCommandPromptAsAdmin;

impl UiCommand for OpenCommandPromptAsAdmin {
    const ID: &'static str = "open_command_prompt_as_admin";
    const SAKURA: &'static str = "F_OPEN_COMMAND_PROMPT_AS_ADMIN";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        open_shell_here(view, Shell::CommandPrompt, true, cx);
        Ok(())
    }
}

register_ui_command!(OpenCommandPromptAsAdmin);
