//! `F_COPY_CRLF`. Sakura: `CViewCommander::Command_COPY` with `EEolType::cr_and_lf`, called from `CViewCommander::HandleCommand` in
//! `sakura_core/cmd/CViewCommander.cpp`. Like `copy`, with every line ending of the copy changed to CRLF.

use ume_core::buffer::Eol;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyCrlf;

impl Command for CopyCrlf {
    const ID: &'static str = "copy_crlf";
    const SAKURA: &'static str = "F_COPY_CRLF";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.copy(Some(Eol::CrLf), false)
    }
}

register_command!(CopyCrlf);
