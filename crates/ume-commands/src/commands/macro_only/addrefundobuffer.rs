//! `F_ADDREFUNDOBUFFER` (the macro `AddRefUndoBuffer()`): from now on, the commands that follow
//! are one undo step, until the same number of `SetUndoBuffer()` calls ends it.
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp` makes the operation block
//! (`COpeBlk`) of the view if there is none and adds one to its reference count. Every command
//! (`CViewCommander::HandleCommand`) does the same before it runs and calls
//! `CEditView::SetUndoBuffer` after it; that call lowers the count and stores the block as an
//! undo step when the count is 0. So a block that a macro entered stays open after the command
//! that entered it, and the edits of the following commands go into it.
//!
//! Here the undo history keeps the same count (`ume_core::undo::History::begin`), and the
//! registry opens and leaves a block around each command, so this command only enters the block
//! once more. A macro that enters more often than it leaves does not keep the block open past
//! its end: the window leaves it (`CommandContext::close_undo_blocks`, Sakura's
//! `CMacroBeforeAfter::ExecKeyMacroAfter`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct AddRefUndoBuffer;

impl Command for AddRefUndoBuffer {
    const ID: &'static str = "addrefundobuffer";
    const SAKURA: &'static str = "F_ADDREFUNDOBUFFER";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.begin_block();
        Ok(())
    }
}

register_command!(AddRefUndoBuffer);

#[cfg(test)]
mod tests {
    use crate::command::Arg;
    use crate::testing::Editor;

    fn type_char(e: &mut Editor, c: char) {
        e.run_with("input_char", &[Arg::Char(c)]);
    }

    #[test]
    fn the_commands_after_it_are_one_undo_step() {
        let mut e = Editor::new("|");
        e.run("addrefundobuffer");
        type_char(&mut e, 'a');
        type_char(&mut e, 'b');
        type_char(&mut e, 'c');
        e.run("setundobuffer");
        e.assert_marked("abc|");
        e.run("undo");
        e.assert_marked("|");
        assert!(!e.model().history().can_undo());
        e.run("redo");
        e.assert_marked("abc|");
    }

    #[test]
    fn without_it_every_command_is_a_step() {
        let mut e = Editor::new("|");
        type_char(&mut e, 'a');
        type_char(&mut e, 'b');
        e.run("undo");
        e.assert_marked("a|");
    }

    #[test]
    fn two_calls_need_two_set_calls() {
        let mut e = Editor::new("|");
        e.run("addrefundobuffer").run("addrefundobuffer");
        assert_eq!(e.model().history().depth(), 2);
        type_char(&mut e, 'a');
        e.run("setundobuffer");
        assert_eq!(e.model().history().depth(), 1);
        // Still open: the next character joins the first.
        type_char(&mut e, 'b');
        e.run("setundobuffer");
        assert_eq!(e.model().history().depth(), 0);
        e.run("undo");
        e.assert_marked("|");
    }

    #[test]
    fn the_block_is_one_modification_of_the_document() {
        let mut e = Editor::new("|");
        e.run("addrefundobuffer");
        type_char(&mut e, 'a');
        assert!(e.model().is_modified());
        e.run("setundobuffer");
        assert!(e.model().is_modified());
    }

    #[test]
    fn the_end_of_a_macro_leaves_the_blocks_that_stayed_open() {
        let mut e = Editor::new("|");
        e.run("addrefundobuffer").run("addrefundobuffer");
        type_char(&mut e, 'x');
        e.with_context(|cx| {
            assert_eq!(cx.undo_depth(), 2);
            cx.close_undo_blocks();
            assert_eq!(cx.undo_depth(), 0);
        });
        // The typed character is stored as a step.
        e.run("undo");
        e.assert_marked("|");
    }
}
