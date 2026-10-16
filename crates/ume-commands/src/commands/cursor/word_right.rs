//! `F_WORDRIGHT`: moves the caret to the start of the next word.
//!
//! Sakura: `CViewCommander::Command_WORDRIGHT` in `sakura_core/cmd/CViewCommander_Cursor.cpp`,
//! which asks `CSearchAgent::PrevOrNextWord` (`sakura_core/agent/CSearchAgent.cpp`) and
//! `CWordParse::SearchNextWordPosition` (`sakura_core/parse/CWordParse.cpp`). Spaces and tabs
//! are skipped, and the move stops at the end of the line (before the line ending) when the
//! word is the last one. At the end of a line it moves one char right, to the start of the next
//! line, and there tries once more if that line starts with a space or a tab. A selection is
//! dropped and the move starts from the caret.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_BOX`; `F_BEGIN_SEL` is not implemented yet) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordRight;

impl Command for WordRight {
    const ID: &'static str = "word_right";
    const SAKURA: &'static str = "F_WORDRIGHT";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_word_right(false)
    }
}

register_command!(WordRight);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves_to_the_start_of_the_next_word() {
        check("word_right", "|foo bar", "foo |bar");
        check("word_right", "f|oo  \t bar", "foo  \t |bar");
        check("word_right", "|foo+bar", "foo|+bar");
        check("word_right", "foo|+bar", "foo+|bar");
        check("word_right", "|漢字ひらがな", "漢字|ひらがな");
        check("word_right", "|コーヒーを", "コーヒー|を");
        // the last word stops at the end of the line
        check("word_right", "foo |bar", "foo bar|");
        check("word_right", "foo |bar\r\nbaz", "foo bar|\r\nbaz");
        check("word_right", "foo |bar  \r\nbaz", "foo bar  |\r\nbaz");
    }

    #[test]
    fn at_the_end_of_a_line_it_goes_to_the_next_line() {
        check("word_right", "foo|\r\nbar", "foo\r\n|bar");
        check("word_right", "foo|\nbar", "foo\n|bar");
        check("word_right", "foo|\n\nbar", "foo\n|\nbar");
        // a line that starts with blanks gets a second search
        check("word_right", "foo|\n  bar", "foo\n  |bar");
        check("word_right", "foo|\n\tbar", "foo\n\t|bar");
        check("word_right", "foo|\n  ", "foo\n  |");
        // the end of the document
        check("word_right", "foo|", "foo|");
        check("word_right", "foo\n|", "foo\n|");
        check("word_right", "|", "|");
    }

    #[test]
    fn drops_the_selection_and_moves_from_the_caret() {
        check("word_right", "[foo b|ar", "foo bar|");
        check("word_right", "f|oo b[ar", "foo |bar");
    }
}
