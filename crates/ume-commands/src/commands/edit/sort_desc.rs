//! `F_SORT_DESC`: sorts the selected lines in descending order.
//!
//! Sakura: `CViewCommander::Command_SORT( FALSE )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`; bound to Alt+D in
//! `sakura_core/func/CKeyBind.cpp`. It is `F_SORT_ASC` (see there for the range of lines, the
//! comparison and the selection afterwards) with the opposite order; equal lines keep their
//! order (`std::stable_sort` with `>`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct SortDesc;

impl Command for SortDesc {
    const ID: &'static str = "sort_desc";
    const SAKURA: &'static str = "F_SORT_DESC";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.sort_selected_lines(false)
    }
}

register_command!(SortDesc);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn sorts_the_selected_lines_backwards() {
        check("sort_desc", "[a\nc\nb\n|", "[c\nb\na\n|");
        check("sort_desc", "[a\r\nc\r\nb|\r\nz", "[c\r\nb\r\na\r\n|z");
    }

    #[test]
    fn a_prefix_comes_last() {
        check("sort_desc", "[a\nab\n|", "[ab\na\n|");
    }

    #[test]
    fn the_last_line_without_a_line_ending_is_not_sorted() {
        check("sort_desc", "[a\nb\nc|", "[b\na\n|c");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        let mut e = Editor::new("a\n|b\n");
        e.run("sort_desc").assert_marked("a\n|b\n");
        assert!(!e.changes().text_changed());
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("[a\nb\n|");
        e.run("sort_desc").assert_marked("[b\na\n|");
        e.run("undo").assert_marked("[a\nb\n|");
    }
}
