//! `F_COPYLINESASPASSAGE`. Sakura: `CViewCommander::Command_COPYLINESASPASSAGE` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Like `copylines`, with the quote
//! (`> ` by default, `m_szInyouKigou`) at the start of each row.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::context::QUOTE;
use crate::register_command;

#[derive(Default)]
pub struct CopyLinesAsPassage;

impl Command for CopyLinesAsPassage {
    const ID: &'static str = "copylinesaspassage";
    const SAKURA: &'static str = "F_COPYLINESASPASSAGE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.copy_selected_lines(Some(QUOTE), false)
    }
}

register_command!(CopyLinesAsPassage);
