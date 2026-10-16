//! The caret and the selection of one view.
//!
//! A [`Cursor`] belongs to a view, not to the document: two views of one document have their own
//! carets (ADR-0013 section 3). Positions are [`Pos`] (line and byte column in the buffer).
//!
//! # The selection model
//!
//! The selection is the text between the *anchor* (the end that stays where the selection was
//! started) and the *caret* (the end that moves). Either can come first, so a selection has a
//! direction. Sakura keeps the selection as a range plus a separate origin (`m_sSelect` and
//! `m_sSelectBgn` in `sakura_core/view/CViewSelect.h`); the anchor and caret pair holds the same
//! information and lets every command ask "where is the caret" in one place. An anchor equal to
//! the caret is no selection.
//!
//! # Goal column
//!
//! Moving up and down keeps the column the caret had before the first vertical move, so a short
//! line in the middle does not make the caret forget it (Sakura: `m_nCaretPosX_Prev` in
//! `sakura_core/view/CCaret.h`). The goal is a *display* column, which only the layout can
//! compute, so a [`Cursor`] just stores it. Every change of the caret that is not a vertical move
//! clears it; the next vertical move then takes the column of the caret.

use crate::buffer::{Document, Pos, Result};
use crate::jump_history::JumpHistory;

/// The kind of a selection. Only [`Normal`](Self::Normal) is implemented by the commands.
///
/// A rectangular selection (Sakura's box selection) is between two display columns and two lines,
/// so it will be stored as the same anchor and caret and the layout will turn them into
/// columns. The variant exists so that the data can hold it later (roadmap: 選択範囲).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    #[default]
    Normal,
    Rectangular,
}

/// The caret and the selection of a view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    caret: Pos,
    anchor: Option<Pos>,
    mode: SelectionMode,
    goal_col: Option<usize>,
    jump_history: JumpHistory,
}

impl Cursor {
    /// A cursor with the caret at `caret` and no selection.
    pub fn new(caret: Pos) -> Self {
        Self {
            caret,
            ..Self::default()
        }
    }

    /// The caret: where typed text goes and where the selection moves.
    pub fn caret(&self) -> Pos {
        self.caret
    }

    /// The fixed end of the selection, if there is a selection.
    pub fn anchor(&self) -> Option<Pos> {
        self.anchor
    }

    pub fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// The display column that vertical moves aim for, if a vertical move has set one.
    pub fn goal_col(&self) -> Option<usize> {
        self.goal_col
    }

    /// Whether any text is selected (an anchor equal to the caret selects nothing).
    pub fn has_selection(&self) -> bool {
        self.anchor.is_some()
    }

    /// The selected range, smaller position first. `None` if nothing is selected.
    pub fn selection(&self) -> Option<(Pos, Pos)> {
        let a = self.anchor?;
        Some((a.min(self.caret), a.max(self.caret)))
    }

    /// Moves the caret. With `extend` the selection grows or shrinks from its anchor (a new
    /// selection starts at the old caret); without it the selection is dropped. Clears the goal
    /// column; call [`set_goal_col`](Self::set_goal_col) after a vertical move.
    pub fn move_to(&mut self, pos: Pos, extend: bool) {
        if extend {
            let anchor = self.anchor.unwrap_or(self.caret);
            self.anchor = (anchor != pos).then_some(anchor);
        } else {
            self.anchor = None;
            self.mode = SelectionMode::Normal;
        }
        self.caret = pos;
        self.goal_col = None;
    }

    /// Selects from `anchor` to `caret` (the caret may come first).
    pub fn select(&mut self, anchor: Pos, caret: Pos) {
        self.caret = caret;
        self.anchor = (anchor != caret).then_some(anchor);
        self.goal_col = None;
    }

    /// Drops the selection and keeps the caret.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
        self.mode = SelectionMode::Normal;
    }

    /// The places this view jumped from (Sakura: `CEditView::m_cHistory`). It belongs to the
    /// view like the caret does, and a new `Cursor` (a file opened) starts with an empty one.
    pub fn jump_history(&self) -> &JumpHistory {
        &self.jump_history
    }

    pub fn jump_history_mut(&mut self) -> &mut JumpHistory {
        &mut self.jump_history
    }

    /// Becomes `other` (the caret and the selection of an older or newer version, for undo and
    /// redo) but keeps this view's jump history: the history is not part of a version.
    pub fn restore(&mut self, other: Cursor) {
        let history = std::mem::take(&mut self.jump_history);
        *self = other;
        self.jump_history = history;
    }

    pub fn set_goal_col(&mut self, col: Option<usize>) {
        self.goal_col = col;
    }

    /// Clamps the caret and the anchor into `doc`. Call it after the text changed under a cursor
    /// that was not the one that edited (another view of the same document).
    pub fn clamp(&mut self, doc: &Document) -> Result<()> {
        self.caret = doc.clamp_pos(self.caret)?;
        if let Some(a) = self.anchor {
            let a = doc.clamp_pos(a)?;
            self.anchor = (a != self.caret).then_some(a);
        }
        Ok(())
    }
}

