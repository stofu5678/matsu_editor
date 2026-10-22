//! `F_WIN_OUTPUT`: shows the output (アウトプット).
//!
//! Sakura: `Command_WIN_OUTPUT` (`sakura_core/cmd/CViewCommander_Window.cpp`) opens the output
//! window (`CShareData::OpenDebugWindow`), an editor window for what macros print with
//! `TraceOut` and what Grep reports. This program has no such window; what macros print goes to
//! the output panel under the text of the window that ran the macro (`crate::macros`), so this
//! shows that panel (with what it holds).

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct WinOutput;

impl UiCommand for WinOutput {
    const ID: &'static str = "win_output";
    const SAKURA: &'static str = "F_WIN_OUTPUT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.macros.output.show();
        cx.notify();
        Ok(())
    }
}

register_ui_command!(WinOutput);
