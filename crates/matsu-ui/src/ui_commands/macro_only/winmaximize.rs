//! `F_WINMAXIMIZE` (the macro `WinMaximize()`): maximizes the window.
//!
//! Sakura: `cmd::window::Command_WINMAXIMIZE` (`sakura_core/cmd/CViewCommander_Window.cpp`) is
//! `ShowWindow(SW_MAXIMIZE)` on the edit window. GPUI only toggles the zoom of a window, so a
//! window that is maximized already is left alone.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct WinMaximize;

impl UiCommand for WinMaximize {
    const ID: &'static str = "winmaximize";
    const SAKURA: &'static str = "F_WINMAXIMIZE";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        _cx: &mut Context<EditorView>,
    ) -> Result {
        super::maximize(window);
        Ok(())
    }
}

register_ui_command!(WinMaximize);
