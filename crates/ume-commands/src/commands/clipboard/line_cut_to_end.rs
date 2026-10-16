//! `F_LineCutToEnd`. Sakura: `CViewCommander::Command_LineCutToEnd` in
//! `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`. Cuts from the caret to the end of the
//! text of the line, without the line ending. With a selection it cuts the selection; at the end of
//! the line it beeps.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct LineCutToEnd;

impl Command for LineCutToEnd {
    const ID: &'static str = "line_cut_to_end";
    const SAKURA: &'static str = "F_LineCutToEnd";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.cut_to_line_end()
    }
}

register_command!(LineCutToEnd);
