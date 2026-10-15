//! `F_LineDeleteToStart`: deletes from the caret back to the start of the line.
//!
//! Sakura: `CViewCommander::Command_LineDeleteToStart` in
//! `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`; bound to Ctrl+Shift+U in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * "Line" is the line of the document (up to the line ending), not the row on the screen:
//!   with wrapping it deletes back to the start of the first row.
//! * With a selection it deletes the selection, as the Delete key does.
//! * At the start of the line, and on the empty line after a final line ending, it beeps.

use ume_core::buffer::Pos;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct LineDeleteToStart;

impl Command for LineDeleteToStart {
    const ID: &'static str = "line_delete_to_start";
    const SAKURA: &'static str = "F_LineDeleteToStart";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        if cx.delete_selection()? {
            return Ok(());
        }
        let caret = cx.caret();
        if caret.line >= cx.doc().line_count() || caret.col == 0 {
            cx.beep();
            return Ok(());
        }
        cx.delete_range(Pos::new(caret.line, 0), caret)
    }
}

register_command!(LineDeleteToStart);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn deletes_back_to_the_start_of_the_line() {
        check("line_delete_to_start", "ab|cd\r\nef", "|cd\r\nef");
        check("line_delete_to_start", "x\nab|", "x\n|");
        check("line_delete_to_start", "x\n日本|語", "x\n|語");
        check("line_delete_to_start", "ab|\n", "|\n");
    }

    #[test]
    fn deletes_the_selection() {
        check("line_delete_to_start", "a[bc|d", "a|d");
        check("line_delete_to_start", "a|bc[d", "a|d");
        check("line_delete_to_start", "a[b\nc|d", "a|d");
    }

    #[test]
    fn beeps_at_the_start_of_a_line_and_on_the_last_empty_line() {
        for text in ["|ab", "x\n|ab", "ab\n|", "|"] {
            let mut e = Editor::new(text);
            e.run("line_delete_to_start").assert_marked(text);
            assert!(e.changes().beep, "{text:?}");
            assert!(!e.changes().text_changed(), "{text:?}");
        }
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("ab|cd");
        e.run("line_delete_to_start").assert_marked("|cd");
        e.run("undo").assert_marked("ab|cd");
    }
}
