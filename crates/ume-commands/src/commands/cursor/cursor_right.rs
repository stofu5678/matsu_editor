//! `F_RIGHT`: moves the caret one char right. With a selection it goes to the end of the selection instead.
//!
//! Sakura: `CViewCommander::Command_RIGHT` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`, switched by `F_BEGIN_SEL`) as `bSelect`
//! here. That mode is not implemented yet, so this command never extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorRight;

impl Command for CursorRight {
    const ID: &'static str = "cursor_right";
    const SAKURA: &'static str = "F_RIGHT";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_right(false)
    }
}

register_command!(CursorRight);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("cursor_right", "ab|cd", "abc|d");
        check("cursor_right", "ab|", "ab|");
        check("cursor_right", "|日本", "日|本");
        check("cursor_right", "|😀a", "😀|a");
        check("cursor_right", "ab|\r\ncd", "ab\r\n|cd");
        check("cursor_right", "ab|\n", "ab\n|");
        check("cursor_right", "ab\n|", "ab\n|");
        check("cursor_right", "a[bc|d", "abc|d");
        check("cursor_right", "a|bc[d", "abc|d");
    }
}
