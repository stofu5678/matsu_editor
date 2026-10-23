//! `F_DIFF_DIALOG`: opens the DIFF dialog (DIFF差分表示...).
//!
//! Sakura: `CViewCommander::Command_Diff_Dialog` (`sakura_core/cmd/CViewCommander_Diff.cpp`),
//! `CDlgDiff` (`sakura_core/dlg/CDlgDiff.cpp`). The dialog is `crate::diff::panel`; OK marks the
//! lines of this window that differ from the file or the window that was chosen.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct DiffDialog;

impl UiCommand for DiffDialog {
    const ID: &'static str = "diff_dialog";
    const SAKURA: &'static str = "F_DIFF_DIALOG";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_diff_panel(window, cx);
        Ok(())
    }
}

register_ui_command!(DiffDialog);
