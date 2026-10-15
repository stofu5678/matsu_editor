//! `F_UP2_SEL`: moves the caret two rows up and extends the selection.
//!
//! Sakura: `CViewCommander::Command_UP2` with `bSelect` set, in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`; see `cursor_up2`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorUp2Sel;

impl Command for CursorUp2Sel {
    const ID: &'static str = "cursor_up2_sel";
    const SAKURA: &'static str = "F_UP2_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_vertical(-2, true)
    }
}

register_command!(CursorUp2Sel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn extends_the_selection() {
        check("cursor_up2_sel", "ab\ncd\nef|\ngh", "ab|\ncd\nef[\ngh");
        check("cursor_up2_sel", "ab\ncd\ne|f", "a|b\ncd\ne[f");
        // an existing selection keeps its anchor
        check(
            "cursor_up2_sel",
            "ab\ncd\nef\ng[h\nij|",
            "ab\ncd\nef|\ng[h\nij",
        );
    }
}
