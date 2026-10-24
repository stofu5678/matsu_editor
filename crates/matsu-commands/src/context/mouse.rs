//! What the mouse does to the selection, apart from the pixels: selecting by words, by rows or
//! lines and by boxes while the button is down, and moving or copying the selection by dragging.
//! `matsu-ui` turns a point of the window into a position, a row and a display column and calls
//! these; they are tested here without a window.
//!
//! Sakura: `CEditView::OnLBUTTONDOWN`, `OnMOUSEMOVE` and `Drop` in
//! `sakura_core/view/CEditView_Mouse.cpp`. Sakura keeps the part of the selection that the press
//! made as `m_sSelectBgn` (a range), and every move of the mouse recomputes the selection from
//! that range and the pointer; the functions here take that range as `bgn`.

use matsu_core::buffer::Pos;

use super::CommandContext;
use crate::command::Result;

/// The move of the mouse while a selection is being dragged: where it began and whether it goes
/// by words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DragSelection {
    /// The range the press selected (`m_sSelectBgn`): a word, a URL, a line, or the empty range at
    /// the click.
    pub bgn: (Pos, Pos),
    /// The ends of the selection snap to word boundaries (`m_bBeginWordSelect`).
    pub words: bool,
}

impl CommandContext<'_> {
    // ---- a drag of a normal selection ----

    /// Sets the selection for the pointer at `at` while a drag that began with `drag` goes on
    /// (`OnMOUSEMOVE`, `ChangeSelectAreaByCurrentCursor`): from the start of `drag.bgn` to `at` when
    /// the pointer is after it, from its end to `at` when before it, and `drag.bgn` itself when
    /// the pointer is in it. By words, the end that the pointer is at moves to the edge of the
    /// word under it when the word reaches into the selection.
    pub fn mouse_drag_to(&mut self, drag: DragSelection, at: Pos) -> Result<()> {
        let (start, end) = drag.bgn;
        let (anchor, mut caret) = if at < start {
            (end, at)
        } else if at > end {
            (start, at)
        } else {
            (start, end)
        };
        if drag.words
            && let Some((word_from, word_to)) = self.word_range_at(at)?
        {
            // The part of the selection the word shares with it: Sakura looks at the selection
            // the caret would make, and moves the end that the caret is at.
            let (from, to) = (anchor.min(caret), anchor.max(caret));
            if from < word_to && word_from < to {
                caret = if caret == to { word_to } else { word_from };
            }
        }
        self.select_range(anchor, caret)
    }

    // ---- a drag of rows or lines ----

    /// The row (or, with `logical`, the logical line) that contains `pos`, as the position where
    /// it starts and the position where the next one starts (the end of the document for the
    /// last one). The empty row after a final line ending is empty.
    pub fn mouse_unit_at(&self, pos: Pos, logical: bool) -> Result<(Pos, Pos)> {
        let doc_end = self.doc().end_pos()?;
        if logical {
            let start = Pos::new(pos.line, 0);
            if pos.line >= self.doc().line_count() {
                return Ok((doc_end, doc_end));
            }
            let end = Pos::new(pos.line + 1, 0);
            return Ok((start, self.doc().clamp_pos(end)?));
        }
        let start = self.layout.row_start(self.doc(), pos)?;
        let end = self
            .layout
            .next_row_start(self.doc(), start)?
            .unwrap_or(doc_end);
        Ok((start, end))
    }

    /// A press in the line number column (a row) or the third click of a triple click (a logical
    /// line): selects the unit at `pos`, the anchor at its start and the caret at the start of
    /// the next. Returns the range for [`mouse_drag_units_to`](Self::mouse_drag_units_to), or
    /// `None` and only the caret moved when there is nothing to select (the empty row at the
    /// end).
    pub fn mouse_begin_units(&mut self, pos: Pos, logical: bool) -> Result<Option<(Pos, Pos)>> {
        let (start, end) = self.mouse_unit_at(pos, logical)?;
        if start == end {
            self.move_caret(start, false)?;
            return Ok(None);
        }
        self.select_range(start, end)?;
        Ok(Some((start, end)))
    }

    /// A drag that began with [`mouse_begin_units`](Self::mouse_begin_units) as `bgn`, now with the
    /// pointer at `pos`: the units from `bgn` to the unit under the pointer, whole. Above `bgn` the
    /// caret goes to the start of the pointer's unit, from the end of `bgn`; elsewhere to the
    /// start of the unit after it, from the start of `bgn` (`OnMOUSEMOVE`, the branch of the
    /// line selection).
    pub fn mouse_drag_units_to(&mut self, bgn: (Pos, Pos), pos: Pos, logical: bool) -> Result<()> {
        let (start, end) = self.mouse_unit_at(pos, logical)?;
        if start < bgn.0 {
            self.select_range(bgn.1, start)
        } else {
            self.select_range(bgn.0, end)
        }
    }

    // ---- a drag of a box ----

    /// Alt and a press: drops the selection, puts the caret at `pos` and begins a box there that
    /// covers nothing yet (`SelectBeginBox`, `BeginSelectArea`). The plain moves do not extend it:
    /// the mouse is the one that moves the corner.
    pub fn mouse_begin_box(&mut self, pos: Pos) -> Result<()> {
        self.move_caret(pos, false)?;
        let col = self.caret_display_col()?;
        self.cursor.begin_box(col, false);
        Ok(())
    }

    /// The pointer of a box drag is on the row that contains `row_pos`, at display column `col`:
    /// moves the corner there. Past the end of the text the corner is in the empty space
    /// (`MoveCursorToClientPoint` in `OnMOUSEMOVE`).
    pub fn mouse_box_to(&mut self, row_pos: Pos, col: usize) -> Result<()> {
        self.box_move_to_col(row_pos, col.min(super::MAX_BOX_COL))
    }

    /// The mouse button was released: a box that covers nothing is not a selection (`m_sSelect.
    /// IsOne()` in `OnLBUTTONUP`), so the box mode ends. Returns whether it did.
    pub fn mouse_end_empty_box(&mut self) -> Result<bool> {
        if self.cursor.is_box() && !self.cursor.has_selection() {
            let caret = self.caret();
            self.move_caret(caret, false)?;
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Pos;

    use super::DragSelection;
    use crate::testing::Editor;

    fn drag(e: &mut Editor, bgn: (Pos, Pos), words: bool, at: Pos) {
        e.with_context(|c| c.mouse_drag_to(DragSelection { bgn, words }, at))
            .unwrap();
    }

    #[test]
    fn a_drag_selects_from_the_start_of_the_press() {
        let mut e = Editor::new("ab|cdef");
        let p = |col| Pos::new(0, col);
        drag(&mut e, (p(2), p(2)), false, p(5));
        e.assert_marked("ab[cde|f");
        drag(&mut e, (p(2), p(2)), false, p(0));
        e.assert_marked("|ab[cdef");
        drag(&mut e, (p(2), p(2)), false, p(2));
        e.assert_marked("ab|cdef");
    }

    #[test]
    fn a_drag_after_a_double_click_goes_by_words() {
        // The word "bar" was selected by the double click.
        let mut e = Editor::new("foo [bar| baz qux");
        let bgn = (Pos::new(0, 4), Pos::new(0, 7));
        drag(&mut e, bgn, true, Pos::new(0, 10)); // inside "baz"
        e.assert_marked("foo [bar baz| qux");
        drag(&mut e, bgn, true, Pos::new(0, 1)); // inside "foo"
        e.assert_marked("|foo bar[ baz qux");
        drag(&mut e, bgn, true, Pos::new(0, 5)); // inside the word itself
        e.assert_marked("foo [bar| baz qux");
        // on the edge of a word the selection takes no more
        drag(&mut e, bgn, true, Pos::new(0, 8));
        e.assert_marked("foo [bar |baz qux");
    }

    #[test]
    fn a_drag_by_words_over_japanese_text() {
        let mut e = Editor::new("[漢字| ひらがな カタカナ");
        let bgn = (Pos::new(0, 0), Pos::new(0, 6));
        drag(&mut e, bgn, true, Pos::new(0, "漢字 ひら".len()));
        e.assert_marked("[漢字 ひらがな| カタカナ");
    }

    #[test]
    fn rows_and_lines_are_selected_whole() {
        let mut e = Editor::new("a|b\r\ncd\r\nef");
        let bgn = e
            .with_context(|c| c.mouse_begin_units(Pos::new(1, 1), true))
            .unwrap();
        assert_eq!(bgn, Some((Pos::new(1, 0), Pos::new(2, 0))));
        e.assert_marked("ab\r\n[cd\r\n|ef");
        let bgn = bgn.unwrap();
        // down to the last line: it is selected to the end of the document
        e.with_context(|c| c.mouse_drag_units_to(bgn, Pos::new(2, 1), true))
            .unwrap();
        e.assert_marked("ab\r\n[cd\r\nef|");
        // up: the caret goes to the start of the line, from the end of the first line (the start
        // of the line after `bgn`)
        e.with_context(|c| c.mouse_drag_units_to(bgn, Pos::new(0, 1), true))
            .unwrap();
        e.assert_marked("|ab\r\ncd\r\n[ef");
    }

    #[test]
    fn the_empty_row_at_the_end_has_nothing_to_select() {
        let mut e = Editor::new("ab\n|");
        let r = e
            .with_context(|c| c.mouse_begin_units(Pos::new(1, 0), false))
            .unwrap();
        assert_eq!(r, None);
        e.assert_marked("ab\n|");
    }

    #[test]
    fn a_box_follows_the_pointer_and_ends_when_empty() {
        let mut e = Editor::new("a|bcd\nefgh\nij");
        e.with_context(|c| c.mouse_begin_box(Pos::new(0, 1)))
            .unwrap();
        e.with_context(|c| c.mouse_box_to(Pos::new(1, 0), 3))
            .unwrap();
        e.assert_box((0, 1), (1, 3));
        // back on the corner: nothing is covered, and the release ends the box
        e.with_context(|c| c.mouse_box_to(Pos::new(0, 0), 1))
            .unwrap();
        assert!(e.with_context(|c| c.mouse_end_empty_box()).unwrap());
        e.assert_marked("a|bcd\nefgh\nij");
        // the corner can be in the empty space of a short row
        e.with_context(|c| c.mouse_begin_box(Pos::new(0, 1)))
            .unwrap();
        e.with_context(|c| c.mouse_box_to(Pos::new(2, 0), 5))
            .unwrap();
        e.assert_box((0, 1), (2, 5));
        assert!(!e.with_context(|c| c.mouse_end_empty_box()).unwrap());
    }
}
