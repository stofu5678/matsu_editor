//! `F_LEFT_BOX`: moves the caret one char left and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_LEFT` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorLeftBox;

impl Command for CursorLeftBox {
    const ID: &'static str = "cursor_left_box";
    const SAKURA: &'static str = "F_LEFT_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_left(true)
    }
}

register_command!(CursorLeftBox);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Pos;

    use crate::testing::{Editor, check_box};

    #[test]
    fn moves_left_and_begins_the_box() {
        check_box("cursor_left_box", "a|bc", (0, 1), (0, 0));
        check_box("cursor_left_box", "ab|c", (0, 2), (0, 1));
        // a full-width char and a tab are one step
        check_box("cursor_left_box", "日|本", (0, 2), (0, 0));
        check_box("cursor_left_box", "\ta|", (0, 5), (0, 4));
        check_box("cursor_left_box", "\t|a", (0, 4), (0, 0));
    }

    #[test]
    fn does_not_go_to_the_previous_row() {
        check_box("cursor_left_box", "ab\n|cd", (1, 0), (1, 0));
        check_box("cursor_left_box", "ab\r\n|cd", (1, 0), (1, 0));
    }

    #[test]
    fn comes_back_through_the_empty_space() {
        let mut e = Editor::new("ab|");
        e.run("cursor_right_box").run("cursor_right_box");
        e.assert_box((0, 2), (0, 4));
        e.run("cursor_left_box").assert_box((0, 2), (0, 3));
        assert_eq!(e.cursor().caret(), Pos::new(0, 2));
        e.run("cursor_left_box").run("cursor_left_box");
        e.assert_box((0, 2), (0, 1));
        assert_eq!(e.cursor().caret(), Pos::new(0, 1));
    }
}
