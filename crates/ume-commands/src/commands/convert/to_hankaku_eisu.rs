//! `F_TOHANEI`: converts full-width alphanumerics and symbols to half width.
//!
//! Sakura: `CViewCommander::Command_TOHANEI` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_ZeneisuToHaneisu.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct ToHankakuEisu;

impl Command for ToHankakuEisu {
    const ID: &'static str = "to_hankaku_eisu";
    const SAKURA: &'static str = "F_TOHANEI";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::kana::zeneisu_to_haneisu)
    }
}

register_command!(ToHankakuEisu);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn converts_alphanumerics_and_symbols() {
        check("to_hankaku_eisu", "[Ａｚ０　！ｱ|", "[Az0 !ｱ|");
    }
}
