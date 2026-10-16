//! `F_COPYLINESWITHLINENUMBER`. Sakura: `CViewCommander::Command_COPYLINESWITHLINENUMBER` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Like `copy_lines`, with the
//! line number (` 12:`) at the start of each row. Difference: Sakura numbers the rows of the
//! layout; this numbers the lines of the document (they differ only in wrapped text).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyLinesWithLineNumber;

impl Command for CopyLinesWithLineNumber {
    const ID: &'static str = "copy_lines_with_line_number";
    const SAKURA: &'static str = "F_COPYLINESWITHLINENUMBER";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.copy_selected_lines(None, true)
    }
}

register_command!(CopyLinesWithLineNumber);
