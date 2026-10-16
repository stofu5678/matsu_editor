//! `F_OPEN_POWERSHELL_AS_ADMIN`: opens PowerShell as administrator in the folder of the file.
//!
//! Sakura: `Command_OPEN_POWERSHELL(TRUE)` (`sakura_core/cmd/CViewCommander_File.cpp`), the same
//! with the `runas` verb of `ShellExecute`. Only Windows can elevate: elsewhere the status bar
//! says it is not available (`ume_platform::shell`).

use crate::file_ops::external::open_shell_here;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_platform::shell::Shell;

#[derive(Default)]
pub struct OpenPowershellAsAdmin;

impl UiCommand for OpenPowershellAsAdmin {
    const ID: &'static str = "open_powershell_as_admin";
    const SAKURA: &'static str = "F_OPEN_POWERSHELL_AS_ADMIN";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        open_shell_here(view, Shell::PowerShell, true, cx);
        Ok(())
    }
}

register_ui_command!(OpenPowershellAsAdmin);
