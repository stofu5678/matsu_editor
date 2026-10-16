//! `F_EXECKEYMACRO`: stops a recording, then plays the key macro. Each call is one undo step.
//!
//! Sakura: `CViewCommander::Command_EXECKEYMACRO` (`sakura_core/cmd/CViewCommander_Macro.cpp`).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ExecKeyMacro;

impl UiCommand for ExecKeyMacro {
    const ID: &'static str = "exec_key_macro";
    const SAKURA: &'static str = "F_EXECKEYMACRO";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.exec_key_macro(window, cx)
    }
}

register_ui_command!(ExecKeyMacro);
