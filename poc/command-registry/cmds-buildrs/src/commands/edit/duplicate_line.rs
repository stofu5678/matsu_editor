//! Spike command; the real ones cite the Sakura source here.
use crate::{Command, CommandContext, Result};

#[derive(Default)]
pub struct DuplicateLine;

impl Command for DuplicateLine {
    const ID: &'static str = "duplicate_line";
    fn run(&self, cx: &mut CommandContext) -> Result<()> {
        cx.log.push(Self::ID.to_string());
        Ok(())
    }
}
