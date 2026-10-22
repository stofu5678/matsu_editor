//! `F_WORDRIGHT_SEL`: moves the caret to the start of the next word and extends the selection.
//!
//! Sakura: `CViewCommander::Command_WORDRIGHT` with `bSelect` set, in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`; see `word_right` for the rules.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct WordRightSel;

impl Command for WordRightSel {
    const ID: &'static str = "word_right_sel";
    const SAKURA: &'static str = "F_WORDRIGHT_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_word_right(true)
    }
}

register_command!(WordRightSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn selects_up_to_the_start_of_the_next_word() {
        check("word_right_sel", "|foo bar", "[foo |bar");
        check("word_right_sel", "[foo |bar", "[foo bar|");
        // back to the anchor: no selection
        check("word_right_sel", "|foo [bar", "foo |bar");
        check("word_right_sel", "foo|", "foo|");
    }

    #[test]
    fn crosses_the_line_ending() {
        check("word_right_sel", "foo|\n  bar", "foo[\n  |bar");
        check("word_right_sel", "foo|\r\nbar", "foo[\r\n|bar");
    }
}
