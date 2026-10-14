//! `F_UP`: moves the caret one row up, to the display column it had before the first vertical move.
//!
//! Sakura: `CViewCommander::Command_UP` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`, switched by `F_BEGIN_SEL`) as `bSelect`
//! here. That mode is not implemented yet, so this command never extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorUp;

impl Command for CursorUp {
    const ID: &'static str = "cursor_up";
    const SAKURA: &'static str = "F_UP";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_vertical(-1, false)
    }
}

register_command!(CursorUp);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn moves() {
        check("cursor_up", "ab\ncd|", "ab|\ncd");
        check("cursor_up", "abcd\nef|", "ab|cd\nef");
        check("cursor_up", "ab|cd", "ab|cd");
        check("cursor_up", "日本\nabc|", "日|本\nabc");
        check("cursor_up", "abcdefgh\n\tx|", "abcde|fgh\n\tx");
        check("cursor_up", "a[b\nc|d", "a|b\ncd");
    }

    #[test]
    fn remembers_the_column_over_short_rows() {
        let mut e = Editor::new("abcdef\nab\nabcdef|");
        e.run("cursor_up").assert_marked("abcdef\nab|\nabcdef");
        e.run("cursor_up").assert_marked("abcdef|\nab\nabcdef");
        // any other move forgets the column
        e.run("cursor_left").run("cursor_down").run("cursor_down");
        e.assert_marked("abcdef\nab\nabcde|f");
    }
}