/// The position one char before `pos`: the previous char of the line, or the end of the text of
/// the previous line. `None` at the start of the document.
pub fn char_left(doc: &Document, pos: Pos) -> Result<Option<Pos>> {
    let pos = doc.clamp_pos(pos)?;
    if pos.col > 0 {
        let line = doc.line(pos.line)?.map(|l| l.text).unwrap_or_default();
        let len = line
            .get(..pos.col)
            .and_then(|s| s.chars().next_back())
            .map_or(pos.col, char::len_utf8);
        return Ok(Some(Pos::new(pos.line, pos.col - len)));
    }
    if pos.line == 0 {
        return Ok(None);
    }
    let prev = pos.line - 1;
    Ok(Some(Pos::new(prev, doc.line_len(prev)?.unwrap_or(0))))
}

/// The position one char after `pos`: the next char of the line, or the start of the next line
/// when `pos` is at the end of the text of a line that has a line ending. `None` at the end of
/// the document.
pub fn char_right(doc: &Document, pos: Pos) -> Result<Option<Pos>> {
    let pos = doc.clamp_pos(pos)?;
    if pos >= doc.end_pos()? {
        return Ok(None);
    }
    let line = doc.line(pos.line)?.map(|l| l.text).unwrap_or_default();
    Ok(Some(
        match line.get(pos.col..).and_then(|s| s.chars().next()) {
            Some(c) => Pos::new(pos.line, pos.col + c.len_utf8()),
            None => Pos::new(pos.line + 1, 0),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(s: &str) -> Document {
        Document::from_utf8(s.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn extending_and_shrinking_a_selection() {
        let mut c = Cursor::new(Pos::new(0, 2));
        c.move_to(Pos::new(0, 4), true);
        assert_eq!(c.selection(), Some((Pos::new(0, 2), Pos::new(0, 4))));
        // past the anchor: the selection flips direction
        c.move_to(Pos::new(0, 0), true);
        assert_eq!(c.selection(), Some((Pos::new(0, 0), Pos::new(0, 2))));
        assert_eq!(c.anchor(), Some(Pos::new(0, 2)));
        // back to the anchor: no selection
        c.move_to(Pos::new(0, 2), true);
        assert!(!c.has_selection());
        assert_eq!(c, Cursor::new(Pos::new(0, 2)));
    }

    #[test]
    fn moving_without_extend_drops_the_selection_and_the_goal() {
        let mut c = Cursor::new(Pos::new(0, 0));
        c.move_to(Pos::new(0, 3), true);
        c.set_goal_col(Some(7));
        c.move_to(Pos::new(1, 0), false);
        assert_eq!(c, Cursor::new(Pos::new(1, 0)));
    }

    #[test]
    fn char_moves_cross_lines_and_chars() {
        let d = doc("a日\r\n\nb");
        assert_eq!(
            char_right(&d, Pos::new(0, 0)).unwrap(),
            Some(Pos::new(0, 1))
        );
        assert_eq!(
            char_right(&d, Pos::new(0, 1)).unwrap(),
            Some(Pos::new(0, 4))
        );
        // the end of a line with a line ending goes to the next line (CRLF is one step)
        assert_eq!(
            char_right(&d, Pos::new(0, 4)).unwrap(),
            Some(Pos::new(1, 0))
        );
        assert_eq!(
            char_right(&d, Pos::new(1, 0)).unwrap(),
            Some(Pos::new(2, 0))
        );
        assert_eq!(char_right(&d, Pos::new(2, 1)).unwrap(), None);
        assert_eq!(char_left(&d, Pos::new(0, 4)).unwrap(), Some(Pos::new(0, 1)));
        assert_eq!(char_left(&d, Pos::new(1, 0)).unwrap(), Some(Pos::new(0, 4)));
        assert_eq!(char_left(&d, Pos::new(2, 0)).unwrap(), Some(Pos::new(1, 0)));
        assert_eq!(char_left(&d, Pos::new(0, 0)).unwrap(), None);
    }

    #[test]
    fn the_empty_line_after_a_final_line_ending_is_reachable() {
        let d = doc("ab\n");
        assert_eq!(
            char_right(&d, Pos::new(0, 2)).unwrap(),
            Some(Pos::new(1, 0))
        );
        assert_eq!(char_right(&d, Pos::new(1, 0)).unwrap(), None);
        assert_eq!(char_left(&d, Pos::new(1, 0)).unwrap(), Some(Pos::new(0, 2)));
    }

    #[test]
    fn clamp_follows_the_document() {
        let d = doc("ab");
        let mut c = Cursor::new(Pos::new(5, 9));
        c.select(Pos::new(0, 1), Pos::new(5, 9));
        c.clamp(&d).unwrap();
        assert_eq!(c.selection(), Some((Pos::new(0, 1), Pos::new(0, 2))));
    }
}
