//! Tests of the free cursor in the gap after a wrapped row (see the module documentation of
//! [`free_cursor`](super::free_cursor)). The layout wraps every line after 3 chars: "abcdefgh" is
//! the rows "abc" (to `(0, 3)`), "def" (to `(0, 6)`) and "gh".
//!
//! The expectations come from `CCaret::MoveCursorProperly`, `CCaret::Cursor_UPDOWN`,
//! `CViewCommander::Command_LEFT`, `Command_RIGHT`, `Command_DELETE` and `Command_DELETE_BACK`
//! (`sakura_core/`), which treat the end of a wrapped layout row like the end of any row. No test
//! here reads a path or depends on the line ending of the platform: the documents carry their own.

use matsu_core::buffer::{Document, Eol, Pos};
use matsu_core::cursor::Cursor;
use matsu_core::model::DocumentModel;

use super::CommandContext;
use super::box_edit_tests::Wrap;
use crate::clipboard::MemoryClipboard;
use crate::command::Arg;
use crate::edit_options::{EditOptions, with_options};
use crate::layout::X;
use crate::registry::Registry;

fn free() -> EditOptions {
    EditOptions {
        free_cursor: true,
        ..EditOptions::DEFAULT
    }
}

/// A document on the wrapping layout, with a cursor.
struct Rig {
    model: DocumentModel,
    cursor: Cursor,
    layout: Wrap,
    clip: MemoryClipboard,
    overwrite: bool,
}

impl Rig {
    fn new(text: &str) -> Self {
        let mut model = DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap());
        model.set_new_line(Eol::Lf);
        Self {
            model,
            cursor: Cursor::default(),
            layout: Wrap(3),
            clip: MemoryClipboard::default(),
            overwrite: false,
        }
    }

    fn cx<R>(&mut self, f: impl FnOnce(&mut CommandContext<'_>) -> R) -> R {
        let mut cx = CommandContext::new(&mut self.model, &mut self.cursor, &mut self.layout, None)
            .with_clipboard(&mut self.clip)
            .with_overwrite(self.overwrite);
        f(&mut cx)
    }

    fn run(&mut self, id: &str) -> &mut Self {
        self.run_with(id, &[])
    }

    fn run_with(&mut self, id: &str, args: &[Arg]) -> &mut Self {
        self.cx(|cx| Registry::global().run(id, cx, args).unwrap());
        self
    }

    /// A click at `pos` (the end of a row) and `x` columns from the left edge.
    fn click(&mut self, pos: Pos, x: u32) -> &mut Self {
        self.cx(|cx| cx.move_caret_free(pos, X::from_raw(x), false).unwrap());
        self
    }

    fn caret(&self) -> Pos {
        self.cursor.caret()
    }

    fn col(&self) -> Option<usize> {
        self.cursor.free_col()
    }

    fn row_pos(&mut self) -> Pos {
        self.cx(|cx| cx.caret_row_pos().unwrap())
    }

    fn text(&self) -> String {
        let doc = self.model.doc();
        doc.text_between(Pos::new(0, 0), doc.end_pos().unwrap())
            .unwrap()
    }
}

const P1: Pos = Pos::new(0, 3);
const P2: Pos = Pos::new(0, 6);

#[test]
fn a_click_right_of_a_wrapped_row_puts_the_caret_there() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 7);
        // the position is the end of the row, which is the start of the next one; the column
        // is the mark that the caret is on the row before
        assert_eq!((r.caret(), r.col()), (P1, Some(7)));
        assert_eq!(r.row_pos(), Pos::new(0, 2));
        assert_eq!(r.cx(|cx| cx.caret_free_x().unwrap()), Some(X::from_raw(7)));
    });
}

#[test]
fn the_end_of_the_text_of_a_wrapped_row_is_a_place_of_its_own() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        // left of the end: not in the gap, the caret goes to the char there
        r.click(P1, 2);
        assert_eq!(r.col(), None);
        // at the end: the end of the row, not the start of the next one
        r.click(P1, 3);
        assert_eq!((r.caret(), r.col()), (P1, Some(3)));
        assert_eq!(r.row_pos(), Pos::new(0, 2));
        // but it is not beyond the text
        assert_eq!(r.cx(|cx| cx.caret_free_x().unwrap()), None);
    });
}

#[test]
fn without_the_mode_a_click_right_of_the_row_marks_only_the_end_of_its_text() {
    let mut r = Rig::new("abcdefgh");
    r.click(P1, 7);
    // the space right of the text is for the mode; the end of the row is a place in every mode
    assert_eq!((r.caret(), r.col()), (P1, Some(3)));
    assert_eq!(r.row_pos(), Pos::new(0, 2));
    // left of the end of the text: the char there
    r.click(P1, 2);
    assert_eq!(r.col(), None);
}

#[test]
fn the_gap_of_the_last_row_of_a_line_is_the_one_it_was() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh\nxy");
        r.click(Pos::new(0, 8), 6);
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 8), Some(6)));
        assert_eq!(r.row_pos(), Pos::new(0, 8));
        // the start of a row is not in a gap
        r.click(Pos::new(1, 0), 6);
        assert_eq!(r.col(), None);
    });
}

#[test]
fn typing_in_the_gap_fills_it_with_spaces_at_the_end_of_the_row() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 7);
        r.run_with("input_char", &[Arg::Char('x')]);
        // 7 - 3 = 4 spaces, then the char, before the rest of the line (`InsertData_CEditView`)
        assert_eq!(r.text(), "abc    xdefgh");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 8), None));
        // one undo step
        r.run("undo");
        assert_eq!(r.text(), "abcdefgh");
    });
}

