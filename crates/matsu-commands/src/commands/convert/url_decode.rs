//! `F_URL_DECODE`: MATSU-only (Sakura has no such function). Decodes the percent sequences of the selected text and reads the bytes as UTF-8.
//!
//! The conversion is in [`crate::convert::text_encoding`]. Without a selection nothing happens, as in
//! Sakura's conversions (`CEditView::ConvSelectedArea` returns at once, `sakura_core/view/CEditView.cpp`).
//! Text that cannot be decoded is left as it is and the command beeps.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct UrlDecode;

impl Command for UrlDecode {
    const ID: &'static str = "url_decode";
    const SAKURA: &'static str = "F_URL_DECODE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection_or_beep(convert::text_encoding::url_decode)
    }
}

register_command!(UrlDecode);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn converts_the_selection() {
        check("url_decode", "[abc%20xyz+|", "[abc xyz+|");
        check("url_decode", "[%E6%97%A5%E6%9C%AC%E8%AA%9E|", "[日本語|");
        check("url_decode", "[%F0%9F%98%80|", "[😀|");
        check("url_decode", "[a%0D%0Ab%0Ac|", "[a\r\nb\nc|");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("url_decode", "ab|c", "ab|c");
    }

    #[test]
    fn beeps_and_keeps_the_text_that_cannot_be_decoded() {
        for text in ["[100%|", "[%zz|", "[%80|"] {
            let mut e = Editor::new(text);
            e.run("url_decode").assert_marked(text);
            assert!(e.changes().beep, "{text}");
        }
    }
}
