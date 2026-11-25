//! The horizontal positions that the commands keep, as [`X`] (ADR-0034).
//!
//! `matsu-core`'s [`Cursor`](matsu_core::cursor::Cursor) keeps the goal column, the free-cursor
//! column and the corners of a box as plain numbers: `matsu-layout`, where [`X`] lives, depends on
//! `matsu-core`, so the core cannot name the type, and it only stores and compares them (it never
//! converts them to columns). Their meaning is the layout's `X`, and this file is the one place
//! where a command turns them into [`X`] and back, so that no command does arithmetic on the raw
//! number.
//!
//! It also holds the two things that Sakura counts in `X` and that more than one command needs:
//! the widest the caret can go, and the spaces that fill a gap.

use matsu_core::buffer::Pos;
use matsu_layout::X;

use super::CommandContext;

/// The widest line in columns that the caret can stand at: Sakura's `GetMaxLineLayout()` is
/// `m_nMaxLineKetas`, 10240 by default (`MAXLINEKETAS`). Without wrapping at the width of the
/// window that is the limit; this crate has no setting for it yet.
pub const MAX_BOX_COLUMNS: u32 = 10240;

/// The stored number as an [`X`]. `matsu-ui` reads the corners of a box and the free caret with
/// this (re-exported as `cursor_x`), so that it too has no arithmetic on the raw number.
pub fn x_of(raw: usize) -> X {
    X::from_raw(u32::try_from(raw).unwrap_or(u32::MAX))
}

/// An [`X`] as the number that the cursor stores (re-exported as `cursor_raw`).
pub fn raw_of(x: X) -> usize {
    x.raw() as usize
}

impl CommandContext<'_> {
    /// The rightmost `X` of a box caret and of a free caret: the width of a row, which is the wrap
    /// column, or [`MAX_BOX_COLUMNS`] columns without wrapping (`CLayoutMgr::GetMaxLineLayout`).
    pub fn max_box_x(&self) -> X {
        self.layout.max_line_x()
    }

    /// The goal of the vertical moves (`m_nCaretPosX_Prev`).
    pub(super) fn goal_x(&self) -> Option<X> {
        self.cursor.goal_col().map(x_of)
    }

    pub(super) fn set_goal_x(&mut self, x: Option<X>) {
        self.cursor.set_goal_col(x.map(raw_of));
    }

    /// Where the caret stands when it is to the right of the text, as the cursor keeps it. It may
    /// be stale: [`caret_free_x`](Self::caret_free_x) checks it against the text.
    pub(super) fn stored_free_x(&self) -> Option<X> {
        self.cursor.free_col().map(x_of)
    }

    pub(super) fn set_free_x(&mut self, x: Option<X>) {
        self.cursor.set_free_col(x.map(raw_of));
    }

    /// The corner of the box that the cursor keeps for the caret, if a box is selected.
    pub(super) fn stored_box_caret_x(&self) -> Option<X> {
        self.cursor.box_caret_col().map(x_of)
    }

    /// The two corners of the box selection, each as a position and an `X`: the anchor and then
    /// the caret. `None` when no box is selected.
    pub fn box_corner_xs(&self) -> Option<((Pos, X), (Pos, X))> {
        let ((a, a_x), (c, c_x)) = self.cursor.box_corners()?;
        Some(((a, x_of(a_x)), (c, x_of(c_x))))
    }

    /// Starts a box at the caret, which is at `x` (`Cursor::begin_box`).
    pub(super) fn begin_box_at(&mut self, x: X, lock: bool) {
        self.cursor.begin_box(raw_of(x), lock);
    }

    /// Moves the caret of a box to `pos`, standing at `x` (`Cursor::move_box_to`).
    pub(super) fn set_box_caret(&mut self, pos: Pos, x: X) {
        self.cursor.move_box_to(pos, raw_of(x));
    }

    /// Moves the caret of a box into the gap at the end of the wrapped row before `pos`, standing
    /// at `x` (`Cursor::move_box_to_gap`).
    pub(super) fn set_box_gap_caret(&mut self, pos: Pos, x: X) {
        self.cursor.move_box_to_gap(pos, raw_of(x));
    }

    /// How many half-width spaces fill a gap of `gap`, the way Sakura counts them when something
    /// is put to the right of the end of a line: one space for each step of the width of a space
    /// that starts before the end of the gap, so a gap that is not a whole number of spaces gets
    /// one more, and the text that follows starts a little right of where it was asked for.
    ///
    /// Sakura: `CEditView::InsertData_CEditView` (`sakura_core/view/CEditView_Command_New.cpp`,
    /// "行終端より右に挿入しようとした"): `for (i = 0; i < x - lineAllColLen; i += nSpWidth)`
    /// adds a `L' '`, with `nSpWidth = CalcTextWidth3(L" ", 1)`, the width of a space with the
    /// character spacing. With a column of 1 `X` it is `gap` spaces, as before.
    ///
    /// (On a line with a line ending, Sakura starts the count from the end of the ending, which is
    /// 1 pixel wide per ending character: the loop's end is that much shorter, so a gap that is
    /// one pixel more than a whole number of spaces gets no extra space there. MATSU does not
    /// give the ending a width, so the count is the one without a line ending.)
    pub fn spaces_to_fill(&self, gap: X) -> usize {
        let space = self.layout.char_x(' ').raw().max(1);
        gap.raw().div_ceil(space) as usize
    }

    /// The whole columns in `x`, the rest dropped: what a conversion that counts columns is given
    /// (`nStartColumn = GetFrom().GetX2() / GetLayoutXDefault()`, `CEditView::ConvSelectedArea`).
    pub fn whole_columns(&self, x: X) -> usize {
        (x.raw() / self.layout.column_x().raw().max(1)) as usize
    }

    /// The width of the `n` spaces that [`spaces_to_fill`](Self::spaces_to_fill) puts in.
    pub fn width_of_spaces(&self, n: usize) -> X {
        self.layout
            .char_x(' ')
            .saturating_mul(u32::try_from(n).unwrap_or(u32::MAX))
    }
}
