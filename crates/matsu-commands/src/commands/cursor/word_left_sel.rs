//! `F_WORDLEFT_SEL`: moves the caret to the start of the word before it and extends the selection.
//!
//! Sakura: `CViewCommander::Command_WORDLEFT` with `bSelect` set, in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`; see `word_left` for the rules.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordLeftSel;

impl Command for WordLeftSel {
    const ID: &'static str = "word_left_sel";
    const SAKURA: &'static str = "F_WORDLEFT_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_word_left(true)
    }
}

register_command!(WordLeftSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn selects_back_to_the_start_of_the_word() {
        check("word_left_sel", "foo bar|", "foo |bar[");
        check("word_left_sel", "foo |bar[", "|foo bar[");
        // back to the anchor: no selection
        check("word_left_sel", "foo [b|ar", "foo |bar");
        // the selection grows to the right of the anchor, then shrinks
        check("word_left_sel", "[foo bar|", "[foo |bar");
        check("word_left_sel", "foo\n|bar", "foo|\n[bar");
        check("word_left_sel", "|foo", "|foo");
    }
}
