//! `F_GOFILETOP`: moves the caret to the start of the document.
//!
//! Sakura: `CViewCommander::Command_GOFILETOP` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_BOX`; `F_BEGIN_SEL` is not implemented yet) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoFileTop;

impl Command for GoFileTop {
    const ID: &'static str = "go_file_top";
    const SAKURA: &'static str = "F_GOFILETOP";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_file_top(false)
    }
}

register_command!(GoFileTop);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("go_file_top", "ab\ncd|", "|ab\ncd");
        check("go_file_top", "|ab", "|ab");
        check("go_file_top", "a[b\nc|d", "|ab\ncd");
        check("go_file_top", "|", "|");
    }
}
