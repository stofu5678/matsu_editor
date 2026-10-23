//! `F_UP_BOX`: moves the caret one row up and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_UP` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorUpBox;

impl Command for CursorUpBox {
    const ID: &'static str = "cursor_up_box";
    const SAKURA: &'static str = "F_UP_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_vertical(-1, true)
    }
}

register_command!(CursorUpBox);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Pos;

    use crate::testing::{Editor, check_box};

    #[test]
    fn moves_up_and_begins_the_box() {
        check_box("cursor_up_box", "abcd\nef|gh", (1, 2), (0, 2));
        // on the first row nothing moves, but the box is begun
        check_box("cursor_up_box", "ab|cd", (0, 2), (0, 2));
        // the column is in display columns
        check_box("cursor_up_box", "日本\nab|cd", (1, 2), (0, 2));
        check_box("cursor_up_box", "abc\r\nd|ef", (1, 1), (0, 1));
    }

    #[test]
    fn a_selection_is_dropped_when_the_box_begins() {
        let mut e = Editor::new("ab\ncd\ne[f|g");
        e.run("cursor_up_box");
        e.assert_box((2, 2), (1, 2));
        assert!(e.cursor().has_selection());
        assert_eq!(e.cursor().anchor(), Some(Pos::new(2, 2)));
    }

    #[test]
    fn goes_through_empty_space_to_the_goal_column() {
        // the short row in the middle does not make the caret forget its column
        let mut e = Editor::new("abcd\nx\nefg|h");
        e.run("cursor_up_box").assert_box((2, 3), (1, 3));
        assert_eq!(e.cursor().caret(), Pos::new(1, 1));
        e.run("cursor_up_box").assert_box((2, 3), (0, 3));
        assert_eq!(e.cursor().caret(), Pos::new(0, 3));
        e.run("cursor_down_box").assert_box((2, 3), (1, 3));
    }
}
