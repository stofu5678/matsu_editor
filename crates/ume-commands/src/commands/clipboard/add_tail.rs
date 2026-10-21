//! `F_ADDTAIL_W` (the macro `AddTail`). Sakura: `CViewCommander::Command_ADDTAIL` in
//! `sakura_core/cmd/CViewCommander_Clipboard.cpp`.
//!
//! Goes to the end of the document without selecting (`Command_GOFILEEND( false )`, so a
//! selection, a box too, is dropped and not replaced), inserts the text there and leaves the
//! caret after it. Sakura does not keep AddTail in a key macro (`key_macro: false`). Without a
//! text argument the macro fails (`STR_ERR_DLGMACRO04`).

use crate::command::{Arg, Command, CommandError, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct AddTail;

impl Command for AddTail {
    const ID: &'static str = "add_tail";
    const SAKURA: &'static str = "F_ADDTAIL_W";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        let Some(Arg::Text(text)) = args.first() else {
            return Err(CommandError::BadArgs("AddTail takes a text".to_owned()));
        };
        // `Command_GOFILEEND( false )` never extends; the lock of `F_BEGIN_SEL` is only passed on
        // by `HandleCommand` to the functions that move by key.
        if cx.cursor().selecting_lock() {
            cx.toggle_selecting_lock();
        }
        cx.move_file_end(false)?;
        cx.insert_text(text).map(|_| ())
    }
}

register_command!(AddTail);

#[cfg(test)]
mod tests {
    use crate::command::{Arg, CommandError};
    use crate::testing::Editor;

    fn text(s: &str) -> [Arg; 1] {
        [Arg::Text(s.to_owned())]
    }

    #[test]
    fn adds_at_the_end_and_leaves_the_caret_after_it() {
        let mut e = Editor::new("a|b\r\ncd");
        e.run_with("add_tail", &text("い\n😀"))
            .assert_marked("ab\r\ncdい\n😀|");
    }

    #[test]
    fn a_selection_is_dropped_not_replaced() {
        let mut e = Editor::new("a[bc|d");
        e.run_with("add_tail", &text("X")).assert_marked("abcdX|");
    }

    #[test]
    fn a_box_selection_is_dropped_too() {
        let mut e = Editor::new("abc\ndef|");
        e.run("begin_box").run("cursor_up_box");
        e.run_with("add_tail", &text("X"))
            .assert_marked("abc\ndefX|");
        assert!(e.box_state().is_none());
    }

    #[test]
    fn the_selection_lock_does_not_make_it_replace_the_text() {
        let mut e = Editor::new("a|bcd");
        e.run("begin_sel");
        e.run_with("add_tail", &text("X")).assert_marked("abcdX|");
    }

    #[test]
    fn one_undo_takes_it_back() {
        let mut e = Editor::new("a|b");
        e.run_with("add_tail", &text("X\n")).run("undo");
        e.assert_marked("a|b");
    }

    #[test]
    fn needs_a_text() {
        let mut e = Editor::new("a|b");
        assert!(matches!(
            e.try_run("add_tail", &[]),
            Err(CommandError::BadArgs(_))
        ));
    }
}
