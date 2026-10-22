//! `F_REPLACE_ALL`: replaces every match, as one undo step. A macro gives it the strings and the options (`search::macro_args`).
//!
//! Sakura: `CViewCommander::Command_REPLACE_ALL` (`sakura_core/cmd/CViewCommander_Search.cpp`).

use crate::search::PanelMode;
use crate::search::macro_args::MacroSearch;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ReplaceAll;

impl UiCommand for ReplaceAll {
    const ID: &'static str = "replace_all";
    const SAKURA: &'static str = "F_REPLACE_ALL";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.with_search_args(
            args,
            MacroSearch::ReplaceAll,
            window,
            cx,
            |view, window, cx| {
                if view.replace_all(window, cx).is_none()
                    && view.search_state().pattern().is_empty()
                {
                    view.open_search_panel(PanelMode::Replace, window, cx);
                }
            },
        )?;
        Ok(())
    }
}

register_ui_command!(ReplaceAll);
