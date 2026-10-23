//! `F_PASTEBOX`. Sakura: `CViewCommander::Command_PASTEBOX` in
//! `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Pastes the clipboard text as a box: each line
//! goes into the next row at the display column of the caret, and a row that is shorter is padded
//! with spaces (see `CommandContext::paste_box`). The text is pasted as a box even if it was not
//! copied from one. Beeps when the clipboard has no text.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct PasteBox;

impl Command for PasteBox {
    const ID: &'static str = "pastebox";
    const SAKURA: &'static str = "F_PASTEBOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        let Some(clip) = cx.clipboard_get()? else {
            cx.beep();
            return Ok(());
        };
        cx.paste_box(&clip.text)
    }
}

register_command!(PasteBox);
