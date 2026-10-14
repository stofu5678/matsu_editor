//! `F_DELETE_LINE`: deletes the row that has the caret, with its line ending.
//!
//! Sakura: `CViewCommander::Command_DELETE_LINE` in `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`.
//! The command is "delete a row" (wrap unit): it selects from the start of the row to the start of
//! the next row and runs `Command_DELETE`. With a selection, or with the caret on the empty
//! line after a final line ending (no row), it only beeps. The caret then goes to the same
//! display column of the row that took its place, or to the end of that row if it is shorter.
//!
//! Difference: for a last row without a line ending, Sakura puts the caret one cell before the
//! end of the row that moved up (it subtracts 1 from the end column; the code is written for
//! a row whose end column includes the line ending mark). This command puts it at the end.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct DeleteLine;

impl Command for DeleteLine {
    const ID: &'static str = "delete_line";
    const SAKURA: &'static str = "F_DELETE_LINE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        let caret = cx.caret();
        if cx.selection().is_some() || caret.line >= cx.doc().line_count() {
            cx.beep();
            return Ok(());
        }
        let col = cx.caret_display_col()?;
        let start = cx.layout().row_start(cx.doc(), caret)?;
        let end = match cx.layout().next_row_start(cx.doc(), caret)? {
            Some(next) => next,
            None => cx.layout().row_end(cx.doc(), caret)?,
        };
        cx.delete_range(start, end)?;
        // keep the display column on the row that took the place of the deleted one
        if start.line < cx.doc().line_count() {
            let pos = cx.layout().pos_at_col(cx.doc(), start, col)?;
            cx.move_caret(pos, false)?;
        }
        Ok(())
    }
}

register_command!(DeleteLine);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn deletes_the_row_and_keeps_the_column() {
        check("delete_line", "ab|cd\r\nef", "ef|");
        check("delete_line", "a|b\r\ncdef", "c|def");
        check("delete_line", "x\nab|\nc\nd", "x\nc|\nd");
        check("delete_line", "|a\nb", "|b");
    }

    #[test]
    fn the_column_is_a_display_column() {
        // the caret was after a full-width char (column 2); on "日本" column 2 is before 本
        check("delete_line", "日|\nxy日本", "xy|日本");
        check("delete_line", "ab日|\n日本z", "日本|z");
    }

    #[test]
    fn the_last_row() {
        check("delete_line", "ab\r\ncd|", "ab\r\n|");
        check("delete_line", "ab|", "|");
        check("delete_line", "ab\ncd|\n", "ab\n|");
    }

    #[test]
    fn beeps_with_a_selection_and_on_the_empty_last_line() {
        for text in ["a[b|c", "ab\n|", "|"] {
            let mut e = Editor::new(text);
            e.run("delete_line").assert_marked(text);
            assert!(e.changes().beep, "{text:?}");
            assert!(!e.changes().text_changed(), "{text:?}");
        }
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("ab|cd\r\nef");
        e.run("delete_line").assert_marked("ef|");
        e.run("undo").assert_marked("ab|cd\r\nef");
    }
}
