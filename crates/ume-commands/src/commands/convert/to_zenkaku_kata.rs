//! `F_TOZENKAKUKATA`: converts half-width katakana, hiragana, alphanumerics and symbols to full-width katakana and full width.
//!
//! Sakura: `CViewCommander::Command_TOZENKAKUKATA` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_ToZenkata.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct ToZenkakuKata;

impl Command for ToZenkakuKata {
    const ID: &'static str = "to_zenkaku_kata";
    const SAKURA: &'static str = "F_TOZENKAKUKATA";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::kana::to_zenkata)
    }
}

register_command!(ToZenkakuKata);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn converts_to_full_width_katakana() {
        check("to_zenkaku_kata", "[ｶﾞ ひ a|", "[ガ　ヒ　ａ|");
    }
}
