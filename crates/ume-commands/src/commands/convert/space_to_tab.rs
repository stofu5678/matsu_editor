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
    use crate::testing::check;

    #[test]
    fn replaces_runs_of_spaces_by_tabs() {
        check("space_to_tab", "[    x|", "[\tx|");
        check("space_to_tab", "[a b  c|", "[a b\t c|");
        check("space_to_tab", "[x   \r\ny    z|", "[x\t\r\ny\t z|");
    }

    #[test]
    fn the_tab_width_is_guessed_from_nearby_tabs_and_defaults_to_4() {
        // the only tab (column 7 to 8) fits 4 and 8; 4 is taken
        check("space_to_tab", "[    |x\nabcdefg\tz", "[\t|x\nabcdefg\tz");
        // a tab from column 0 to 4 settles it
        check("space_to_tab", "\ty\n[    |x", "\ty\n[\t|x");
    }

    #[test]
    fn counts_from_the_column_of_the_selection() {
        check("space_to_tab", "ab[  c|", "ab[\tc|");
    }
}
