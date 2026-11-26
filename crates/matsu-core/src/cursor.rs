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
//!
//! # Free cursor
//!
//! With Sakura's free cursor mode the caret may stand to the right of the end of a line
//! (`m_bIsFreeCursorMode`). The position stays the end of the line and the cursor keeps the
//! display column the caret stands at ([`Cursor::free_col`]), like the corners of a box do.
//!
//! # The unit of the columns
//!
//! The "display columns" here (the goal column, the free column, the corners of a box) are the
//! layout's horizontal positions in its own unit, `matsu_layout::X` (ADR-0034), kept as plain
//! numbers: `matsu-layout` depends on this crate, so the core cannot name the type. The cursor
//! only stores and compares them and never turns them into columns or does arithmetic with
//! them, so it does not need to know the unit. `matsu-commands` is where they are read and
//! written as `X`; one half-width column is as many of them as the layout's `per_column` (1
//! today).

use crate::buffer::{Document, Pos, Result};
use crate::jump_history::JumpHistory;

/// The kind of a selection.
///
/// A rectangular selection (Sakura's box selection) is between two display columns and two rows.
/// It is stored as the same anchor and caret, plus the display column of each corner, because a
/// corner can be to the right of the end of its row where there is no text (Sakura lets the caret
/// go there while a box is selected). See [`Cursor::begin_box`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    #[default]
    Normal,
    Rectangular,
}

/// The display columns of the two corners of a box selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BoxCols {
    /// The column of the anchor. Equal to `caret` while there is no anchor.
    anchor: usize,
    caret: usize,
    /// No corner has moved since [`Cursor::begin_box`]: Sakura's `BeginSelectArea` made a range
    /// that `IsTextSelected` counts, until a move of the caret clears it
    /// (`ChangeSelectAreaByCurrentCursorTEST`, `view/CViewSelect.cpp`, when the caret is on the
    /// start of the range).
    begun: bool,
}

