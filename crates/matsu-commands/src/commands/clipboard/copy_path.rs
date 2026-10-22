//! `F_COPYPATH`. Sakura: `CViewCommander::Command_COPYPATH` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Copies the full path of the document.
//! A document without a file only beeps.

use crate::clipboard::ClipKind;
use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyPath;

impl Command for CopyPath {
    const ID: &'static str = "copy_path";
    const SAKURA: &'static str = "F_COPYPATH";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        if let Some(path) = cx.path_text() {
            cx.clipboard_set(path, ClipKind::Normal)?;
        }
        Ok(())
    }
}

register_command!(CopyPath);
