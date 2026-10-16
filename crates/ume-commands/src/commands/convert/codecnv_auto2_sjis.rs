//! `F_CODECNV_AUTO2SJIS`: converts the selected text, which is the Shift_JIS reading of bytes of
//! another code, to what those bytes mean; the code is detected.
//!
//! Sakura: `CViewCommander::Command_CODECNV_AUTO2SJIS` in
//! `sakura_core/cmd/CViewCommander_Convert.cpp` calls `CEditView::ConvSelectedArea`, which converts
//! the selected text with `sakura_core/convert/CConvert_CodeAutoToSjis.cpp` (ported in
//! `crate::convert::code`). See [`CommandContext::convert_selection`] for what happens without a
//! selection, which text stays selected, and the differences from Sakura.
//!
//! When the detected code is none of ISO-2022-JP, EUC-JP, UTF-16, UTF-16BE, UTF-8 and UTF-7 (for
//! example plain Shift_JIS), the conversion fails: the text stays as it is and the command beeps
//! (Sakura shows "変換でエラーが発生しました", `STR_CONVERT_ERR`, and replaces the selection by
//! itself). The default code of the detection is UTF-8, Sakura's default; the document type's
//! setting is not available to commands yet.

use ume_core::encoding::DetectOptions;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert::code;
use crate::register_command;

#[derive(Default)]
pub struct CodecnvAuto2Sjis;

impl Command for CodecnvAuto2Sjis {
    const ID: &'static str = "codecnv_auto2_sjis";
    const SAKURA: &'static str = "F_CODECNV_AUTO2SJIS";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        let options = DetectOptions::default();
        let mut failed = false;
        cx.convert_selection(|text| {
            code::auto_to_sjis(text, &options).unwrap_or_else(|| {
                failed = true;
                text.to_owned()
            })
        })?;
        if failed {
            cx.beep();
        }
        Ok(())
    }
}

register_command!(CodecnvAuto2Sjis);

#[cfg(test)]
mod tests {
    use ume_core::encoding::Encoding;

    use crate::convert::code;
    use crate::testing::{Editor, check};

    #[test]
    fn detects_utf8_and_converts() {
        let garbled = code::from_sjis(Encoding::Utf8, "日本語のテキストです。");
        check(
            "codecnv_auto2_sjis",
            &format!("[{garbled}|"),
            "[日本語のテキストです。|",
        );
    }

    #[test]
    fn detects_iso_2022_jp_and_converts() {
        // The JIS bytes of 日本語 contain `|`, which a marked text cannot hold.
        let garbled = code::from_sjis(Encoding::Jis, "あいう");
        check("codecnv_auto2_sjis", &format!("[{garbled}|"), "[あいう|");
    }

    #[test]
    fn beeps_and_keeps_the_text_when_the_code_is_not_convertible() {
        let sjis = code::to_sjis(Encoding::ShiftJis, "日本語");
        let mut e = Editor::new(&format!("[{sjis}|"));
        e.run("codecnv_auto2_sjis")
            .assert_marked(&format!("[{sjis}|"));
        assert!(e.changes().beep);
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("codecnv_auto2_sjis", "ab|c", "ab|c");
    }
}
