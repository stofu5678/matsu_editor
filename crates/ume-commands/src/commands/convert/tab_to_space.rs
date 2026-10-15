//! `F_TABTOSPACE`: replaces the tabs in the selected text by spaces, up to the next tab stop.
//!
//! Sakura: `CViewCommander::Command_TABTOSPACE` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_TabToSpace.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct TabToSpace;

impl Command for TabToSpace {
    const ID: &'static str = "tab_to_space";
    const SAKURA: &'static str = "F_TABTOSPACE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection_with_columns(|text, tab_width, start_col| {
            convert::tabs::tab_to_space(text, tab_width, start_col, convert::tabs::char_width)
        })
    }
}

register_command!(TabToSpace);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn replaces_tabs_by_the_spaces_to_the_next_tab_stop() {
        check("tab_to_space", "[a\tb|", "[a   b|");
        check("tab_to_space", "a[\tb|", "a[   b|");
        check("tab_to_space", "ab[\tc|", "ab[  c|");
    }

    #[test]
    fn only_the_first_line_starts_at_the_column_of_the_selection() {
        check("tab_to_space", "ab[\tx\r\n\ty|", "ab[  x\r\n    y|");
    }
}
