//! `F_TOZENKAKUHIRA`: converts half-width katakana, katakana, alphanumerics and symbols to full-width hiragana and full width.
//!
//! Sakura: `CViewCommander::Command_TOZENKAKUHIRA` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_ToZenhira.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct ToZenkakuHira;

impl Command for ToZenkakuHira {
    const ID: &'static str = "to_zenkaku_hira";
    const SAKURA: &'static str = "F_TOZENKAKUHIRA";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::kana::to_zenhira)
    }
}

register_command!(ToZenkakuHira);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn converts_to_full_width_hiragana() {
        check("to_zenkaku_hira", "[ｶﾞ ヒ a|", "[が　ひ　ａ|");
    }
}
