//! `F_SEARCH_NEXT`: selects the next match of the current pattern from the caret (F3 / Shift+F3).
//!
//! Sakura: `CViewCommander::Command_SEARCH_NEXT` and `Command_SEARCH_PREV`
//! (`sakura_core/cmd/CViewCommander_Search.cpp`). With no pattern yet it opens the find panel.

use ume_core::search::Direction;

use crate::search::macro_args::MacroSearch;
use crate::search::{PanelMode, SearchOutcome};
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SearchNext;

impl UiCommand for SearchNext {
    const ID: &'static str = "search_next";
    const SAKURA: &'static str = "F_SEARCH_NEXT";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.with_search_args(args, MacroSearch::Find, window, cx, |view, window, cx| {
            if view.search(Direction::Forward, window, cx) == SearchOutcome::NoPattern {
                view.open_search_panel(PanelMode::Find, window, cx);
            }
        })?;
        Ok(())
    }
}

register_ui_command!(SearchNext);
