//! `F_MERGE`: removes consecutive duplicate lines from the selection (like `uniq`).
//!
//! Sakura: `CViewCommander::Command_MERGE` in `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`;
//! bound to Alt+M in `sakura_core/func/CKeyBind.cpp`.
//!
//! * The lines are the same as for `F_SORT_ASC`: from the line of the start of the selection to
//!   the line of its end (the last one only if the selection ends after its column 0 and it has a
//!   line ending), so a last line without a line ending is never touched.
//! * A line is a duplicate if it is the same as the line before it, line ending included
//!   (`wmemcmp` of the lines with their endings).
//! * Afterwards the lines that are left are selected from the start of the first to the start
//!   of the line after the last; the caret is at the start if it was in the first line,
//!   otherwise at the end.
//! * Nothing selected, or no line in the range: nothing happens.
//!
//! Not done: the message box with the number of removed lines (`STR_ERR_DLGEDITVWCMDNW7` and
//! `STR_ERR_DLGEDITVWCMDNW8`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Merge;

impl Command for Merge {
    const ID: &'static str = "merge";
    const SAKURA: &'static str = "F_MERGE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.merge_selected_lines()
    }
}

register_command!(Merge);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn removes_consecutive_duplicates() {
        check("merge", "[a\na\nb\nb\nb\na\n|", "[a\nb\na\n|");
        check("merge", "[a\r\na\r\nb\r\n|c", "[a\r\nb\r\n|c");
    }

    #[test]
    fn lines_with_other_line_endings_are_other_lines() {
        check("merge", "[a\r\na\n|", "[a\r\na\n|");
    }

    #[test]
    fn the_selection_covers_the_lines_that_are_left() {
        check("merge", "x\n[a\na|\ny", "x\n[a\n|y");
    }

    #[test]
    fn the_last_line_without_a_line_ending_is_not_merged() {
        check("merge", "[a\na\na|", "[a\n|a");
    }

    #[test]
    fn empty_lines_are_lines() {
        check("merge", "[\n\n\na\n|", "[\na\n|");
    }

    #[test]
    fn nothing_to_remove_makes_no_edit() {
        let mut e = Editor::new("[a\nb\n|");
        e.run("merge").assert_marked("[a\nb\n|");
        assert!(!e.changes().text_changed());
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("merge", "a\n|a\n", "a\n|a\n");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("[a\na\n|");
        e.run("merge").assert_marked("[a\n|");
        e.run("undo").assert_marked("[a\na\n|");
    }
}
