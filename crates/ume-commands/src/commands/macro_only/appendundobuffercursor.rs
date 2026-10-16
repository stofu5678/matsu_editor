//! `F_APPENDUNDOBUFFERCURSOR` (the macro `AppendUndoBufferCursor()`): adds the caret position to
//! the undo step that is being made.
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp` makes the operation block if
//! there is none, adds one to its reference count, appends a `CMoveCaretOpe` with the position of
//! the caret (an operation that moves the caret and changes no text), and calls
//! `CEditView::SetUndoBuffer` (`sakura_core/view/CEditView.cpp`). A block with an operation is
//! stored when the count reaches 0, so a macro that only moved the caret becomes a step that
//! undo goes back over, and undo puts the caret back. That step moves the pointer of the undo
//! buffer, so the document counts as modified after it (`COpeBuf::IsModified`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct AppendUndoBufferCursor;

impl Command for AppendUndoBufferCursor {
    const ID: &'static str = "appendundobuffercursor";
    const SAKURA: &'static str = "F_APPENDUNDOBUFFERCURSOR";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.begin_block();
        cx.add_caret_step();
        cx.end_block();
        Ok(())
    }
}

register_command!(AppendUndoBufferCursor);

#[cfg(test)]
mod tests {
    use crate::command::Arg;
    use crate::testing::Editor;

    #[test]
    fn a_caret_move_becomes_an_undo_step() {
        let mut e = Editor::new("a|bc\ndef");
        e.run("appendundobuffercursor");
        assert!(e.model().history().can_undo());
        e.run("go_file_end");
        e.assert_marked("abc\ndef|");
        e.run("undo");
        e.assert_marked("a|bc\ndef");
        assert!(!e.model().history().can_undo());
        e.run("redo");
        e.assert_marked("a|bc\ndef");
    }

    #[test]
    fn the_text_is_the_same_but_the_document_counts_as_modified() {
        let mut e = Editor::new("a|b");
        e.run("appendundobuffercursor");
        assert!(e.model().is_modified());
        assert!(!e.changes().text_changed());
        e.run("undo");
        assert!(!e.model().is_modified());
    }

    #[test]
    fn it_joins_the_open_block() {
        let mut e = Editor::new("|");
        e.run("addrefundobuffer");
        e.run_with("input_char", &[Arg::Char('a')]);
        e.run("appendundobuffercursor");
        // The block is not stored yet: the count did not reach 0.
        assert_eq!(e.model().history().depth(), 1);
        e.run("setundobuffer");
        e.run("undo");
        e.assert_marked("|");
        assert!(!e.model().history().can_undo());
    }

    #[test]
    fn two_calls_are_two_steps() {
        let mut e = Editor::new("a|b");
        e.run("appendundobuffercursor");
        e.run("go_file_end");
        e.run("appendundobuffercursor");
        e.run("undo");
        e.assert_marked("ab|");
        e.run("undo");
        e.assert_marked("a|b");
    }
}
