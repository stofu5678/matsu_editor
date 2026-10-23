//! `F_GREP_REPLACE_DLG`: opens the dialog of Grep 置換 (Grep置換...), starting with the selection or
//! the word at the caret and the folder of the file in the window.
//!
//! Sakura: `CViewCommander::Command_GREP_REPLACE_DLG` (`sakura_core/cmd/CViewCommander_Grep.cpp`),
//! `CDlgGrepReplace` (`sakura_core/dlg/CDlgGrepReplace.cpp`). The replacement itself is
//! `F_GREP_REPLACE` (`grep_replace.rs`).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct GrepReplaceDlg;

impl UiCommand for GrepReplaceDlg {
    const ID: &'static str = "grep_replace_dlg";
    const SAKURA: &'static str = "F_GREP_REPLACE_DLG";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_grep_replace_panel(window, cx);
        Ok(())
    }
}

register_ui_command!(GrepReplaceDlg);
