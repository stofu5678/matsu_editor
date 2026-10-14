//! Spike command; the real ones cite the Sakura source here.
use crate::{Command, CommandContext, Result};

#[derive(Default)]
pub struct DeleteLine;

impl Command for DeleteLine {
    const ID: &'static str = "delete_line";
    fn run(&self, cx: &mut CommandContext) -> Result<()> {
        cx.log.push(Self::ID.to_string());
        Ok(())
    }
}

crate::register_command!(DeleteLine);
