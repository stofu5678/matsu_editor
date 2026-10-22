//! `F_DOWN2`: moves the caret two rows down, to the display column it had before the first
//! vertical move.
//!
//! Sakura: `CViewCommander::Command_DOWN2` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, which
//! calls `CCaret::Cursor_UPDOWN(2)` (`sakura_core/view/CCaret.cpp`). Near the bottom it moves as
//! far as it can; the empty row after a final line ending counts as a row.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_SEL` or `F_BEGIN_BOX`) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorDown2;

impl Command for CursorDown2 {
    const ID: &'static str = "cursor_down2";
    const SAKURA: &'static str = "F_DOWN2";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_vertical(2, false)
    }
}

register_command!(CursorDown2);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn moves_two_rows() {
        check("cursor_down2", "a|b\ncd\nef\ngh", "ab\ncd\ne|f\ngh");
        check("cursor_down2", "日|本\nab\ncdef", "日本\nab\ncd|ef");
        // near the bottom it goes as far as there is
        check("cursor_down2", "a|b\ncd", "ab\nc|d");
        check("cursor_down2", "a|b\ncd\n", "ab\ncd\n|");
        check("cursor_down2", "ab|", "ab|");
        check("cursor_down2", "a[b\nc|d\nef\ngh", "ab\ncd\nef\ng|h");
    }

    #[test]
    fn remembers_the_column() {
        let mut e = Editor::new("abcdef|\nx\nab\nabcdef\nabcdef");
        e.run("cursor_down2")
            .assert_marked("abcdef\nx\nab|\nabcdef\nabcdef");
        e.run("cursor_down2")
            .assert_marked("abcdef\nx\nab\nabcdef\nabcdef|");
    }
}
