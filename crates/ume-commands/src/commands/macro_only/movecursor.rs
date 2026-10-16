//! `F_MOVECURSOR` (the macro `MoveCursor(line, column, option)`): moves the caret to a line and a
//! column of the text, counted from 1.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) subtracts 1 from the line and
//! the column and needs all three arguments (without them the macro stops with an error), then
//! `CViewCommander::Command_MOVECURSOR` (`sakura_core/cmd/CViewCommander_Cursor.cpp`) turns the
//! logical position into a layout position and moves there. The column counts UTF-16 units. The
//! option is 0 (do not select), 1 (select from the old position) or 2 (select a box); the help is
//! `help/macro/source/reference/cursor/S_MoveCursor.html`. A line or a column below 1 beeps and
//! does nothing.

use crate::command::{Arg, Command, CommandError, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct MoveCursor;

/// The three numbers of `MoveCursor` and `MoveCursorLayout`.
pub(super) fn numbers(name: &str, args: &[Arg]) -> Result<(i64, i64, i64)> {
    match args {
        [Arg::Int(line), Arg::Int(column), Arg::Int(option)] => Ok((*line, *column, *option)),
        _ => Err(CommandError::BadArgs(format!(
            "{name} takes the line, the column and the option (three numbers)"
        ))),
    }
}

impl Command for MoveCursor {
    const ID: &'static str = "movecursor";
    const SAKURA: &'static str = "F_MOVECURSOR";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        let (line, column, option) = numbers("MoveCursor", args)?;
        cx.move_cursor_logical(line.saturating_sub(1), column.saturating_sub(1), option)
    }
}

register_command!(MoveCursor);

#[cfg(test)]
mod tests {
    use crate::command::{Arg, CommandError};
    use crate::testing::Editor;

    fn go(e: &mut Editor, line: i64, column: i64, option: i64) {
        e.run_with(
            "movecursor",
            &[Arg::Int(line), Arg::Int(column), Arg::Int(option)],
        );
    }

    #[test]
    fn the_line_and_the_column_count_from_1() {
        let mut e = Editor::new("|abc\ndef\nghi");
        go(&mut e, 2, 3, 0);
        e.assert_marked("abc\nde|f\nghi");
        go(&mut e, 1, 1, 0);
        e.assert_marked("|abc\ndef\nghi");
    }

    #[test]
    fn the_column_counts_utf16_units() {
        // 'a' is 1 unit and the emoji is 2: column 4 is before 'b'.
        let mut e = Editor::new("|a😀b\nxyz");
        go(&mut e, 1, 4, 0);
        e.assert_marked("a😀|b\nxyz");
        // The middle of the surrogate pair goes to its start.
        go(&mut e, 1, 3, 0);
        e.assert_marked("a|😀b\nxyz");
        // Japanese text: one unit per character.
        let mut e = Editor::new("|あいう\nえお");
        go(&mut e, 1, 3, 0);
        e.assert_marked("あい|う\nえお");
    }

    #[test]
    fn past_the_end_goes_to_the_end() {
        let mut e = Editor::new("|abc\ndef");
        go(&mut e, 1, 100, 0);
        e.assert_marked("abc|\ndef");
        go(&mut e, 100, 1, 0);
        e.assert_marked("abc\ndef|");
        // After a final line ending the end of the text is the empty line.
        let mut e = Editor::new("|abc\r\n");
        go(&mut e, 5, 5, 0);
        e.assert_marked("abc\r\n|");
    }

    #[test]
    fn below_1_beeps_and_does_nothing() {
        let mut e = Editor::new("ab|c\ndef");
        for (line, column) in [(0, 1), (1, 0), (-1, -1)] {
            go(&mut e, line, column, 0);
            assert!(e.changes().beep, "{line} {column}");
            e.assert_marked("ab|c\ndef");
        }
    }

    #[test]
    fn option_1_selects_from_the_old_position() {
        let mut e = Editor::new("a|bc\ndef");
        go(&mut e, 2, 3, 1);
        e.assert_marked("a[bc\nde|f");
        // The selection grows from the same anchor.
        go(&mut e, 1, 3, 1);
        e.assert_marked("a[b|c\ndef");
        // Option 0 drops it.
        go(&mut e, 1, 1, 0);
        e.assert_marked("|abc\ndef");
    }

    #[test]
    fn option_2_selects_a_box() {
        let mut e = Editor::new("a|bcd\nefgh\nijkl");
        go(&mut e, 3, 4, 2);
        // The box from column 1 of line 1 to column 3 of line 3.
        e.assert_box((0, 1), (2, 3));
        // Option 0 leaves the box mode.
        go(&mut e, 1, 1, 0);
        assert!(!e.cursor().is_box());
        assert!(!e.cursor().has_selection());
    }

    #[test]
    fn a_box_stops_at_the_end_of_a_short_line() {
        let mut e = Editor::new("|abcdef\nab\nabcdef");
        go(&mut e, 1, 5, 2);
        // MoveCursor takes a position of the text, so it stops at the end of the short line.
        go(&mut e, 2, 5, 2);
        e.assert_box((0, 0), (1, 2));
    }

    #[test]
    fn all_three_arguments_are_needed() {
        let mut e = Editor::new("|ab");
        for args in [
            vec![],
            vec![Arg::Int(1), Arg::Int(2)],
            vec![Arg::Int(1), Arg::Int(2), Arg::Text("0".to_owned())],
            vec![Arg::Int(1), Arg::Int(2), Arg::Int(0), Arg::Int(0)],
        ] {
            let result = e.try_run("movecursor", &args);
            assert!(matches!(result, Err(CommandError::BadArgs(_))), "{args:?}");
        }
        e.assert_marked("|ab");
    }

    #[test]
    fn it_is_not_an_edit_and_not_an_undo_step() {
        let mut e = Editor::new("|ab\ncd");
        go(&mut e, 2, 2, 1);
        assert!(!e.changes().text_changed());
        assert!(!e.model().history().can_undo());
        assert!(!e.model().is_modified());
    }
}
