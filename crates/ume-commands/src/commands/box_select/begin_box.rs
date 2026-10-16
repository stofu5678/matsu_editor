//! `F_BEGIN_BOX`: begins a box selection at the caret.
//!
//! Sakura: `CViewCommander::Command_BEGIN_BOXSELECT` in `sakura_core/cmd/CViewCommander_Select.cpp`,
//! called with the lock on from `CViewCommander::HandleCommand`. A selection that is there is
//! dropped, nothing is selected until the caret moves, and the plain moves extend the box until
//! something drops it. Sakura does nothing with a proportional font; the view here is always
//! fixed-pitch.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct BeginBox;

impl Command for BeginBox {
    const ID: &'static str = "begin_box";
    const SAKURA: &'static str = "F_BEGIN_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.begin_box()
    }
}

register_command!(BeginBox);

#[cfg(test)]
mod tests {
    use ume_core::buffer::Pos;

    use crate::command::Arg;
    use crate::testing::{Editor, check_box};

    #[test]
    fn begins_a_box_that_covers_nothing() {
        let mut e = Editor::new("ab|cd");
        e.run("begin_box").assert_box((0, 2), (0, 2));
        assert!(!e.cursor().has_selection());
        assert!(e.cursor().is_box());
        assert!(e.cursor().selecting_lock());
        assert_eq!(e.cursor().caret(), Pos::new(0, 2));
        assert!(e.changes().selection_changed);
        check_box("begin_box", "\t|a", (0, 4), (0, 4));
        check_box("begin_box", "日本|", (0, 4), (0, 4));
    }

    #[test]
    fn drops_a_selection() {
        let mut e = Editor::new("a[bc|d");
        e.run("begin_box").assert_box((0, 3), (0, 3));
        assert!(!e.cursor().has_selection());
    }

    #[test]
    fn the_plain_moves_extend_the_box_while_it_is_locked() {
        let mut e = Editor::new("ab|cd\nefgh\nij");
        e.run("begin_box").run("cursor_down");
        e.assert_box((0, 2), (1, 2));
        assert!(e.cursor().has_selection());
        e.run("cursor_right").assert_box((0, 2), (1, 3));
        e.run("go_file_end").assert_box((0, 2), (2, 3));
    }

    #[test]
    fn typing_ends_a_box_that_covers_nothing() {
        let mut e = Editor::new("ab|cd");
        e.run("begin_box");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.assert_marked("abx|cd");
        assert!(!e.cursor().is_box());
        assert!(!e.cursor().selecting_lock());
        assert!(!e.changes().beep);
    }

    #[test]
    fn a_move_that_drops_the_selection_ends_the_box() {
        let mut e = Editor::new("ab|cd");
        e.run("begin_box").run("cursor_right");
        // the lock is on, so a plain move extends; clearing it is what a click or an edit does
        e.run("select_all");
        assert!(!e.cursor().is_box());
        assert!(!e.cursor().selecting_lock() || e.cursor().has_selection());
    }
}
