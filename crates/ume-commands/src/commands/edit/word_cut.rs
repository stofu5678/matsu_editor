//! `F_WordCut`: cuts the selection, or the word at the caret, to the clipboard (Ctrl+D).
//!
//! Sakura: `CViewCommander::Command_WordCut` in `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`.
//! With a selection it is `F_CUT`. Without one it runs `Command_SELECTWORD` and cuts what that
//! selected. If there is no word there (the caret is at the end of a line, or on the empty line
//! after the last line ending), it selects the next char instead (`Command_RIGHT( true )`; at
//! the end of a line that is the line ending) and cuts that. At the very end of the text nothing
//! is selected, and `Command_CUT` then does what it does without a selection (it cuts the row,
//! `bEnableNoSelectCopy` is on by default).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordCut;

impl Command for WordCut {
    const ID: &'static str = "word_cut";
    const SAKURA: &'static str = "F_WordCut";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        if cx.selection().is_none() {
            cx.select_word()?;
            if cx.selection().is_none() {
                cx.move_right(true)?;
            }
        }
        cx.cut()
    }
}

register_command!(WordCut);

#[cfg(test)]
mod tests {
    use crate::clipboard::ClipKind;
    use crate::testing::{Editor, check};

    fn clip(e: &Editor) -> (String, ClipKind) {
        let data = e.clipboard().expect("something on the clipboard");
        (data.text.clone(), data.kind)
    }

    #[test]
    fn cuts_the_word_at_the_caret() {
        let mut e = Editor::new("foo b|ar baz");
        e.run("word_cut").assert_marked("foo | baz");
        assert_eq!(clip(&e), ("bar".to_owned(), ClipKind::Normal));
        check("word_cut", "|foo bar", "| bar");
        check("word_cut", "漢字ひ|らがなカタ", "漢字|カタ");
        // on a blank it cuts the blanks
        check("word_cut", "foo|  bar", "foo|bar");
    }

    #[test]
    fn cuts_the_selection_as_it_is() {
        let mut e = Editor::new("f[oo b|ar");
        e.run("word_cut").assert_marked("f|ar");
        assert_eq!(clip(&e), ("oo b".to_owned(), ClipKind::Normal));
    }

    #[test]
    fn at_the_end_of_a_line_it_cuts_the_line_ending() {
        let mut e = Editor::new("foo|\nbar");
        e.run("word_cut").assert_marked("foo|bar");
        assert_eq!(clip(&e).0, "\n");
        let mut e = Editor::new("foo|\r\nbar");
        e.run("word_cut").assert_marked("foo|bar");
        assert_eq!(clip(&e).0, "\r\n");
    }

    #[test]
    fn at_the_end_of_the_text_it_cuts_the_row() {
        // nothing can be selected: Command_CUT cuts the row
        let mut e = Editor::new("ab\ncd|");
        e.run("word_cut");
        assert_eq!(clip(&e), ("cd".to_owned(), ClipKind::Line));
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("foo b|ar baz");
        e.run("word_cut").assert_marked("foo | baz");
        e.run("undo").assert_marked("foo b|ar baz");
    }

    #[test]
    fn view_mode_refuses_it() {
        let mut e = Editor::new("foo b|ar baz");
        e.set_view_mode(true);
        assert!(e.try_run("word_cut", &[]).is_ok());
        e.assert_marked("foo [bar| baz");
    }
}
