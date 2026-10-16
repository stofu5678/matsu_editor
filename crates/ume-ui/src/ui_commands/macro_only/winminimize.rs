//! `F_WINMINIMIZE` (the macro `WinMinimize()`): minimizes the window.
//!
//! Sakura: `cmd::window::Command_WINMINIMIZE` (`sakura_core/cmd/CViewCommander_Window.cpp`) is
//! `ShowWindow(SW_MINIMIZE)` on the edit window. GPUI minimizes a window on every OS.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct WinMinimize;

impl UiCommand for WinMinimize {
    const ID: &'static str = "winminimize";
    const SAKURA: &'static str = "F_WINMINIMIZE";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        _cx: &mut Context<EditorView>,
    ) -> Result {
        super::minimize(window);
        Ok(())
    }
}

register_ui_command!(WinMinimize);
