//! `F_COPY_ADDCRLF`. Sakura: `CViewCommander::Command_COPY( false, true )` in
//! `sakura_core/cmd/CViewCommander.cpp` (`HandleCommand`) and
//! `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Like `copy`, and every row that the wrapping
//! cuts (and the last line) ends with a line ending of the document in the copy
//! (`bAddCRLFWhenCopy`; `CEditView::GetSelectedData` and `CEditView::CopyCurLine`). A box copy is
//! not changed by it.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyAddCrlf;

impl Command for CopyAddCrlf {
    const ID: &'static str = "copy_addcrlf";
    const SAKURA: &'static str = "F_COPY_ADDCRLF";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.copy(None, true)
    }
}

register_command!(CopyAddCrlf);
