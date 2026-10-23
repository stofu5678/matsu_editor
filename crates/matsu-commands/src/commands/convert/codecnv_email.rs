//! `F_CODECNV_EMAIL`: converts the selected text, which is the Shift_JIS reading of ISO-2022-JP (mail) bytes, to what those bytes mean; MIME encoded words (`=?ISO-2022-JP?B?...?=`) are decoded.
//!
//! Sakura: `CViewCommander::Command_CODECNV_EMAIL` in `sakura_core/cmd/CViewCommander_Convert.cpp` calls
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
pub struct CodecnvEmail;

impl Command for CodecnvEmail {
    const ID: &'static str = "codecnv_email";
    const SAKURA: &'static str = "F_CODECNV_EMAIL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(|text| code::to_sjis(Encoding::Jis, text))
    }
}

register_command!(CodecnvEmail);

#[cfg(test)]
mod tests {
    use matsu_core::encoding::Encoding;

    use crate::convert::code;
    use crate::testing::check;

    #[test]
    fn fixes_text_that_was_read_as_shift_jis() {
        let garbled = code::from_sjis(Encoding::Jis, "あいう abc");
        check("codecnv_email", &format!("[{garbled}|"), "[あいう abc|");
        check("codecnv_email", "[abc\r\nxyz|", "[abc\r\nxyz|");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("codecnv_email", "ab|c", "ab|c");
    }
}