#[test]
fn typing_at_the_end_of_the_row_adds_no_spaces() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 3);
        r.run_with("input_char", &[Arg::Char('x')]);
        assert_eq!(r.text(), "abcxdefgh");
    });
}

#[test]
fn a_new_line_and_a_tab_in_the_gap_follow_the_spaces() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run_with("input_char", &[Arg::Char('\n')]);
        assert_eq!(r.text(), "abc  \ndefgh");
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run("indent_tab");
        assert_eq!(r.text(), "abc  \tdefgh");
    });
}

#[test]
fn a_crlf_line_keeps_its_line_ending_after_the_filled_gap() {
    with_options(free(), || {
        // "abcdef" is the rows "abc" and "def"; the gap of "def" (the last row) is the old one
        let mut r = Rig::new("abcdef\r\nxy");
        r.click(P2, 5);
        r.run_with("input_char", &[Arg::Char('z')]);
        assert_eq!(r.text(), "abcdef  z\r\nxy");
        // the gap of the wrapped row "abc": the spaces and the char go before "def"
        let mut r = Rig::new("abcdef\r\nxy");
        r.click(P1, 5);
        r.run_with("input_char", &[Arg::Char('z')]);
        assert_eq!(r.text(), "abc  zdef\r\nxy");
    });
}

#[test]
fn delete_in_the_gap_fills_it_and_deletes_the_next_char() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run("delete");
        // the 'd' that follows the spaces is deleted, not a line ending
        assert_eq!(r.text(), "abc  efgh");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 5), None));
        r.run("undo");
        assert_eq!(r.text(), "abcdefgh");
    });
}

#[test]
fn backspace_in_the_gap_only_moves_and_at_the_end_of_the_row_it_deletes() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run("delete_back");
        assert_eq!(
            (r.caret(), r.col(), r.text()),
            (P1, Some(4), "abcdefgh".into())
        );
        r.run("delete_back");
        // at the end of the row: nothing is deleted yet, as `Command_LEFT` stops there
        assert_eq!(
            (r.caret(), r.col(), r.text()),
            (P1, Some(3), "abcdefgh".into())
        );
        // from there the char before the end of the row goes
        r.run("delete_back");
        assert_eq!(
            (r.caret(), r.col(), r.text()),
            (Pos::new(0, 2), None, "abdefgh".into())
        );
    });
}

#[test]
fn left_comes_back_one_column_at_a_time_and_then_onto_the_last_char() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run("cursor_left");
        assert_eq!((r.caret(), r.col()), (P1, Some(4)));
        r.run("cursor_left");
        assert_eq!((r.caret(), r.col()), (P1, Some(3)));
        // `Command_LEFT` at the end of the row goes to the start of the last char
        r.run("cursor_left");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 2), None));
    });
}

#[test]
fn right_from_the_gap_goes_to_the_start_of_the_next_row_at_once() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run("cursor_right");
        assert_eq!((r.caret(), r.col()), (P1, None));
        assert_eq!(r.row_pos(), P1);
        // the end of the row too
        r.click(P1, 3);
        r.run("cursor_right");
        assert_eq!((r.caret(), r.col()), (P1, None));
        // and the next Right is the ordinary one
        r.run("cursor_right");
        assert_eq!(r.caret(), Pos::new(0, 4));
        // the key does not go into the gap from the last char (`to_x >= x_max`)
        let mut r = Rig::new("abcdefgh");
        r.click(Pos::new(0, 2), 2);
        r.run("cursor_right");
        assert_eq!((r.caret(), r.col()), (P1, None));
        r.run("cursor_right");
        assert_eq!(r.caret(), Pos::new(0, 4));
    });
}

#[test]
fn right_with_the_selection_extends_to_the_start_of_the_next_row() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run("cursor_right_sel");
        assert_eq!(r.cursor.selection(), None);
        assert_eq!((r.caret(), r.col()), (P1, None));
    });
}

#[test]
fn vertical_moves_keep_the_column_through_wrapped_rows() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 7);
        // down from the gap of the first row: the gap of the second, the goal column kept
        r.run("cursor_down");
        assert_eq!((r.caret(), r.col()), (P2, Some(7)));
        assert_eq!(r.row_pos(), Pos::new(0, 5));
        // and the last row, the end of the line
        r.run("cursor_down");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 8), Some(7)));
        // back up
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (P2, Some(7)));
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (P1, Some(7)));
        // on the first row it stays
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (P1, Some(7)));
    });
}

#[test]
fn a_vertical_move_from_the_text_onto_a_wrapped_row_goes_into_its_gap() {
    with_options(free(), || {
        // the row "ghi" is the last of the line; up goes to "def", which wraps
        let mut r = Rig::new("abcdefghi");
        r.cursor.move_to(Pos::new(0, 7), false);
        r.cursor.set_goal_col(Some(8));
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (P2, Some(8)));
        // a goal on the text: the char there
        let mut r = Rig::new("abcdefghi");
        r.cursor.move_to(Pos::new(0, 7), false);
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 4), None));
        // a goal at the end of the text of the row: the end of the row, which is its own place
        let mut r = Rig::new("abcdefghi");
        r.cursor.move_to(Pos::new(0, 7), false);
        r.cursor.set_goal_col(Some(3));
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
    });
}

#[test]
fn a_move_that_leaves_the_gap_forgets_it() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 7);
        r.run("go_line_top");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 0), None));
        r.click(P1, 7);
        r.run("go_file_end");
        assert_eq!(r.col(), None);
        r.click(P1, 7);
        r.run("select_all");
        assert_eq!(r.col(), None);
    });
}

