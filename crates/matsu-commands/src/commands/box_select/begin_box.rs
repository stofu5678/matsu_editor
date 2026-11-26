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
    use matsu_core::buffer::Pos;

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
    fn typing_in_a_box_that_covers_nothing_inserts_and_keeps_it_begun() {
        // `Command_INDENT` leaves the box selected, moved right by the width of the char
        let mut e = Editor::new("ab|cd");
        e.run("begin_box");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.assert_marked("abx|cd");
        assert!(e.cursor().is_box() && e.cursor().box_just_begun());
        // (`DisableSelectArea` drops the lock of the plain moves)
        assert!(!e.cursor().selecting_lock());
        assert!(!e.changes().beep);
        // a move ends it
        e.run("cursor_left");
        assert!(!e.cursor().is_box());
    }

    /// Begins a box, optionally types `x` (which leaves it begun and unlocked), runs the moves
    /// that cannot move, and then runs `then`.
    fn after_a_move_that_cannot_move(text: &str, typed: bool, moves: &[&str]) -> Editor {
        let mut e = Editor::new(text);
        e.run("begin_box");
        if typed {
            e.run_with("input_char", &[Arg::Char('x')]);
            assert!(e.cursor().box_just_begun() && !e.cursor().selecting_lock());
        }
        for id in moves {
            e.run(id);
        }
        e
    }

    #[test]
    fn a_move_that_cannot_move_ends_the_just_begun_state() {
        // `Command_RIGHT` / `Command_LEFT` (`cmd/CViewCommander_Cursor.cpp`), `Cursor_UPDOWN`
        // (`view/CCaret.cpp`) and the box moves end in `DisableSelectArea` or in
        // `ChangeSelectAreaByCurrentCursor`, whether or not the caret moves. So the box no longer
        // counts as selected text: overwrite mode replaces, Backspace deletes.
        for typed in [false, true] {
            for (text, id, after) in [
                // Right at the end of the document, Left at the start of the file
                ("ab|", "cursor_right", "ab|"),
                ("|ab", "cursor_left", "|ab"),
                // Up on the first row, Down on the last
                ("a|b", "cursor_up", "a|b"),
                ("a|b", "cursor_down", "a|b"),
            ] {
                let mut e = after_a_move_that_cannot_move(text, typed, &[id]);
                assert!(!e.cursor().box_just_begun(), "{id} typed={typed}");
                // the move did not move the caret
                let (caret, text) = (e.cursor().caret(), e.marked().replace(['|', '['], ""));
                assert_eq!(text.contains('x'), typed, "{id}");
                assert_eq!(
                    caret.col,
                    after.find('|').unwrap() + usize::from(typed),
                    "{id}"
                );
                let before = e.marked();
                // (Left at the start of the file has nothing before it: Delete is the key there)
                e.run(if id == "cursor_left" {
                    "delete"
                } else {
                    "delete_back"
                });
                assert_ne!(e.marked(), before, "{id} typed={typed}: it deletes");
            }
        }
    }

    #[test]
    fn a_plain_move_that_cannot_move_ends_a_begun_box_without_moving_the_caret() {
        // The scenario of the review: an empty document, begin a box, type `x`, Right (nothing to
        // go to), Backspace. The Right uses itself up ending the box and Backspace deletes `x`.
        let mut e = Editor::new("|");
        e.run("begin_box");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.assert_marked("x|");
        e.run("cursor_right");
        assert!(!e.cursor().is_box());
        e.run("delete_back");
        e.assert_marked("|");
        // `Command_RIGHT` / `Command_LEFT` only end the box and keep the caret, also where the
        // caret could move (`Command_CANCEL_MODE`, "移動を 1消費")
        let mut e = Editor::new("a|bc");
        e.run("begin_box");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.run("cursor_right");
        e.assert_marked("ax|bc");
        assert!(!e.cursor().is_box());
        let mut e = Editor::new("a|bc");
        e.run("begin_box");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.run("cursor_left");
        e.assert_marked("ax|bc");
        // Up and Down end the box (`DisableSelectArea`) and then move
        let mut e = Editor::new("ab\nc|d");
        e.run("begin_box");
        e.run_with("input_char", &[Arg::Char('x')]);
        e.run("cursor_up");
        assert!(!e.cursor().is_box());
        assert_eq!(e.cursor().caret().line, 0);
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
