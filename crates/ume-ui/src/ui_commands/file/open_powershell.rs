//! `F_OPEN_POWERSHELL`: opens PowerShell in the folder of the file.
//!
//! Sakura: `Command_OPEN_POWERSHELL(FALSE)` (`sakura_core/cmd/CViewCommander_File.cpp`) starts
//! `powershell.exe -NoExit -Command "Set-Location -Path '<folder>'"` with `ShellExecute` (here
//! `-LiteralPath`, so that `[` in a folder name is not a wildcard). On macOS and Linux `pwsh` runs
//! in a terminal in the folder if it is installed (`ume_platform::shell`).

use crate::file_ops::external::open_shell_here;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_platform::shell::Shell;

#[derive(Default)]
pub struct OpenPowershell;

impl UiCommand for OpenPowershell {
    const ID: &'static str = "open_powershell";
    const SAKURA: &'static str = "F_OPEN_POWERSHELL";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        open_shell_here(view, Shell::PowerShell, false, cx);
        Ok(())
    }
}

register_ui_command!(OpenPowershell);
