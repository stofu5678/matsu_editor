//! `F_SELECTWORD`: selects the word at the caret. A double click runs it too.
//!
//! Sakura: `CViewCommander::Command_SELECTWORD` in `sakura_core/cmd/CViewCommander_Select.cpp`,
//! which asks `CSearchAgent::WhereCurrentWord` and `CWordParse::WhereCurrentWord_2` (see
//! [`crate::text::word`]). The old selection is dropped first. The word is the run of chars of
//! one kind at the caret, so on a space it selects the spaces. On a line ending, past the end of
//! a line, or on the empty line after a final line ending there is no word: nothing is selected
//! and Sakura does not beep.
//!
//! Difference: Sakura sets the selection range and puts the caret at its end. Here the caret is
//! one end of the selection, so the anchor is the start of the word and the caret its end.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct SelectWord;

impl Command for SelectWord {
    const ID: &'static str = "select_word";
    const SAKURA: &'static str = "F_SELECTWORD";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.select_word().map(|_| ())
    }
}

register_command!(SelectWord);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn selects_the_word_with_the_caret_at_its_end() {
        check("select_word", "foo b|ar baz", "foo [bar| baz");
        check("select_word", "foo |bar baz", "foo [bar| baz");
        check("select_word", "|foo bar", "[foo| bar");
        check("select_word", "漢字ひ|らがな", "漢字[ひらがな|");
        check("select_word", "コー|ヒーを", "[コーヒー|を");
        check("select_word", "a|b\r\ncd", "[ab|\r\ncd");
    }

    #[test]
    fn selects_a_run_of_blanks_and_symbols_as_a_word() {
        // the char at the caret decides, not the one before it
        check("select_word", "foo| bar", "foo[ |bar");
        check("select_word", "foo  |\tbar", "foo  [\t|bar");
        check("select_word", "a|+-b", "a[+-|b");
        check("select_word", "x|　　y", "x[　　|y");
    }

    #[test]
    fn selects_nothing_at_the_end_of_a_line() {
        for text in ["foo|", "foo|\nbar", "foo|\r\nbar", "|", "ab\n|"] {
            let mut e = Editor::new(text);
            e.run("select_word").assert_marked(text);
            assert!(!e.changes().beep, "{text:?}");
        }
    }

    #[test]
    fn drops_the_old_selection_even_when_there_is_no_word() {
        check("select_word", "[foo b|ar", "foo [bar|");
        check("select_word", "a[b|", "ab|");
    }
}
