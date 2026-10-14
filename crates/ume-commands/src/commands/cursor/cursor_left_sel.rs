//! `F_LEFT_SEL`: moves the caret one char left and extends the selection.
//!
//! Sakura: `CViewCommander::Command_LEFT` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorLeftSel;

impl Command for CursorLeftSel {
    const ID: &'static str = "cursor_left_sel";
    const SAKURA: &'static str = "F_LEFT_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_left(true)
    }
}

register_command!(CursorLeftSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("cursor_left_sel", "ab|cd", "a|b[cd");
        check("cursor_left_sel", "a[bc|d", "a[b|cd");
        check("cursor_left_sel", "a|bc[d", "|abc[d");
        check("cursor_left_sel", "ab\r\n|cd", "ab|\r\n[cd");
        check("cursor_left_sel", "|ab", "|ab");
    }
}
