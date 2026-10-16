//! `F_DOWN_BOX`: moves the caret one row down and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_DOWN` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorDownBox;

impl Command for CursorDownBox {
    const ID: &'static str = "cursor_down_box";
    const SAKURA: &'static str = "F_DOWN_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_vertical(1, true)
    }
}

register_command!(CursorDownBox);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check_box};

    #[test]
    fn moves_down_and_begins_the_box() {
        check_box("cursor_down_box", "ab|cd\nefgh", (0, 2), (1, 2));
        check_box("cursor_down_box", "ab|cd", (0, 2), (0, 2));
        // the column of a wide char: the caret lands on the start of the char that covers it
        check_box("cursor_down_box", "abc|d\n日本", (0, 3), (1, 2));
    }

    #[test]
    fn stays_off_the_empty_row_after_the_last_line_ending() {
        // Cursor_UPDOWN: not in the first column, so that the column does not change
        check_box("cursor_down_box", "ab|\n", (0, 2), (0, 2));
        // in the first column it goes there
        check_box("cursor_down_box", "|ab\n", (0, 0), (1, 0));
        // on that row already, nothing moves
        check_box("cursor_down_box", "ab\n|", (1, 0), (1, 0));
    }

    #[test]
    fn extends_the_box_over_the_rows() {
        let mut e = Editor::new("a|bcd\nefgh\nij");
        e.run("cursor_down_box").run("cursor_down_box");
        // the short last row: the caret is in the empty space
        e.assert_box((0, 1), (2, 1));
        e.run("cursor_up_box").run("cursor_up_box");
        // back on the anchor: nothing is selected, the box goes on
        e.assert_box((0, 1), (0, 1));
        assert!(!e.cursor().has_selection());
        assert!(e.cursor().is_box());
    }
}
