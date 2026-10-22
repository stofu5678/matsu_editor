//! `F_GREP_DIALOG`: opens the Grep dialog (Grep...), starting with the selection or the word at the
//! caret and the folder of the file in the window.
//!
//! Sakura: `CViewCommander::Command_GREP_DIALOG` (`sakura_core/cmd/CViewCommander_Grep.cpp`),
//! `CDlgGrep` (`sakura_core/dlg/CDlgGrep.cpp`). The search itself is `F_GREP` (`grep.rs`).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct GrepDialog;

impl UiCommand for GrepDialog {
    const ID: &'static str = "grep_dialog";
    const SAKURA: &'static str = "F_GREP_DIALOG";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_grep_panel(window, cx);
        Ok(())
    }
}

register_ui_command!(GrepDialog);
