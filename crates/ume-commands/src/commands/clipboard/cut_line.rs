//! `F_CUT_LINE`. Sakura: `CViewCommander::Command_CUT_LINE` in `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`.
//! Copies the row of the caret as a line copy and deletes it. With a selection, or on the empty line
//! after a final line ending, it only beeps.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CutLine;

impl Command for CutLine {
    const ID: &'static str = "cut_line";
    const SAKURA: &'static str = "F_CUT_LINE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.cut_row()
    }
}

register_command!(CutLine);
