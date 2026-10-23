//! Dropping the selection of a view on the same view: moving it, or with Ctrl copying it.
//!
//! Sakura: `CEditView::Drop` (the part for a drag that started in this view), `DragSelection`,
//! `TranslateDropEffect` and `IsCurrentPositionSelected` in `sakura_core/view/CEditView_Mouse.cpp`
//! and `CEditView.cpp`. Sakura puts the text on an OLE data object, so the same code takes a drop
//! from another window or program; this file is the case of the same view, where Sakura does not
//! go through the clipboard and the whole drop is one undo step (`SetUndoBuffer` at the end of
//! `Drop`).

use matsu_core::buffer::Pos;

use super::CommandContext;
use crate::command::Result;

/// Where `p` is after the text from `a` to `b` was deleted (`p` is not inside it).
fn after_delete(p: Pos, a: Pos, b: Pos) -> Pos {
    if p <= a {
        p
    } else if p.line == b.line {
        Pos::new(a.line, a.col + p.col.saturating_sub(b.col))
    } else {
        Pos::new(p.line - (b.line - a.line), p.col)
    }
}

impl CommandContext<'_> {
    /// Whether dropping at `at` does nothing because the pointer is on the dragged selection
    /// (`IsCurrentPositionSelected` for a drag source). The edges count as inside for a move, so
    /// that a small slip of the pointer does not move the text; with Ctrl (`copy`) only the
    /// inside of the selection does.
    pub fn mouse_drop_is_inside(&self, at: Pos, copy: bool) -> Result<bool> {
        if self.cursor.is_box() {
            let Some(rect) = self.box_rect()? else {
                return Ok(false);
            };
            let row = self.layout.row_start(self.doc(), at)?;
            let col = self.layout.display_col(self.doc(), at)?;
            // `++rcSel.bottom`, then `++rcSel.right` (or `++rcSel.left` with Ctrl) for `PtInRect`
            let across = if copy {
                col > rect.left && col < rect.right
            } else {
                col >= rect.left && col <= rect.right
            };
            return Ok(rect.rows.contains(&row) && across);
        }
        let Some((from, to)) = self.selection() else {
            return Ok(false);
        };
        Ok(if copy {
            from < at && at < to
        } else {
            from <= at && at <= to
        })
    }

    /// The selection was dragged and dropped at `at` of this view: moves its text there, or
    /// copies it when `copy` is set. The text that was dropped is selected afterwards, with the
    /// caret at its end; a dropped box is left as the caret at its top left corner. One undo step.
    /// Returns `false` and changes nothing when there is no selection or the pointer is on it
    /// ([`mouse_drop_is_inside`](Self::mouse_drop_is_inside)).
    pub fn mouse_drop_selection(&mut self, at: Pos, copy: bool) -> Result<bool> {
        self.refuse_in_view_mode()?;
        if !self.cursor.has_selection() || self.mouse_drop_is_inside(at, copy)? {
            return Ok(false);
        }
        let at = self.doc().clamp_pos(at)?;
        self.begin_block();
        let done = if self.cursor.is_box() {
            self.drop_box(at, copy)
        } else {
            self.drop_text(at, copy)
        };
        self.end_block();
        done.map(|()| true)
    }

    fn drop_text(&mut self, at: Pos, copy: bool) -> Result<()> {
        let Some((from, to)) = self.selection() else {
            return Ok(());
        };
        let text = self.doc().text_between(from, to)?;
        self.cursor.clear_selection();
        // Sakura deletes the old text first when the drop is before it and afterwards when it is
        // after it; the result is the same, so the old text always goes first here.
        let at = if copy {
            at
        } else {
            self.replace(from, to, "")?;
            after_delete(at, from, to)
        };
        let edit = self.replace(at, at, &text)?;
        // `SetSelectArea(ptSelectFrom, caret)`: the dropped text is selected.
        self.cursor.select(at, edit.end);
        Ok(())
    }

    fn drop_box(&mut self, at: Pos, copy: bool) -> Result<()> {
        let Some(rect) = self.box_rect()? else {
            return Ok(());
        };
        let Some(text) = self.box_text()? else {
            return Ok(());
        };
        let mut at = at;
        if !copy {
            // Sakura pastes before or after deleting the old box, whichever keeps the drop place
            // where it is in layout coordinates (`bMoveToPrev` in `Drop`). Here the place is a
            // position, so the old box always goes first, and the place is moved by what was
            // deleted before it in its line (the box is deleted row by row, each inside a line).
            let mut deleted = Vec::with_capacity(rect.rows.len());
            for &row in &rect.rows {
                deleted.push(self.box_row_range(row, rect.left, rect.right)?);
            }
            self.delete_box()?;
            for (from, to) in deleted {
                if from.line == at.line && to.col <= at.col {
                    at.col -= to.col - from.col;
                }
            }
        }
        self.cursor.clear_selection();
        self.cursor.move_to(at, false);
        self.paste_box(&text)
    }
}

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Pos;
    use matsu_core::cursor::Cursor;

    use crate::layout::{Layout, PlainLayout};
    use crate::testing::Editor;

    fn drop_at(e: &mut Editor, at: Pos, copy: bool) -> bool {
        e.with_context(|c| c.mouse_drop_selection(at, copy))
            .unwrap()
    }

    fn text(e: &Editor) -> String {
        let doc = e.model().doc();
        doc.text_between(Pos::new(0, 0), doc.end_pos().unwrap())
            .unwrap()
    }

    #[test]
    fn moving_text_forward_and_back() {
        // forward: the text lands after the old place, which closes up
        let mut e = Editor::new("[ab|cdef");
        assert!(drop_at(&mut e, Pos::new(0, 4), false));
        e.assert_marked("cd[ab|ef");
        // back
        let mut e = Editor::new("abcd[ef|");
        assert!(drop_at(&mut e, Pos::new(0, 1), false));
        e.assert_marked("a[ef|bcd");
    }

    #[test]
    fn copying_text() {
        let mut e = Editor::new("[ab|cdef");
        assert!(drop_at(&mut e, Pos::new(0, 4), true));
        e.assert_marked("abcd[ab|ef");
        // with Ctrl the edges of the selection are a place to drop; inside is not
        let mut e = Editor::new("[abc|d");
        assert!(drop_at(&mut e, Pos::new(0, 3), true));
        e.assert_marked("abc[abc|d");
        let mut e = Editor::new("[abc|d");
        assert!(!drop_at(&mut e, Pos::new(0, 1), true));
        e.assert_marked("[abc|d");
    }

    #[test]
    fn dropping_on_the_selection_does_nothing() {
        for col in 0..=3 {
            let mut e = Editor::new("[abc|d");
            assert!(!drop_at(&mut e, Pos::new(0, col), false));
            e.assert_marked("[abc|d");
        }
    }

    #[test]
    fn a_drop_over_lines_with_crlf_and_japanese_text() {
        let mut e = Editor::new("あ[い\r\nう|え\r\nお");
        assert!(drop_at(&mut e, Pos::new(2, "お".len()), false));
        e.assert_marked("あえ\r\nお[い\r\nう|");
        let mut e = Editor::new("あい\r\n[うえ|\r\nお");
        assert!(drop_at(&mut e, Pos::new(0, 0), true));
        e.assert_marked("[うえ|あい\r\nうえ\r\nお");
    }

    #[test]
    fn a_move_is_one_undo_step() {
        let mut e = Editor::new("[ab|cdef");
        drop_at(&mut e, Pos::new(0, 4), false);
        e.run("undo");
        assert_eq!(text(&e), "abcdef");
        e.run("redo");
        assert_eq!(text(&e), "cdabef");
    }

    #[test]
    fn view_mode_refuses() {
        let mut e = Editor::new("[ab|cdef");
        e.set_view_mode(true);
        let r = e.with_context(|c| c.mouse_drop_selection(Pos::new(0, 4), false));
        assert!(r.is_err());
        assert_eq!(text(&e), "abcdef");
    }

    /// An editor with a box selected: corners as `(row, display column)`.
    fn boxed(text: &str, anchor: (usize, usize), caret: (usize, usize)) -> Editor {
        let doc = matsu_core::buffer::Document::from_utf8(text.as_bytes().to_vec()).unwrap();
        let layout = PlainLayout::default();
        let pos = |(line, col): (usize, usize)| {
            layout
                .pos_at_col(&doc, Pos::new(line, 0), col)
                .expect("a position")
        };
        let mut cursor = Cursor::default();
        cursor.move_to(pos(anchor), false);
        cursor.begin_box(anchor.1, false);
        cursor.move_box_to(pos(caret), caret.1);
        Editor::from_parts(text, cursor)
    }

    #[test]
    fn moving_a_box_down_and_right() {
        let mut e = boxed("abcd\nefgh\nijkl\n", (0, 0), (1, 2));
        // a box of "ab" and "ef", dropped on the third row, column 2
        assert!(drop_at(&mut e, Pos::new(2, 2), false));
        assert_eq!(text(&e), "cd\ngh\nijabkl\n  ef\n");
    }

    #[test]
    fn moving_a_box_to_the_right_on_its_own_rows() {
        // the drop place is in a row the box is deleted from: it moves left by the box's width
        let mut e = boxed("abcdefgh\nijklmnop\n", (0, 0), (1, 2));
        assert!(drop_at(&mut e, Pos::new(0, 5), false));
        assert_eq!(text(&e), "cdeabfgh\nklmijnop\n");
    }

    #[test]
    fn copying_a_box() {
        let mut e = boxed("abcd\nefgh\nijkl\n", (0, 1), (1, 3));
        assert!(drop_at(&mut e, Pos::new(2, 0), true));
        assert_eq!(text(&e), "abcd\nefgh\nbcijkl\nfg\n");
    }

    #[test]
    fn moving_a_box_up_when_the_drop_is_before_it() {
        let mut e = boxed("abcd\nefgh\nijkl\n", (1, 2), (2, 4));
        assert!(drop_at(&mut e, Pos::new(0, 0), false));
        // the old box is gone from rows 1 and 2, and its text is pasted from the top left
        assert_eq!(text(&e), "ghabcd\nklef\nij\n");
    }

    #[test]
    fn dropping_a_box_on_itself_does_nothing() {
        let mut e = boxed("abcd\nefgh\n", (0, 1), (1, 3));
        assert!(!drop_at(&mut e, Pos::new(1, 2), false));
        e.assert_box((0, 1), (1, 3));
    }
}
