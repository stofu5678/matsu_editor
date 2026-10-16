//! `F_SETUNDOBUFFER` (the macro `SetUndoBuffer()`): leaves the undo block once more, which ends
//! the one step that `AddRefUndoBuffer()` began when the count is 0.
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp` calls
//! `CEditView::SetUndoBuffer` (`sakura_core/view/CEditView.cpp`): it lowers the reference count of
//! the operation block, and when the count reaches 0 it stores the block as an undo step (a block
//! with no operation is dropped). The call that every command makes after it runs lowers the count
//! once more; a call without a block does nothing.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct SetUndoBuffer;

impl Command for SetUndoBuffer {
    const ID: &'static str = "setundobuffer";
    const SAKURA: &'static str = "F_SETUNDOBUFFER";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.end_block();
        Ok(())
    }
}

register_command!(SetUndoBuffer);

#[cfg(test)]
mod tests {
    use crate::command::Arg;
    use crate::testing::Editor;

    #[test]
    fn without_a_block_it_does_nothing() {
        let mut e = Editor::new("a|b");
        e.run("setundobuffer");
        e.assert_marked("a|b");
        assert_eq!(e.model().history().depth(), 0);
        assert!(!e.model().history().can_undo());
        assert!(!e.changes().beep);
    }

    #[test]
    fn it_stores_the_block_when_the_count_reaches_zero() {
        let mut e = Editor::new("|");
        e.run("addrefundobuffer");
        e.run_with("input_char", &[Arg::Char('a')]);
        assert_eq!(e.model().history().depth(), 1);
        e.run("setundobuffer");
        assert_eq!(e.model().history().depth(), 0);
        assert!(e.model().history().can_undo());
    }

    #[test]
    fn a_block_without_an_edit_is_dropped() {
        let mut e = Editor::new("a|b");
        e.run("addrefundobuffer").run("setundobuffer");
        assert!(!e.model().history().can_undo());
        assert!(!e.model().is_modified());
    }

    #[test]
    fn the_edits_of_a_command_that_ends_the_block_are_kept() {
        // The command that closes the block is itself inside the block of its own call.
        let mut e = Editor::new("|");
        e.run("addrefundobuffer");
        e.run_with("input_char", &[Arg::Char('a')]);
        e.run("setundobuffer");
        e.run_with("input_char", &[Arg::Char('b')]);
        e.run("undo");
        e.assert_marked("a|");
        e.run("undo");
        e.assert_marked("|");
    }
}
