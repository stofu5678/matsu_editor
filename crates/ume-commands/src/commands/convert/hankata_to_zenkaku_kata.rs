//! `F_HANKATATOZENKATA`: converts half-width katakana to full-width katakana and leaves everything else.
//!
//! Sakura: `CViewCommander::Command_HANKATATOZENKAKUKATA` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_HankataToZenkata.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct HankataToZenkakuKata;

impl Command for HankataToZenkakuKata {
    const ID: &'static str = "hankata_to_zenkaku_kata";
    const SAKURA: &'static str = "F_HANKATATOZENKATA";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::kana::hankata_to_zenkata)
    }
}

register_command!(HankataToZenkakuKata);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn joins_the_dakuten() {
        check("hankata_to_zenkaku_kata", "[ｶﾞｱ ab ひ|", "[ガア ab ひ|");
    }
}
