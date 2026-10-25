//! `F_EXECEXTMACRO`: runs a macro file (`.mac`, `.mjs`, `.js`, `.cjs`, `.ts`, `.py`, `.rb`, `.php`, `.ps1`, `.pl`, `.el`, `.applescript`, `.lua`,
//! `.mruby` or `.upy`). With a text argument that is the file; without one the user chooses it.
//! With a second text argument (a language, written as its file extension) the first one is the
//! code of the macro, not a file.
//!
//! Sakura: `CViewCommander::Command_EXECEXTMACRO` (`sakura_core/cmd/CViewCommander_Macro.cpp`):
//! `pszType` is `nullptr` for a file, and otherwise the extension that picks the language of the
//! macro, which `CSMacroMgr::Load` reads from the string. An empty second argument is a type
//! with no name, which Sakura does not find; it is taken as "no type" here, as an empty file name
//! asks for the dialog.

use std::path::Path;

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

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
        match (args.first(), args.get(1)) {
            (Some(Arg::Text(code)), Some(Arg::Text(ext))) if !ext.is_empty() => {
                view.exec_macro_text(code, ext, window, cx)
            }
            (Some(Arg::Text(path)), _) if !path.is_empty() => {
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
