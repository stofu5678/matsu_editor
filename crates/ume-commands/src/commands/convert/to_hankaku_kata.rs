//! `F_TOHANKATA`: converts full-width katakana (and the kana symbols) to half-width katakana.
//!
//! Sakura: `CViewCommander::Command_TOHANKATA` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_ZenkataToHankata.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct ToHankakuKata;

impl Command for ToHankakuKata {
    const ID: &'static str = "to_hankaku_kata";
    const SAKURA: &'static str = "F_TOHANKATA";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::kana::zenkata_to_hankata)
    }
}

register_command!(ToHankakuKata);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn splits_the_dakuten() {
        check("to_hankaku_kata", "[ガー。ひ|", "[ｶﾞｰ｡ひ|");
    }
}
