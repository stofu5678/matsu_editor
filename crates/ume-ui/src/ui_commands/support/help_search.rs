//! `F_HELP_SEARCH`: shows the index of the help for a keyword search.
//!
//! Sakura: `CViewCommander::Command_HELP_SEARCH` (`sakura_core/cmd/CViewCommander_Support.cpp`)
//! opens its own help file at the index. UME Editor has no help file; it opens the list of the
//! decisions of the project on the web ([`crate::support::help::INDEX_URL`]).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct HelpSearch;

impl UiCommand for HelpSearch {
    const ID: &'static str = "help_search";
    const SAKURA: &'static str = "F_HELP_SEARCH";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.program_help(false, cx);
        Ok(())
    }
}

register_ui_command!(HelpSearch);
