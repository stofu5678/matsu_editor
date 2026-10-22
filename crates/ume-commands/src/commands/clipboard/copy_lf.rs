//! `F_COPY_LF`: UME-only (Sakura has only `F_COPY_CRLF`). Like `copy_crlf`, with every line ending of the
//! copy changed to LF. Added at the user's request, for editing across operating systems. A box
//! selection is copied with LF after each row (Sakura's copies always use CRLF there).

use ume_core::buffer::Eol;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyLf;

impl Command for CopyLf {
    const ID: &'static str = "copy_lf";
    const SAKURA: &'static str = "F_COPY_LF";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.copy(Some(Eol::Lf), false)
    }
}

register_command!(CopyLf);
