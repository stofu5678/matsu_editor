//! `F_TOHANKAKU`: converts everything that has a half-width form to half width: alphanumerics, symbols, hiragana, katakana.
//!
//! Sakura: `CViewCommander::Command_TOHANKAKU` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_ToHankaku.cpp` (ported in `crate::convert`). See
//! [`CommandContext::convert_selection`] for what happens without a selection, which text stays
//! selected, and the differences from Sakura.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct ToHankaku;

impl Command for ToHankaku {
    const ID: &'static str = "to_hankaku";
    const SAKURA: &'static str = "F_TOHANKAKU";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::kana::to_hankaku)
    }
}

register_command!(ToHankaku);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn converts_to_half_width_and_the_selection_follows_the_new_length() {
        check("to_hankaku", "[ＡＢ１　ガひ|", "[AB1 ｶﾞﾋ|");
        check("to_hankaku", "a[ゔ|b", "a[ｳﾞ|b");
    }
}
