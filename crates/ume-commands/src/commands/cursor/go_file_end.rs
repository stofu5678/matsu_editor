//! `F_GOFILEEND`: moves the caret to the end of the document.
//!
//! Sakura: `CViewCommander::Command_GOFILEEND` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_SEL` or `F_BEGIN_BOX`) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoFileEnd;

impl Command for GoFileEnd {
    const ID: &'static str = "go_file_end";
    const SAKURA: &'static str = "F_GOFILEEND";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_file_end(false)
    }
}

register_command!(GoFileEnd);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("go_file_end", "a|b\ncd", "ab\ncd|");
        check("go_file_end", "a|b\ncd\r\n", "ab\ncd\r\n|");
        check("go_file_end", "|", "|");
        check("go_file_end", "a|b\nc[d", "ab\ncd|");
    }
}
