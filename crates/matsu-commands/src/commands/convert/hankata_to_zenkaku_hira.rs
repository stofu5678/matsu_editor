//! `F_HANKATATOZENHIRA`: converts half-width katakana to full-width hiragana and leaves everything else.
//!
//! Sakura: `CViewCommander::Command_HANKATATOZENKAKUHIRA` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_HankataToZenhira.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct HankataToZenkakuHira;

impl Command for HankataToZenkakuHira {
    const ID: &'static str = "hankata_to_zenkaku_hira";
    const SAKURA: &'static str = "F_HANKATATOZENHIRA";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::kana::hankata_to_zenhira)
    }
}

register_command!(HankataToZenkakuHira);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn converts_to_hiragana() {
        check("hankata_to_zenkaku_hira", "[ｶﾞｱ ab カ|", "[があ ab カ|");
    }
}
