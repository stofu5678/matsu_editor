//! `F_SORT_ASC`: sorts the selected lines in ascending order.
//!
//! Sakura: `CViewCommander::Command_SORT( TRUE )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`; bound to Alt+A in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * The lines are those from the line of the start of the selection to the line of its end; the
//!   last one only if the selection ends after its column 0 and it has a line ending. So the last
//!   line of the file, if it has no line ending, is never sorted.
//! * Lines are compared with their line endings, by UTF-16 code unit (not by code point), and a
//!   line that is a prefix of another one comes first. The sort is stable.
//! * Afterwards the lines are selected from the start of the first to the start of the line after
//!   the last. The caret is at the start if it was in the first line, otherwise at the end.
//! * Nothing selected, or no line in the range (a selection inside the last line, which has no
//!   line ending): nothing happens, and there is no beep.
//!
//! Difference: if the order does not change, no edit is made. Not done yet: rectangular
//! selection (Sakura sorts by the selected columns).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct SortAsc;

impl Command for SortAsc {
    const ID: &'static str = "sort_asc";
    const SAKURA: &'static str = "F_SORT_ASC";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.sort_selected_lines(true)
    }
}

register_command!(SortAsc);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn sorts_the_selected_lines() {
        check("sort_asc", "[c\nb\na\n|", "[a\nb\nc\n|");
        check("sort_asc", "[c\r\nb\r\na\r\n|d", "[a\r\nb\r\nc\r\n|d");
    }

    #[test]
    fn a_selection_that_ends_inside_a_line_includes_it() {
        check("sort_asc", "[c\nb\na|\nz", "[a\nb\nc\n|z");
        check("sort_asc", "c\n[b\na\ndx|\ne", "c\n[a\nb\ndx\n|e");
    }

    #[test]
    fn the_last_line_without_a_line_ending_is_not_sorted() {
        // the selection reaches the end of the file, but "a" has no line ending
        check("sort_asc", "[c\nb\na|", "[b\nc\n|a");
    }

    #[test]
    fn compares_by_utf16_code_unit() {
        // U+FF5E is one unit (0xFF5E); U+1F600 is two units that start with 0xD83D
        check(
            "sort_asc",
            "[\u{ff5e}\n\u{1f600}\n|",
            "[\u{1f600}\n\u{ff5e}\n|",
        );
    }

    #[test]
    fn a_prefix_comes_first_and_line_endings_count() {
        check("sort_asc", "[ab\na\n|", "[a\nab\n|");
        // the line endings are compared too: LF is smaller than CR
        check("sort_asc", "[a\r\na\na\r\n|", "[a\na\r\na\r\n|");
    }

    #[test]
    fn the_caret_stays_at_the_end_it_was_at() {
        // in the first line: at the start
        check("sort_asc", "|b\na\n[", "|a\nb\n[");
        // in another line: at the end
        check("sort_asc", "[b\na|\nc", "[a\nb\n|c");
    }

    #[test]
    fn does_nothing_without_a_selection_or_inside_the_last_line() {
        for text in ["b\n|a\n", "a\n[b|", "a\nb|"] {
            let mut e = Editor::new(text);
            e.run("sort_asc").assert_marked(text);
            assert!(!e.changes().beep && !e.changes().text_changed(), "{text:?}");
        }
    }

    #[test]
    fn a_selection_inside_one_line_selects_the_whole_line() {
        // as in Sakura: the line of the end is added to the range, so it is one line, sorted alone
        check("sort_asc", "a[b|c\nd", "|abc\n[d");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("[b\na\n|");
        e.run("sort_asc").assert_marked("[a\nb\n|");
        e.run("undo").assert_marked("[b\na\n|");
    }
}
