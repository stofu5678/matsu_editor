//! `F_GOPREVPARAGRAPH_SEL`: moves the caret up to the previous paragraph and extends the
//! selection.
//!
//! Sakura: `CViewCommander::Command_GOPREVPARAGRAPH( true )` in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from `CViewCommander::HandleCommand` in
//! `sakura_core/cmd/CViewCommander.cpp`; see `goprevparagraph`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoPrevParagraphSel;

impl Command for GoPrevParagraphSel {
    const ID: &'static str = "goprevparagraph_sel";
    const SAKURA: &'static str = "F_GOPREVPARAGRAPH_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_prev_paragraph(true)
    }
}

register_command!(GoPrevParagraphSel);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn extends_the_selection_to_the_previous_paragraph() {
        // the line above has text: up to the first line of that block of text
        check(
            "goprevparagraph_sel",
            "ab\n\n\ncd\n|ef",
            "ab\n\n\n|cd\n[ef",
        );
        // the line above is empty: the empty lines and then the text above are skipped
        check(
            "goprevparagraph_sel",
            "ab\ncd\n\n\n|ef",
            "|ab\ncd\n\n\n[ef",
        );
    }

    #[test]
    fn crlf_and_japanese_text() {
        check("goprevparagraph_sel", "日本\r\n\r\n|語", "|日本\r\n\r\n[語");
        // the goal column is kept: 語 is two columns wide
        check("goprevparagraph_sel", "日本\n\n語|", "日|本\n\n語[");
    }

    #[test]
    fn an_existing_selection_keeps_its_anchor() {
        check(
            "goprevparagraph_sel",
            "ab\n\ncd\n[ef\n|gh",
            "ab\n\n|cd\n[ef\ngh",
        );
    }

    #[test]
    fn does_nothing_above_the_first_line() {
        check("goprevparagraph_sel", "|ab\ncd", "|ab\ncd");
        check("goprevparagraph_sel", "|", "|");
    }

    #[test]
    fn the_locked_plain_move_extends_too() {
        let mut e = Editor::new("ab\n\ncd|");
        e.run("begin_sel").run("goprevparagraph");
        e.assert_marked("ab|\n\ncd[");
    }
}
