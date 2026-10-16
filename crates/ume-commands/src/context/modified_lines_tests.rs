//! Tests of the jumps to the modified lines (`modified_lines.rs`). The rules of which lines are
//! modified are tested in `ume_core::modified` and `ume_core::model`.

use ume_core::buffer::{Document, Pos};
use ume_core::cursor::Cursor;
use ume_core::model::DocumentModel;
use ume_core::search::Direction::{self, Backward, Forward};

use super::{CommandContext, ModifiedLineJump};
use crate::layout::PlainLayout;

struct Fixture {
    model: DocumentModel,
    cursor: Cursor,
    layout: PlainLayout,
}

impl Fixture {
    fn new(text: &str) -> Self {
        let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
        Self {
            model: DocumentModel::new(doc),
            cursor: Cursor::default(),
            layout: PlainLayout::default(),
        }
    }

    /// The lines `l0` to `l9` (no line break after the last one) with the given lines edited.
    fn ten(modified: &[usize]) -> Self {
        let text = (0..10)
            .map(|i| format!("l{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut f = Self::new(&text);
        for &line in modified {
            f.edit((line, 1), (line, 1), "x");
        }
        f
    }

    fn edit(&mut self, from: (usize, usize), to: (usize, usize), text: &str) {
        self.model.begin_block(&self.cursor);
        self.model
            .replace(Pos::new(from.0, from.1), Pos::new(to.0, to.1), text)
            .unwrap();
        self.model.end_block(&self.cursor);
    }

    fn save(&mut self) {
        self.model.mark_saved(&self.cursor);
    }

    fn at(&mut self, line: usize, col: usize) -> &mut Self {
        self.cursor.move_to(Pos::new(line, col), false);
        self
    }

    fn jump(&mut self, direction: Direction, extend: bool, wrap: bool) -> ModifiedLineJump {
        let mut cx = CommandContext::new(&mut self.model, &mut self.cursor, &mut self.layout, None);
        cx.jump_to_modified_line(direction, extend, wrap).unwrap()
    }

    fn caret(&self) -> (usize, usize) {
        (self.cursor.caret().line, self.cursor.caret().col)
    }
}

const FOUND: ModifiedLineJump = ModifiedLineJump::Found { wrapped: false };
const WRAPPED: ModifiedLineJump = ModifiedLineJump::Found { wrapped: true };
const NOT_FOUND: ModifiedLineJump = ModifiedLineJump::NotFound;

#[test]
fn next_stops_at_both_edges_of_a_block() {
    let mut f = Fixture::ten(&[3, 4, 7]);
    for want in [3, 5, 7, 8] {
        assert_eq!(f.jump(Forward, false, false), FOUND);
        assert_eq!(f.caret(), (want, 0));
    }
    assert_eq!(f.jump(Forward, false, false), NOT_FOUND);
    assert_eq!(f.caret(), (8, 0));
}

#[test]
fn prev_stops_at_the_same_places() {
    let mut f = Fixture::ten(&[3, 4, 7]);
    f.at(9, 0);
    for want in [8, 7, 5, 3] {
        assert_eq!(f.jump(Backward, false, false), FOUND);
        assert_eq!(f.caret(), (want, 0));
    }
    assert_eq!(f.jump(Backward, false, false), NOT_FOUND);
    assert_eq!(f.caret(), (3, 0));
}

#[test]
fn next_starts_from_the_state_of_the_caret_line() {
    let mut f = Fixture::ten(&[3, 4, 7]);
    // inside a block: the next stop is its end
    f.at(4, 1);
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (5, 0));
}

#[test]
fn wrapping_goes_on_from_the_other_end() {
    let mut f = Fixture::ten(&[3, 4, 7]);
    f.at(8, 0);
    assert_eq!(f.jump(Forward, false, true), WRAPPED);
    assert_eq!(f.caret(), (3, 0));
    let mut f = Fixture::ten(&[3, 4, 7]);
    f.at(3, 0);
    assert_eq!(f.jump(Backward, false, true), WRAPPED);
    assert_eq!(f.caret(), (8, 0));
}

#[test]
fn a_modified_first_line_is_a_stop_of_its_own() {
    let mut f = Fixture::ten(&[0, 1]);
    f.at(5, 0);
    assert_eq!(f.jump(Forward, false, false), NOT_FOUND);
    assert_eq!(f.jump(Forward, false, true), WRAPPED);
    assert_eq!(f.caret(), (0, 0));
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (2, 0));
    // and backwards
    f.at(5, 0);
    assert_eq!(f.jump(Backward, false, false), FOUND);
    assert_eq!(f.caret(), (2, 0));
    assert_eq!(f.jump(Backward, false, false), FOUND);
    assert_eq!(f.caret(), (0, 0));
    assert_eq!(f.jump(Backward, false, false), NOT_FOUND);
}

