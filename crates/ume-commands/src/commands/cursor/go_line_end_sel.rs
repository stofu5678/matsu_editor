//! `F_GOLINEEND_SEL`: moves the caret to the end of the row and extends the selection.
//!
//! Sakura: `CViewCommander::Command_GOLINEEND` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoLineEndSel;

impl Command for GoLineEndSel {
    const ID: &'static str = "go_line_end_sel";
    const SAKURA: &'static str = "F_GOLINEEND_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_line_end(true)
    }
}

register_command!(GoLineEndSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("go_line_end_sel", "ab|cd", "ab[cd|");
        check("go_line_end_sel", "ab|cd\r\nef", "ab[cd|\r\nef");
    }
}
