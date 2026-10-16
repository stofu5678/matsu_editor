//! `F_MOVECURSORLAYOUT` (the macro `MoveCursorLayout(row, column, option)`): moves the caret to a
//! row and a display column of the layout, counted from 1.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) subtracts 1 from both and
//! needs all three arguments, then `CViewCommander::Command_MOVECURSORLAYOUT`
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). A row is one line on the screen (with wrapping
//! a line has several), a column counts cells (a tab and a full-width character take more than
//! one). A row past the last is the end of the text (`CCaret::GetAdjustCursorPos`). The option is
//! 0, 1 or 2 as for `MoveCursor`; a box can end in the empty space right of a short row.

use super::movecursor::numbers;
use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct MoveCursorLayout;

impl Command for MoveCursorLayout {
    const ID: &'static str = "movecursorlayout";
    const SAKURA: &'static str = "F_MOVECURSORLAYOUT";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        let (row, column, option) = numbers("MoveCursorLayout", args)?;
        cx.move_cursor_layout(row.saturating_sub(1), column.saturating_sub(1), option)
    }
}

register_command!(MoveCursorLayout);

#[cfg(test)]
mod tests {
    use crate::command::{Arg, CommandError};
    use crate::testing::Editor;

    fn go(e: &mut Editor, row: i64, column: i64, option: i64) {
        e.run_with(
            "movecursorlayout",
            &[Arg::Int(row), Arg::Int(column), Arg::Int(option)],
        );
    }

    #[test]
    fn the_column_counts_cells() {
        // Full-width characters take 2 cells: column 3 is the start of the second character.
        let mut e = Editor::new("|あいう\nabc");
        go(&mut e, 1, 3, 0);
        e.assert_marked("あ|いう\nabc");
        // The middle of a full-width character goes to its start.
        go(&mut e, 1, 4, 0);
        e.assert_marked("あ|いう\nabc");
        go(&mut e, 2, 2, 0);
        e.assert_marked("あいう\na|bc");
    }

    #[test]
    fn a_tab_takes_cells_up_to_the_next_tab_stop() {
        let mut e = Editor::new("|\tabc").with_tab_width(4);
        go(&mut e, 1, 5, 0);
        e.assert_marked("\t|abc");
        go(&mut e, 1, 3, 0);
        e.assert_marked("|\tabc");
    }

    #[test]
    fn it_differs_from_move_cursor_when_the_line_has_wide_characters() {
        let mut a = Editor::new("|あいう");
        a.run_with("movecursor", &[Arg::Int(1), Arg::Int(3), Arg::Int(0)]);
        let mut b = Editor::new("|あいう");
        go(&mut b, 1, 3, 0);
        a.assert_marked("あい|う");
        b.assert_marked("あ|いう");
    }

    #[test]
    fn past_the_end_goes_to_the_end() {
        let mut e = Editor::new("|abc\ndef");
        go(&mut e, 1, 50, 0);
        e.assert_marked("abc|\ndef");
        go(&mut e, 9, 1, 0);
        e.assert_marked("abc\ndef|");
    }

    #[test]
    fn below_1_beeps_and_does_nothing() {
        let mut e = Editor::new("a|b");
        go(&mut e, 0, 1, 0);
        assert!(e.changes().beep);
        go(&mut e, 1, 0, 0);
        assert!(e.changes().beep);
        e.assert_marked("a|b");
    }

    #[test]
    fn options_select() {
        let mut e = Editor::new("a|bc\ndef");
        go(&mut e, 2, 3, 1);
        e.assert_marked("a[bc\nde|f");
        let mut e = Editor::new("a|bcd\nefgh\nijkl");
        go(&mut e, 3, 4, 2);
        e.assert_box((0, 1), (2, 3));
    }

    #[test]
    fn a_box_goes_into_the_empty_space() {
        let mut e = Editor::new("|abcdef\nab\nabcdef");
        go(&mut e, 2, 6, 2);
        // The caret is at the end of "ab", and the box reaches column 5 (0-based) of line 2.
        e.assert_box((0, 0), (1, 5));
        assert_eq!(e.cursor().caret().col, 2);
    }

    #[test]
    fn all_three_arguments_are_needed() {
        let mut e = Editor::new("|ab");
        let result = e.try_run("movecursorlayout", &[Arg::Int(1), Arg::Int(1)]);
        assert!(matches!(result, Err(CommandError::BadArgs(_))));
    }
}
