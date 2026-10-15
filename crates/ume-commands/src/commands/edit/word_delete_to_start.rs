//! `F_WordDeleteToStart`: deletes from the start of the word before the caret to the caret.
//!
//! Sakura: `CViewCommander::Command_WordDeleteToStart` in
//! `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`. It runs `Command_WORDLEFT` with the
//! selection on (so an existing selection is extended from its anchor) and deletes the selection.
//! If nothing is selected afterwards (the caret is at the start of the document) it beeps. At the
//! start of a line the word move is one char left, so the line ending before is deleted.
//!
//! Not done: Sakura beeps when a rectangular selection is active.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordDeleteToStart;

impl Command for WordDeleteToStart {
    const ID: &'static str = "word_delete_to_start";
    const SAKURA: &'static str = "F_WordDeleteToStart";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_word_left(true)?;
        if !cx.delete_selection()? {
            cx.beep();
        }
        Ok(())
    }
}

register_command!(WordDeleteToStart);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn deletes_back_to_the_start_of_the_word() {
        check("word_delete_to_start", "foo bar|", "foo |");
        check("word_delete_to_start", "foo b|ar", "foo |ar");
        check("word_delete_to_start", "foo |bar", "|bar");
        check("word_delete_to_start", "foo  |bar", "|bar");
        check("word_delete_to_start", "漢字ひらがな|", "漢字|");
    }

    #[test]
    fn at_the_start_of_a_line_it_deletes_the_line_ending() {
        check("word_delete_to_start", "foo\n|bar", "foo|bar");
        check("word_delete_to_start", "foo\r\n|bar", "foo|bar");
    }

    #[test]
    fn extends_an_existing_selection() {
        // the selection from the anchor to the new caret is deleted
        check("word_delete_to_start", "a[bc d|ef", "a|def");
    }

    #[test]
    fn beeps_at_the_start_of_the_document() {
        for text in ["|foo", "|"] {
            let mut e = Editor::new(text);
            e.run("word_delete_to_start").assert_marked(text);
            assert!(e.changes().beep, "{text:?}");
            assert!(!e.changes().text_changed(), "{text:?}");
        }
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("foo bar|");
        e.run("word_delete_to_start").assert_marked("foo |");
        e.run("undo").assert_marked("foo bar|");
    }
}
