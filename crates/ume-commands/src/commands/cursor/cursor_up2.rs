//! `F_UP2`: moves the caret two rows up, to the display column it had before the first vertical
//! move.
//!
//! Sakura: `CViewCommander::Command_UP2` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, which
//! calls `CCaret::Cursor_UPDOWN(-2)` (`sakura_core/view/CCaret.cpp`). Near the top it moves as
//! far as it can: from the second row it goes one row up.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_BOX`; `F_BEGIN_SEL` is not implemented yet) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorUp2;

impl Command for CursorUp2 {
    const ID: &'static str = "cursor_up2";
    const SAKURA: &'static str = "F_UP2";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_vertical(-2, false)
    }
}

register_command!(CursorUp2);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn moves_two_rows() {
        check("cursor_up2", "ab\ncd\nef|\ngh", "ab|\ncd\nef\ngh");
        check("cursor_up2", "abc\nd\nef\ngh|i", "abc\nd|\nef\nghi");
        check("cursor_up2", "日本\nab\ncd|ef", "日|本\nab\ncdef");
        // from the second row it goes as far as there is
        check("cursor_up2", "ab\nc|d", "a|b\ncd");
        check("cursor_up2", "ab|", "ab|");
        check("cursor_up2", "a[b\ncd\ne|f", "a|b\ncd\nef");
    }

    #[test]
    fn remembers_the_column() {
        let mut e = Editor::new("abcdef\nabcdef\nab\nx\nabcdef|");
        e.run("cursor_up2")
            .assert_marked("abcdef\nabcdef\nab|\nx\nabcdef");
        e.run("cursor_up2")
            .assert_marked("abcdef|\nabcdef\nab\nx\nabcdef");
    }
}
