//! `F_REDO`: redoes the block that the last undo took back (Ctrl+Y).
//!
//! Sakura: `CViewCommander::Command_REDO` in `sakura_core/cmd/CViewCommander_Edit.cpp` and
//! `COpeBuf::DoRedo` in `sakura_core/cmd/COpeBuf.cpp`. The cursor goes to where the redone command
//! left it. With nothing to redo it does nothing. A new edit drops what could be redone.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Redo;

impl Command for Redo {
    const ID: &'static str = "redo";
    const SAKURA: &'static str = "F_REDO";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.redo();
        Ok(())
    }
}

register_command!(Redo);

#[cfg(test)]
mod tests {
    use crate::command::Arg;
    use crate::testing::{Editor, check};

    #[test]
    fn nothing_to_redo() {
        let mut e = Editor::new("ab|cd");
        e.run("redo").assert_marked("ab|cd");
        assert!(!e.changes().text_changed());
        check("redo", "|", "|");
    }

    #[test]
    fn redoes_what_undo_took_back() {
        let mut e = Editor::new("a[bc|d");
        e.run("delete").assert_marked("a|d");
        e.run("undo").assert_marked("a[bc|d");
        e.run("redo").assert_marked("a|d");
        e.run("redo").assert_marked("a|d");
    }

    #[test]
    fn a_new_edit_drops_the_redo_steps() {
        let mut e = Editor::new("|");
        e.run_with("input_char", &[Arg::Char('a')]);
        e.run("undo");
        e.run_with("input_char", &[Arg::Char('b')]);
        e.run("redo").assert_marked("b|");
    }
}
