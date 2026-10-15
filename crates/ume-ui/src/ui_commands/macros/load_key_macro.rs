//! `F_LOADKEYMACRO`: stops recording and makes a file the key macro. Like Sakura, the file is
//! read when the key macro runs, so a mistake in it shows then.
//!
//! Sakura: `CViewCommander::Command_LOADKEYMACRO` (`sakura_core/cmd/CViewCommander_Macro.cpp`).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct LoadKeyMacro;

impl UiCommand for LoadKeyMacro {
    const ID: &'static str = "load_key_macro";
    const SAKURA: &'static str = "F_LOADKEYMACRO";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.ask_key_macro_load(window, cx);
        Ok(())
    }
}

register_ui_command!(LoadKeyMacro);
