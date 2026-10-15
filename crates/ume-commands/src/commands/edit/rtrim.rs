//! `F_RTRIM`: removes the white space at the end of the selected lines.
//!
//! Sakura: `CViewCommander::Command_TRIM( FALSE )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`, which calls `CEditView::ConvSelectedArea`
//! (`sakura_core/view/CEditView.cpp`) and `CConvert_Trim::DoConvert`
//! (`sakura_core/convert/CConvert_Trim.cpp`).
//!
//! It works as `F_LTRIM` (see there: white space, the selected part, the caret), from the other
//! side: a selection that ends in the middle of a line trims the white space where it ends.
//!
//! Not done yet: rectangular selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Rtrim;

impl Command for Rtrim {
    const ID: &'static str = "rtrim";
    const SAKURA: &'static str = "F_RTRIM";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.trim_selection(false)
    }
}

register_command!(Rtrim);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn trims_the_row_of_the_caret_without_a_selection() {
        check("rtrim", "  a|b  \r\ncd", "  ab|\r\ncd");
        check("rtrim", "x\nab \t\u{3000}|\ny", "x\nab|\ny");
        check("rtrim", "|  ", "|");
        check("rtrim", "|", "|");
    }

    #[test]
    fn the_ideographic_space_is_white_space() {
        check("rtrim", "[a\u{3000}\u{3000}|", "[a|");
        // other spaces are not: U+00A0 and U+2003 stay
        check("rtrim", "[a\u{a0}\u{2003}|", "[a\u{a0}\u{2003}|");
    }

    #[test]
    fn trims_every_line_of_the_selection() {
        check("rtrim", "[a \r\nb\t\r\nc  |", "[a\r\nb\r\nc|");
    }

    #[test]
    fn only_the_selected_part_counts() {
        // the selection ends before the last space
        check("rtrim", "[a  b |  ", "[a  b|  ");
        check("rtrim", "a \n[b  \nc  |  d", "a \n[b\nc|  d");
    }

    #[test]
    fn nothing_to_remove_makes_no_edit() {
        let mut e = Editor::new("ab|c\nd");
        e.run("rtrim").assert_marked("abc|\nd");
        assert!(!e.changes().text_changed());
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("[a \nb |");
        e.run("rtrim").assert_marked("[a\nb|");
        e.run("undo").assert_marked("[a \nb |");
    }
}
