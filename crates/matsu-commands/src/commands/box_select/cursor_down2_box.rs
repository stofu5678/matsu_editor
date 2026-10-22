//! `F_DOWN2_BOX`: moves the caret two rows down and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_DOWN2` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorDown2Box;

impl Command for CursorDown2Box {
    const ID: &'static str = "cursor_down2_box";
    const SAKURA: &'static str = "F_DOWN2_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_vertical(2, true)
    }
}

register_command!(CursorDown2Box);

#[cfg(test)]
mod tests {
    use crate::testing::check_box;

    #[test]
    fn moves_two_rows_down() {
        check_box("cursor_down2_box", "a|bc\nd\nefg", (0, 1), (2, 1));
        // the last rows: as far as there is
        check_box("cursor_down2_box", "a|bc\nd", (0, 1), (1, 1));
        check_box("cursor_down2_box", "a|bc", (0, 1), (0, 1));
    }
}
