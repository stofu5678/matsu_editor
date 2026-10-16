//! `F_COMMITUNDOBUFFER` (the macro `CommitUndoBuffer()`): ends the undo step that is being made
//! and starts a new one, without changing how deep the block is entered.
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp`: it reads the reference
//! count of the operation block, sets it to 1, calls `CEditView::SetUndoBuffer` (which stores the
//! block, or drops it when it has no operation), and makes a new block with the old count if the
//! old count was above 0. With `AddRefUndoBuffer()` before it, the edits so far become one step
//! and the edits after it the next one.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CommitUndoBuffer;

impl Command for CommitUndoBuffer {
    const ID: &'static str = "commitundobuffer";
    const SAKURA: &'static str = "F_COMMITUNDOBUFFER";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.commit_undo_block();
        Ok(())
    }
}

register_command!(CommitUndoBuffer);

#[cfg(test)]
mod tests {
    use crate::command::Arg;
    use crate::testing::Editor;

    fn type_char(e: &mut Editor, c: char) {
        e.run_with("input_char", &[Arg::Char(c)]);
    }

    #[test]
    fn it_splits_one_block_into_two_steps() {
        let mut e = Editor::new("|");
        e.run("addrefundobuffer");
        type_char(&mut e, 'a');
        type_char(&mut e, 'b');
        e.run("commitundobuffer");
        type_char(&mut e, 'c');
        // The block is still open at the same depth.
        assert_eq!(e.model().history().depth(), 1);
        e.run("setundobuffer");
        assert_eq!(e.model().history().depth(), 0);
        e.run("undo");
        e.assert_marked("ab|");
        e.run("undo");
        e.assert_marked("|");
        assert!(!e.model().history().can_undo());
    }

    #[test]
    fn it_keeps_the_depth() {
        let mut e = Editor::new("|");
        e.run("addrefundobuffer").run("addrefundobuffer");
        type_char(&mut e, 'a');
        e.run("commitundobuffer");
        assert_eq!(e.model().history().depth(), 2);
    }

    #[test]
    fn with_nothing_to_store_it_does_nothing() {
        let mut e = Editor::new("a|b");
        e.run("commitundobuffer");
        assert!(!e.model().history().can_undo());
        e.run("addrefundobuffer").run("commitundobuffer");
        assert!(!e.model().history().can_undo());
        assert_eq!(e.model().history().depth(), 1);
    }

    #[test]
    fn the_next_step_starts_where_the_commit_was_even_without_an_edit_before_it() {
        let mut e = Editor::new("|ab\ncd");
        e.run("addrefundobuffer");
        // only the caret moved so far: there is nothing to store
        e.run("go_file_end");
        e.run("commitundobuffer");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.run("setundobuffer");
        e.assert_marked("ab\ncdx|");
        // undo puts the caret back to where the commit was, not to where the block began
        e.run("undo");
        e.assert_marked("ab\ncd|");
    }

    #[test]
    fn the_stored_step_can_be_undone_while_the_block_is_open() {
        let mut e = Editor::new("|");
        e.run("addrefundobuffer");
        type_char(&mut e, 'a');
        e.run("commitundobuffer");
        assert!(e.model().is_modified());
        e.run("undo");
        e.assert_marked("|");
    }
}
