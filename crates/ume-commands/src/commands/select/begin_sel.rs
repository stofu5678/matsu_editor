//! `F_BEGIN_SEL`: starts (or ends) a selection with the plain moves.
//!
//! Sakura: `CViewCommander::Command_BEGIN_SELECT` in `sakura_core/cmd/CViewCommander_Select.cpp`,
//! called from `CViewCommander::HandleCommand`. It flips the selection lock (`m_bSelectingLock`):
//! with the lock on, the cursor moves, which are plain moves otherwise, extend the selection from
//! where the caret was. A second `F_BEGIN_SEL` switches the lock off and keeps the selection.
//! Anything that drops the selection (a move without the lock, typing, `F_CANCEL_MODE`) ends the
//! lock too. `F_BEGIN_BOX` does the same for a box selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct BeginSel;

impl Command for BeginSel {
    const ID: &'static str = "begin_sel";
    const SAKURA: &'static str = "F_BEGIN_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.toggle_selecting_lock();
        Ok(())
    }
}

register_command!(BeginSel);

#[cfg(test)]
mod tests {
    use crate::command::Arg;
    use crate::testing::Editor;

    #[test]
    fn locks_the_selection_and_the_plain_moves_extend_it() {
        let mut e = Editor::new("ab|cd\nefgh\nij");
        e.run("begin_sel");
        assert!(e.cursor().selecting_lock());
        assert!(!e.cursor().has_selection());
        e.assert_marked("ab|cd\nefgh\nij");
        e.run("cursor_right").assert_marked("ab[c|d\nefgh\nij");
        e.run("cursor_down").assert_marked("ab[cd\nefg|h\nij");
        e.run("go_file_end").assert_marked("ab[cd\nefgh\nij|");
        e.run("go_line_top").assert_marked("ab[cd\nefgh\n|ij");
    }

    #[test]
    fn a_second_call_unlocks_and_keeps_the_selection() {
        let mut e = Editor::new("ab|cd");
        e.run("begin_sel").run("cursor_right").run("begin_sel");
        assert!(!e.cursor().selecting_lock());
        e.assert_marked("ab[c|d");
        // now a plain move drops the selection and goes to its end
        e.run("cursor_right").assert_marked("abc|d");
    }

    #[test]
    fn the_lock_starts_from_an_existing_selection() {
        let mut e = Editor::new("a[bc|d");
        e.run("begin_sel").run("cursor_right");
        e.assert_marked("a[bcd|");
    }

    #[test]
    fn works_on_japanese_text_and_crlf() {
        let mut e = Editor::new("日|本\r\n語");
        e.run("begin_sel").run("cursor_right").run("cursor_right");
        e.assert_marked("日[本\r\n|語");
    }

    #[test]
    fn typing_and_cancelling_end_the_lock() {
        let mut e = Editor::new("ab|cd");
        e.run("begin_sel");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.assert_marked("abx|cd");
        assert!(!e.cursor().selecting_lock());
        let mut e = Editor::new("ab|cd");
        e.run("begin_sel").run("cancel_mode");
        assert!(!e.cursor().selecting_lock());
        e.run("cursor_right").assert_marked("abc|d");
    }

    #[test]
    fn it_is_not_an_edit() {
        let mut e = Editor::new("ab|cd");
        e.run("begin_sel");
        assert!(!e.changes().text_changed());
        assert!(!e.model().is_modified());
    }
}
