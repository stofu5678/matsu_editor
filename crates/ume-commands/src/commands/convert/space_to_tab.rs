//! `F_SPACETOTAB`: replaces runs of spaces in the selected text by tabs where they reach a tab stop.
//!
//! Sakura: `CViewCommander::Command_SPACETOTAB` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_SpaceToTab.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct SpaceToTab;

impl Command for SpaceToTab {
    const ID: &'static str = "space_to_tab";
    const SAKURA: &'static str = "F_SPACETOTAB";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection_with_columns(|text, tab_width, start_col| {
            convert::tabs::space_to_tab(text, tab_width, start_col, convert::tabs::char_width)
        })
    }
}

register_command!(SpaceToTab);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn replaces_runs_of_spaces_by_tabs() {
        check("space_to_tab", "[    x|", "[\tx|");
        check("space_to_tab", "[a b  c|", "[a b\t c|");
        check("space_to_tab", "[x   \r\ny    z|", "[x\t\r\ny\t z|");
    }

    #[test]
    fn the_tab_width_comes_from_the_layout() {
        let run = |before: &str, width: usize| {
            Editor::new(before)
                .with_tab_width(width)
                .run("space_to_tab")
                .marked()
        };
        // no tab in the document: 8 spaces make one tab when the layout's tab width is 8
        assert_eq!(run("[        |x", 8), "[\t|x");
        // 4 spaces do not reach a tab stop of 8 and stay
        assert_eq!(run("[    |x", 8), "[    |x");
        assert_eq!(run("[    |x", 4), "[\t|x");
        // a tab in the text does not change what the layout says
        assert_eq!(run("\ty\n[    |x", 8), "\ty\n[    |x");
    }

    #[test]
    fn counts_from_the_column_of_the_selection() {
        check("space_to_tab", "ab[  c|", "ab[\tc|");
    }
}
