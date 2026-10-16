//! `F_JUMPHIST_SET`: adds the caret position to the jump history.
//!
//! Sakura: `CViewCommander::Command_JUMPHIST_SET` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct JumphistSet;

impl Command for JumphistSet {
    const ID: &'static str = "jumphist_set";
    const SAKURA: &'static str = "F_JUMPHIST_SET";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.add_to_jump_history();
        Ok(())
    }
}

register_command!(JumphistSet);

#[cfg(test)]
mod tests {
    use crate::testing::Editor;

    #[test]
    fn the_registered_place_can_be_come_back_to() {
        let mut e = Editor::new("a\nb|\nc");
        e.run("jumphist_set");
        e.assert_marked("a\nb|\nc");
        e.run("cursor_up").assert_marked("a|\nb\nc");
        e.run("jumphist_prev").assert_marked("a\nb|\nc");
    }

    #[test]
    fn does_not_move_the_caret_or_the_text() {
        let mut e = Editor::new("[ab|c");
        e.run("jumphist_set");
        e.assert_marked("[ab|c");
        assert!(!e.changes().caret_moved);
    }
}
