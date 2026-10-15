//! `F_WordDeleteToEnd`: deletes from the caret to the start of the next word.
//!
//! Sakura: `CViewCommander::Command_WordDeleteToEnd` in
//! `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`. It runs `Command_WORDRIGHT` with the
//! selection on (so an existing selection is extended from its anchor) and deletes the selection.
//! If nothing is selected afterwards (the caret is at the end of the document) it beeps. The
//! command is named "to the right end of the word", but the word move goes to the *start of the
//! next* word, so the blanks after the word are deleted with it. At the end of a line the word
//! move is one char right, so the line ending is deleted.
//!
//! Not done: Sakura beeps when a rectangular selection is active.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordDeleteToEnd;

impl Command for WordDeleteToEnd {
    const ID: &'static str = "word_delete_to_end";
    const SAKURA: &'static str = "F_WordDeleteToEnd";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_word_right(true)?;
        if !cx.delete_selection()? {
            cx.beep();
        }
        Ok(())
    }
}

register_command!(WordDeleteToEnd);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn deletes_up_to_the_start_of_the_next_word() {
        check("word_delete_to_end", "|foo bar", "|bar");
        check("word_delete_to_end", "f|oo  bar", "f|bar");
        check("word_delete_to_end", "foo| bar", "foo|bar");
        // the last word of a line: up to the end of the line
        check("word_delete_to_end", "foo |bar\nbaz", "foo |\nbaz");
        check("word_delete_to_end", "|漢字ひらがな", "|ひらがな");
    }

    #[test]
    fn at_the_end_of_a_line_it_deletes_the_line_ending() {
        check("word_delete_to_end", "foo|\nbar", "foo|bar");
        check("word_delete_to_end", "foo|\r\nbar", "foo|bar");
        // and the blanks that start the next line
        check("word_delete_to_end", "foo|\n  bar", "foo|bar");
    }

    #[test]
    fn extends_an_existing_selection() {
        check("word_delete_to_end", "a[bc |def ghi", "a|ghi");
    }

    #[test]
    fn beeps_at_the_end_of_the_document() {
        for text in ["foo|", "|", "foo\n|"] {
            let mut e = Editor::new(text);
            e.run("word_delete_to_end").assert_marked(text);
            assert!(e.changes().beep, "{text:?}");
            assert!(!e.changes().text_changed(), "{text:?}");
        }
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("|foo bar");
        e.run("word_delete_to_end").assert_marked("|bar");
        e.run("undo").assert_marked("|foo bar");
    }
}