/// The caret and the selection of a view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    caret: Pos,
    anchor: Option<Pos>,
    mode: SelectionMode,
    goal_col: Option<usize>,
    jump_history: JumpHistory,
    /// The columns of the corners of a box selection: `Some` exactly when `mode` is
    /// [`Rectangular`](SelectionMode::Rectangular).
    box_cols: Option<BoxCols>,
    /// Plain moves extend the selection (Sakura: `m_bSelectingLock` of `CViewSelect`, switched on
    /// by `F_BEGIN_SEL` and `F_BEGIN_BOX`, and off by anything that drops the selection).
    selecting_lock: bool,
    /// The display column of the caret when it stands to the right of the end of its line (the
    /// free cursor mode). `None` when the caret is on the text. The commands that set it check
    /// the rest (the caret is at the end of a row, the column is past the text).
    free_col: Option<usize>,
    /// The mark of the anchor, as `free_col` is the mark of the caret: the display column that the
    /// anchor stood at when it was a caret at the end of a wrapped row (Sakura copies the layout
    /// point of the caret into `m_sSelectBgn` in `BeginSelectArea`, so the anchor stays on the
    /// row). `None` exactly when there is no anchor or it carries no mark. It is checked where it
    /// is read, by the same rule as the caret's mark.
    anchor_free_col: Option<usize>,
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

    /// Whether a box selection is going on, even one that covers nothing yet (Sakura:
    /// `IsBoxSelecting`).
    pub fn is_box(&self) -> bool {
        self.mode == SelectionMode::Rectangular
    }

    /// Whether a box was begun and no corner has moved since, so that it covers nothing but
    /// Sakura still counts it as selected text (`IsTextSelected`, see `BoxCols::begun`). A box
    /// that a move made empty again is not. Typing goes into the box in the first case and
    /// follows the typing mode in the second.
    pub fn box_just_begun(&self) -> bool {
        self.anchor.is_none() && self.box_cols.is_some_and(|c| c.begun)
    }

    /// Whether a typed char is box input (`Command_INDENT`): a box that covers something, or one
    /// just begun (`IsTextSelected` and `IsBoxSelecting` in `Command_WCHAR`). The one place that
    /// says it; a box that a move made empty again is not.
    pub fn box_input_applies(&self) -> bool {
        self.is_box() && (self.has_selection() || self.box_just_begun())
    }

    /// Records that a box move was asked for, whether or not a corner moved. Every cursor command
    /// ends with `ChangeSelectAreaByCurrentCursor`, and its `...TEST` clears an empty range when
    /// the caret is on its start, also when the caret did not move (`view/CViewSelect.cpp`, the
    /// `ptCaretPos==m_sSelectBgn.GetFrom()` branch), so the box is no longer selected text.
    pub fn box_moved(&mut self) {
        if let Some(cols) = self.box_cols.as_mut() {
            cols.begun = false;
        }
    }

    /// Whether plain moves extend the selection (Sakura: `m_bSelectingLock`).
    pub fn selecting_lock(&self) -> bool {
        self.selecting_lock
    }

    pub fn set_selecting_lock(&mut self, lock: bool) {
        self.selecting_lock = lock;
    }

    /// The display column of the caret in a box selection. It can be to the right of the end of
    /// the row, where the caret position is the end of the row.
    pub fn box_caret_col(&self) -> Option<usize> {
        self.box_cols.map(|c| c.caret)
    }

    /// The display column of the anchor in a box selection (the column of the caret while there
    /// is no anchor).
    pub fn box_anchor_col(&self) -> Option<usize> {
        self.box_cols.map(|c| c.anchor)
    }

    /// Starts a box selection at the caret, which is at display column `col`. A selection that
    /// was there is dropped. Nothing is selected until the caret moves (Sakura:
    /// `Command_BEGIN_BOXSELECT`). With `lock` the plain moves extend the box.
    pub fn begin_box(&mut self, col: usize, lock: bool) {
        self.anchor = None;
        self.anchor_free_col = None;
        self.free_col = None;
        self.mode = SelectionMode::Rectangular;
        self.box_cols = Some(BoxCols {
            anchor: col,
            caret: col,
            begun: true,
        });
        self.selecting_lock = lock;
    }

    /// Moves the caret of a box selection to `pos` at display column `col` (`pos` is the end of
    /// the row when `col` is past the text). The anchor stays; a caret on the anchor corner leaves
    /// nothing selected. Call [`begin_box`](Self::begin_box) first: without a box, one starts at
    /// the old caret and its anchor column is `col`. Clears the goal column. The corner is on its
    /// row; for one in the gap at the end of a wrapped row use
    /// [`move_box_to_gap`](Self::move_box_to_gap).
    pub fn move_box_to(&mut self, pos: Pos, col: usize) {
        self.move_box_corner(pos, col, false);
    }

    /// As [`move_box_to`](Self::move_box_to), for a corner in the gap at the end of a wrapped row
    /// (`pos` is the start of the next row, which is where a position at the end of the row is).
    /// The corner carries the mark of the caret, [`free_col`](Self::free_col), set to `col`: the
    /// same mark that says it for a free caret. The caller has checked the rest (`pos` starts a
    /// row that continues a line, `col` is not left of the end of the text before it); whoever
    /// reads the mark checks it again.
    pub fn move_box_to_gap(&mut self, pos: Pos, col: usize) {
        self.move_box_corner(pos, col, true);
    }

    fn move_box_corner(&mut self, pos: Pos, col: usize, in_gap: bool) {
        if !self.is_box() {
            self.begin_box(col, false);
        }
        let cols = self.box_cols.unwrap_or(BoxCols {
            anchor: col,
            caret: col,
            begun: false,
        });
        // The anchor stays where it is with its mark; a box that has no anchor yet starts at the
        // caret, and the caret's mark goes with it (`BeginSelectArea` copies the caret's point).
        let (anchor_pos, anchor_mark) = match self.anchor {
            Some(anchor) => (anchor, self.anchor_free_col),
            None => (self.caret, self.free_col),
        };
        let same = anchor_pos == pos && cols.anchor == col && anchor_mark.is_some() == in_gap;
        self.caret = pos;
        self.anchor = (!same).then_some(anchor_pos);
        self.anchor_free_col = anchor_mark.filter(|_| self.anchor.is_some());
        self.box_cols = Some(BoxCols {
            anchor: cols.anchor,
            caret: col,
            begun: false,
        });
        self.goal_col = None;
        self.free_col = in_gap.then_some(col);
    }

    /// The corners of a box selection as `(position, display column)`: the anchor and the caret.
    /// `None` if no box covers anything.
    pub fn box_corners(&self) -> Option<((Pos, usize), (Pos, usize))> {
        let cols = self.box_cols?;
        let anchor = self.anchor?;
        Some(((anchor, cols.anchor), (self.caret, cols.caret)))
    }

    /// The display column of the caret when it stands to the right of the end of its line (the
    /// free cursor mode of Sakura), else `None`. The position is the end of the line then; on the
    /// start of a row that continues a wrapped line it says that the caret is on the row before.
    /// Moves and edits of the cursor drop it; so does the view when its layout changes or the mode
    /// is off, because the cursor does not know the layout.
    pub fn free_col(&self) -> Option<usize> {
        self.free_col
    }

    /// Sets or clears [`free_col`](Self::free_col). Every other change of the caret clears it.
    pub fn set_free_col(&mut self, col: Option<usize>) {
        self.free_col = col;
    }

    /// The mark of the anchor: the display column it stood at when the selection started from a
    /// caret that had a [`free_col`](Self::free_col). Like the caret's mark it says, on the start
    /// of a row that continues a wrapped line, that the anchor is at the end of the row before.
    pub fn anchor_free_col(&self) -> Option<usize> {
        self.anchor_free_col
    }

    /// Sets or clears the mark of the anchor. Without an anchor it stays `None`.
    pub fn set_anchor_free_col(&mut self, col: Option<usize>) {
        self.anchor_free_col = col.filter(|_| self.anchor.is_some());
    }

    /// Drops the marks of the caret and of the anchor: for a change that the cursor cannot see
    /// (the layout of the view changed, the text changed under it).
    pub fn drop_free_cols(&mut self) {
        self.free_col = None;
        self.anchor_free_col = None;
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
    ///
    /// A box selection that is extended this way becomes a normal selection from its anchor; the
    /// box moves use [`move_box_to`](Self::move_box_to).
    pub fn move_to(&mut self, pos: Pos, extend: bool) {
        if extend {
            // A selection that starts here starts at the caret with its mark (`BeginSelectArea`).
            let (anchor, mark) = match self.anchor {
                Some(anchor) => (anchor, self.anchor_free_col),
                None => (self.caret, self.free_col),
            };
            self.anchor = (anchor != pos).then_some(anchor);
            self.anchor_free_col = mark.filter(|_| self.anchor.is_some());
            self.drop_box();
        } else {
            self.anchor = None;
            self.anchor_free_col = None;
            self.drop_box();
            self.selecting_lock = false;
        }
        self.caret = pos;
        self.goal_col = None;
        self.free_col = None;
    }

    /// Moves the caret and the anchor to where `f` maps them, after the text changed under them
    /// (an edit made in another view). The goal column stays; a selection that no longer covers
    /// anything is dropped. A box selection whose corners moved becomes a normal selection
    /// between the moved positions: its display columns are not the columns of the new text.
    pub fn map_positions(&mut self, f: impl Fn(Pos) -> Pos) {
        let (caret, anchor) = (self.caret, self.anchor);
        self.caret = f(caret);
        self.anchor = anchor.map(&f);
        if self.is_box() && (self.caret != caret || self.anchor != anchor) {
            self.drop_box();
        }
        if !self.is_box() && self.anchor == Some(self.caret) {
            self.anchor = None;
        }
        self.free_col = None;
        self.anchor_free_col = None;
    }

    /// Selects from `anchor` to `caret` (the caret may come first). The anchor has no mark; see
    /// [`set_anchor_free_col`](Self::set_anchor_free_col).
    pub fn select(&mut self, anchor: Pos, caret: Pos) {
        self.drop_box();
        self.selecting_lock = false;
        self.caret = caret;
        self.anchor = (anchor != caret).then_some(anchor);
        self.anchor_free_col = None;
        self.goal_col = None;
        self.free_col = None;
    }

    /// Drops the selection and keeps the caret.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
        self.anchor_free_col = None;
        self.drop_box();
        self.selecting_lock = false;
    }

    /// Leaves box mode (the selection, if any, stays as a normal one).
    fn drop_box(&mut self) {
        self.mode = SelectionMode::Normal;
        self.box_cols = None;
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

    /// As [`clamp`](Self::clamp), but only when a position is outside `doc`. A cursor that is
    /// inside is left as it is: `clamp` drops an anchor that equals the caret, which a box
    /// selection in one row may have with different display columns.
    pub fn clamp_if_outside(&mut self, doc: &Document) -> Result<()> {
        let outside = doc.clamp_pos(self.caret)? != self.caret
            || match self.anchor {
                Some(a) => doc.clamp_pos(a)? != a,
                None => false,
            };
        if outside { self.clamp(doc) } else { Ok(()) }
    }

    /// Clamps the caret and the anchor into `doc`. Call it after the text changed under a cursor
    /// that was not the one that edited (another view of the same document).
    pub fn clamp(&mut self, doc: &Document) -> Result<()> {
        self.free_col = None;
        self.anchor_free_col = None;
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
        let len = if doc.is_long_line(pos.line) {
            // Only the end of the text before the caret is read (ADR-0020, 9.10).
            let from = pos.col.saturating_sub(8);
            doc.line_slice(pos.line, from, pos.col)?
                .and_then(|s| s.text.chars().next_back().map(char::len_utf8))
        } else {
            let line = doc.line(pos.line)?.map(|l| l.text).unwrap_or_default();
            line.get(..pos.col)
                .and_then(|s| s.chars().next_back())
                .map(char::len_utf8)
        }
        .unwrap_or(pos.col);
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
    let next = if doc.is_long_line(pos.line) {
        doc.line_slice(pos.line, pos.col, pos.col + 8)?
            .and_then(|s| s.text.chars().next())
    } else {
        let line = doc.line(pos.line)?.map(|l| l.text).unwrap_or_default();
        line.get(pos.col..).and_then(|s| s.chars().next())
    };
    Ok(Some(match next {
        Some(c) => Pos::new(pos.line, pos.col + c.len_utf8()),
        None => Pos::new(pos.line + 1, 0),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(s: &str) -> Document {
        Document::from_utf8(s.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn a_selection_that_starts_at_a_marked_caret_gives_the_anchor_the_mark() {
        // `BeginSelectArea` copies the caret's layout point into `m_sSelectBgn`
        let mut c = Cursor::new(Pos::new(0, 3));
        c.set_free_col(Some(3));
        c.move_to(Pos::new(0, 6), true);
        assert_eq!(c.anchor(), Some(Pos::new(0, 3)));
        assert_eq!(c.anchor_free_col(), Some(3));
        // the caret's own mark is gone, as every move clears it
        assert_eq!(c.free_col(), None);
        // growing the selection keeps the anchor and its mark, also when the caret has a mark
        c.set_free_col(Some(3));
        c.move_to(Pos::new(0, 9), true);
        assert_eq!(c.anchor_free_col(), Some(3));
        // the caret passes the anchor: the anchor stays
        c.move_to(Pos::new(0, 0), true);
        assert_eq!(c.anchor(), Some(Pos::new(0, 3)));
        assert_eq!(c.anchor_free_col(), Some(3));
    }

    #[test]
    fn the_mark_of_the_anchor_goes_with_the_anchor() {
        let marked = || {
            let mut c = Cursor::new(Pos::new(0, 3));
            c.set_free_col(Some(3));
            c.move_to(Pos::new(0, 6), true);
            c
        };
        // no selection left: no mark
        let mut c = marked();
        c.move_to(Pos::new(0, 3), true);
        assert_eq!((c.anchor(), c.anchor_free_col()), (None, None));
        // the selection is dropped
        let mut c = marked();
        c.move_to(Pos::new(0, 9), false);
        assert_eq!(c.anchor_free_col(), None);
        let mut c = marked();
        c.clear_selection();
        assert_eq!(c.anchor_free_col(), None);
        // a selection made by positions has no mark, unless it is given one
        let mut c = marked();
        c.select(Pos::new(0, 3), Pos::new(0, 8));
        assert_eq!(c.anchor_free_col(), None);
        c.set_anchor_free_col(Some(3));
        assert_eq!(c.anchor_free_col(), Some(3));
        // a text change under the cursor, and a layout change, drop it
        let mut c = marked();
        c.map_positions(|p| p);
        assert_eq!(c.anchor_free_col(), None);
        let mut c = marked();
        c.drop_free_cols();
        assert_eq!((c.free_col(), c.anchor_free_col()), (None, None));
        // a box selection has no marked anchor
        let mut c = marked();
        c.begin_box(2, false);
        assert_eq!(c.anchor_free_col(), None);
        // no anchor, no mark
        let mut c = Cursor::new(Pos::new(0, 3));
        c.set_anchor_free_col(Some(3));
        assert_eq!(c.anchor_free_col(), None);
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
    fn every_move_but_the_free_one_clears_the_free_column() {
        let mut c = Cursor::new(Pos::new(0, 2));
        c.set_free_col(Some(9));
        assert_eq!(c.free_col(), Some(9));
        c.move_to(Pos::new(0, 2), false);
        assert_eq!(c.free_col(), None);
        c.set_free_col(Some(9));
        c.select(Pos::new(0, 0), Pos::new(0, 2));
        assert_eq!(c.free_col(), None);
        c.set_free_col(Some(9));
        c.begin_box(9, false);
        assert_eq!(c.free_col(), None);
        c.set_free_col(Some(9));
        c.move_box_to(Pos::new(0, 2), 9);
        assert_eq!(c.free_col(), None);
        // dropping the selection keeps the caret where it stands
        c.set_free_col(Some(9));
        c.clear_selection();
        assert_eq!(c.free_col(), Some(9));
        let doc = Document::from_utf8(b"ab".to_vec()).unwrap();
        c.clamp(&doc).unwrap();
        assert_eq!(c.free_col(), None);
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
    fn a_box_selection_keeps_the_columns_of_its_corners() {
        let mut c = Cursor::new(Pos::new(0, 2));
        c.begin_box(2, true);
        assert!(c.is_box() && c.selecting_lock() && !c.has_selection());
        assert_eq!(c.box_corners(), None);
        // down to a short row: the position is its end, the column is not
        c.move_box_to(Pos::new(1, 1), 5);
        assert_eq!(
            c.box_corners(),
            Some(((Pos::new(0, 2), 2), (Pos::new(1, 1), 5)))
        );
        // back to the anchor corner: nothing is selected, the box goes on
        c.move_box_to(Pos::new(0, 2), 2);
        assert!(c.is_box() && !c.has_selection());
        assert_eq!((c.box_anchor_col(), c.box_caret_col()), (Some(2), Some(2)));
        // the same position in another column is another corner
        c.move_box_to(Pos::new(0, 2), 3);
        assert!(c.has_selection());
        assert_eq!(
            c.box_corners(),
            Some(((Pos::new(0, 2), 2), (Pos::new(0, 2), 3)))
        );
    }

    #[test]
    fn a_box_corner_in_a_gap_carries_the_marks() {
        let mut c = Cursor::new(Pos::new(0, 4));
        c.begin_box(4, true);
        // a box starts without marks; the caller sets the caret's, which goes with the anchor
        assert_eq!((c.free_col(), c.anchor_free_col()), (None, None));
        c.set_free_col(Some(4));
        c.move_box_to_gap(Pos::new(0, 8), 4);
        assert_eq!(
            c.box_corners(),
            Some(((Pos::new(0, 4), 4), (Pos::new(0, 8), 4)))
        );
        assert_eq!((c.anchor_free_col(), c.free_col()), (Some(4), Some(4)));
        // a move onto the text drops the caret's mark and keeps the anchor's
        c.move_box_to(Pos::new(0, 7), 3);
        assert_eq!((c.anchor_free_col(), c.free_col()), (Some(4), None));
        // the same position and column, but the anchor has the mark and the caret does not: two
        // corners
        c.move_box_to(Pos::new(0, 4), 4);
        assert!(c.has_selection());
        // back to the anchor corner, mark and all: nothing is selected
        c.move_box_to_gap(Pos::new(0, 4), 4);
        assert!(c.is_box() && !c.has_selection());
        assert_eq!((c.anchor_free_col(), c.free_col()), (None, Some(4)));
        // leaving the box ends its marks with the move
        let mut c = Cursor::new(Pos::new(0, 4));
        c.begin_box(4, true);
        c.set_free_col(Some(4));
        c.move_box_to_gap(Pos::new(0, 8), 5);
        c.move_to(Pos::new(0, 9), false);
        assert_eq!((c.anchor_free_col(), c.free_col()), (None, None));
    }

    #[test]
    fn anything_but_a_box_move_ends_the_box() {
        let begin = || {
            let mut c = Cursor::new(Pos::new(0, 1));
            c.begin_box(1, true);
            c.move_box_to(Pos::new(1, 1), 1);
            c
        };
        let mut c = begin();
        c.move_to(Pos::new(0, 0), false);
        assert_eq!(c, Cursor::new(Pos::new(0, 0)));
        let mut c = begin();
        c.clear_selection();
        assert!(!c.is_box() && !c.has_selection() && !c.selecting_lock());
        // a normal selection from the anchor of the box
        let mut c = begin();
        c.move_to(Pos::new(2, 0), true);
        assert!(!c.is_box());
        assert_eq!(c.selection(), Some((Pos::new(0, 1), Pos::new(2, 0))));
        let mut c = begin();
        c.select(Pos::new(0, 0), Pos::new(0, 3));
        assert!(!c.is_box());
        assert!(!c.selecting_lock());
        assert_eq!(c.box_caret_col(), None);
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
