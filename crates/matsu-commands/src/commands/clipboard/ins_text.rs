//! `F_INSTEXT_W` (the macro `InsText`). Sakura: `CViewCommander::Command_INSTEXT` in
//! `sakura_core/cmd/CViewCommander_Clipboard.cpp`, called with the text of the macro by
//! `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`).
//!
//! Inserts the text in place of the selection and puts the caret after it. With a box selected
//! the first line of the text goes into every row of the box (`CommandContext::insert_text`).
//! The text is inserted as it is: line endings are not converted. A key macro records it
//! (`CSMacroMgr::CanFuncIsKeyMacro`). Without a text argument the macro fails
//! (`STR_ERR_DLGMACRO04`).

use crate::command::{Arg, Command, CommandError, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct InsText;

impl Command for InsText {
    const ID: &'static str = "ins_text";
    const SAKURA: &'static str = "F_INSTEXT_W";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        match args.first() {
            Some(Arg::Text(text)) => cx.insert_text(text).map(|_| ()),
            _ => Err(CommandError::BadArgs("InsText takes a text".to_owned())),
        }
    }
}

register_command!(InsText);

#[cfg(test)]
mod tests {
    use crate::command::{Arg, CommandError};
    use crate::testing::Editor;

    fn text(s: &str) -> [Arg; 1] {
        [Arg::Text(s.to_owned())]
    }

    #[test]
    fn inserts_at_the_caret() {
        let mut e = Editor::new("ab|cd");
        e.run_with("ins_text", &text("あ\r\n😀"))
            .assert_marked("abあ\r\n😀|cd");
    }

    #[test]
    fn replaces_the_selection() {
        let mut e = Editor::new("a[bc|d");
        e.run_with("ins_text", &text("X")).assert_marked("aX|d");
    }

    #[test]
    fn keeps_the_line_endings_of_the_text() {
        let mut e = Editor::new("a|b");
        e.run_with("ins_text", &text("1\n2\r3"))
            .assert_marked("a1\n2\r3|b");
    }

    #[test]
    fn one_undo_takes_it_back() {
        let mut e = Editor::new("a[bc|d");
        e.run_with("ins_text", &text("X")).run("undo");
        e.assert_marked("a[bc|d");
    }

    #[test]
    fn a_box_selection_gets_the_first_line_in_every_row() {
        let mut e = Editor::new("a|bc\ndef\nghi");
        e.run("begin_box").run("cursor_right_box");
        e.run("cursor_down_box").run("cursor_down_box");
        e.run_with("ins_text", &text("X\nY"));
        // the first line goes before the box in every row; the box is selected again, moved right by the width
        e.assert_box((2, 3), (0, 2));
        let doc = e.marked();
        assert!(!doc.contains('Y'), "only the first line goes in: {doc}");
        assert_eq!(doc.matches('X').count(), 3, "{doc}");
        e.run("undo");
        assert_eq!(e.marked().replace(['|', '['], ""), "abc\ndef\nghi");
    }

    #[test]
    fn needs_a_text() {
        let mut e = Editor::new("a|b");
        assert!(matches!(
            e.try_run("ins_text", &[]),
            Err(CommandError::BadArgs(_))
        ));
        assert!(matches!(
            e.try_run("ins_text", &[Arg::Int(1)]),
            Err(CommandError::BadArgs(_))
        ));
    }
}
