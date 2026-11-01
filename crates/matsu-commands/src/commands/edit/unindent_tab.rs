//! `F_UNINDENT_TAB`: takes a tab, or up to four spaces, off the start of each selected line.
//!
//! Sakura: `CViewCommander::Command_UNINDENT( WCODE::TAB )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`; bound to Shift+Tab in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * No selection: types a tab, like `F_INDENT_TAB` (Sakura calls `Command_INDENT`, then sends
//!   `STR_ERR_UNINDENT1` to the status bar: `Changes::status_message`).
//! * A selection, also inside one row: a row that starts a line loses the tab at its start, or
//!   else the spaces at its start, up to the tab width. The rows are selected as `F_INDENT_TAB`
//!   does.
//!
//! * With the soft tab (the type's `m_bInsSpace`), the tab typed without a selection is the
//!   spaces up to the next tab stop (see `F_INDENT_TAB`).
//! * A box selection: Sakura only beeps, and so does this.
//!
//! The tab width is the one of the layout.

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
    fn types_a_soft_tab_without_a_selection() {
        for (width, before, after) in [
            (4, "ab|cd", "ab  |cd"),
            (8, "ab|cd", "ab      |cd"),
            (4, "あ|", "あ  |"),
            (4, "a\r\nb|", "a\r\nb   |"),
        ] {
            let mut e = Editor::new(before)
                .with_tab_width(width)
                .with_ins_space(true);
            e.run("unindent_tab").assert_marked(after);
        }
    }

    #[test]
    fn a_selection_is_unindented_as_before_with_the_soft_tab() {
        let mut e = Editor::new("\ta[b\n    cd|e").with_ins_space(true);
        e.run("unindent_tab").assert_marked("[ab\ncde|");
    }

    #[test]
    fn removes_up_to_the_tab_width_of_spaces() {
        let mut e = Editor::new("[        ab|").with_tab_width(8);
        e.run("unindent_tab").assert_marked("[ab|");
        let mut e = Editor::new("[    ab|").with_tab_width(2);
        e.run("unindent_tab").assert_marked("[  ab|");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("\ta[b\n\tcd|e");
        e.run("unindent_tab").assert_marked("[ab\ncde|");
        e.run("undo").assert_marked("\ta[b\n\tcd|e");
    }
}