#[test]
fn home_and_end_work_on_the_row_of_the_gap() {
    with_options(free(), || {
        // the second row, "def": its gap starts at `(0, 6)`
        let mut r = Rig::new("abcdefgh");
        r.click(P2, 7);
        r.run("go_line_top");
        assert_eq!((r.caret(), r.col()), (P1, None));
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 7);
        r.run("go_line_end");
        // the end of the first row, not of the second: the mark keeps the caret on the first
        assert_eq!((r.caret(), r.col()), (P1, Some(3)));
        assert_eq!(r.row_pos(), Pos::new(0, 2));
        let mut r = Rig::new("abcdefgh");
        r.click(P2, 7);
        r.run("go_line_end");
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
    });
}

#[test]
fn a_selection_that_starts_in_the_gap_keeps_the_row_of_its_anchor() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 3);
        r.run("cursor_down_sel");
        // the caret is at the end of the row "def", the anchor at the end of "abc"
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        assert_eq!(r.cursor.anchor(), Some(P1));
        assert_eq!(r.cursor.anchor_free_col(), Some(3));
        // the mark goes on while the selection grows ...
        r.run("cursor_down_sel");
        assert_eq!(r.cursor.anchor_free_col(), Some(3));
        // ... and is gone with the selection
        r.run("cursor_up");
        assert_eq!(r.cursor.anchor_free_col(), None);
        // a start at the start of the next row has no mark
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| cx.move_caret(P1, false).unwrap());
        r.run("cursor_down_sel");
        assert_eq!(r.cursor.anchor_free_col(), None);
    });
}

#[test]
fn the_column_of_a_caret_in_the_gap_is_where_a_box_starts() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 7);
        // the corner of a box there is on the row of the gap, at its column
        assert_eq!(r.cx(|cx| cx.box_caret_x().unwrap()), X::from_raw(7));
    });
}

#[test]
fn a_box_starts_in_the_gap_of_a_wrapped_row() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 7);
        r.run("begin_box");
        // the corner is in the gap after the row "abc": its position is the start of the next
        // row, and the mark and the column say that it is on the row before
        assert_eq!((r.caret(), r.col()), (P1, Some(7)));
        assert_eq!(r.row_pos(), Pos::new(0, 2));
        assert_eq!(r.cx(|cx| cx.box_caret_x().unwrap()), X::from_raw(7));
        r.run("cursor_down_box");
        assert_eq!(
            r.cursor.box_corners(),
            Some(((P1, 7), (P2, 7))),
            "the corners keep their rows"
        );
        assert_eq!(r.cursor.anchor_free_col(), Some(7));
        assert_eq!(r.col(), Some(7));
    });
}

#[test]
fn the_other_gaps_are_unchanged() {
    with_options(free(), || {
        // a short line below: down keeps the column, right goes through the gap
        let mut r = Rig::new("abcdefgh\nxy");
        r.cursor.move_to(Pos::new(1, 2), false);
        r.run("cursor_right");
        assert_eq!((r.caret(), r.col()), (Pos::new(1, 2), Some(3)));
        r.run("cursor_left");
        assert_eq!((r.caret(), r.col()), (Pos::new(1, 2), None));
    });
}

#[test]
fn row_commands_work_on_the_row_that_the_caret_is_drawn_on() {
    with_options(free(), || {
        // `Command_DELETE_LINE`, `Command_CUT_LINE` and `Command_DUPLICATELINE` take the row of
        // the caret's layout Y, which is the first row for a caret in its gap, not the second
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run("delete_line");
        assert_eq!(r.text(), "defgh");
        // the end of the row is the same place
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 3);
        r.run("delete_line");
        assert_eq!(r.text(), "defgh");
        // without the mark the caret is at the start of the second row, as before
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(P1, false);
        r.run("delete_line");
        assert_eq!(r.text(), "abcgh");

        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run("duplicate_line");
        assert_eq!(r.text(), "abcabcdefgh");

        let mut r = Rig::new("abcdefgh");
        r.click(P1, 5);
        r.run("cut_line");
        assert_eq!(r.text(), "defgh");
        assert_eq!(
            r.clip.data().map(|d| d.text.clone()).as_deref(),
            Some("abc")
        );
    });
}

/// The end of a wrapped row with the free cursor mode off: Sakura's caret is in layout
/// coordinates (`CLayoutPoint`), so `End`, a click right of the row and a vertical move reach the
/// end of the row, not the start of the next one. The expectations come from
/// `Command_GOLINEEND`, `MoveCursorProperly`, `Cursor_UPDOWN`, `Command_LEFT` and `Command_RIGHT`.
mod end_of_a_wrapped_row_without_the_mode {
    use super::*;

    const END: Option<usize> = Some(3);

