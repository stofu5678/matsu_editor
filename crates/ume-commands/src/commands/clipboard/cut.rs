//! `F_CUT`. Sakura: `CViewCommander::Command_CUT` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Copies the selection to the clipboard and deletes it. Without a selection it cuts the row (`Command_CUT_LINE`) because
//! `bEnableNoSelectCopy` is on by default.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Cut;

impl Command for Cut {
    const ID: &'static str = "cut";
    const SAKURA: &'static str = "F_CUT";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.cut()
    }
}

register_command!(Cut);
