//! `F_LineDeleteToEnd`: deletes from the caret to the end of the line, before the line ending.
//!
//! Sakura: `CViewCommander::Command_LineDeleteToEnd` in
//! `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`; bound to Ctrl+Shift+K in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * "Line" is the line of the document (up to the line ending), not the row on the screen:
//!   with wrapping it deletes to the end of the last row.
//! * With a selection it deletes the selection, as the Delete key does.
//! * At the end of the line (the line ending stays), and on the empty line after a final line
//!   ending, it beeps.

use matsu_core::buffer::Pos;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct LineDeleteToEnd;

impl Command for LineDeleteToEnd {
    const ID: &'static str = "line_delete_to_end";
    const SAKURA: &'static str = "F_LineDeleteToEnd";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        if cx.delete_selection()? {
            return Ok(());
        }
        let caret = cx.caret();
        match cx.doc().line_len(caret.line)? {
            Some(end) if caret.col < end => cx.delete_range(caret, Pos::new(caret.line, end)),
            _ => {
                cx.beep();
                Ok(())
            }
        }
    }
}

register_command!(LineDeleteToEnd);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn deletes_to_the_end_of_the_line() {
        check("line_delete_to_end", "ab|cd\r\nef", "ab|\r\nef");
        check("line_delete_to_end", "x\n|ab", "x\n|");
        check("line_delete_to_end", "x\nab|日本語\ny", "x\nab|\ny");
        check("line_delete_to_end", "|ab\n", "|\n");
    }

    #[test]
    fn deletes_the_selection() {
        check("line_delete_to_end", "a[bc|d", "a|d");
        check("line_delete_to_end", "a|b\ncd[e", "a|e");
    }

    #[test]
    fn beeps_at_the_end_of_a_line_and_on_the_last_empty_line() {
        for text in ["ab|", "ab|\r\ncd", "|\nab", "ab\n|", "|"] {
            let mut e = Editor::new(text);
            e.run("line_delete_to_end").assert_marked(text);
            assert!(e.changes().beep, "{text:?}");
            assert!(!e.changes().text_changed(), "{text:?}");
        }
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("ab|cd");
        e.run("line_delete_to_end").assert_marked("ab|");
        e.run("undo").assert_marked("ab|cd");
    }
}