    /// "abcdefgh" with the caret on the first row, at its end, by `End`.
    fn at_end() -> Rig {
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 1), false);
        r.run("go_line_end");
        r
    }

    #[test]
    fn end_stays_on_the_row() {
        let r = &mut at_end();
        assert_eq!((r.caret(), r.col()), (P1, END));
        assert_eq!(r.row_pos(), Pos::new(0, 2));
        // the caret is at the end of the text: no gap to fill
        assert_eq!(r.cx(|cx| cx.caret_free_x().unwrap()), None);
        assert_eq!(r.cx(|cx| cx.caret_row_x().unwrap()), X::from_raw(3));
        // End again stays
        r.run("go_line_end");
        assert_eq!((r.caret(), r.col()), (P1, END));
        // the second row, and the last row of the line
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 4), false);
        r.run("go_line_end");
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        r.run("go_line_end");
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 7), false);
        r.run("go_line_end");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 8), None));
    }

    #[test]
    fn end_with_the_flags_of_a_macro() {
        // flag 8: the end of the line, which is not the end of a row
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 1), false);
        r.run_with("go_line_end", &[Arg::Int(8)]);
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 8), None));
        // flag 4: the selection grows to the end of the row
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 1), false);
        r.run_with("go_line_end", &[Arg::Int(4)]);
        assert_eq!(r.cursor.selection(), Some((Pos::new(0, 1), P1)));
        assert_eq!((r.caret(), r.col()), (P1, END));
        r.run("go_line_end_sel");
        assert_eq!((r.caret(), r.col()), (P1, END));
    }

    #[test]
    fn a_click_right_of_the_row_stands_on_its_end() {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 40);
        assert_eq!((r.caret(), r.col()), (P1, END));
        assert_eq!(r.row_pos(), Pos::new(0, 2));
        // the last row of the line is no wrapped row: the end of the line, no mark
        r.click(Pos::new(0, 8), 40);
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 8), None));
    }

    #[test]
    fn typing_inserts_at_the_end_of_the_row() {
        let mut r = at_end();
        r.run_with("input_char", &[Arg::Char('x')]);
        assert_eq!(r.text(), "abcxdefgh");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 4), None));
        // a new line: the same place, nothing to fill
        let mut r = at_end();
        r.run_with("input_char", &[Arg::Char('\n')]);
        assert_eq!(r.text(), "abc\ndefgh");
    }

    #[test]
    fn delete_and_backspace_act_around_the_end_of_the_row() {
        let mut r = at_end();
        r.run("delete");
        assert_eq!(r.text(), "abcefgh");
        let mut r = at_end();
        r.run("delete_back");
        assert_eq!(r.text(), "abdefgh");
    }

    #[test]
    fn left_goes_before_the_last_char_and_right_to_the_next_row() {
        let mut r = at_end();
        r.run("cursor_left");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 2), None));
        let mut r = at_end();
        r.run("cursor_right");
        // `x_max <= to_x`: the start of the next row at once, and then the ordinary move
        assert_eq!((r.caret(), r.col()), (P1, None));
        assert_eq!(r.row_pos(), P1);
        r.run("cursor_right");
        assert_eq!(r.caret(), Pos::new(0, 4));
        // the key from before the last char goes to the start of the next row too
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 2), false);
        r.run("cursor_right");
        assert_eq!((r.caret(), r.col()), (P1, None));
    }

    #[test]
    fn vertical_moves_go_from_the_row_and_keep_its_end_as_the_goal() {
        let mut r = at_end();
        r.run("cursor_down");
        // the goal column is 3, the end of the text of "def": the end of that row
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        assert_eq!(r.row_pos(), Pos::new(0, 5));
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (P1, END));
        // up from the start of the second row: column 0 of the first row
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(P1, false);
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 0), None));
    }

    #[test]
    fn a_vertical_move_onto_a_shorter_wrapped_row_stands_on_its_end() {
        let mut r = Rig::new("abcdefghi");
        r.cursor.move_to(Pos::new(0, 7), false);
        r.cursor.set_goal_col(Some(8));
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        assert_eq!(r.row_pos(), Pos::new(0, 5));
        // a goal on the text: the char there
        let mut r = Rig::new("abcdefghi");
        r.cursor.move_to(Pos::new(0, 7), false);
        r.run("cursor_up");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 4), None));
    }

    #[test]
    fn home_and_the_row_commands_act_on_the_row_of_the_caret() {
        let mut r = at_end();
        r.run("go_line_top");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 0), None));
        let mut r = at_end();
        r.run("delete_line");
        assert_eq!(r.text(), "defgh");
        let mut r = at_end();
        r.run("duplicate_line");
        assert_eq!(r.text(), "abcabcdefgh");
        let mut r = at_end();
        r.run("cut_line");
        assert_eq!(r.text(), "defgh");
        // without the mark the caret is on the second row, as it was
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(P1, false);
        r.run("delete_line");
        assert_eq!(r.text(), "abcgh");
    }

    #[test]
    fn a_selection_from_the_end_of_the_row_runs_to_the_next_row() {
        let mut r = at_end();
        r.run("cursor_down_sel");
        assert_eq!(r.cursor.selection(), Some((P1, P2)));
    }

    #[test]
    fn the_mark_is_dropped_by_moves_and_by_the_text_changing_under_it() {
        let mut r = at_end();
        r.run("go_file_end");
        assert_eq!(r.col(), None);
        let mut r = at_end();
        r.run("select_all");
        assert_eq!(r.col(), None);
        // an edit of another view clamps the cursor of this one
        let mut r = at_end();
        r.cursor.clamp(r.model.doc()).unwrap();
        assert_eq!(r.col(), None);
    }

    #[test]
    fn rows_that_do_not_wrap_have_no_mark() {
        let mut r = Rig::new("ab\ncd");
        r.run("go_line_end");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 2), None));
        // a line of exactly one row
        let mut r = Rig::new("abc\ndef");
        r.run("go_line_end");
        assert_eq!((r.caret(), r.col()), (P1, None));
        r.click(P1, 40);
        assert_eq!(r.col(), None);
    }

    #[test]
    fn a_column_right_of_the_text_is_the_modes_alone() {
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(P1, false);
        r.cursor.set_free_col(Some(7));
        assert_eq!(r.row_pos(), P1);
        assert_eq!(r.cx(|cx| cx.caret_free_x().unwrap()), None);
        // the end of the text is taken
        r.cursor.set_free_col(Some(3));
        assert_eq!(r.row_pos(), Pos::new(0, 2));
    }
}

