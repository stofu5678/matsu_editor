//! `F_CANCEL_MODE`: cancels the selection, the selection lock and a box selection (Esc).
//!
//! Sakura: `CViewCommander::Command_CANCEL_MODE` in `sakura_core/cmd/CViewCommander_ModeChange.cpp`,
//! called from `CViewCommander::HandleCommand` with the default `whereCursorIs = 0`: the caret
//! stays where it is. The rules are in `CommandContext::cancel_mode`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CancelMode;

impl Command for CancelMode {
    const ID: &'static str = "cancel_mode";
    const SAKURA: &'static str = "F_CANCEL_MODE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.cancel_mode()
    }
}

register_command!(CancelMode);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn drops_the_selection_and_keeps_the_caret() {
        check("cancel_mode", "a[bc|d", "abc|d");
        check("cancel_mode", "a|bc[d", "a|bcd");
        check("cancel_mode", "日[本\r\n語|", "日本\r\n語|");
    }

    #[test]
    fn nothing_to_cancel_is_not_an_error() {
        check("cancel_mode", "ab|cd", "ab|cd");
        let mut e = Editor::new("ab|cd");
        e.run("cancel_mode");
        assert!(!e.changes().beep);
        assert!(!e.changes().selection_changed);
    }

    #[test]
    fn switches_off_the_lock_that_selects_nothing_yet() {
        let mut e = Editor::new("ab|cd");
        e.run("begin_sel");
        assert!(e.cursor().selecting_lock());
        e.run("cancel_mode");
        assert!(!e.cursor().selecting_lock());
    }

    #[test]
    fn ends_a_box_selection() {
        let mut e = Editor::new("|abcd\nefgh\nijkl");
        e.run("begin_box").run("cursor_down").run("cursor_right");
        assert!(e.cursor().is_box());
        e.run("cancel_mode");
        assert!(!e.cursor().is_box());
        assert!(!e.cursor().has_selection());
        assert!(!e.cursor().selecting_lock());
        assert!(e.changes().selection_changed);
        // a box that covers nothing too
        let mut e = Editor::new("ab|cd");
        e.run("begin_box").run("cancel_mode");
        assert!(!e.cursor().is_box());
        assert!(!e.cursor().selecting_lock());
    }

    #[test]
    fn the_goal_column_follows_the_caret() {
        // a vertical move after cancelling aims for the column of the caret, not for the column
        // the earlier vertical move remembered (`m_nCaretPosX_Prev`)
        let mut e = Editor::new("abcdef|\nab\nabcdef");
        e.run("cursor_down").assert_marked("abcdef\nab|\nabcdef");
        e.run("begin_sel").run("cancel_mode");
        e.run("cursor_down").assert_marked("abcdef\nab\nab|cdef");
        // without the cancel the goal column is kept
        let mut e = Editor::new("abcdef|\nab\nabcdef");
        e.run("cursor_down").run("cursor_down");
        e.assert_marked("abcdef\nab\nabcdef|");
    }
}
