//! `F_GONEXTPARAGRAPH`: moves the caret down to the next paragraph.
//!
//! Sakura: `CViewCommander::Command_GONEXTPARAGRAPH` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_SEL` or `F_BEGIN_BOX`) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoNextParagraph;

impl Command for GoNextParagraph {
    const ID: &'static str = "gonextparagraph";
    const SAKURA: &'static str = "F_GONEXTPARAGRAPH";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_next_paragraph(false)
    }
}

register_command!(GoNextParagraph);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn from_text_it_goes_past_the_empty_lines_to_the_next_text() {
        check(
            "gonextparagraph",
            "|ab\ncd\n\n\nef\ngh",
            "ab\ncd\n\n\n|ef\ngh",
        );
        check("gonextparagraph", "|ab\r\n\r\ncd", "ab\r\n\r\n|cd");
        // spaces and tabs only is empty too
        check("gonextparagraph", "|ab\n \t\ncd", "ab\n \t\n|cd");
        check("gonextparagraph", "|日本\n\n語", "日本\n\n|語");
    }

    #[test]
    fn from_an_empty_line_it_goes_to_the_next_text() {
        check("gonextparagraph", "ab\n|\n\ncd", "ab\n\n\n|cd");
        check("gonextparagraph", "ab\n|\n  \ncd", "ab\n\n  \n|cd");
    }

    #[test]
    fn the_goal_column_is_kept() {
        check("gonextparagraph", "abc|d\n\nxyz\nzz", "abcd\n\nxyz|\nzz");
        check("gonextparagraph", "ab|cd\n\nx", "abcd\n\nx|");
    }

    #[test]
    fn at_the_end_of_the_document() {
        // no empty line below: the move goes to the last line
        check("gonextparagraph", "|ab\ncd\nef", "ab\ncd\n|ef");
        // the line after the last line ending is not a line: nothing happens
        check("gonextparagraph", "ab\n|", "ab\n|");
        check("gonextparagraph", "|", "|");
        // empty lines to the end of the document: the move reaches the line after the last one
        check("gonextparagraph", "|ab\n\n", "ab\n\n|");
    }

    #[test]
    fn a_selection_is_dropped() {
        check("gonextparagraph", "[ab\n|cd\n\nef", "ab\ncd\n\n|ef");
    }

    #[test]
    fn records_nothing_in_the_jump_history() {
        let mut e = crate::testing::Editor::new("|ab\n\ncd");
        e.run("gonextparagraph");
        assert!(e.cursor().jump_history().is_empty());
    }
}
