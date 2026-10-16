//! `F_GOLINEEND`: moves the caret to the end of the row, before the line ending.
//!
//! Sakura: `CViewCommander::Command_GOLINEEND` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_BOX`; `F_BEGIN_SEL` is not implemented yet) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoLineEnd;

impl Command for GoLineEnd {
    const ID: &'static str = "go_line_end";
    const SAKURA: &'static str = "F_GOLINEEND";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_line_end(false)
    }
}

register_command!(GoLineEnd);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn moves() {
        check("go_line_end", "ab|cd\r\nef", "abcd|\r\nef");
        check("go_line_end", "ab|cd", "abcd|");
        check("go_line_end", "a|b\n\nc", "ab|\n\nc");
        check("go_line_end", "|\n", "|\n");
        check("go_line_end", "[ab|cd", "abcd|");
    }

    #[test]
    fn the_column_of_the_end_is_the_goal_of_the_next_vertical_move() {
        // Sakura sets the goal column to the column of the end (`m_nCaretPosX_Prev = nPosXY.x`)
        let mut e = Editor::new("ab|cd\nabcdefgh");
        e.run("go_line_end").run("cursor_down");
        e.assert_marked("abcd\nabcd|efgh");
    }
}
