//! `F_CLIPBOARDEMPTY` (the macro `ClipboardEmpty()`): empties the clipboard.
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp` makes a `CClipboard` and calls
//! `Empty()` (`EmptyClipboard`, `sakura_core/_os/CClipboard.cpp`). A paste after it finds no text
//! and beeps.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct ClipboardEmpty;

impl Command for ClipboardEmpty {
    const ID: &'static str = "clipboardempty";
    const SAKURA: &'static str = "F_CLIPBOARDEMPTY";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.clipboard_clear()
    }
}

register_command!(ClipboardEmpty);

#[cfg(test)]
mod tests {
    use crate::clipboard::ClipKind;
    use crate::testing::Editor;

    #[test]
    fn the_clipboard_has_no_text_afterwards() {
        let mut e = Editor::new("|ab").with_clipboard("text", ClipKind::Normal);
        e.run("clipboardempty");
        assert_eq!(e.clipboard(), None);
        // A paste finds nothing: it beeps and the text stays.
        e.run("paste");
        assert!(e.changes().beep);
        e.assert_marked("|ab");
    }

    #[test]
    fn an_empty_clipboard_stays_empty() {
        let mut e = Editor::new("|ab");
        e.run("clipboardempty");
        assert_eq!(e.clipboard(), None);
    }

    #[test]
    fn a_copy_fills_it_again() {
        let mut e = Editor::new("[ab|c");
        e.run("copy").run("clipboardempty").run("copy");
        assert_eq!(e.clipboard().map(|c| c.text.as_str()), Some("ab"));
    }

    #[test]
    fn it_is_not_an_edit() {
        let mut e = Editor::new("a|b").with_clipboard("x", ClipKind::Line);
        e.run("clipboardempty");
        assert!(!e.changes().text_changed());
        assert!(!e.model().history().can_undo());
    }
}
