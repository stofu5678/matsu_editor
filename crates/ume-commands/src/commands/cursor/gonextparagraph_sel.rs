//! `F_GONEXTPARAGRAPH_SEL`: moves the caret down to the next paragraph and extends the selection.
//!
//! Sakura: `CViewCommander::Command_GONEXTPARAGRAPH( true )` in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from `CViewCommander::HandleCommand` in
//! `sakura_core/cmd/CViewCommander.cpp`; see `gonextparagraph`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoNextParagraphSel;

impl Command for GoNextParagraphSel {
    const ID: &'static str = "gonextparagraph_sel";
    const SAKURA: &'static str = "F_GONEXTPARAGRAPH_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_next_paragraph(true)
    }
}

register_command!(GoNextParagraphSel);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn extends_the_selection_to_the_next_paragraph() {
        check(
            "gonextparagraph_sel",
            "|ab\ncd\n\n\nef\ngh",
            "[ab\ncd\n\n\n|ef\ngh",
        );
        // from an empty line: to the next line with text
        check("gonextparagraph_sel", "ab\n|\n\ncd", "ab\n[\n\n|cd");
    }

    #[test]
    fn crlf_and_japanese_text() {
        check("gonextparagraph_sel", "|ab\r\n\r\n日本", "[ab\r\n\r\n|日本");
        // the goal column is kept: 日 is two columns wide
        check("gonextparagraph_sel", "日|本\n\n語本", "日[本\n\n語|本");
    }

    #[test]
    fn an_existing_selection_keeps_its_anchor() {
        check("gonextparagraph_sel", "a[b\n|cd\n\nef", "a[b\ncd\n\n|ef");
        // the anchor can be below the caret: the selection shrinks
        check("gonextparagraph_sel", "|ab\n\ncd\n\nef", "[ab\n\n|cd\n\nef");
    }

    #[test]
    fn does_nothing_below_the_last_line() {
        check("gonextparagraph_sel", "ab\n|", "ab\n|");
    }

    #[test]
    fn the_locked_plain_move_extends_too() {
        let mut e = Editor::new("|ab\n\ncd");
        e.run("begin_sel").run("gonextparagraph");
        e.assert_marked("[ab\n\n|cd");
    }
}
