//! `F_INDENT_TAB`: types a tab, or puts a tab before each selected line.
//!
//! Sakura: `CViewCommander::Command_INDENT( WCODE::TAB, INDENT_TAB )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`; bound to Tab in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * No selection, or a selection inside one row: types a tab in place of the selection
//!   (Sakura calls `Command_WCHAR`).
//! * Several rows: a tab goes before the text of every row that starts a line and has text. The
//!   rows are selected from their start to the start of the row after the last, caret at the end.
//!
//! Not done yet: the soft tab (setting `m_bInsSpace`, off by default) and rectangular selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct IndentTab;

impl Command for IndentTab {
    const ID: &'static str = "indent_tab";
    const SAKURA: &'static str = "F_INDENT_TAB";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.indent_selection('\t')
    }
}

register_command!(IndentTab);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn types_a_tab_without_a_selection() {
        check("indent_tab", "ab|cd", "ab\t|cd");
        check("indent_tab", "|", "\t|");
        check("indent_tab", "ab\r\n|", "ab\r\n\t|");
    }

    #[test]
    fn a_selection_inside_one_row_is_replaced_by_a_tab() {
        check("indent_tab", "a[bc|d", "a\t|d");
        check("indent_tab", "a|bc[d", "a\t|d");
    }

    #[test]
    fn indents_every_row_of_a_selection_over_several_rows() {
        check("indent_tab", "a[b\ncd|e\nfg", "[\tab\n\tcde\n|fg");
        // the caret was before the anchor: it ends at the end all the same
        check("indent_tab", "a|b\ncd[e\nfg", "[\tab\n\tcde\n|fg");
    }

    #[test]
    fn a_selection_that_ends_at_column_0_leaves_that_row_alone() {
        check("indent_tab", "a[b\ncd\n|ef", "[\tab\n\tcd\n|ef");
        check("indent_tab", "[ab\ncd\n|", "[\tab\n\tcd\n|");
    }

    #[test]
    fn empty_rows_are_skipped() {
        check("indent_tab", "[ab\n\ncd|", "[\tab\n\n\tcd|");
        check("indent_tab", "[ab\r\n\r\ncd\r\n|", "[\tab\r\n\r\n\tcd\r\n|");
    }

    #[test]
    fn the_last_row_without_a_line_ending_is_indented() {
        check("indent_tab", "[ab\ncd|", "[\tab\n\tcd|");
    }

    #[test]
    fn the_selection_starts_at_the_start_of_the_first_row() {
        check("indent_tab", "ab\nc[d\ne|f", "ab\n[\tcd\n\tef|");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("a[b\ncd|e");
        e.run("indent_tab").assert_marked("[\tab\n\tcde|");
        e.run("undo").assert_marked("a[b\ncd|e");
    }
}
