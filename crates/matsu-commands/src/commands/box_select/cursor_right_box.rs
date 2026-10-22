//! `F_RIGHT_BOX`: moves the caret one char right and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_RIGHT` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorRightBox;

impl Command for CursorRightBox {
    const ID: &'static str = "cursor_right_box";
    const SAKURA: &'static str = "F_RIGHT_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_right(true)
    }
}

register_command!(CursorRightBox);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Pos;
    use matsu_core::cursor::Cursor;

    use crate::context::MAX_BOX_COL;
    use crate::testing::{Editor, check_box};

    #[test]
    fn moves_right_and_begins_the_box() {
        check_box("cursor_right_box", "a|bc", (0, 1), (0, 2));
        check_box("cursor_right_box", "|日本", (0, 0), (0, 2));
        check_box("cursor_right_box", "|\ta", (0, 0), (0, 4));
    }

    #[test]
    fn goes_on_through_the_empty_space_but_not_to_the_next_row() {
        let mut e = Editor::new("ab|\ncd");
        e.run("cursor_right_box").assert_box((0, 2), (0, 3));
        // the position is the end of the row
        assert_eq!(e.cursor().caret(), Pos::new(0, 2));
        e.run("cursor_right_box").assert_box((0, 2), (0, 4));
        // down, in the same column: on the other row it is over the text
        e.run("cursor_down_box").assert_box((0, 2), (1, 4));
        assert_eq!(e.cursor().caret(), Pos::new(1, 2));
    }

    #[test]
    fn stops_at_the_longest_line() {
        let mut cursor = Cursor::new(Pos::new(0, 2));
        cursor.begin_box(MAX_BOX_COL - 1, true);
        let mut e = Editor::from_parts("ab", cursor);
        e.run("cursor_right_box")
            .assert_box((0, MAX_BOX_COL - 1), (0, MAX_BOX_COL));
        e.run("cursor_right_box")
            .assert_box((0, MAX_BOX_COL - 1), (0, MAX_BOX_COL));
    }
}
