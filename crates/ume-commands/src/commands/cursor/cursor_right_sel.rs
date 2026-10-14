//! `F_RIGHT_SEL`: moves the caret one char right and extends the selection.
//!
//! Sakura: `CViewCommander::Command_RIGHT` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorRightSel;

impl Command for CursorRightSel {
    const ID: &'static str = "cursor_right_sel";
    const SAKURA: &'static str = "F_RIGHT_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_right(true)
    }
}

register_command!(CursorRightSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("cursor_right_sel", "ab|cd", "ab[c|d");
        check("cursor_right_sel", "a|bc[d", "ab|c[d");
        check("cursor_right_sel", "ab|\r\ncd", "ab[\r\n|cd");
        check("cursor_right_sel", "ab|", "ab|");
    }
}
