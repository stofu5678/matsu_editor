//! `F_GOFILEEND_BOX`: moves the caret to the last row, in the same column and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_GOFILEEND` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoFileEndBox;

impl Command for GoFileEndBox {
    const ID: &'static str = "go_file_end_box";
    const SAKURA: &'static str = "F_GOFILEEND_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_file_end(true)
    }
}

register_command!(GoFileEndBox);

#[cfg(test)]
mod tests {
    use crate::testing::check_box;

    #[test]
    fn keeps_the_column() {
        check_box("go_file_end_box", "a|b\ncd\nefg", (0, 1), (2, 1));
        check_box("go_file_end_box", "a|bc\nd", (0, 1), (1, 1));
    }

    #[test]
    fn stays_off_the_empty_row_after_the_last_line_ending() {
        check_box("go_file_end_box", "a|b\ncd\n", (0, 1), (1, 1));
        // in the first column it does go there
        check_box("go_file_end_box", "|ab\ncd\n", (0, 0), (2, 0));
    }
}
