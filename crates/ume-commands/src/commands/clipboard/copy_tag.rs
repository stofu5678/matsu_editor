//! `F_COPYTAG`. Sakura: `CViewCommander::Command_COPYTAG` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Copies `path (line,column): ` with the
//! caret's line and column, both counted from 1; the column counts UTF-16 units like Sakura's
//! `wchar_t`. A document without a file only beeps.

use ume_core::buffer::col_to_utf16;

use crate::clipboard::ClipKind;
use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyTag;

impl Command for CopyTag {
    const ID: &'static str = "copy_tag";
    const SAKURA: &'static str = "F_COPYTAG";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        let Some(path) = cx.path_text() else {
            return Ok(());
        };
        let caret = cx.caret();
        let text = cx
            .doc()
            .line(caret.line)?
            .map(|l| l.text)
            .unwrap_or_default();
        let col = col_to_utf16(&text, caret.col);
        let tag = format!("{path} ({},{}): ", caret.line + 1, col + 1);
        cx.clipboard_set(tag, ClipKind::Normal)?;
        Ok(())
    }
}

register_command!(CopyTag);
