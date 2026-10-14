//! Spike command; the real ones cite the Sakura source here.
use crate::{Command, CommandContext, Result};

#[derive(Default)]
pub struct OpenFile;

impl Command for OpenFile {
    const ID: &'static str = "open_file";
    fn run(&self, cx: &mut CommandContext) -> Result<()> {
        cx.log.push(Self::ID.to_string());
        Ok(())
    }
}
