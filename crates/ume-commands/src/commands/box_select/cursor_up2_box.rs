//! `F_UP2_BOX`: moves the caret two rows up and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_UP2` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorUp2Box;

impl Command for CursorUp2Box {
    const ID: &'static str = "cursor_up2_box";
    const SAKURA: &'static str = "F_UP2_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_vertical(-2, true)
    }
}

register_command!(CursorUp2Box);

#[cfg(test)]
mod tests {
    use crate::testing::check_box;

    #[test]
    fn moves_two_rows_up() {
        check_box("cursor_up2_box", "abc\nd\ne|f", (2, 1), (0, 1));
        check_box("cursor_up2_box", "abcd\nx\nefg|h", (2, 3), (0, 3));
        check_box("cursor_up2_box", "ab\nc|d", (1, 1), (0, 1));
    }
}
