//! `F_DELETE`: deletes the selection, or the char after the caret.
//!
//! Sakura: `CViewCommander::Command_DELETE` in `sakura_core/cmd/CViewCommander_Edit.cpp`. At the
//! end of a line this deletes the line ending and joins the next line; at the end of the
//! document nothing happens (and no beep).
//!
//! Not done yet: with the free cursor mode (`m_bIsFreeCursorMode`) a caret past the end of the
//! line first pads the line with spaces (the 2008.08.03 nasukoji case in the source).

use ume_core::cursor::char_right;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Delete;

impl Command for Delete {
    const ID: &'static str = "delete";
    const SAKURA: &'static str = "F_DELETE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        if cx.delete_selection()? {
            return Ok(());
        }
        let caret = cx.caret();
        if let Some(next) = char_right(cx.doc(), caret)? {
            cx.delete_range(caret, next)?;
        }
        Ok(())
    }
}

register_command!(Delete);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn deletes_the_char_after_the_caret() {
        check("delete", "ab|cd", "ab|d");
        check("delete", "|日本", "|本");
        check("delete", "|😀a", "|a");
        check("delete", "|ab", "|b");
    }

    #[test]
    fn at_the_end_of_a_line_it_joins_the_next_line() {
        check("delete", "ab|\r\ncd", "ab|cd");
        check("delete", "ab|\ncd", "ab|cd");
        check("delete", "ab|\n", "ab|");
        check("delete", "a\n|\nb", "a\n|b");
    }

    #[test]
    fn at_the_end_of_the_document_nothing_happens() {
        let mut e = Editor::new("ab|");
        e.run("delete").assert_marked("ab|");
        assert!(!e.changes().text_changed());
        assert!(!e.changes().beep);
        check("delete", "ab\n|", "ab\n|");
        check("delete", "|", "|");
    }

    #[test]
    fn deletes_the_selection_from_either_direction() {
        check("delete", "a[bc|d", "a|d");
        check("delete", "a|bc[d", "a|d");
        check("delete", "[ab\ncd|", "|");
        check("delete", "a[b\r\nc|d", "a|d");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("ab|\r\ncd");
        e.run("delete").run("undo").assert_marked("ab|\r\ncd");
        e.run("redo").assert_marked("ab|cd");
        // undo brings the selection back
        let mut e = Editor::new("a[bc|d");
        e.run("delete").assert_marked("a|d");
        e.run("undo").assert_marked("a[bc|d");
    }

    #[test]
    fn reports_the_edit() {
        use ume_core::buffer::Pos;
        let mut e = Editor::new("a[bc|d");
        e.run("delete");
        let edits = &e.changes().edits;
        assert_eq!(edits.len(), 1);
        assert_eq!(
            (edits[0].start, edits[0].old_end, edits[0].new_end),
            (Pos::new(0, 1), Pos::new(0, 3), Pos::new(0, 1))
        );
        assert!(e.changes().caret_moved);
        assert!(e.changes().selection_changed);
    }
}
