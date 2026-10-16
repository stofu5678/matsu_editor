//! `F_CODECNV_SJIS2JIS`: writes the selected text as Jis and shows those bytes as Shift_JIS text.
//!
//! Sakura: `CViewCommander::Command_CODECNV_SJIS2JIS` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
//! `CEditView::ConvSelectedArea`, which converts the selected text with
//! `sakura_core/convert/CConvert_CodeFromSjis.cpp` (ported in `crate::convert::code`).
//! See [`CommandContext::convert_selection`] for what happens without a selection, which text
//! stays selected, and the differences from Sakura.

use ume_core::encoding::Encoding;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert::code;
use crate::register_command;

#[derive(Default)]
pub struct CodecnvSjis2Jis;

impl Command for CodecnvSjis2Jis {
    const ID: &'static str = "codecnv_sjis2_jis";
    const SAKURA: &'static str = "F_CODECNV_SJIS2JIS";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(|text| code::from_sjis(Encoding::Jis, text))
    }
}

register_command!(CodecnvSjis2Jis);

#[cfg(test)]
mod tests {
    use ume_core::encoding::Encoding;

    use crate::convert::code;
    use crate::testing::check;

    #[test]
    fn shows_the_bytes_of_the_code_as_shift_jis() {
        let shown = code::from_sjis(Encoding::Jis, "あいう abc");
        check("codecnv_sjis2_jis", "[あいう abc|", &format!("[{shown}|"));
        check("codecnv_sjis2_jis", "[abc\r\nxyz|", "[abc\r\nxyz|");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("codecnv_sjis2_jis", "日|本", "日|本");
    }
}
