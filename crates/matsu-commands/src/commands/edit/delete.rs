//! `F_DELETE`: deletes the selection, or the char after the caret.
//!
//! Sakura: `CViewCommander::Command_DELETE` in `sakura_core/cmd/CViewCommander_Edit.cpp`. At the
//! end of a line this deletes the line ending and joins the next line; at the end of the
//! document nothing happens (and no beep).
//!
//! With the free cursor mode (`m_bIsFreeCursorMode`) a caret to the right of the end of the line
//! first fills the gap with spaces (the 2008.08.03 nasukoji case in the source, which calls
//! `Command_INSTEXT` with an empty text to do it), so the line ending that follows is deleted
//! and the next line joins after the spaces. On the last line, which has no line ending, the
//! spaces stay and nothing is deleted.
//!
//! Wrapping changes nothing here. Sakura's other branch of the same code (the "行終端が折り返し"
//! case) deletes the first char of the next row when the caret is at the wrap point; a caret
//! at the end of a wrapped row is the start of the next row in this program (`Layout::row_end`),
//! so the ordinary deletion of the char after the caret is that deletion.

use matsu_core::cursor::char_right;

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
        cx.pad_free_caret()?;
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
    fn in_the_free_cursor_gap_it_pads_with_spaces_and_joins_the_next_line() {
        use crate::edit_options::{EditOptions, with_options};
        let free = EditOptions {
            free_cursor: true,
            ..EditOptions::DEFAULT
        };
        with_options(free, || {
            let mut e = Editor::new("ab|\ncd");
            e.run("cursor_right").run("cursor_right");
            e.run("delete");
            assert_eq!(e.marked(), "ab  |cd");
            // one undo step
            e.run("undo");
            assert_eq!(e.marked().replace(['|', '['], ""), "ab\ncd");
            // CRLF and Japanese text: the gap is counted in columns
            let mut e = Editor::new("日本|\r\nxy");
            e.run("cursor_right")
                .run("cursor_right")
                .run("cursor_right");
            e.run("delete");
            assert_eq!(e.marked(), "日本   |xy");
            // the last line has no line ending: the spaces stay, nothing is deleted
            let mut e = Editor::new("ab|");
            e.run("cursor_right").run("cursor_right");
            e.run("delete");
            assert_eq!(e.marked(), "ab  |");
            // a selection is deleted as it is
            let mut e = Editor::new("a[bc|d\nxy");
            e.run("delete");
            assert_eq!(e.marked(), "a|d\nxy");
        });
        // without the mode the same keys go to the next line
        let mut e = Editor::new("ab|\ncd");
        e.run("cursor_right").run("cursor_right");
        assert_eq!(e.marked(), "ab\nc|d");
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
        use matsu_core::buffer::Pos;
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
