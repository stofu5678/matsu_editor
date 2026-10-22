//! `F_INDENT_SPACE`: types a space, or puts a space before each selected line.
//!
//! Sakura: `CViewCommander::Command_INDENT( WCODE::SPACE, INDENT_SPACE )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`; bound to Space with Shift in
//! `sakura_core/func/CKeyBind.cpp`. It works as `F_INDENT_TAB` (see there) with a space.
//!
//! Not done yet: rectangular selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct IndentSpace;

impl Command for IndentSpace {
    const ID: &'static str = "indent_space";
    const SAKURA: &'static str = "F_INDENT_SPACE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.indent_selection(' ')
    }
}

register_command!(IndentSpace);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn types_a_space_without_a_selection() {
        check("indent_space", "ab|cd", "ab |cd");
    }

    #[test]
    fn a_selection_inside_one_row_is_replaced_by_a_space() {
        check("indent_space", "a[bc|d", "a |d");
    }

    #[test]
    fn indents_every_row_with_text() {
        check("indent_space", "a[b\n\ncd|e\nfg", "[ ab\n\n cde\n|fg");
        check("indent_space", "[ab\n cd\n|", "[ ab\n  cd\n|");
    }
}
