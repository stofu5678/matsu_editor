//! `F_HTML_UNESCAPE`: MATSU-only (Sakura has no such function). Turns the character references of the selected text (`&amp; &lt; &gt; &quot; &apos;` and numeric ones) back into characters.
//!
//! The conversion is in [`crate::convert::text_encoding`]. Without a selection nothing happens, as in
//! Sakura's conversions (`CEditView::ConvSelectedArea` returns at once, `sakura_core/view/CEditView.cpp`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert;
use crate::register_command;

#[derive(Default)]
pub struct HtmlUnescape;

impl Command for HtmlUnescape {
    const ID: &'static str = "html_unescape";
    const SAKURA: &'static str = "F_HTML_UNESCAPE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.convert_selection(convert::text_encoding::html_unescape)
    }
}

register_command!(HtmlUnescape);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn converts_the_selection() {
        check(
            "html_unescape",
            "[a&lt;b&gt;&amp;&quot;c&#39;d&#x1F600;|",
            "[a<b>&\"c'd😀|",
        );
        check("html_unescape", "[&amp;lt;\r\n&nbsp;|", "[&lt;\r\n&nbsp;|");
    }

    #[test]
    fn does_nothing_without_a_selection() {
        check("html_unescape", "ab|c", "ab|c");
    }

    #[test]
    fn does_not_beep() {
        let mut e = Editor::new("[a|");
        e.run("html_unescape");
        assert!(!e.changes().beep);
    }
}
