//! `F_HELP_CONTENTS`: shows the table of contents of the help.
//!
//! Sakura: `CViewCommander::Command_HELP_CONTENTS` (`sakura_core/cmd/CViewCommander_Support.cpp`)
//! opens its own help file at the contents. MATSU Editor opens the `index.html` of the help that comes
//! with the program, else the contents on the web ([`crate::support::help::plan_contents`]).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct HelpContents;

impl UiCommand for HelpContents {
    const ID: &'static str = "help_contents";
    const SAKURA: &'static str = "F_HELP_CONTENTS";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.program_help(true, cx);
        Ok(())
    }
}

register_ui_command!(HelpContents);
