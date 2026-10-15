//! `F_TOZENEI`: converts half-width alphanumerics and symbols to full width (the space too).
//!
//! Sakura: `CViewCommander::Command_TOZENEI` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_HaneisuToZeneisu.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct ToZenkakuEisu;

impl Command for ToZenkakuEisu {
    const ID: &'static str = "to_zenkaku_eisu";
    const SAKURA: &'static str = "F_TOZENEI";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::kana::haneisu_to_zeneisu)
    }
}

register_command!(ToZenkakuEisu);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn converts_alphanumerics_and_symbols() {
        check("to_zenkaku_eisu", "[Az0 !ｱ|", "[Ａｚ０　！ｱ|");
    }
}
