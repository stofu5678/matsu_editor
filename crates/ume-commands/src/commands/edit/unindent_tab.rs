//! `F_UNINDENT_TAB`: takes a tab, or up to four spaces, off the start of each selected line.
//!
//! Sakura: `CViewCommander::Command_UNINDENT( WCODE::TAB )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`; bound to Shift+Tab in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * No selection: types a tab, like `F_INDENT_TAB` (Sakura calls `Command_INDENT`, then shows
//!   the message `STR_ERR_UNINDENT1`; the message is not done here).
//! * A selection, also inside one row: a row that starts a line loses the tab at its start, or
//!   else the spaces at its start, up to the tab width. The rows are selected as `F_INDENT_TAB`
//!   does.
//!
//! The tab width is Sakura's default, 4; the setting is not read yet (`context/indent.rs`).
//! Not done yet: rectangular selection (Sakura only beeps for it).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct UnindentTab;

impl Command for UnindentTab {
    const ID: &'static str = "unindent_tab";
    const SAKURA: &'static str = "F_UNINDENT_TAB";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.unindent_selection('\t')
    }
}

register_command!(UnindentTab);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn types_a_tab_without_a_selection() {
        check("unindent_tab", "ab|cd", "ab\t|cd");
    }

    #[test]
    fn removes_a_tab_from_every_row() {
        check("unindent_tab", "\ta[b\n\tcd|e\n\tfg", "[ab\ncde\n|\tfg");
        check("unindent_tab", "\t\ta[b\n\tcd|e", "[\tab\ncde|");
    }

    #[test]
    fn removes_up_to_four_spaces() {
        check("unindent_tab", "[ \tab|", "[\tab|");
        check("unindent_tab", "[  ab|", "[ab|");
        check("unindent_tab", "[      ab|", "[  ab|");
        check("unindent_tab", "[   \tab|", "[\tab|");
        // a tab first: only the tab goes
        check("unindent_tab", "[\t  ab|", "[  ab|");
    }

    #[test]
    fn rows_without_indent_stay() {
        check("unindent_tab", "[ab\n\tcd|", "[ab\ncd|");
        check("unindent_tab", "[ab\n\n\tcd\n|", "[ab\n\ncd\n|");
    }

    #[test]
    fn a_selection_that_ends_at_column_0_leaves_that_row_alone() {
        check("unindent_tab", "\ta[b\n\tcd\n|\tef", "[ab\ncd\n|\tef");
    }

    #[test]
    fn works_on_a_selection_inside_one_row() {
        check("unindent_tab", "\ta[b|c", "[abc|");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("\ta[b\n\tcd|e");
        e.run("unindent_tab").assert_marked("[ab\ncde|");
        e.run("undo").assert_marked("\ta[b\n\tcd|e");
    }
}
