//! `F_GOLINETOP_BOX`: moves the caret to the first char that is not indentation, or to the start of the row and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_GOLINETOP` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The first macro argument (the flags of the line moves) is passed on; the second (the lock option, `Sub_BoxSelectLock`) is not supported, as for
//! the other box commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoLineTopBox;

impl Command for GoLineTopBox {
    const ID: &'static str = "go_line_top_box";
    const SAKURA: &'static str = "F_GOLINETOP_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_line_top_flags(true, Arg::first_int(args))
    }
}

register_command!(GoLineTopBox);

#[cfg(test)]
mod tests {
    use crate::testing::check_box;

    #[test]
    fn moves_to_the_first_char_or_the_start() {
        check_box("go_line_top_box", "x\n  ab|c", (1, 4), (1, 2));
        check_box("go_line_top_box", "  |abc", (0, 2), (0, 0));
        check_box("go_line_top_box", "|  abc", (0, 0), (0, 2));
        check_box("go_line_top_box", "x\r\n\tab|c", (1, 6), (1, 4));
    }
}
