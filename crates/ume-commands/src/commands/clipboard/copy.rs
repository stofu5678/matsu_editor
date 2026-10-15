//! `F_COPY`. Sakura: `CViewCommander::Command_COPY` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Copies the selection. Without a selection it copies the row and marks it as a line copy, so that a paste puts it at the start of a row
//! (`bEnableNoSelectCopy` and `bEnableLineModePaste` are on by default). The selection stays
//! (the setting that drops it after a copy is off by default).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Copy;

impl Command for Copy {
    const ID: &'static str = "copy";
    const SAKURA: &'static str = "F_COPY";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.copy(None)
    }
}

register_command!(Copy);
