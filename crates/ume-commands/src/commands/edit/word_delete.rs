//! `F_WordDelete`: deletes the selection, or the word at the caret.
//!
//! Sakura: `CViewCommander::Command_WordDelete` in
//! `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`. Without a selection it runs
//! `Command_SELECTWORD` and deletes what that selected. If there is no word there (the caret is
//! at the end of a line) nothing is selected, and `CEditView::DeleteData` then deletes the one
//! char after the caret, as `F_DELETE` does: at the end of a line that joins the next line.

use ume_core::cursor::char_right;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordDelete;

impl Command for WordDelete {
    const ID: &'static str = "word_delete";
    const SAKURA: &'static str = "F_WordDelete";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        if cx.selection().is_none() {
            cx.select_word()?;
        }
        if cx.delete_selection()? {
            return Ok(());
        }
        let caret = cx.caret();
        if let Some(next) = char_right(cx.doc(), caret)? {
            cx.delete_range(caret, next)?;
        }
        Ok(())
    }
}

register_command!(WordDelete);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn deletes_the_word_at_the_caret() {
        check("word_delete", "foo b|ar baz", "foo | baz");
        check("word_delete", "|foo bar", "| bar");
        check("word_delete", "漢字ひ|らがなカタ", "漢字|カタ");
        // on a blank it deletes the blanks
        check("word_delete", "foo|  bar", "foo|bar");
    }

    #[test]
    fn deletes_the_selection_as_it_is() {
        check("word_delete", "f[oo b|ar", "f|ar");
    }

    #[test]
    fn at_the_end_of_a_line_it_deletes_one_char() {
        check("word_delete", "foo|\nbar", "foo|bar");
        check("word_delete", "foo|\r\nbar", "foo|bar");
        check("word_delete", "foo|", "foo|");
        check("word_delete", "|", "|");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("foo b|ar baz");
        e.run("word_delete").assert_marked("foo | baz");
        e.run("undo").assert_marked("foo b|ar baz");
    }
}
