//! `F_GOFILEEND_SEL`: moves the caret to the end of the document and extends the selection.
//!
//! Sakura: `CViewCommander::Command_GOFILEEND` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoFileEndSel;

impl Command for GoFileEndSel {
    const ID: &'static str = "go_file_end_sel";
    const SAKURA: &'static str = "F_GOFILEEND_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_file_end(true)
    }
}

register_command!(GoFileEndSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("go_file_end_sel", "a|b\ncd", "a[b\ncd|");
        check("go_file_end_sel", "ab|", "ab|");
    }
}
