//! `F_BROWSE`: opens the file with the program the OS has for it.
//!
//! Sakura: `Command_BROWSE` (`sakura_core/cmd/CViewCommander_File.cpp`) calls `ShellExecuteEx` on
//! the file with no verb, which is the default action; a document without a file beeps. GPUI's
//! `open_with_system` is that on each OS (`ShellExecute`, `open`, `xdg-open`).

use crate::file_ops::external::document_file;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Browse;

impl UiCommand for Browse {
    const ID: &'static str = "browse";
    const SAKURA: &'static str = "F_BROWSE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if let Some(path) = document_file(view, cx) {
            cx.open_with_system(&path);
        }
        Ok(())
    }
}

register_ui_command!(Browse);
