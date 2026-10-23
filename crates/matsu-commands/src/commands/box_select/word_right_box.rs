//! `F_WORDRIGHT_BOX`: moves the caret to the start of the next word and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_WORDRIGHT` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro arguments (the lock option and the
//! flags of the line moves) are not supported, as for the other cursor commands.
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordRightBox;

impl Command for WordRightBox {
    const ID: &'static str = "word_right_box";
    const SAKURA: &'static str = "F_WORDRIGHT_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.ensure_box()?;
        cx.move_word_right(true)
    }
}

register_command!(WordRightBox);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Pos;

    use crate::testing::{Editor, check_box};

    #[test]
    fn moves_to_the_start_of_the_next_word() {
        check_box("word_right_box", "|foo bar", (0, 0), (0, 4));
        check_box("word_right_box", "foo |bar baz", (0, 4), (0, 8));
    }

    #[test]
    fn goes_into_the_empty_space_at_the_end_of_the_row() {
        // there is no next word on the row: one column right, and a box does not wrap
        let mut e = Editor::new("foo|\nbar");
        e.run("word_right_box").assert_box((0, 3), (0, 4));
        assert_eq!(e.cursor().caret(), Pos::new(0, 3));
    }
}
