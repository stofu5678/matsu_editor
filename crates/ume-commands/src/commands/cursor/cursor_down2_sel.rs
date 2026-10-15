//! `F_DOWN2_SEL`: moves the caret two rows down and extends the selection.
//!
//! Sakura: `CViewCommander::Command_DOWN2` with `bSelect` set, in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`; see `cursor_down2`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorDown2Sel;

impl Command for CursorDown2Sel {
    const ID: &'static str = "cursor_down2_sel";
    const SAKURA: &'static str = "F_DOWN2_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_vertical(2, true)
    }
}

register_command!(CursorDown2Sel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn extends_the_selection() {
        check("cursor_down2_sel", "a|b\ncd\nef\ngh", "a[b\ncd\ne|f\ngh");
        check("cursor_down2_sel", "a|b\ncd", "a[b\nc|d");
        // an existing selection keeps its anchor
        check("cursor_down2_sel", "a[b\n|cd\nef\ngh", "a[b\ncd\nef\n|gh");
    }
}
