//! `F_GOFILETOP_BOX`: moves the caret to the first row, in the same column and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_GOFILETOP` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoFileTopBox;

impl Command for GoFileTopBox {
    const ID: &'static str = "go_file_top_box";
    const SAKURA: &'static str = "F_GOFILETOP_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_file_top(true)
    }
}

register_command!(GoFileTopBox);

#[cfg(test)]
mod tests {
    use crate::testing::check_box;

    #[test]
    fn keeps_the_column() {
        check_box("go_file_top_box", "abcd\nxy|z", (1, 2), (0, 2));
        // a short first row: the caret is in the empty space
        check_box("go_file_top_box", "a\nbcd\nxy|z", (2, 2), (0, 2));
        check_box("go_file_top_box", "ab|c", (0, 2), (0, 2));
    }
}
