//! `F_UNDO`: undoes the last undo block (Ctrl+Z).
//!
//! Sakura: `CViewCommander::Command_UNDO` in `sakura_core/cmd/CViewCommander_Edit.cpp` and
//! `COpeBuf::DoUndo` in `sakura_core/cmd/COpeBuf.cpp`. One command is one block. Undo puts back
//! the text and the cursor from before the block; with nothing to undo it does nothing (no beep).
//!
//! Difference: Sakura puts the caret where the edit started and drops the selection; here the
//! cursor comes back as it was before the command, selection included, so undoing "delete the
//! selection" selects the text again. See `ume_core::undo` for how blocks are kept.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Undo;

impl Command for Undo {
    const ID: &'static str = "undo";
    const SAKURA: &'static str = "F_UNDO";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.refuse_in_view_mode()?;
        cx.undo();
        Ok(())
    }
}

register_command!(Undo);

#[cfg(test)]
mod tests {
    use crate::command::Arg;
    use crate::testing::{Editor, check};

    #[test]
    fn nothing_to_undo() {
        let mut e = Editor::new("ab|cd");
        e.run("undo").assert_marked("ab|cd");
        assert!(!e.changes().text_changed());
        assert!(!e.changes().beep);
        // moves are not undone
        e.run("cursor_left").run("undo").assert_marked("a|bcd");
        check("undo", "|", "|");
    }

    #[test]
    fn restores_the_text_and_the_cursor() {
        let mut e = Editor::new("abc|");
        e.run("delete_back").assert_marked("ab|");
        e.run("go_file_top").assert_marked("|ab");
        // the caret comes back to where the undone command had it
        e.run("undo").assert_marked("abc|");
        assert!(e.changes().replaced_all);
        assert!(e.changes().text_changed());
    }

    #[test]
    fn steps_back_one_block_at_a_time() {
        let mut e = Editor::new("|");
        for c in ['a', 'b', 'c'] {
            e.run_with("input_char", &[Arg::Char(c)]);
        }
        e.assert_marked("abc|");
        e.run("undo").assert_marked("ab|");
        e.run("undo").assert_marked("a|");
        e.run("undo").assert_marked("|");
        e.run("undo").assert_marked("|");
        assert!(!e.model().is_modified());
    }

    #[test]
    fn forgets_the_goal_column() {
        let mut e = Editor::new("abcdef|\nab\nabcdef");
        e.run("cursor_down").run("delete").run("undo");
        e.run("cursor_down").assert_marked("abcdef\nab\nab|cdef");
    }
}
