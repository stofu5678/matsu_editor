//! `F_TOLOWER`: converts the selected text to lower case, by Sakura's ranges (not Unicode's).
//!
//! Sakura: `CViewCommander::Command_TOLOWER` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_ToLower.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct ToLower;

impl Command for ToLower {
    const ID: &'static str = "to_lower";
    const SAKURA: &'static str = "F_TOLOWER";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::case::to_lower)
    }
}

register_command!(ToLower);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn does_nothing_without_a_selection_or_when_the_text_does_not_change() {
        for text in ["AB|C", "[abc|", "|"] {
            let mut e = Editor::new(text);
            e.run("to_lower").assert_marked(text);
            assert!(!e.changes().text_changed(), "{text:?}");
            assert!(!e.changes().beep, "{text:?}");
        }
    }

    #[test]
    fn is_one_undo_step_that_brings_the_selection_back() {
        let mut e = Editor::new("[ABC|");
        e.run("to_lower").assert_marked("[abc|");
        assert!(e.changes().text_changed());
        e.run("undo").assert_marked("[ABC|");
    }

    #[test]
    fn lower_cases_the_selection() {
        check("to_lower", "[ABC xyz|", "[abc xyz|");
        check("to_lower", "[Ａ日Ｂ\r\nÀ|", "[ａ日ｂ\r\nà|");
    }

    #[test]
    fn the_caret_goes_to_the_end_of_a_backwards_selection() {
        check("to_lower", "|AB[c", "[ab|c");
    }
}
