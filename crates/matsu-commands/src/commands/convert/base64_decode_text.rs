//! `F_BASE64_DECODE_TEXT`: MATSU-only (Sakura has no such function). Decodes the selected Base64 text and reads the bytes as UTF-8.
//!
//! The conversion is in [`crate::convert::text_encoding`]. Without a selection nothing happens, as in
//! Sakura's conversions (`CEditView::ConvSelectedArea` returns at once, `sakura_core/view/CEditView.cpp`).
//! Text that cannot be decoded is left as it is and the command beeps.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct Base64DecodeText;

impl Command for Base64DecodeText {
    const ID: &'static str = "base64_decode_text";
    const SAKURA: &'static str = "F_BASE64_DECODE_TEXT";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection_or_beep(convert::text_encoding::base64_decode_text)
    }
}

register_command!(Base64DecodeText);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn converts_the_selection() {
        check("base64_decode_text", "[Zm9vYmFy|", "[foobar|");
        check("base64_decode_text", "[5pel5pys6Kqe|", "[日本語|");
        check("base64_decode_text", "[8J+YgA|", "[😀|");
        check("base64_decode_text", "[YQ0K\r\nYg==|", "[a\r\nb|");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("base64_decode_text", "ab|c", "ab|c");
    }

    #[test]
    fn beeps_and_keeps_the_text_that_cannot_be_decoded() {
        for text in ["[Zm9v!|", "[Zg=|", "[//4=|"] {
            let mut e = Editor::new(text);
            e.run("base64_decode_text").assert_marked(text);
            assert!(e.changes().beep, "{text}");
        }
    }
}
