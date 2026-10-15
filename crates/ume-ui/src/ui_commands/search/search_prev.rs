//! `F_SEARCH_PREV`: selects the previous match of the current pattern from the caret (F3 / Shift+F3).
//!
//! Sakura: `CViewCommander::Command_SEARCH_NEXT` and `Command_SEARCH_PREV`
//! (`sakura_core/cmd/CViewCommander_Search.cpp`). With no pattern yet it opens the find panel.

use ume_core::search::Direction;

use crate::search::{PanelMode, SearchOutcome};
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SearchPrev;

impl UiCommand for SearchPrev {
    const ID: &'static str = "search_prev";
    const SAKURA: &'static str = "F_SEARCH_PREV";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if view.search(Direction::Backward, window, cx) == SearchOutcome::NoPattern {
            view.open_search_panel(PanelMode::Find, window, cx);
        }
        Ok(())
    }
}

register_ui_command!(SearchPrev);
