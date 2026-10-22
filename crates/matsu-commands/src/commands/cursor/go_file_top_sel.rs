//! `F_GOFILETOP_SEL`: moves the caret to the start of the document and extends the selection.
//!
//! Sakura: `CViewCommander::Command_GOFILETOP` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoFileTopSel;

impl Command for GoFileTopSel {
    const ID: &'static str = "go_file_top_sel";
    const SAKURA: &'static str = "F_GOFILETOP_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_file_top(true)
    }
}

register_command!(GoFileTopSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("go_file_top_sel", "ab\nc|d", "|ab\nc[d");
        check("go_file_top_sel", "|ab", "|ab");
    }
}
