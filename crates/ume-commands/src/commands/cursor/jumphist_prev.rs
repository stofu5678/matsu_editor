//! `F_JUMPHIST_PREV`: goes back one place in the jump history.
//!
//! Sakura: `CViewCommander::Command_JUMPHIST_PREV` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_SEL` or `F_BEGIN_BOX`) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct JumphistPrev;

impl Command for JumphistPrev {
    const ID: &'static str = "jumphist_prev";
    const SAKURA: &'static str = "F_JUMPHIST_PREV";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.jump_history_prev(false)
    }
}

register_command!(JumphistPrev);

#[cfg(test)]
mod tests {
    use crate::testing::Editor;

    #[test]
    fn comes_back_to_where_a_jump_started() {
        let mut e = Editor::new("a\nb|\nc");
        e.run("go_file_top").assert_marked("|a\nb\nc");
        e.run("jumphist_prev").assert_marked("a\nb|\nc");
        // and forward again to the place where "previous" was pressed
        e.run("jumphist_next").assert_marked("|a\nb\nc");
    }

    #[test]
    fn walks_back_through_several_jumps() {
        let mut e = Editor::new("a\nb|\nc");
        e.run("go_file_top")
            .run("go_file_end")
            .assert_marked("a\nb\nc|");
        e.run("jumphist_prev").assert_marked("|a\nb\nc");
        e.run("jumphist_prev").assert_marked("a\nb|\nc");
        // the oldest place: nothing more
        e.run("jumphist_prev").assert_marked("a\nb|\nc");
        e.run("jumphist_next").assert_marked("|a\nb\nc");
        e.run("jumphist_next").assert_marked("a\nb\nc|");
        e.run("jumphist_next").assert_marked("a\nb\nc|");
    }

    #[test]
    fn with_no_history_it_stays() {
        let mut e = Editor::new("a|b");
        e.run("jumphist_prev").assert_marked("a|b");
        e.run("jumphist_next").assert_marked("a|b");
    }

    #[test]
    fn a_new_jump_in_the_middle_drops_the_newer_places() {
        let mut e = Editor::new("a\nb|\nc");
        e.run("go_file_top").run("go_file_end");
        e.run("jumphist_prev").run("jumphist_prev"); // at the oldest place
        e.run("go_file_top"); // adds the place, drops the newer ones
        e.run("jumphist_next").assert_marked("|a\nb\nc");
    }

    #[test]
    fn a_place_past_the_end_is_clamped() {
        let mut e = Editor::new("a\nbcd|");
        e.run("go_file_top").run("select_all").run("delete");
        e.run("jumphist_prev").assert_marked("|");
    }

    #[test]
    fn extends_the_box_while_it_is_locked() {
        let mut e = Editor::new("ab|cd\nefgh");
        e.run("go_file_end").run("begin_box");
        e.run("jumphist_prev").assert_box((1, 4), (0, 2));
        e.run("jumphist_next").assert_box((1, 4), (1, 4));
    }

    #[test]
    fn undo_keeps_the_history() {
        let mut e = Editor::new("a|b\nc");
        e.run("go_file_end").run("delete_back").run("undo");
        e.run("jumphist_prev").assert_marked("a|b\nc");
    }
}
