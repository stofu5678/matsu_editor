//! `F_URL_ENCODE`: MATSU-only (Sakura has no such function). Percent-encodes the selected text (its UTF-8 bytes), except RFC 3986's unreserved characters.
//!
//! The conversion is in [`crate::convert::text_encoding`]. Without a selection nothing happens, as in
//! Sakura's conversions (`CEditView::ConvSelectedArea` returns at once, `sakura_core/view/CEditView.cpp`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct UrlEncode;

impl Command for UrlEncode {
    const ID: &'static str = "url_encode";
    const SAKURA: &'static str = "F_URL_ENCODE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::text_encoding::url_encode)
    }
}

register_command!(UrlEncode);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn converts_the_selection() {
        check("url_encode", "[abc xyz|", "[abc%20xyz|");
        check("url_encode", "[日本語|", "[%E6%97%A5%E6%9C%AC%E8%AA%9E|");
        check("url_encode", "[😀|", "[%F0%9F%98%80|");
        check("url_encode", "[a\r\nb\nc|", "[a%0D%0Ab%0Ac|");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("url_encode", "ab|c", "ab|c");
    }

    #[test]
    fn does_not_beep() {
        let mut e = Editor::new("[a|");
        e.run("url_encode");
        assert!(!e.changes().beep);
    }
}
