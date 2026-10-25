//! `F_DELETE_BACK`: deletes the selection, or the char before the caret (Backspace).
//!
//! Sakura: `CViewCommander::Command_DELETE_BACK` in `sakura_core/cmd/CViewCommander_Edit.cpp`.
//! Sakura moves the caret left (`Command_LEFT`) and deletes from there; at the start of a line
//! that deletes the line ending of the line before. At the start of the document nothing
//! happens.
//!
//! * In the free cursor mode a caret to the right of the end of the line only moves one column
//!   left and deletes nothing: `Command_LEFT` puts the caret at or past the end of the text, and
//!   the code deletes only when the caret ends on a char (`LineColumnToIndex2` reports a column
//!   past the line through `nLineLen`) or when it went up a row.
//! * Wrapping: from the start of a wrapped row `Command_LEFT` puts the caret on the last char of
//!   the row before, and that char is deleted; the line ending is deleted only when the row before
//!   ends with one (`nIdx < GetLengthWithoutEOL() || y < y_old`). The end of a wrapped row is the
//!   start of the next row in this program, so deleting the char before the caret is the same.

use matsu_core::cursor::char_left;

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
        // `Command_LEFT` first: through the gap of the free cursor mode it moves one column and
        // nothing is deleted.
        if cx.caret_free_col()?.is_some() {
            return cx.move_left(false);
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
    fn in_the_free_cursor_gap_it_only_moves_left() {
        use crate::edit_options::{EditOptions, with_options};
        let free = EditOptions {
            free_cursor: true,
            ..EditOptions::DEFAULT
        };
        with_options(free, || {
            let mut e = Editor::new("ab|\ncd");
            e.run("cursor_right").run("cursor_right");
            e.run("delete_back");
            assert!(!e.changes().text_changed());
            assert_eq!(e.cursor().free_col(), Some(3));
            e.run("delete_back");
            assert_eq!(e.cursor().free_col(), None);
            assert!(!e.changes().text_changed());
            assert_eq!(e.marked(), "ab|\ncd");
            // on the text it deletes as usual
            e.run("delete_back");
            assert_eq!(e.marked(), "a|\ncd");
        });
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
