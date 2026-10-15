//! `F_COPYFNAME`. Sakura: `CViewCommander::Command_COPYFILENAME` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Copies the file name of the document.
//! A document without a file only beeps.

use crate::clipboard::ClipKind;
use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyFname;

impl Command for CopyFname {
    const ID: &'static str = "copyfname";
    const SAKURA: &'static str = "F_COPYFNAME";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        match cx
            .path()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
        {
            Some(name) => {
                cx.clipboard_set(name, ClipKind::Normal)?;
            }
            None => cx.beep(),
        }
        Ok(())
    }
}

register_command!(CopyFname);
