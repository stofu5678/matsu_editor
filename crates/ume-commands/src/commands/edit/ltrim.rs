//! `F_LTRIM`: removes the white space at the start of the selected lines.
//!
//! Sakura: `CViewCommander::Command_TRIM( TRUE )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`, which calls `CEditView::ConvSelectedArea`
//! (`sakura_core/view/CEditView.cpp`) and `CConvert_Trim::DoConvert`
//! (`sakura_core/convert/CConvert_Trim.cpp`).
//!
//! White space is a space, a tab and the ideographic space U+3000 (`WCODE::IsBlank`). Only the
//! selected part of the first and the last line counts, so a selection that starts in the middle
//! of a line trims the white space where the selection starts. With no selection the row of the
//! caret is trimmed, and the caret goes to the end of it. With a selection, the trimmed text is
//! selected, and the caret goes to the end.
//!
//! Difference: when there is nothing to remove Sakura still replaces the text; this command
//! makes no edit then (the selection and the caret move in the same way).
//!
//! Not done yet: rectangular selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Ltrim;

impl Command for Ltrim {
    const ID: &'static str = "ltrim";
    const SAKURA: &'static str = "F_LTRIM";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.trim_selection(true)
    }
}

register_command!(Ltrim);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn trims_the_row_of_the_caret_without_a_selection() {
        // the caret goes to the end of the row
        check("ltrim", "  a|b  \r\ncd", "ab  |\r\ncd");
        check("ltrim", "x\n \t\u{3000}ab|\ny", "x\nab|\ny");
        check("ltrim", "  |", "|");
        check("ltrim", "|", "|");
        check("ltrim", "ab|", "ab|");
    }

    #[test]
    fn trims_every_line_of_the_selection() {
        check("ltrim", "[ a\r\n  b\r\n c|", "[a\r\nb\r\nc|");
        check("ltrim", " [ a\n  b|  \nc", " [a\nb|  \nc");
    }

    #[test]
    fn only_the_selected_part_counts() {
        // the selection starts after the first space of " \t a"
        check("ltrim", " [ \t a|", " [a|");
        check("ltrim", "a  [  b\n  c|d", "a  [b\nc|d");
        // a backward selection ends with the caret at the end all the same
        check("ltrim", "|  a\n  b[", "[a\nb|");
    }

    #[test]
    fn lines_that_are_only_white_space_become_empty() {
        check("ltrim", "[ \n\t\n|", "[\n\n|");
    }

    #[test]
    fn nothing_to_remove_makes_no_edit() {
        let mut e = Editor::new("ab|c\nd");
        e.run("ltrim").assert_marked("abc|\nd");
        assert!(!e.changes().text_changed());
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("[ a\n b|");
        e.run("ltrim").assert_marked("[a\nb|");
        e.run("undo").assert_marked("[ a\n b|");
    }
}
