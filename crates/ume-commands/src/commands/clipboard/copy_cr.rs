//! `F_COPY_CR`: UME-only (Sakura has only `F_COPY_CRLF`). Like `copy_crlf`, with every line ending of the
//! copy changed to CR. Added at the user's request, for editing across operating systems. A box
//! selection is copied with CR after each row (Sakura's copies always use CRLF there).

use ume_core::buffer::Eol;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyCr;

impl Command for CopyCr {
    const ID: &'static str = "copy_cr";
    const SAKURA: &'static str = "F_COPY_CR";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.copy(Some(Eol::Cr), false)
    }
}

register_command!(CopyCr);
