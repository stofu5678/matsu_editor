//! `F_JUMPHIST_NEXT`: goes forward one place in the jump history.
//!
//! Sakura: `CViewCommander::Command_JUMPHIST_NEXT` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`, switched by `F_BEGIN_SEL`) as `bSelect`
//! here. That mode is not implemented yet, so this command never extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct JumphistNext;

impl Command for JumphistNext {
    const ID: &'static str = "jumphist_next";
    const SAKURA: &'static str = "F_JUMPHIST_NEXT";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.jump_history_next(false)
    }
}

register_command!(JumphistNext);

#[cfg(test)]
mod tests {
    use crate::testing::Editor;

    #[test]
    fn goes_forward_after_going_back() {
        let mut e = Editor::new("a\n|b\nc");
        e.run("go_file_end").run("go_file_top");
        e.run("jumphist_prev").assert_marked("a\nb\nc|");
        e.run("jumphist_prev").assert_marked("a\n|b\nc");
        e.run("jumphist_next").assert_marked("a\nb\nc|");
    }

    #[test]
    fn does_nothing_at_the_newest_place() {
        let mut e = Editor::new("a\n|b");
        e.run("go_file_top");
        e.run("jumphist_next").assert_marked("|a\nb");
    }
}
