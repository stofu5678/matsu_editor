//! `F_SELECTLINE`: selects the line of the caret with its line ending. A triple click runs it.
//!
//! Sakura: `CViewCommander::Command_SELECTLINE` in `sakura_core/cmd/CViewCommander_Select.cpp`.
//! It goes to the start of the logical line (`Command_GOLINETOP` with the flag for lines, not
//! rows) and selects to the start of the next logical line, so a wrapped line is selected as a
//! whole. In the last line, which has no line ending, it selects to the end of the line. On the
//! empty line after a final line ending (what Sakura calls the EOF row) nothing can be selected,
//! and the selection is cleared.
//!
//! Difference: Sakura keeps the caret at the end of the selection too (the start of the next
//! line); here the anchor is the start of the line and the caret the start of the next line.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct SelectLine;

impl Command for SelectLine {
    const ID: &'static str = "select_line";
    const SAKURA: &'static str = "F_SELECTLINE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.select_line()
    }
}

register_command!(SelectLine);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn selects_the_line_with_its_ending() {
        check("select_line", "a|b\ncd", "[ab\n|cd");
        check("select_line", "ab|\r\ncd", "[ab\r\n|cd");
        check("select_line", "|ab\rcd\r", "[ab\r|cd\r");
        check("select_line", "ab\nc|d\nef", "ab\n[cd\n|ef");
        check("select_line", "x\n|\ny", "x\n[\n|y");
        // the old selection does not matter
        check("select_line", "a[b\nc|d\nef", "ab\n[cd\n|ef");
    }

    #[test]
    fn the_last_line_is_selected_to_its_end() {
        check("select_line", "ab\ncd|", "ab\n[cd|");
        check("select_line", "a|b", "[ab|");
        // with a final line ending the next line start is the end of the document
        check("select_line", "ab\ncd|\n", "ab\n[cd\n|");
    }

    #[test]
    fn nothing_to_select_on_the_empty_last_line() {
        for (before, after) in [("ab\n|", "ab\n|"), ("|", "|"), ("[ab\n|", "ab\n|")] {
            let mut e = Editor::new(before);
            e.run("select_line").assert_marked(after);
            assert!(!e.cursor().has_selection());
        }
    }
}
