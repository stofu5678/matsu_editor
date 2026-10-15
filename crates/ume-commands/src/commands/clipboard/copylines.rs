//! `F_COPYLINES`. Sakura: `CViewCommander::Command_COPYLINES` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`, `CEditView::CopySelectedAllLines`
//! in `sakura_core/view/CEditView.cpp`. Widens the selection to whole rows and copies them.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyLines;

impl Command for CopyLines {
    const ID: &'static str = "copylines";
    const SAKURA: &'static str = "F_COPYLINES";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.copy_selected_lines(None, false)
    }
}

register_command!(CopyLines);
