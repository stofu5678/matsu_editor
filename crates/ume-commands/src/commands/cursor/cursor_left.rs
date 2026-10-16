//! `F_LEFT`: moves the caret one char left. With a selection it goes to the start of the selection instead.
//!
//! Sakura: `CViewCommander::Command_LEFT` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_BOX`; `F_BEGIN_SEL` is not implemented yet) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorLeft;

impl Command for CursorLeft {
    const ID: &'static str = "cursor_left";
    const SAKURA: &'static str = "F_LEFT";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_left(false)
    }
}

register_command!(CursorLeft);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("cursor_left", "ab|cd", "a|bcd");
        check("cursor_left", "|ab", "|ab");
        check("cursor_left", "日|本", "|日本");
        check("cursor_left", "😀|a", "|😀a");
        check("cursor_left", "ab\r\n|cd", "ab|\r\ncd");
        check("cursor_left", "ab\n|", "ab|\n");
        check("cursor_left", "a[bc|d", "a|bcd");
        check("cursor_left", "a|bc[d", "a|bcd");
    }
}
