//! `F_CODECNV_EUC2SJIS`: converts the selected text, which is the Shift_JIS reading of EucJp bytes, to what those bytes mean.
//!
//! Sakura: `CViewCommander::Command_CODECNV_EUC2SJIS` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_CodeToSjis.cpp` (ported in `crate::convert::code`).
//! See [`CommandContext::convert_selection`] for what happens without a selection, which text
//! stays selected, and the differences from Sakura.

use matsu_core::encoding::Encoding;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert::code;
use crate::register_command;

#[derive(Default)]
pub struct CodecnvEuc2Sjis;

impl Command for CodecnvEuc2Sjis {
    const ID: &'static str = "codecnv_euc2_sjis";
    const SAKURA: &'static str = "F_CODECNV_EUC2SJIS";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(|text| code::to_sjis(Encoding::EucJp, text))
    }
}

register_command!(CodecnvEuc2Sjis);

#[cfg(test)]
mod tests {
    use matsu_core::encoding::Encoding;

    use crate::convert::code;
    use crate::testing::check;

    #[test]
    fn fixes_text_that_was_read_as_shift_jis() {
        let garbled = code::from_sjis(Encoding::EucJp, "日本語 abc");
        check("codecnv_euc2_sjis", &format!("[{garbled}|"), "[日本語 abc|");
        check("codecnv_euc2_sjis", "[abc\r\nxyz|", "[abc\r\nxyz|");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("codecnv_euc2_sjis", "ab|c", "ab|c");
    }
}
