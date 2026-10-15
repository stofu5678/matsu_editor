//! `F_LineCutToStart`. Sakura: `CViewCommander::Command_LineCutToStart` in
//! `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`. Cuts from the start of the line (not the
//! row) to the caret. With a selection it cuts the selection; at the start of the line it beeps.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct LineCutToStart;

impl Command for LineCutToStart {
    const ID: &'static str = "line_cut_to_start";
    const SAKURA: &'static str = "F_LineCutToStart";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.cut_to_line_start()
    }
}

register_command!(LineCutToStart);
