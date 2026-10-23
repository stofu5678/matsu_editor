//! `F_SEARCH_DIALOG`: opens the find panel (検索), starting with the selection or the word at the caret.
//!
//! Sakura: `CViewCommander::Command_SEARCH_DIALOG` (`sakura_core/cmd/CViewCommander_Search.cpp`).

use crate::search::PanelMode;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SearchDialog;

impl UiCommand for SearchDialog {
    const ID: &'static str = "search_dialog";
    const SAKURA: &'static str = "F_SEARCH_DIALOG";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_search_panel(PanelMode::Find, window, cx);
        Ok(())
    }
}

register_ui_command!(SearchDialog);
