//! `F_WORDLEFT`: moves the caret to the start of the word before it.
//!
//! Sakura: `CViewCommander::Command_WORDLEFT` in `sakura_core/cmd/CViewCommander_Cursor.cpp`,
//! which asks `CSearchAgent::PrevOrNextWord` (`sakura_core/agent/CSearchAgent.cpp`) and
//! `CWordParse::SearchPrevWordPosition` (`sakura_core/parse/CWordParse.cpp`). Spaces and tabs
//! before the caret are skipped. Where there is no word to go to (the caret is at the start of
//! the line) it moves one char left, so it goes to the end of the previous line. A selection is
//! dropped and the move starts from the caret.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_BOX`; `F_BEGIN_SEL` is not implemented yet) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordLeft;

impl Command for WordLeft {
    const ID: &'static str = "word_left";
    const SAKURA: &'static str = "F_WORDLEFT";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_word_left(false)
    }
}

register_command!(WordLeft);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves_to_the_start_of_the_word() {
        check("word_left", "foo bar|", "foo |bar");
        check("word_left", "foo b|ar", "foo |bar");
        check("word_left", "foo |bar", "|foo bar");
        check("word_left", "foo  \t |bar", "|foo  \t bar");
        check("word_left", "foo+bar|", "foo+|bar");
        check("word_left", "foo+|bar", "foo|+bar");
        check("word_left", "漢字ひらがな|", "漢字|ひらがな");
        check("word_left", "コーヒー|", "|コーヒー");
        // before the line ending too
        check("word_left", "foo|\r\nbar", "|foo\r\nbar");
    }

    #[test]
    fn at_the_start_of_a_line_it_moves_one_char() {
        check("word_left", "foo\n|bar", "foo|\nbar");
        check("word_left", "foo\r\n|bar", "foo|\r\nbar");
        check("word_left", "foo\n|", "foo|\n");
        check("word_left", "|foo", "|foo");
        check("word_left", "|", "|");
    }

    #[test]
    fn drops_the_selection_and_moves_from_the_caret() {
        check("word_left", "[foo b|ar", "foo |bar");
        check("word_left", "foo b|ar and[ x", "foo |bar and x");
        // at the start of the document the selection is dropped all the same
        check("word_left", "|foo[", "|foo");
    }
}
