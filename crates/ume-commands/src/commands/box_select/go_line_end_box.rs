//! `F_GOLINEEND_BOX`: moves the caret to the end of the row and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_GOLINEEND` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoLineEndBox;

impl Command for GoLineEndBox {
    const ID: &'static str = "go_line_end_box";
    const SAKURA: &'static str = "F_GOLINEEND_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_line_end(true)
    }
}

register_command!(GoLineEndBox);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check_box};

    #[test]
    fn moves_to_the_end_of_the_row() {
        check_box("go_line_end_box", "a|bc\r\nd", (0, 1), (0, 3));
        check_box("go_line_end_box", "日|本\nx", (0, 2), (0, 4));
    }

    #[test]
    fn comes_back_from_the_empty_space() {
        let mut e = Editor::new("ab|");
        e.run("cursor_right_box").run("cursor_right_box");
        e.run("go_line_end_box").assert_box((0, 2), (0, 2));
    }
}
