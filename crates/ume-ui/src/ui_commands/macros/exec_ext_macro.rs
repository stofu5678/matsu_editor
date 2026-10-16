//! `F_EXECEXTMACRO`: runs a macro file (`.mac`, `.mjs`, `.js`, `.cjs`, `.py`, `.rb`, `.php` or `.lua`).
//! With a text argument that is the file; without one the user chooses it.
//!
//! Sakura: `CViewCommander::Command_EXECEXTMACRO` (`sakura_core/cmd/CViewCommander_Macro.cpp`).
//! Not done yet: the second argument (the macro's code and its language instead of a file).

use std::path::Path;

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ExecExtMacro;

impl UiCommand for ExecExtMacro {
    const ID: &'static str = "exec_ext_macro";
    const SAKURA: &'static str = "F_EXECEXTMACRO";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        match args.first() {
            Some(Arg::Text(path)) if !path.is_empty() => {
                view.exec_ext_macro(Path::new(path), window, cx)
            }
            _ => {
                view.ask_macro_to_run(window, cx);
                Ok(())
            }
        }
    }
}

register_ui_command!(ExecExtMacro);
