//! `F_SET_QUOTESTRING` (the macro `SetMsgQuoteStr(text)`): sets the quote string that
//! `F_COPYLINESASPASSAGE` puts before every row (引用符付きコピー).
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp` stops with an error when the
//! text is missing, then `CViewCommander::Command_SET_QUOTESTRING`
//! (`sakura_core/cmd/CViewCommander_Settings.cpp`) copies it into `m_szInyouKigou` of the shared
//! common settings (`StaticString<32>`: it keeps 31 UTF-16 units and cuts the rest,
//! `wcsncpy_s(.., _TRUNCATE)`). The setting is the same for every window and is saved with the
//! common settings; here the window keeps it for the windows of the session until the settings
//! can hold it.

use crate::command::{Arg, Command, CommandError, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct SetQuoteString;

impl Command for SetQuoteString {
    const ID: &'static str = "set_quotestring";
    const SAKURA: &'static str = "F_SET_QUOTESTRING";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        match args {
            [Arg::Text(quote)] => {
                cx.set_quote_string(quote);
                Ok(())
            }
            _ => Err(CommandError::BadArgs(
                "SetMsgQuoteStr takes one text".to_owned(),
            )),
        }
    }
}

register_command!(SetQuoteString);

#[cfg(test)]
mod tests {
    use crate::clipboard::ClipKind;
    use crate::command::{Arg, CommandError};
    use crate::testing::Editor;

    fn set(e: &mut Editor, quote: &str) {
        e.run_with("set_quotestring", &[Arg::Text(quote.to_owned())]);
    }

    fn passage(e: &mut Editor) -> String {
        e.run("copy_lines_as_passage");
        e.clipboard().map(|c| c.text.clone()).unwrap_or_default()
    }

    #[test]
    fn the_default_quote_is_greater_than_and_a_space() {
        let mut e = Editor::new("[ab\ncd|");
        assert_eq!(passage(&mut e), "> ab\n> cd\n");
    }

    #[test]
    fn copy_lines_as_passage_uses_the_new_quote() {
        let mut e = Editor::new("[ab\ncd|");
        set(&mut e, "# ");
        assert_eq!(passage(&mut e), "# ab\n# cd\n");
        set(&mut e, "　＞");
        assert_eq!(passage(&mut e), "　＞ab\n　＞cd\n");
    }

    #[test]
    fn only_31_utf16_units_are_kept() {
        let long: String = "x".repeat(40);
        let mut e = Editor::new("[ab|");
        set(&mut e, &long);
        assert_eq!(
            e.changes().quote_string.as_deref(),
            Some("x".repeat(31).as_str())
        );
        // A surrogate pair is two units and is not cut in the middle.
        let emoji = format!("{}😀", "y".repeat(30));
        set(&mut e, &emoji);
        assert_eq!(
            e.changes().quote_string.as_deref(),
            Some("y".repeat(30).as_str())
        );
        let exact = format!("{}😀", "y".repeat(29));
        set(&mut e, &exact);
        assert_eq!(e.changes().quote_string.as_deref(), Some(exact.as_str()));
    }

    #[test]
    fn a_missing_text_is_a_mistake() {
        let mut e = Editor::new("|ab");
        for args in [
            vec![],
            vec![Arg::Int(1)],
            vec![Arg::Text("a".into()), Arg::Int(1)],
        ] {
            let result = e.try_run("set_quotestring", &args);
            assert!(matches!(result, Err(CommandError::BadArgs(_))), "{args:?}");
        }
        assert_eq!(e.changes().quote_string, None);
    }

    #[test]
    fn it_changes_nothing_in_the_document_or_the_clipboard() {
        let mut e = Editor::new("a|b").with_clipboard("keep", ClipKind::Normal);
        set(&mut e, "> ");
        assert_eq!(e.clipboard().map(|c| c.text.as_str()), Some("keep"));
        assert!(!e.model().history().can_undo());
        e.assert_marked("a|b");
    }
}
