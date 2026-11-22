//! `F_WORDLEFT_BOX`: moves the caret to the start of the word before the caret and begins or extends the box selection.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_WORDLEFT` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The macro argument is the lock option
//! (`Sub_BoxSelectLock`: 1 locks the box, 2 does not, 0 is the setting).
//!
//! The rules of the box are in `context/boxsel.rs`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordLeftBox;

impl Command for WordLeftBox {
    const ID: &'static str = "word_left_box";
    const SAKURA: &'static str = "F_WORDLEFT_BOX";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        cx.ensure_box(Arg::int_at(args, 0))?;
        cx.move_word_left(true)
    }
}

register_command!(WordLeftBox);

#[cfg(test)]
mod tests {
    use crate::testing::check_box;

    #[test]
    fn moves_to_the_start_of_the_word() {
        check_box("word_left_box", "foo bar|", (0, 7), (0, 4));
        check_box("word_left_box", "foo |bar", (0, 4), (0, 0));
        check_box("word_left_box", "日本 語|", (0, 7), (0, 5));
    }

    #[test]
    fn stays_on_the_row_at_its_start() {
        // no word before the caret: one char left, which a box does not do over a row break
        check_box("word_left_box", "foo\n|bar", (1, 0), (1, 0));
    }
}
