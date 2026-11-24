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
}

impl Rig {
    fn new(text: &str) -> Self {
        let mut model = DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap());
        model.set_new_line(Eol::Lf);
        Self {
            model,
            cursor: Cursor::default(),
            layout: Wrap(3),
        }
    }

    fn cx<R>(&mut self, f: impl FnOnce(&mut CommandContext<'_>) -> R) -> R {
        let mut cx = CommandContext::new(&mut self.model, &mut self.cursor, &mut self.layout, None);
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
fn without_the_mode_a_click_leaves_no_mark() {
    let mut r = Rig::new("abcdefgh");
    r.click(P1, 7);
    assert_eq!((r.caret(), r.col()), (P1, None));
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
        // the end of the first row, not of the second
        assert_eq!((r.caret(), r.col()), (P1, None));
        let mut r = Rig::new("abcdefgh");
        r.click(P2, 7);
        r.run("go_line_end");
        assert_eq!(r.caret(), P2);
    });
}

#[test]
fn a_box_does_not_start_in_the_gap_of_a_wrapped_row() {
    with_options(free(), || {
        let mut r = Rig::new("abcdefgh");
        r.click(P1, 7);
        // the corner is where the position is: the start of the next row, column 0
        assert_eq!(r.cx(|cx| cx.box_caret_x().unwrap()), X::from_raw(0));
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