#[test]
fn a_block_that_reaches_the_end_stops_at_the_end_of_the_text() {
    // the last line has no line break: the end of its text
    let mut f = Fixture::ten(&[8, 9]);
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (8, 0));
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (9, 3));
    assert_eq!(f.jump(Forward, false, false), NOT_FOUND);
    // backwards from the end of the text
    assert_eq!(f.jump(Backward, false, false), FOUND);
    assert_eq!(f.caret(), (8, 0));
}

#[test]
fn with_a_final_line_break_the_end_of_the_text_is_the_empty_row() {
    let mut f = Fixture::new("a\nb\nc\n");
    f.edit((1, 1), (1, 1), "x");
    f.edit((2, 1), (2, 1), "y");
    f.at(0, 0);
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (1, 0));
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (3, 0));
    assert_eq!(f.jump(Forward, false, false), NOT_FOUND);
    // from the empty row the move back goes to the start of the block
    assert_eq!(f.jump(Backward, false, false), FOUND);
    assert_eq!(f.caret(), (1, 0));
}

#[test]
fn extending_makes_a_selection() {
    let mut f = Fixture::ten(&[3]);
    f.at(0, 1);
    assert_eq!(f.jump(Forward, true, false), FOUND);
    assert_eq!(f.cursor.selection(), Some((Pos::new(0, 1), Pos::new(3, 0))));
    // the selection lock extends the plain move too
    let mut f = Fixture::ten(&[3]);
    f.at(0, 1);
    f.cursor.set_selecting_lock(true);
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.cursor.selection(), Some((Pos::new(0, 1), Pos::new(3, 0))));
    // without either, a selection is dropped
    let mut f = Fixture::ten(&[3]);
    f.cursor.select(Pos::new(0, 0), Pos::new(1, 0));
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert!(!f.cursor.has_selection());
}

#[test]
fn nothing_is_modified_after_a_save_or_when_undone() {
    let mut f = Fixture::ten(&[3, 7]);
    f.save();
    f.at(0, 0);
    assert_eq!(f.jump(Forward, false, true), NOT_FOUND);
    assert_eq!(f.jump(Backward, false, true), NOT_FOUND);
    // an edit after the save, then undone
    f.edit((5, 1), (5, 1), "z");
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (5, 0));
    f.model.undo(&mut f.cursor);
    f.at(0, 0);
    assert_eq!(f.jump(Forward, false, true), NOT_FOUND);
}

#[test]
fn an_empty_document_does_nothing() {
    let mut f = Fixture::new("");
    assert_eq!(f.jump(Forward, false, true), ModifiedLineJump::Empty);
    assert_eq!(f.jump(Backward, false, true), ModifiedLineJump::Empty);
}

#[test]
fn japanese_text_and_crlf() {
    let mut f = Fixture::new("あ\r\nい\r\nう\r\nえ");
    f.edit((1, 3), (1, 3), "ん");
    f.at(0, 0);
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (1, 0));
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (2, 0));
}

#[test]
fn lines_that_come_and_go_move_the_stops() {
    let mut f = Fixture::ten(&[5]);
    f.edit((0, 0), (0, 0), "new\n");
    // the new line 0 and the old line 5, now 6
    f.at(1, 0);
    assert_eq!(f.jump(Forward, false, false), FOUND);
    assert_eq!(f.caret(), (6, 0));
    assert_eq!(f.jump(Backward, false, false), FOUND);
    assert_eq!(f.caret(), (1, 0));
}
