//! `F_REPLACE`: replaces the selected match with the replacement text of the panel and selects the next one.
//!
//! Sakura: `CViewCommander::Command_REPLACE` (`sakura_core/cmd/CViewCommander_Search.cpp`).

use crate::search::macro_args::MacroSearch;
use crate::search::{PanelMode, SearchOutcome};
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Replace;

impl UiCommand for Replace {
    const ID: &'static str = "replace";
    const SAKURA: &'static str = "F_REPLACE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.with_search_args(
            args,
            MacroSearch::Replace,
            window,
            cx,
            |view, window, cx| {
                if view.replace_next(window, cx) == SearchOutcome::NoPattern {
                    view.open_search_panel(PanelMode::Replace, window, cx);
                }
            },
        )?;
        Ok(())
    }
}

register_ui_command!(Replace);
