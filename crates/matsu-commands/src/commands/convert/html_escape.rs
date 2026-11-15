//! `F_HTML_ESCAPE`: MATSU-only (Sakura has no such function). Replaces `& < > " '` in the selected text by character references.
//!
//! The conversion is in [`crate::convert::text_encoding`]. Without a selection nothing happens, as in
//! Sakura's conversions (`CEditView::ConvSelectedArea` returns at once, `sakura_core/view/CEditView.cpp`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct HtmlEscape;

impl Command for HtmlEscape {
    const ID: &'static str = "html_escape";
    const SAKURA: &'static str = "F_HTML_ESCAPE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::text_encoding::html_escape)
    }
}

register_command!(HtmlEscape);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn converts_the_selection() {
        check(
            "html_escape",
            "[a<b>&\"c\"'d'|",
            "[a&lt;b&gt;&amp;&quot;c&quot;&#39;d&#39;|",
        );
        check("html_escape", "[日本😀\r\n<|", "[日本😀\r\n&lt;|");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("html_escape", "ab|c", "ab|c");
    }

    #[test]
    fn does_not_beep() {
        let mut e = Editor::new("[a|");
        e.run("html_escape");
        assert!(!e.changes().beep);
    }
}
