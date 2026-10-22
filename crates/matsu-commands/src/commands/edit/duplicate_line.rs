//! `F_DUPLICATELINE`: copies the row that has the caret and puts the copy before it.
//!
//! Sakura: `CViewCommander::Command_DUPLICATELINE` in
//! `sakura_core/cmd/CViewCommander_Edit_word_line.cpp`; bound to Ctrl+I in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * A selection is dropped first (also when the command then beeps).
//! * The row with its line ending is inserted at the start of the row; the caret stays at the same
//!   place in the original, which is now the row below the copy. For the last row of the file,
//!   which has no line ending, the copy gets the line ending that Enter inserts.
//! * The caret on the empty line after a final line ending, or in an empty document, has no row
//!   to copy: it beeps.

use matsu_core::buffer::Pos;
use matsu_core::model::eol_text;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct DuplicateLine;

impl Command for DuplicateLine {
    const ID: &'static str = "duplicate_line";
    const SAKURA: &'static str = "F_DUPLICATELINE";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.clear_selection();
        let caret = cx.caret();
        let Some(line) = cx.doc().line(caret.line)? else {
            cx.beep();
            return Ok(());
        };
        let start = cx.layout().row_start(cx.doc(), caret)?;
        let end = cx.layout().row_end(cx.doc(), caret)?;
        let mut copy = cx.doc().text_between(start, end)?;
        // only the last row of a line has the line ending (a wrapped line has more rows)
        if end.col == line.text.len() {
            copy.push_str(eol_text(line.eol.unwrap_or_else(|| cx.new_line())));
        }
        let inserted = cx.replace(start, start, &copy)?;
        // the text after `start` moved to `inserted.end`
        let col = inserted.end.col + (caret.col - start.col);
        cx.move_caret(Pos::new(inserted.end.line, col), false)
    }
}

register_command!(DuplicateLine);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Eol;

    use crate::testing::{Editor, check};

    #[test]
    fn puts_a_copy_before_the_row() {
        check("duplicate_line", "ab|c\r\nd", "abc\r\nab|c\r\nd");
        check("duplicate_line", "|abc\nd", "abc\n|abc\nd");
        check("duplicate_line", "x\nabc|\ny", "x\nabc\nabc|\ny");
        check("duplicate_line", "|\nb", "\n|\nb");
    }

    #[test]
    fn the_last_row_gets_a_line_ending() {
        // the line ending of the OS when the document has none yet (CRLF on Windows, as in
        // Sakura; LF elsewhere)
        let os = if cfg!(windows) { "\r\n" } else { "\n" };
        check("duplicate_line", "ab|c", &format!("abc{os}ab|c"));
        check("duplicate_line", "x\ny|", "x\ny\ny|");
        let mut e = Editor::new("ab|c").with_new_line(Eol::Lf);
        e.run("duplicate_line").assert_marked("abc\nab|c");
    }

    #[test]
    fn a_selection_is_dropped_and_the_caret_is_the_end_of_it() {
        check("duplicate_line", "a[b|c\nd", "abc\nab|c\nd");
        check("duplicate_line", "a|b[c\nd", "abc\na|bc\nd");
        // a selection over several rows works on the row of the caret
        check("duplicate_line", "a[b\nc|d", "ab\ncd\nc|d");
    }

    #[test]
    fn beeps_on_the_empty_line_after_the_last_line_ending() {
        for text in ["ab\n|", "|"] {
            let mut e = Editor::new(text);
            e.run("duplicate_line").assert_marked(text);
            assert!(e.changes().beep, "{text:?}");
            assert!(!e.changes().text_changed(), "{text:?}");
        }
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("ab|c\nd");
        e.run("duplicate_line").assert_marked("abc\nab|c\nd");
        e.run("undo").assert_marked("ab|c\nd");
    }
}
