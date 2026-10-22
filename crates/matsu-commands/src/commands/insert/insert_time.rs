//! `F_INS_TIME`: inserts the current time at the caret, in place of the selection.
//!
//! Sakura: `CViewCommander::Command_INS_TIME` in `sakura_core/cmd/CViewCommander_Insert.cpp`. It
//! formats `GetLocalTime()` with `CFormatManager::MyGetTimeFormat` and pastes the text
//! (`Command_INSTEXT`).
//!
//! Differences:
//!
//! * Sakura's setting `m_nTimeFormatType` is 0 by default, which means the time format of the
//!   Windows locale, and the picture `m_szTimeFormat` is used only for type 1. There is no such
//!   locale here, so type 0 uses Sakura's default *picture*,
//!   `tthh'時'mm'分'ss'秒'`, which gives `午前09時05分07秒`.
//!   Type 1 uses the picture of the common settings ([`crate::format_options`]).
//! * A rectangular selection is not implemented.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert::datetime::{Clock, SystemClock, format_time};
use crate::format_options;
use crate::register_command;

#[derive(Default)]
pub struct InsertTime;

impl Command for InsertTime {
    const ID: &'static str = "insert_time";
    const SAKURA: &'static str = "F_INS_TIME";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        insert_time(cx, &SystemClock, format_options::get().time_picture())
    }
}

/// Inserts the time of `clock` by `picture` (the command with the clock and the picture given, so
/// that tests can fix them).
fn insert_time(cx: &mut CommandContext<'_>, clock: &dyn Clock, picture: &str) -> Result {
    let text = format_time(&clock.now(), picture);
    cx.insert_text(&text)?;
    Ok(())
}

register_command!(InsertTime);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Document;
    use matsu_core::cursor::Cursor;
    use matsu_core::model::DocumentModel;

    use super::*;
    use crate::convert::datetime::DEFAULT_TIME_PICTURE;
    use crate::convert::datetime::tests::{FixedClock, thursday_morning};
    use crate::layout::PlainLayout;
    use crate::testing::{Editor, parse, render};

    /// Runs `insert_time` with a fixed clock on a marked text and returns the marked result.
    fn run(marked: &str, picture: &str) -> String {
        let m = parse(marked).unwrap();
        let mut model = DocumentModel::new(Document::from_utf8(m.text.into_bytes()).unwrap());
        let mut cursor = Cursor::default();
        cursor.select(m.anchor.unwrap_or(m.caret), m.caret);
        let mut layout = PlainLayout::default();
        let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
        insert_time(&mut cx, &FixedClock(thursday_morning()), picture).unwrap();
        render(model.doc(), &cursor)
    }

    #[test]
    fn inserts_the_default_picture_at_the_caret() {
        assert_eq!(run("ab|cd", DEFAULT_TIME_PICTURE), "ab午前09時05分07秒|cd");
    }

    #[test]
    fn replaces_the_selection() {
        assert_eq!(run("a[bc|d", "HH:mm:ss"), "a09:05:07|d");
        assert_eq!(run("a|bc[d", "HH:mm:ss"), "a09:05:07|d");
    }

    #[test]
    fn the_registered_command_inserts_the_time_of_now() {
        let mut e = Editor::new("x|");
        e.run("insert_time");
        let text = e.marked();
        assert!(
            text.starts_with('x') && text.contains('時') && text.ends_with("秒|"),
            "{text}"
        );
        // one undo step
        e.run("undo").assert_marked("x|");
    }
}
