//! `F_SORT_ASC`: sorts the selected lines in ascending order.
//!
//! Sakura: `CViewCommander::Command_SORT( TRUE )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`; bound to Alt+A in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * The lines are those from the line of the start of the selection to the line of its end; the
//!   last one only if the selection ends after its column 0 and it has a line ending. So the last
//!   line of the file, if it has no line ending, is never sorted.
//! * Lines are compared with their line endings, by UTF-16 code unit (not by code point), and a
//!   line that is a prefix of another one comes first. The sort is stable.
//! * Afterwards the lines are selected from the start of the first to the start of the line after
//!   the last. The caret is at the start if it was in the first line, otherwise at the end.
//! * Nothing selected, or no line in the range (a selection inside the last line, which has no
//!   line ending): nothing happens, and there is no beep.
//!
//! * A box selection sorts the lines from its top to its bottom line by the text in its columns
//!   (see `CommandContext::sort_box_lines`; the `bBeginBoxSelectOld` branch of Sakura).
//!
//! Difference: if the order does not change, no edit is made.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct SortAsc;

impl Command for SortAsc {
    const ID: &'static str = "sort_asc";
    const SAKURA: &'static str = "F_SORT_ASC";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.sort_selected_lines(true)
    }
}

register_command!(SortAsc);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};
    use ume_core::buffer::Pos;
    use ume_core::cursor::Cursor;

    #[test]
    fn sorts_the_selected_lines() {
        check("sort_asc", "[c\nb\na\n|", "[a\nb\nc\n|");
        check("sort_asc", "[c\r\nb\r\na\r\n|d", "[a\r\nb\r\nc\r\n|d");
    }

    #[test]
    fn a_selection_that_ends_inside_a_line_includes_it() {
        check("sort_asc", "[c\nb\na|\nz", "[a\nb\nc\n|z");
        check("sort_asc", "c\n[b\na\ndx|\ne", "c\n[a\nb\ndx\n|e");
    }

    #[test]
    fn the_last_line_without_a_line_ending_is_not_sorted() {
        // the selection reaches the end of the file, but "a" has no line ending
        check("sort_asc", "[c\nb\na|", "[b\nc\n|a");
    }

    #[test]
    fn compares_by_utf16_code_unit() {
        // U+FF5E is one unit (0xFF5E); U+1F600 is two units that start with 0xD83D
        check(
            "sort_asc",
            "[\u{ff5e}\n\u{1f600}\n|",
            "[\u{1f600}\n\u{ff5e}\n|",
        );
    }

    #[test]
    fn a_prefix_comes_first_and_line_endings_count() {
        check("sort_asc", "[ab\na\n|", "[a\nab\n|");
        // the line endings are compared too: LF is smaller than CR
        check("sort_asc", "[a\r\na\na\r\n|", "[a\na\r\na\r\n|");
    }

    #[test]
    fn the_caret_stays_at_the_end_it_was_at() {
        // in the first line: at the start
        check("sort_asc", "|b\na\n[", "|a\nb\n[");
        // in another line: at the end
        check("sort_asc", "[b\na|\nc", "[a\nb\n|c");
    }

    #[test]
    fn does_nothing_without_a_selection_or_inside_the_last_line() {
        for text in ["b\n|a\n", "a\n[b|", "a\nb|"] {
            let mut e = Editor::new(text);
            e.run("sort_asc").assert_marked(text);
            assert!(!e.changes().beep && !e.changes().text_changed(), "{text:?}");
        }
    }

    #[test]
    fn a_selection_inside_one_line_selects_the_whole_line() {
        // as in Sakura: the line of the end is added to the range, so it is one line, sorted alone
        check("sort_asc", "a[b|c\nd", "|abc\n[d");
    }

    type Corners = Option<((usize, usize), (usize, usize))>;

    /// Selects a box from the caret of `marked` `right` columns to the right and `down` lines
    /// down, runs `id`, and returns the text and the corners of the box.
    fn sort_box(id: &str, marked: &str, right: usize, down: usize) -> (String, Corners) {
        let mut e = Editor::new(marked);
        e.run("begin_box");
        for _ in 0..right {
            e.run("cursor_right_box");
        }
        for _ in 0..down {
            e.run("cursor_down_box");
        }
        e.run(id);
        (e.marked().replace(['|', '['], ""), e.box_state())
    }

    #[test]
    fn a_box_selection_sorts_the_lines_by_the_columns_of_the_box() {
        // columns 1..3 are the keys; the lines move whole
        let (text, corners) = sort_box("sort_asc", "x|59z\nx31y\nx92x\nx17w\nrest\n", 2, 3);
        assert_eq!(text, "x17w\nx31y\nx59z\nx92x\nrest\n");
        assert_eq!(corners, Some(((0, 1), (3, 3))));
        // lines below the box are left alone, and so are the ones above
        let (text, _) = sort_box("sort_asc", "top\nx|b\nx\nxa\nlast\n", 1, 2);
        assert_eq!(text, "top\nx\nxa\nxb\nlast\n");
    }

    #[test]
    fn a_box_of_width_zero_sorts_by_the_text_from_its_column() {
        let (text, _) = sort_box("sort_asc", "a|z\nbc\n", 0, 1);
        assert_eq!(text, "bc\naz\n");
    }

    #[test]
    fn a_box_selection_keeps_equal_keys_in_order() {
        let (text, _) = sort_box("sort_asc", "1|b\n2a\n3b\n4a\n", 1, 3);
        assert_eq!(text, "2a\n4a\n1b\n3b\n");
        let (text, _) = sort_box("sort_desc", "1|b\n2a\n3b\n4a\n", 1, 3);
        assert_eq!(text, "1b\n3b\n2a\n4a\n");
    }

    #[test]
    fn a_line_that_ends_before_the_box_has_an_empty_key() {
        let (text, _) = sort_box("sort_asc", "a|bc\nx\nabb\n", 1, 2);
        assert_eq!(text, "x\nabc\nabb\n");
    }

    #[test]
    fn a_box_selection_sorts_the_last_line_without_an_ending_too() {
        // the ending moves from the last line to the one that ends up last
        let (text, _) = sort_box("sort_asc", "|b\r\na", 1, 1);
        assert_eq!(text, "a\r\nb");
        let (text, _) = sort_box("sort_desc", "|a\nb", 1, 1);
        assert_eq!(text, "b\na");
    }

    #[test]
    fn a_box_selection_compares_the_keys_by_utf16_code_unit_and_ends_on_japanese_text() {
        let (text, _) = sort_box("sort_asc", "|\u{ff5e}\u{1f600}\n\u{1f600}\u{ff5e}\n", 2, 1);
        assert_eq!(text, "\u{1f600}\u{ff5e}\n\u{ff5e}\u{1f600}\n");
    }

    #[test]
    fn a_box_selection_that_selects_nothing_does_nothing() {
        let mut e = Editor::new("b|\na\n");
        e.run("begin_box").run("sort_asc");
        e.assert_marked("b|\na\n");
        assert!(!e.changes().text_changed());
    }

    #[test]
    fn the_corners_of_the_box_stay_on_char_boundaries_after_the_lines_moved() {
        // wide chars move to the first line: the corner is recomputed from its display column
        let (text, corners) = sort_box("sort_desc", "|xz\n\u{3042}a\n", 2, 1);
        assert_eq!(text, "\u{3042}a\nxz\n");
        assert_eq!(corners, Some(((0, 0), (1, 2))));
    }

    #[test]
    fn the_corners_are_clamped_when_the_sorted_lines_join_a_cr_and_an_lf() {
        // "b\r" moves to the top, so its CR meets the LF of the next line: one CRLF, one row less
        let mut cursor = Cursor::default();
        cursor.begin_box(0, false);
        cursor.move_box_to(Pos::new(3, 0), 1);
        let mut e = Editor::from_parts("\n\nb\r", cursor);
        assert_eq!(e.box_state(), Some(((0, 0), (3, 1))));
        e.run("sort_desc");
        assert_eq!(e.marked().replace(['|', '['], ""), "b\r\n\n");
        let ((_, _), (caret_row, _)) = e.box_state().unwrap();
        assert!(
            caret_row <= 2,
            "the caret is inside the document: {caret_row}"
        );
    }

    #[test]
    fn a_box_sort_that_the_view_mode_refuses_keeps_the_selection() {
        let mut e = Editor::new("|b\na\n");
        e.set_view_mode(true);
        e.run("begin_box")
            .run("cursor_right_box")
            .run("cursor_down_box");
        let before = e.marked();
        assert!(e.try_run("sort_asc", &[]).is_err());
        assert_eq!(e.marked(), before);
        assert!(e.box_state().is_some());
    }

    #[test]
    fn a_box_sort_is_one_undo_step() {
        let mut e = Editor::new("|b\na\n");
        e.run("begin_box")
            .run("cursor_right_box")
            .run("cursor_down_box");
        e.run("sort_asc").run("undo");
        assert_eq!(e.marked().replace(['|', '['], ""), "b\na\n");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("[b\na\n|");
        e.run("sort_asc").assert_marked("[a\nb\n|");
        e.run("undo").assert_marked("[b\na\n|");
    }
}
