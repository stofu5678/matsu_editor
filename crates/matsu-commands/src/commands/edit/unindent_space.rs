//! `F_UNINDENT_SPACE`: takes one space off the start of each selected line.
//!
//! Sakura: `CViewCommander::Command_UNINDENT( WCODE::SPACE )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`; bound to Shift+Space in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * No selection: types a space, like `F_INDENT_SPACE`, and the status bar says
//!   `STR_ERR_UNINDENT1` (`Changes::status_message`).
//! * A selection: a row that starts a line and starts with a space loses that one space. The
//!   rows are selected as `F_INDENT_TAB` does.
//!
//! * A box selection that covers something: Sakura only beeps (`ErrorBeep`, "box unindent is
//!   put on hold" in the source), and so does this.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct UnindentSpace;

impl Command for UnindentSpace {
    const ID: &'static str = "unindent_space";
    const SAKURA: &'static str = "F_UNINDENT_SPACE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.unindent_selection(' ')
    }
}

register_command!(UnindentSpace);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn types_a_space_without_a_selection() {
        check("unindent_space", "ab|cd", "ab |cd");
    }

    #[test]
    fn removes_one_space_from_every_row() {
        check("unindent_space", "  a[b\n cd|e\nfg", "[ ab\ncde\n|fg");
    }

    #[test]
    fn a_cr_and_an_lf_that_meet_are_one_line_ending() {
        // the second line becomes empty, and its LF joins the CR of the first line
        check("unindent_space", "[a\r \n|tail\nkeep", "[a\r\n|tail\nkeep");
    }

    #[test]
    fn tabs_and_rows_without_a_space_stay() {
        check("unindent_space", "[\tab\ncd|", "[\tab\ncd|");
    }
}
