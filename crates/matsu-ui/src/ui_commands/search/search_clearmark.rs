//! `F_SEARCH_CLEARMARK`: with a selection, marks its text as the search pattern; without one, clears the marks.
//!
//! Sakura: `CViewCommander::Command_SEARCH_CLEARMARK` (`sakura_core/cmd/CViewCommander_Search.cpp`).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SearchClearmark;

impl UiCommand for SearchClearmark {
    const ID: &'static str = "search_clearmark";
    const SAKURA: &'static str = "F_SEARCH_CLEARMARK";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.toggle_search_mark(window, cx);
        Ok(())
    }
}

register_ui_command!(SearchClearmark);
