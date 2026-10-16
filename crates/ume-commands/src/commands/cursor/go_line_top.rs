//! `F_GOLINETOP`: moves the caret to the first char that is not indentation, or to the start of the row if it is already there.
//!
//! Sakura: `CViewCommander::Command_GOLINETOP` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_SEL` or `F_BEGIN_BOX`) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoLineTop;

impl Command for GoLineTop {
    const ID: &'static str = "go_line_top";
    const SAKURA: &'static str = "F_GOLINETOP";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_line_top(false)
    }
}

register_command!(GoLineTop);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("go_line_top", "  ab|c", "  |abc");
        check("go_line_top", "  |abc", "|  abc");
        check("go_line_top", "|  abc", "  |abc");
        check("go_line_top", "x\n  ab|c", "x\n  |abc");
        check("go_line_top", "x\r\n\tab|c", "x\r\n\t|abc");
        check("go_line_top", "\u{3000}ab|c", "\u{3000}|abc");
        check("go_line_top", "  | ", "|   ");
        check("go_line_top", "|", "|");
        check("go_line_top", "abc|", "|abc");
        check("go_line_top", "[ab|c", "|abc");
    }
}
