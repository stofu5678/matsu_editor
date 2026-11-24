//! `F_UP2_BOX`: moves the caret two rows up and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_UP2` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro argument is the lock option
//! (`Sub_BoxSelectLock`: 1 locks the box, 2 does not, 0 is the setting).
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::{BoxLock, CommandContext};
use crate::register_command;

#[derive(Default)]
pub struct CursorUp2Box;

impl Command for CursorUp2Box {
    const ID: &'static str = "cursor_up2_box";
    const SAKURA: &'static str = "F_UP2_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        cx.ensure_box(BoxLock::from_args(args, 0))?;
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