/// A box corner in the gap after a wrapped row (`Command_RIGHT`, `Cursor_UPDOWN`, a click: Sakura's
/// corners are layout points, `CViewSelect`). The measurement in Sakura 2.4.2 with rows of 40
/// columns and the free cursor off: End on row 0, Shift+F6 (begin a box), Down, Left, copy gives
/// the char of column 39 of rows 0 and 1; here the rows have 3 columns, so column 2. No test here
/// reads a path or depends on the line ending of the platform: the document is `Rig`'s, and the
/// copied rows end in the CRLF that Sakura writes.
mod box_corners_in_the_gap {
    use super::*;

    /// "abcdefgh" with the caret at the end of the first row, by `End` (no free cursor mode).
    fn at_end() -> Rig {
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 1), false);
        r.run("go_line_end");
        r
    }

    fn copied(r: &mut Rig) -> String {
        r.run("copy");
        r.clip.data().expect("a copy").text.clone()
    }

    #[test]
    fn begin_box_down_left_copies_the_column_before_the_end() {
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_left");
        // the box is columns 2..3 of rows 0 and 1
        assert_eq!(r.cursor.box_corners(), Some(((P1, 3), (Pos::new(0, 5), 2))));
        assert_eq!(copied(&mut r), "c\r\nf\r\n");
    }

    #[test]
    fn down_alone_makes_a_box_of_no_width() {
        let mut r = at_end();
        r.run("begin_box").run("cursor_down");
        assert_eq!(r.cursor.box_corners(), Some(((P1, 3), (P2, 3))));
        // the caret is in the gap after the second row too, and both rows are copied (empty)
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        assert_eq!(copied(&mut r), "\r\n\r\n");
    }

    #[test]
    fn down_and_right_goes_on_through_the_gap() {
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_right");
        // one column right of the end of the text, still on the row of the gap
        assert_eq!(r.cursor.box_corners(), Some(((P1, 3), (P2, 4))));
        assert_eq!((r.caret(), r.col()), (P2, Some(4)));
        // and left comes back to the end of the text, then onto the last char
        r.run("cursor_left");
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        r.run("cursor_left");
        assert_eq!(r.cursor.box_corners(), Some(((P1, 3), (Pos::new(0, 5), 2))));
        assert_eq!(copied(&mut r), "c\r\nf\r\n");
    }

    #[test]
    fn right_from_the_last_char_enters_the_gap() {
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 2), false);
        r.run("begin_box").run("cursor_right");
        assert_eq!((r.caret(), r.col()), (P1, Some(3)));
        assert_eq!(r.row_pos(), Pos::new(0, 2));
        // the box covers the last char: columns 2..3
        assert_eq!(copied(&mut r), "c\r\n");
    }

    #[test]
    fn shift_alt_down_from_the_end_of_a_wrapped_row_begins_in_the_gap() {
        let mut r = at_end();
        r.run("cursor_down_box");
        assert_eq!(r.cursor.box_corners(), Some(((P1, 3), (P2, 3))));
        assert_eq!(r.cursor.anchor_free_col(), Some(3));
        // the same for the box of the arrow keys that go right
        let mut r = at_end();
        r.run("cursor_right_box");
        assert_eq!((r.caret(), r.col()), (P1, Some(4)));
        assert_eq!(r.cursor.box_anchor_col(), Some(3));
        assert_eq!(r.cursor.anchor_free_col(), Some(3));
    }

    #[test]
    fn a_press_right_of_a_wrapped_row_begins_the_box_in_its_gap() {
        // Alt+click right of the first row, the pointer at column 5; then down and left
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(5))
                .unwrap()
        });
        assert_eq!((r.caret(), r.col()), (P1, Some(5)));
        assert_eq!(r.cursor.box_caret_col(), Some(5));
        r.cx(|cx| cx.mouse_box_to(Pos::new(0, 3), X::from_raw(3)).unwrap());
        // a drag to the end of the text of row 1: the box is columns 3..5
        assert_eq!(r.cursor.box_corners(), Some(((P1, 5), (P2, 3))));
        // a press at the end of the text: the column of the end
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(3))
                .unwrap()
        });
        // (the box of a press is not locked: the box moves go on with it)
        r.run("cursor_down_box").run("cursor_left_box");
        assert_eq!(copied(&mut r), "c\r\nf\r\n");
        // a press inside the last char of the row (the click rounded to the end) is on the left
        // edge of that char, as a drag puts a corner there
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(2))
                .unwrap()
        });
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 2), None));
        assert_eq!(r.cursor.box_caret_col(), Some(2));
        // a press that is not right of a wrapped row is the plain press
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(Pos::new(0, 1), Pos::new(0, 0), X::from_raw(1))
                .unwrap()
        });
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 1), None));
        assert_eq!(r.cursor.box_caret_col(), Some(1));
        // a click right of the last row of a line is not in a wrapped gap either
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(Pos::new(0, 8), Pos::new(0, 6), X::from_raw(9))
                .unwrap()
        });
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 8), None));
    }

    #[test]
    fn a_box_with_a_corner_in_the_gap_is_deleted_by_its_rows() {
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_left");
        r.run("delete");
        // the chars of column 2 of the rows 0 and 1; the caret is at the top left corner
        assert_eq!(r.text(), "abdegh");
        assert_eq!(r.caret(), Pos::new(0, 2));
        assert!(!r.cursor.is_box());
        // a box that starts in the gap and goes right of it covers nothing: nothing is deleted,
        // and the caret stays in the gap
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_right");
        r.run("delete");
        assert_eq!(r.text(), "abcdefgh");
        assert_eq!((r.caret(), r.col()), (P1, Some(3)));
    }

    #[test]
    fn cut_takes_the_text_of_the_rows_of_the_corners() {
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_left");
        r.run("cut");
        assert_eq!(r.clip.data().expect("a cut").text, "c\r\nf\r\n");
        assert_eq!(r.text(), "abdegh");
    }

    #[test]
    fn typing_in_a_box_whose_corners_are_in_the_gap_goes_into_each_row() {
        // the box is columns 3..4 of the rows 0 and 1, both in the gaps: the text goes at the
        // end of the text of each row
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_right");
        r.run_with("input_char", &[Arg::Char('X')]);
        assert_eq!(r.text(), "abcXdefXgh");
        // typing in a box in the gap that covers a column of text
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_left");
        r.run_with("input_char", &[Arg::Char('X')]);
        assert_eq!(r.text(), "abXcdeXfgh");
    }

    #[test]
    fn a_box_paste_with_a_corner_in_the_gap_starts_on_the_row_of_the_gap() {
        // the box is columns 2..3 of rows 0 and 1; pasting replaces it and starts at its top left
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_left");
        r.cx(|cx| cx.paste_box("X\nY").unwrap());
        // (a row that wraps gets a line ending where the pasted text ends, as `paste_box` does
        // for every wrapped row)
        assert_eq!(r.text(), "abX\ndeY\ngh");
        // an empty box in the gap: the text goes in at the end of the row, at the column of the
        // gap (the end of the row gets the line ending, so the text starts the next row)
        let mut r = at_end();
        r.run("begin_box");
        r.cx(|cx| cx.paste_box("X").unwrap());
        assert_eq!(r.text(), "abc\nXdefgh");
        // the top left corner of a box in the gap: the box is the gap of the rows 0 and 1
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_right");
        r.cx(|cx| cx.paste_box("X").unwrap());
        assert_eq!(r.text(), "abc\nXdefgh");
    }

    #[test]
    fn the_column_of_a_gap_corner_is_the_box_column_not_the_mark() {
        // the mark only says "on the upper row": a different number in it changes nothing
        let mut r = at_end();
        r.run("begin_box");
        r.cursor.set_free_col(Some(99));
        r.run("cursor_down").run("cursor_left");
        r.cursor.set_anchor_free_col(Some(1));
        assert_eq!(copied(&mut r), "c\r\nf\r\n");
    }

    #[test]
    fn a_box_that_begins_again_keeps_the_column_of_its_corner() {
        // in the empty space after the last row of a line
        let mut r = Rig::new("ab");
        r.cursor.move_to(Pos::new(0, 2), false);
        r.run("begin_box").run("cursor_right");
        assert_eq!(r.cursor.box_caret_col(), Some(3));
        r.run("begin_box");
        assert_eq!(r.cursor.box_corners(), None);
        assert_eq!(r.cursor.box_caret_col(), Some(3));
        // in the gap after a wrapped row
        let mut r = at_end();
        r.run("begin_box").run("cursor_right").run("begin_box");
        assert_eq!(
            (r.caret(), r.cursor.box_caret_col(), r.col()),
            (P1, Some(4), Some(4))
        );
    }

    #[test]
    fn a_column_right_of_the_text_is_brought_back_when_the_mode_is_off() {
        // a box that ends empty in the gap, or a delete that leaves the caret there: without the
        // free cursor mode the caret stays on the row at the end of its text
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(3))
                .unwrap();
            assert!(cx.mouse_end_empty_box().unwrap());
        });
        assert_eq!((r.caret(), r.col()), (P1, Some(3)));
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(6))
                .unwrap();
            assert!(cx.mouse_end_empty_box().unwrap());
        });
        assert_eq!((r.caret(), r.col()), (P1, Some(3)));
        assert_eq!(r.row_pos(), Pos::new(0, 2));
        // with the mode on the column stays
        with_options(free(), || {
            let mut r = Rig::new("abcdefgh");
            r.cx(|cx| {
                cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(6))
                    .unwrap();
                assert!(cx.mouse_end_empty_box().unwrap());
            });
            assert_eq!((r.caret(), r.col()), (P1, Some(6)));
        });
    }

    #[test]
    fn conversions_of_a_box_in_the_gap_leave_the_caret_in_the_gap() {
        // a box of no width over the gaps of the rows 0 and 1 changes no text, and the caret
        // goes to its top left corner, which is the gap after row 0
        for id in [
            "to_upper",
            "to_lower",
            "tab_to_space",
            "space_to_tab",
            "ltrim",
            "rtrim",
        ] {
            let mut r = at_end();
            r.run("begin_box").run("cursor_down");
            let before = r.text();
            r.run(id);
            assert_eq!(r.text(), before, "{id}");
            assert_eq!((r.caret(), r.col()), (P1, Some(3)), "{id}");
            assert_eq!(r.row_pos(), Pos::new(0, 2), "{id}");
            assert!(!r.cursor.is_box(), "{id}");
        }
        // a box that has text still ends at its top left corner on the text
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_left");
        r.run("to_upper");
        assert_eq!(r.text(), "abCdeFgh");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 2), None));
    }

    /// A 100-character line wrapped (here after 3 columns): End, begin a box, Down, and then what
    /// ends the box where the caret is. The caret stays on the second row (in its gap).
    fn boxed_in_the_gap() -> Rig {
        let mut r = at_end();
        r.run("begin_box").run("cursor_down");
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        r
    }

    #[test]
    fn escape_ends_a_box_in_the_gap_on_its_row() {
        let mut r = boxed_in_the_gap();
        r.run("cancel_mode");
        assert!(!r.cursor.is_box());
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        assert_eq!(r.row_pos(), Pos::new(0, 5));
        // a box that covers nothing (Down not pressed) ends the same way
        let mut r = at_end();
        r.run("begin_box").run("cancel_mode");
        assert!(!r.cursor.is_box());
        assert_eq!((r.caret(), r.col()), (P1, Some(3)));
        // a column right of the text, without the free cursor mode, is brought back to its end
        let mut r = at_end();
        r.run("begin_box").run("cursor_down").run("cursor_right");
        r.run("cancel_mode");
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        // with the mode the column stays
        with_options(free(), || {
            let mut r = at_end();
            r.run("begin_box").run("cursor_down").run("cursor_right");
            r.run("cancel_mode");
            assert_eq!((r.caret(), r.col()), (P2, Some(4)));
        });
    }

    #[test]
    fn a_copy_that_disables_the_selection_ends_a_box_in_the_gap_on_its_row() {
        let mut o = EditOptions::DEFAULT;
        o.copy_and_disable_selection = true;
        with_options(o, || {
            let mut r = at_end();
            r.run("begin_box").run("cursor_down").run("cursor_right");
            r.run("cursor_left").run("cursor_left");
            r.run("copy");
            assert!(!r.cursor.is_box());
            // on the last char of the second row (the lefts went through the gap)
            assert_eq!((r.caret(), r.col()), (Pos::new(0, 5), None));
        });
        // and with a column right of the text, the mode off
        let mut o = EditOptions::DEFAULT;
        o.copy_and_disable_selection = true;
        with_options(o, || {
            let mut r = at_end();
            r.run("begin_box").run("cursor_right").run("cursor_down");
            r.run("copy");
            assert!(!r.cursor.is_box());
            assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        });
    }

    #[test]
    fn dropping_the_selection_for_a_row_command_keeps_the_row_of_the_gap() {
        // `duplicate_line` drops the selection first and works on the row of the caret
        let mut r = boxed_in_the_gap();
        r.run("duplicate_line");
        assert_eq!(r.text(), "abcdefdefgh");
        assert!(!r.cursor.is_box());
    }

    #[test]
    fn a_mark_is_stored_only_where_a_wrapped_row_ends() {
        // after an edit the caret may not be at the end of a wrapped row any more: no mark then
        let mut r = Rig::new("abcdefgh\nxy");
        for (at, expected) in [
            (Pos::new(0, 1), None), // inside the text
            (Pos::new(1, 2), None), // the end of the last row of a line
            (Pos::new(0, 4), None), // inside the text of the second row
            (P1, Some(5)),          // the end of a wrapped row
        ] {
            r.cursor.move_to(at, false);
            r.cx(|cx| cx.keep_gap_mark(Some(X::from_raw(5))).unwrap());
            // (the mode is off: the end of the text is the only column of the gap)
            let expected = expected.map(|_| 3);
            assert_eq!(r.col(), expected, "{at:?}");
            assert_eq!(
                r.cx(|cx| cx.caret_row_x().unwrap()),
                X::from_raw(if expected.is_some() {
                    3
                } else {
                    at.col as u32 % 3
                }),
                "{at:?}"
            );
        }
        // a conversion that changes the row ends without a stale mark: the box is over the rows
        // 0 and 1 and `rtrim` takes the spaces out, so the row no longer wraps at that place
        let mut r = Rig::new("ab \ncd");
        r.cursor.move_to(Pos::new(0, 3), false);
        r.cx(|cx| cx.keep_gap_mark(Some(X::from_raw(3))).unwrap());
        assert_eq!(r.col(), None);
    }

    /// A box that the mouse made and that is not locked: the anchor is on the text of row 0 (column
    /// 1) and the caret is in the gap after row 1 (column 3).
    fn dragged_into_the_gap() -> Rig {
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box(Pos::new(0, 1)).unwrap();
            cx.mouse_box_to(P1, X::from_raw(3)).unwrap();
        });
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        assert_eq!(r.cursor.box_anchor_col(), Some(1));
        assert!(r.cursor.has_selection());
        r
    }

    #[test]
    fn right_and_left_end_a_box_at_the_corners_of_its_rectangle() {
        // `Command_RIGHT` and `Command_LEFT` without Shift: `Command_CANCEL_MODE( 2 )` and `( 1 )`
        // go to the bottom right and the top left corner of the rectangle, and that is the move
        let mut r = dragged_into_the_gap();
        r.run("cursor_right");
        assert!(!r.cursor.is_box() && !r.cursor.has_selection());
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        assert_eq!(r.row_pos(), Pos::new(0, 5));
        let mut r = dragged_into_the_gap();
        r.run("cursor_left");
        assert!(!r.cursor.is_box());
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 1), None));
        // the other way: the anchor is in the gap of row 0, the caret is on the text of row 1
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(3))
                .unwrap();
            cx.mouse_box_to(P1, X::from_raw(1)).unwrap();
        });
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 4), None));
        r.run("cursor_right");
        // the rectangle is the columns 1..3 of the rows 0 and 1: its bottom right is the gap of row 1
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(3))
                .unwrap();
            cx.mouse_box_to(P1, X::from_raw(1)).unwrap();
        });
        r.run("cursor_left");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 1), None));
        // a box with nothing in it is only a mode: the move goes on from the gap (to the next row)
        let mut r = Rig::new("abcdefgh");
        r.cx(|cx| {
            cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(3))
                .unwrap()
        });
        r.run("cursor_right");
        assert!(!r.cursor.is_box());
        assert_eq!((r.caret(), r.col()), (P1, None));
    }

    #[test]
    fn a_collapsed_box_keeps_a_column_right_of_the_text_only_with_the_mode() {
        // the rectangle reaches column 5, right of the text of the rows (3 and 2 columns wide)
        let make = || {
            let mut r = Rig::new("abcdefgh");
            r.cx(|cx| {
                cx.mouse_begin_box_at(P1, Pos::new(0, 0), X::from_raw(5))
                    .unwrap();
                cx.mouse_box_to(P1, X::from_raw(1)).unwrap();
            });
            r
        };
        let mut r = make();
        r.run("cursor_right");
        assert_eq!((r.caret(), r.col()), (P2, Some(3)));
        with_options(free(), || {
            let mut r = make();
            r.run("cursor_right");
            assert_eq!((r.caret(), r.col()), (P2, Some(5)));
        });
        // on the last row of the line: the end of the text, or the free caret
        let make = || {
            let mut r = Rig::new("abcdefgh");
            r.cx(|cx| {
                cx.mouse_begin_box(Pos::new(0, 1)).unwrap();
                cx.mouse_box_to(Pos::new(0, 6), X::from_raw(5)).unwrap();
            });
            r
        };
        let mut r = make();
        r.run("cursor_right");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 8), None));
        with_options(free(), || {
            let mut r = make();
            r.run("cursor_right");
            assert_eq!((r.caret(), r.col()), (Pos::new(0, 8), Some(5)));
        });
    }

    #[test]
    fn the_other_plain_moves_start_from_the_row_of_a_box_caret_in_the_gap() {
        // Home, End, Up, Down, Ctrl+Home and Ctrl+End end the box and move from the caret's row
        // (row 1, in its gap), as `Command_GOLINETOP` and the others do from the layout point
        let cases: [(&str, Pos, Option<usize>); 6] = [
            ("go_line_top", P1, None),
            ("go_line_end", P2, Some(3)),
            ("cursor_up", P1, Some(3)),
            ("cursor_down", Pos::new(0, 8), None),
            ("go_file_top", Pos::new(0, 0), None),
            ("go_file_end", Pos::new(0, 8), None),
        ];
        for (id, caret, col) in cases {
            let mut r = dragged_into_the_gap();
            r.run(id);
            assert!(!r.cursor.is_box() && !r.cursor.has_selection(), "{id}");
            assert_eq!((r.caret(), r.col()), (caret, col), "{id}");
        }
    }

    #[test]
    fn typing_in_a_box_in_the_gap_pads_it_and_overwrites_the_next_char_as_a_free_caret_does() {
        // `Command_WCHAR` with nothing selected, overwrite mode: `DelCharForOverwrite` finds the
        // caret past the end of the row (`nIdxTo >= GetLengthWithoutEOL`) and runs `Command_DELETE`,
        // which at the end of a wrapped row fills the gap with spaces and deletes the next char
        // (`cmd/CViewCommander_Edit.cpp`); the typed char goes in after that. A box with nothing
        // in it is not a selection, so it is the same as a free caret in the gap.
        let mut r = at_end();
        r.run("begin_box").run("cursor_right").run("begin_box");
        assert_eq!((r.caret(), r.col()), (P1, Some(4)));
        assert!(r.cursor.is_box() && !r.cursor.has_selection());
        r.overwrite = true;
        r.run_with("input_char", &[Arg::Char('X')]);
        assert_eq!(r.text(), "abc Xefgh");
        // a box that is one column wide in the gap is a selection: the char goes in front of it
        let mut r = at_end();
        r.run("begin_box").run("cursor_right");
        r.overwrite = true;
        r.run_with("input_char", &[Arg::Char('X')]);
        assert_eq!(r.text(), "abcXdefgh");
        // the insert mode pads and inserts
        let mut r = at_end();
        r.run("begin_box").run("cursor_right").run("begin_box");
        r.run_with("input_char", &[Arg::Char('X')]);
        assert_eq!(r.text(), "abc Xdefgh");
        // at the end of the text of the row, without a gap to fill, overwrite replaces the next char
        let mut r = at_end();
        r.run("begin_box");
        r.overwrite = true;
        r.run_with("input_char", &[Arg::Char('X')]);
        assert_eq!(r.text(), "abcXefgh");
    }

    #[test]
    fn delete_and_backspace_with_an_empty_box_in_the_gap_act_as_for_a_free_caret() {
        let mut r = at_end();
        r.run("begin_box").run("cursor_right").run("begin_box");
        r.run("delete");
        // `Command_DELETE`: fills the gap, then deletes the next char
        assert_eq!(r.text(), "abc efgh");
        let mut r = at_end();
        r.run("begin_box");
        r.run("delete_back");
        // `Command_DELETE_BACK` deletes the char before the caret: the last char of the row
        assert_eq!(r.text(), "abdefgh");
    }

    #[test]
    fn the_rest_of_the_moves_work_from_the_gap() {
        // up from the gap of the second row, onto the gap of the first
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 4), false);
        r.run("go_line_end").run("begin_box").run("cursor_up");
        assert_eq!(r.cursor.box_corners(), Some(((P2, 3), (P1, 3))));
        // End in a box goes into the gap of the row
        let mut r = Rig::new("abcdefgh");
        r.cursor.move_to(Pos::new(0, 1), false);
        r.run("begin_box").run("go_line_end_box");
        assert_eq!((r.caret(), r.col()), (P1, Some(3)));
        assert_eq!(r.cursor.box_corners(), Some(((Pos::new(0, 1), 1), (P1, 3))));
        // a box move off the gap onto a text row drops the mark
        r.run("cursor_down");
        assert_eq!(r.col(), Some(3));
        let mut r = at_end();
        r.run("begin_box").run("cursor_left").run("cursor_left");
        assert_eq!((r.caret(), r.col()), (Pos::new(0, 1), None));
        assert_eq!(r.cursor.anchor_free_col(), Some(3));
    }
}
