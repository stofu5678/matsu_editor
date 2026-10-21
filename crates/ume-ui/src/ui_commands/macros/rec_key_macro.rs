//! `F_RECKEYMACRO`: starts recording a key macro, or stops it and keeps what was recorded.
//!
//! Sakura: `CViewCommander::Command_RECKEYMACRO` (`sakura_core/cmd/CViewCommander_Macro.cpp`).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct RecKeyMacro;

impl UiCommand for RecKeyMacro {
    const ID: &'static str = "rec_key_macro";
    const SAKURA: &'static str = "F_RECKEYMACRO";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        // Stopping writes `RecKey.mac`; a failure is shown like the other macro errors.
        match view.toggle_key_macro_recording(cx) {
            Some(message) => Err(view.macro_error(message, window, cx)),
            None => Ok(()),
        }
    }
}

register_ui_command!(RecKeyMacro);
