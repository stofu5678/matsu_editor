//! `F_UP_SEL`: moves the caret one row up and extends the selection.
//!
//! Sakura: `CViewCommander::Command_UP` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorUpSel;

impl Command for CursorUpSel {
    const ID: &'static str = "cursor_up_sel";
    const SAKURA: &'static str = "F_UP_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_vertical(-1, true)
    }
}

register_command!(CursorUpSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("cursor_up_sel", "ab\ncd|", "ab|\ncd[");
        check("cursor_up_sel", "ab\nc[d|", "ab|\nc[d");
        check("cursor_up_sel", "ab|", "ab|");
    }
}
