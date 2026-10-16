//! `F_GOPREVPARAGRAPH`: moves the caret up to the previous paragraph.
//!
//! Sakura: `CViewCommander::Command_GOPREVPARAGRAPH` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_BOX`; `F_BEGIN_SEL` is not implemented yet) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoPrevParagraph;

impl Command for GoPrevParagraph {
    const ID: &'static str = "goprevparagraph";
    const SAKURA: &'static str = "F_GOPREVPARAGRAPH";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_prev_paragraph(false)
    }
}

register_command!(GoPrevParagraph);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn from_the_second_line_of_a_paragraph_it_goes_to_its_first_line() {
        // Sakura looks at the line above first: text, so it goes up to the first line of the
        // block of text
        check("goprevparagraph", "ab\n\n\ncd\n|ef", "ab\n\n\n|cd\nef");
        check(
            "goprevparagraph",
            "ab\r\n\r\ncd\r\n|ef",
            "ab\r\n\r\n|cd\r\nef",
        );
    }

    #[test]
    fn from_the_first_line_of_a_paragraph_it_goes_to_the_paragraph_above() {
        // the line above is empty, so the empty lines and then the text above are skipped
        check("goprevparagraph", "ab\ncd\n\n\n|ef", "|ab\ncd\n\n\nef");
        check(
            "goprevparagraph",
            "x\n\nab\ncd\n\n|ef",
            "x\n\n|ab\ncd\n\nef",
        );
        // spaces and tabs only is empty too
        check("goprevparagraph", "ab\n \t\n|cd", "|ab\n \t\ncd");
        check("goprevparagraph", "日本\n\n|語", "|日本\n\n語");
    }

    #[test]
    fn from_an_empty_line_it_goes_to_the_start_of_the_text_above() {
        check(
            "goprevparagraph",
            "x\n\nab\ncd\n|\nef",
            "x\n\n|ab\ncd\n\nef",
        );
    }

    #[test]
    fn the_goal_column_is_kept() {
        check(
            "goprevparagraph",
            "abcd\n\nxyz\nzzzz|z",
            "abcd\n\nxyz|\nzzzzz",
        );
    }

    #[test]
    fn above_the_first_line() {
        check("goprevparagraph", "|ab\ncd", "|ab\ncd");
        check("goprevparagraph", "ab\n|cd", "|ab\ncd");
        check("goprevparagraph", "|", "|");
        // an empty first line is a place to stop
        check("goprevparagraph", "\n|ab", "|\nab");
        check("goprevparagraph", "\n\n|ab", "|\n\nab");
    }

    #[test]
    fn on_the_line_after_the_last_line_ending() {
        // the last line is text: the caret goes to the first line of its paragraph
        check("goprevparagraph", "x\n\nab\ncd\n|", "x\n\n|ab\ncd\n");
        // an empty last line counts as empty: the block of empty lines is skipped
        check("goprevparagraph", "ab\n\n|", "|ab\n\n");
    }

    #[test]
    fn a_selection_is_dropped() {
        check("goprevparagraph", "ab\n\n[cd\n|ef", "ab\n\n|cd\nef");
    }
}
