//! `F_COMPARE`: compares the text with another window (ファイル内容比較...).
//!
//! Sakura: `CViewCommander::Command_COMPARE` (`sakura_core/cmd/CViewCommander_Diff.cpp`),
//! `CDlgCompare` (`sakura_core/dlg/CDlgCompare.cpp`). The dialog and the comparison are in
//! `crate::diff::compare`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Compare;

impl UiCommand for Compare {
    const ID: &'static str = "compare";
    const SAKURA: &'static str = "F_COMPARE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_compare_panel(window, cx);
        Ok(())
    }
}

register_ui_command!(Compare);
