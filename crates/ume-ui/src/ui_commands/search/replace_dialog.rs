//! `F_REPLACE_DIALOG`: opens the replace panel (置換).
//!
//! Sakura: `CViewCommander::Command_REPLACE_DIALOG` (`sakura_core/cmd/CViewCommander_Search.cpp`).

use crate::search::PanelMode;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ReplaceDialog;

impl UiCommand for ReplaceDialog {
    const ID: &'static str = "replace_dialog";
    const SAKURA: &'static str = "F_REPLACE_DIALOG";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_search_panel(PanelMode::Replace, window, cx);
        Ok(())
    }
}

register_ui_command!(ReplaceDialog);
