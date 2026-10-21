//! `F_SAVEKEYMACRO`: stops recording and saves the key macro in a file that the user names.
//!
//! Sakura: `CViewCommander::Command_SAVEKEYMACRO` (`sakura_core/cmd/CViewCommander_Macro.cpp`).
//! The dialog starts in the macro folder, as in Sakura.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SaveKeyMacro;

impl UiCommand for SaveKeyMacro {
    const ID: &'static str = "save_key_macro";
    const SAKURA: &'static str = "F_SAVEKEYMACRO";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.ask_key_macro_save(window, cx);
        Ok(())
    }
}

register_ui_command!(SaveKeyMacro);
