//! `F_APPENDUNDOBUFFERCURSOR` (the macro `AppendUndoBufferCursor()`): adds the caret position to
//! the undo step that is being made.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`, `F_APPENDUNDOBUFFERCURSOR`)
//! makes the operation block if there is none, adds one to its reference count, appends a
//! `CMoveCaretOpe` with the position of the caret (an operation that moves the caret and changes
//! no text), and calls `CEditView::SetUndoBuffer` (`sakura_core/view/CEditView.cpp`), which
//! stores the block with `COpeBuf::AppendOpeBlk` when the count reaches 0. So a macro that only
//! moved the caret becomes a step that undo goes back over, and undo puts the caret back.
//!
//! The step does not make the document modified: neither `SetUndoBuffer` nor
//! `CMacroBeforeAfter::ExecKeyMacroAfter` (`macro/CMacroManagerBase.cpp`) calls
//! `CDocEditor::SetModified`. The flag `m_bIsDocModified` is set by the edit commands, cleared by
//! a save, and set by undo and redo as `m_nCurrentPointer != m_nNoModifiedIndex`
//! (`COpeBuf::DoUndo`/`DoRedo` in `cmd/COpeBuf.cpp`, used in `cmd/CViewCommander_Edit.cpp`). So
//! an unmodified document stays unmodified; undoing the step is unmodified, redoing it is
//! modified; an edit after it is modified, and undoing that edit (back on the caret step) still
//! is. A modified document stays modified. See `matsu_core::undo::History::note_caret_step`.

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
    fn an_unmodified_document_stays_unmodified() {
        let mut e = Editor::new("a|b");
        e.run("appendundobuffercursor");
        assert!(!e.model().is_modified());
        assert!(!e.changes().text_changed());
        // the lines are not marked as modified either
        assert!(!e.model().is_line_modified(0));
        assert!(e.model().history().can_undo());
    }

    #[test]
    fn undo_of_the_step_is_unmodified_and_redo_is_modified() {
        let mut e = Editor::new("a|b\ncd");
        e.run("appendundobuffercursor").run("go_file_end");
        e.run("undo");
        e.assert_marked("a|b\ncd");
        assert!(!e.model().is_modified());
        e.run("redo");
        assert!(e.model().is_modified());
        // undo again: back to the saved position
        e.run("undo");
        assert!(!e.model().is_modified());
    }

    #[test]
    fn typing_after_the_step_is_modified_and_so_is_undoing_it() {
        let mut e = Editor::new("a|b");
        e.run("appendundobuffercursor");
        e.run_with("input_char", &[Arg::Char('x')]);
        assert!(e.model().is_modified());
        assert!(e.model().is_line_modified(0));
        e.run("undo");
        e.assert_marked("a|b");
        // back on the caret step, which is not the saved position
        assert!(e.model().is_modified());
        e.run("undo");
        assert!(!e.model().is_modified());
    }

    #[test]
    fn a_modified_document_stays_modified() {
        let mut e = Editor::new("a|b");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.run("appendundobuffercursor");
        assert!(e.model().is_modified());
        e.run("undo");
        assert!(e.model().is_modified());
        e.run("undo");
        assert!(!e.model().is_modified());
    }

    #[test]
    fn after_a_save_undoing_the_step_is_modified() {
        let mut e = Editor::new("a|b");
        e.run("appendundobuffercursor");
        assert!(!e.model().is_modified());
        e.mark_saved();
        assert!(!e.model().is_modified());
        e.run("undo");
        assert!(e.model().is_modified());
        e.run("redo");
        assert!(!e.model().is_modified());
    }

    #[test]
    fn two_steps_in_a_row_on_an_unmodified_document() {
        let mut e = Editor::new("a|b");
        e.run("appendundobuffercursor").run("go_file_end");
        e.run("appendundobuffercursor");
        assert!(!e.model().is_modified());
        // undo recomputes the flag from the position: on the first step it is not the saved one
        e.run("undo");
        assert!(e.model().is_modified());
        e.run("undo");
        assert!(!e.model().is_modified());
    }

    #[test]
    fn the_marks_of_the_modified_lines_do_not_change() {
        let mut e = Editor::new("a|b\ncd");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.mark_saved();
        let before = (e.model().is_line_modified(0), e.model().is_line_modified(1));
        e.run("appendundobuffercursor");
        assert_eq!(
            (e.model().is_line_modified(0), e.model().is_line_modified(1)),
            before
        );
        e.run_with("input_char", &[Arg::Char('y')]);
        assert!(e.model().is_line_modified(0));
        assert!(!e.model().is_line_modified(1));
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
