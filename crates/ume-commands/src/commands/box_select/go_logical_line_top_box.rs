//! `F_GOLOGICALLINETOP_BOX`: moves the caret to the first char that is not indentation, or to the start of the line and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_GOLINETOP` with the flag for lines (8) with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoLogicalLineTopBox;

impl Command for GoLogicalLineTopBox {
    const ID: &'static str = "go_logical_line_top_box";
    const SAKURA: &'static str = "F_GOLOGICALLINETOP_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_logical_line_top(true)
    }
}

register_command!(GoLogicalLineTopBox);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check_box};

    #[test]
    fn moves_to_the_first_char_or_the_start() {
        check_box("go_logical_line_top_box", "x\n  ab|c", (1, 4), (1, 2));
        check_box("go_logical_line_top_box", "  |abc", (0, 2), (0, 0));
        check_box("go_logical_line_top_box", "|  abc", (0, 0), (0, 2));
        check_box("go_logical_line_top_box", "\u{3000}ab|c", (0, 4), (0, 2));
        // a line of indentation only: the start of the line
        check_box("go_logical_line_top_box", "  | ", (0, 2), (0, 0));
    }

    #[test]
    fn a_caret_in_the_empty_space_goes_to_the_first_char() {
        // the row is only indentation and the caret is past its end: not at the start
        let mut e = Editor::new(" |");
        e.run("cursor_right_box");
        e.run("go_logical_line_top_box").assert_box((0, 1), (0, 0));
    }
}
