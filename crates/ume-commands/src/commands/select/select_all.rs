//! `F_SELECTALL`: selects the whole document (Ctrl+A).
//!
//! Sakura: `CViewCommander::Command_SELECTALL` in `sakura_core/cmd/CViewCommander_Select.cpp`. It
//! selects from the start of the document to its end (`GetEndLayoutPos`, which is on the
//! empty line after a final line ending).
//!
//! Difference: Sakura sets the selection range and leaves the caret where it was. Here the caret
//! is one end of the selection, so it goes to the end of the document, with the anchor at the
//! start.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct SelectAll;

impl Command for SelectAll {
    const ID: &'static str = "select_all";
    const SAKURA: &'static str = "F_SELECTALL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.select_all()
    }
}

register_command!(SelectAll);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn selects_everything_with_the_caret_at_the_end() {
        check("select_all", "a|b\ncd", "[ab\ncd|");
        check("select_all", "ab|", "[ab|");
        check("select_all", "a[b|c", "[abc|");
        // the end is the empty line after a final line ending
        check("select_all", "|ab\r\n", "[ab\r\n|");
    }

    #[test]
    fn an_empty_document_has_nothing_to_select() {
        let mut e = Editor::new("|");
        e.run("select_all").assert_marked("|");
        assert!(!e.cursor().has_selection());
    }

    #[test]
    fn typing_replaces_everything() {
        let mut e = Editor::new("a|b\ncd");
        e.run("select_all");
        e.run_with("input_char", &[crate::command::Arg::Char('X')]);
        e.assert_marked("X|");
        e.run("undo").assert_marked("[ab\ncd|");
    }

    #[test]
    fn extending_from_select_all_keeps_the_anchor() {
        let mut e = Editor::new("ab\ncd|");
        e.run("select_all").run("cursor_left_sel");
        e.assert_marked("[ab\nc|d");
    }
}
