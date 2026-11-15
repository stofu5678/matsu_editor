//! `F_BASE64_ENCODE`: MATSU-only (Sakura has no such function). Encodes the selected text (its UTF-8 bytes) as Base64.
//!
//! The conversion is in [`crate::convert::text_encoding`]. Without a selection nothing happens, as in
//! Sakura's conversions (`CEditView::ConvSelectedArea` returns at once, `sakura_core/view/CEditView.cpp`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct Base64Encode;

impl Command for Base64Encode {
    const ID: &'static str = "base64_encode";
    const SAKURA: &'static str = "F_BASE64_ENCODE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::text_encoding::base64_encode)
    }
}

register_command!(Base64Encode);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn converts_the_selection() {
        check("base64_encode", "[foobar|", "[Zm9vYmFy|");
        check("base64_encode", "[日本語|", "[5pel5pys6Kqe|");
        check("base64_encode", "[😀|", "[8J+YgA==|");
        check("base64_encode", "[a\r\nb|", "[YQ0KYg==|");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("base64_encode", "ab|c", "ab|c");
    }

    #[test]
    fn does_not_beep() {
        let mut e = Editor::new("[a|");
        e.run("base64_encode");
        assert!(!e.changes().beep);
    }
}
