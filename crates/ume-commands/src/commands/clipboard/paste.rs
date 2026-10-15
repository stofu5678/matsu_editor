//! `F_PASTE`. Sakura: `CViewCommander::Command_PASTE` and `Command_INSTEXT` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Pastes the clipboard text in place of the selection. A line copy goes to the start of the row and the caret
//! stays on its character. The argument is Sakura's `option` bit set (see `paste_option`); line
//! endings are not converted by default (`bConvertEOLPaste`). A box copy is pasted as normal text:
//! `F_PASTEBOX` is not implemented yet.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Paste;

impl Command for Paste {
    const ID: &'static str = "paste";
    const SAKURA: &'static str = "F_PASTE";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        cx.paste(match args.first() {
            Some(Arg::Int(option)) => *option,
            _ => 0,
        })
    }
}

register_command!(Paste);
