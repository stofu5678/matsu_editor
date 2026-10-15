//! `F_INS_DATE`: inserts the current date at the caret, in place of the selection.
//!
//! Sakura: `CViewCommander::Command_INS_DATE` in `sakura_core/cmd/CViewCommander_Insert.cpp`. It
//! formats `GetLocalTime()` with `CFormatManager::MyGetDateFormat` and pastes the text
//! (`Command_INSTEXT`).
//!
//! Differences:
//!
//! * Sakura's setting `m_nDateFormatType` is 0 by default, which means the date format of the
//!   Windows locale (`DATE_LONGDATE`), and the picture `m_szDateFormat` is used only for type 1.
//!   There is no such locale here, so this command always uses Sakura's default *picture*,
//!   `yyyy'年'M'月'd'日('dddd')'`, which gives `2026年10月15日(木曜日)`. A setting for the
//!   picture will come with the settings (`ume-config`).
//! * A rectangular selection is not implemented.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::convert::datetime::{Clock, DEFAULT_DATE_PICTURE, SystemClock, format_date};
use crate::register_command;

#[derive(Default)]
pub struct InsertDate;

impl Command for InsertDate {
    const ID: &'static str = "insert_date";
    const SAKURA: &'static str = "F_INS_DATE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        insert_date(cx, &SystemClock, DEFAULT_DATE_PICTURE)
    }
}

/// Inserts the date of `clock` by `picture` (the command with the clock and the picture given, so
/// that tests can fix them).
fn insert_date(cx: &mut CommandContext<'_>, clock: &dyn Clock, picture: &str) -> Result {
    let text = format_date(&clock.now(), picture);
    cx.insert_text(&text)?;
    Ok(())
}

register_command!(InsertDate);

#[cfg(test)]
mod tests {
    use ume_core::buffer::Document;
    use ume_core::cursor::Cursor;
    use ume_core::model::DocumentModel;

    use super::*;
    use crate::convert::datetime::tests::{FixedClock, thursday_morning};
    use crate::layout::PlainLayout;
    use crate::testing::{Editor, parse, render};

    /// Runs `insert_date` with a fixed clock on a marked text and returns the marked result.
    fn run(marked: &str, picture: &str) -> String {
        let m = parse(marked).unwrap();
        let mut model = DocumentModel::new(Document::from_utf8(m.text.into_bytes()).unwrap());
        let mut cursor = Cursor::default();
        cursor.select(m.anchor.unwrap_or(m.caret), m.caret);
        let mut layout = PlainLayout::default();
        let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
        insert_date(&mut cx, &FixedClock(thursday_morning()), picture).unwrap();
        render(model.doc(), &cursor)
    }

    #[test]
    fn inserts_the_default_picture_at_the_caret() {
        assert_eq!(
            run("ab|cd", DEFAULT_DATE_PICTURE),
            "ab2026年10月15日(木曜日)|cd"
        );
    }

    #[test]
    fn replaces_the_selection() {
        assert_eq!(run("a[bc|d", "yyyy/MM/dd"), "a2026/10/15|d");
        assert_eq!(run("a|bc[d", "yyyy/MM/dd"), "a2026/10/15|d");
    }

    #[test]
    fn the_registered_command_inserts_the_date_of_today() {
        let mut e = Editor::new("x|");
        e.run("insert_date");
        let text = e.marked();
        assert!(
            text.starts_with('x') && text.contains('年') && text.ends_with("日)|"),
            "{text}"
        );
        // one undo step
        e.run("undo").assert_marked("x|");
    }
}
