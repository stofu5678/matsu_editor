//! `F_INSBOXTEXT` (the macro `InsBoxText`). Sakura: `CViewCommander::Command_INSBOXTEXT` in
//! `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Pastes its argument as a box, as `pastebox`
//! does with the clipboard text.

use crate::command::{Arg, Command, CommandError, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct InsBoxText;

impl Command for InsBoxText {
    const ID: &'static str = "insboxtext";
    const SAKURA: &'static str = "F_INSBOXTEXT";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        match args {
            [Arg::Text(text)] => cx.paste_box(text),
            _ => Err(CommandError::BadArgs(
                "insboxtext takes one text".to_owned(),
            )),
        }
    }
}

register_command!(InsBoxText);
