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
        // `CConvert_CodeAutoToSjis`: the detection of the type of the document
        // (`m_sEncodingConfig`): its default encoding and its CESU-8 preference.
        let options = cx.input_options().detect.clone();
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
    use matsu_core::encoding::{DetectOptions, Encoding};

    use crate::convert::code;
    use crate::input_options::InputOptions;
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

    fn typed(prefer_cesu8: bool, default_encoding: Encoding) -> InputOptions {
        InputOptions {
            detect: DetectOptions {
                default_encoding,
                prefer_cesu8,
            },
            ..Default::default()
        }
    }

    #[test]
    fn the_cesu8_preference_of_the_type_decides_a_tie_of_utf8_and_cesu8() {
        // Text without surrogate pairs reads the same as UTF-8 and as CESU-8: the type's
        // `bPriorCesu8` picks CESU-8, which the conversion does not support (it fails).
        let garbled = code::from_sjis(Encoding::Utf8, "日本語のテキストです。");
        let marked = format!("[{garbled}|");
        let mut e = Editor::new(&marked).with_input_options(typed(true, Encoding::Utf8));
        e.run("codecnv_auto2_sjis").assert_marked(&marked);
        assert!(e.changes().beep);
        let mut e = Editor::new(&marked).with_input_options(typed(false, Encoding::Utf8));
        e.run("codecnv_auto2_sjis")
            .assert_marked("[日本語のテキストです。|");
        assert!(!e.changes().beep);
    }

    #[test]
    fn the_default_encoding_of_the_type_decides_when_nothing_else_does() {
        // ASCII only: the detection finds the type's default encoding, and Shift_JIS is no
        // encoding that the conversion supports.
        let mut e = Editor::new("[abc|").with_input_options(typed(false, Encoding::ShiftJis));
        e.run("codecnv_auto2_sjis");
        assert!(e.changes().beep);
        let mut e = Editor::new("[abc|").with_input_options(typed(false, Encoding::Utf8));
        e.run("codecnv_auto2_sjis").assert_marked("[abc|");
        assert!(!e.changes().beep);
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("codecnv_auto2_sjis", "ab|c", "ab|c");
    }
}
