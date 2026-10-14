//! `F_DELETE_BACK`: deletes the selection, or the char before the caret (Backspace).
//!
//! Sakura: `CViewCommander::Command_DELETE_BACK` in `sakura_core/cmd/CViewCommander_Edit.cpp`.
//! Sakura moves the caret left (`Command_LEFT`) and deletes from there; at the start of a line
//! that deletes the line ending of the line before. At the start of the document nothing
//! happens.
//!
//! Not done yet: the cases that depend on wrapping (it does not delete the line ending when the
//! caret came from the end of a wrapped row; the layout will tell).

use ume_core::cursor::char_left;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct DeleteBack;

impl Command for DeleteBack {
    const ID: &'static str = "delete_back";
    const SAKURA: &'static str = "F_DELETE_BACK";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        if cx.delete_selection()? {
            return Ok(());
        }
        let caret = cx.caret();
        if let Some(prev) = char_left(cx.doc(), caret)? {
            cx.delete_range(prev, caret)?;
        }
        Ok(())
    }
}

register_command!(DeleteBack);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn deletes_the_char_before_the_caret() {
        check("delete_back", "ab|cd", "a|cd");
        check("delete_back", "日|本", "|本");
        check("delete_back", "😀|a", "|a");
        check("delete_back", "ab|", "a|");
    }

    #[test]
    fn at_the_start_of_a_line_it_joins_the_line_before() {
        check("delete_back", "ab\r\n|cd", "ab|cd");
        check("delete_back", "ab\n|cd", "ab|cd");
        check("delete_back", "a\n|\nb", "a|\nb");
        // the empty line at the end of the document
        check("delete_back", "ab\n|", "ab|");
    }

    #[test]
    fn at_the_start_of_the_document_nothing_happens() {
        let mut e = Editor::new("|ab");
        e.run("delete_back").assert_marked("|ab");
        assert!(!e.changes().text_changed());
        check("delete_back", "|", "|");
    }

    #[test]
    fn deletes_the_selection_from_either_direction() {
        check("delete_back", "a[bc|d", "a|d");
        check("delete_back", "a|bc[d", "a|d");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("ab\r\n|cd");
        e.run("delete_back").assert_marked("ab|cd");
        e.run("undo").assert_marked("ab\r\n|cd");
        e.run("redo").assert_marked("ab|cd");
    }
}
