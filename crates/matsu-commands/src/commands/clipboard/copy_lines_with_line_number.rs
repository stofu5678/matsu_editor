//! `F_COPYLINESWITHLINENUMBER`. Sakura: `CViewCommander::Command_COPYLINESWITHLINENUMBER` in
//! `sakura_core/cmd/CViewCommander_Clipboard.cpp`, which calls `CEditView::CopySelectedAllLines`
//! with the numbers on. Like `copy_lines`, with the number of the row of the layout (` 12:`, a
//! wrapped line has one number per row) at the start of each row; the width is that of the digits
//! of the row count, at least the type's `nLineNumWidth` (see `copy_selected_lines`).

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
