//! `F_DOWN`: moves the caret one row down, to the display column it had before the first vertical move.
//!
//! Sakura: `CViewCommander::Command_DOWN` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`, switched by `F_BEGIN_SEL`) as `bSelect`
//! here. That mode is not implemented yet, so this command never extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorDown;

impl Command for CursorDown {
    const ID: &'static str = "cursor_down";
    const SAKURA: &'static str = "F_DOWN";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_vertical(1, false)
    }
}

register_command!(CursorDown);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn moves() {
        check("cursor_down", "ab|cd\nefgh", "abcd\nef|gh");
        check("cursor_down", "a|b\r\ncd", "ab\r\nc|d");
        check("cursor_down", "ab|", "ab|");
        check("cursor_down", "ab|\n", "ab\n|");
        check("cursor_down", "日|本\nabcd", "日本\nab|cd");
        check("cursor_down", "a[b\nc|d\nef", "ab\ncd\ne|f");
    }

    #[test]
    fn remembers_the_column_over_empty_rows() {
        let mut e = Editor::new("abc|\n\nabc");
        e.run("cursor_down").assert_marked("abc\n|\nabc");
        e.run("cursor_down").assert_marked("abc\n\nabc|");
    }
}
