//! `F_TOUPPER`: converts the selected text to upper case, by Sakura's ranges (not Unicode's).
//!
//! Sakura: `CViewCommander::Command_TOUPPER` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_ToUpper.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct ToUpper;

impl Command for ToUpper {
    const ID: &'static str = "to_upper";
    const SAKURA: &'static str = "F_TOUPPER";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::case::to_upper)
    }
}

register_command!(ToUpper);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn upper_cases_the_selection() {
        check("to_upper", "[abc XYZ|", "[ABC XYZ|");
        check("to_upper", "[ａ日ｂ\r\nà|", "[Ａ日Ｂ\r\nÀ|");
    }
}
