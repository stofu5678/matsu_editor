// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/window/CSplitterWnd.cpp (DoSplit, VSplitOnOff, HSplitOnOff, VHSplitOnOff)
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2002, aroka, YAZAKI
// Copyright (C) 2003, MIK
// Copyright (C) 2007, ryoji
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the window handles, the split boxes
// and the size box are gone, the rules work on a `Grid` and return a `Plan` of what to do, and
// the tests are new.
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! The rules of Sakura's splitter, without any window: which panes a split shows, which pane
//! takes the view of which, and which pane is active afterwards.
//!
//! Sakura numbers the panes 0 (top left), 1 (top right), 2 (bottom left) and 3 (bottom right): a
//! split into rows shows 0 and 2, a split into columns shows 0 and 1, and both show all four.
//! Pane 0 is always there. [`do_split`] follows the branches of `CSplitterWnd::DoSplit`.

/// The split of the text area: how many rows and columns of panes (1 or 2 each).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Grid {
    /// `m_nAllSplitRows`
    pub rows: u8,
    /// `m_nAllSplitCols`
    pub cols: u8,
}

impl Grid {
    pub(crate) const ONE: Grid = Grid { rows: 1, cols: 1 };

    /// The panes that are shown.
    pub(crate) fn panes(self) -> &'static [usize] {
        match (self.rows > 1, self.cols > 1) {
            (false, false) => &[0],
            (true, false) => &[0, 2],
            (false, true) => &[0, 1],
            (true, true) => &[0, 1, 2, 3],
        }
    }

    pub(crate) fn shows(self, pane: usize) -> bool {
        self.panes().contains(&pane)
    }

    pub(crate) fn is_split(self) -> bool {
        self != Grid::ONE
    }
}

/// `F_SPLIT_V`, `F_SPLIT_H` and `F_SPLIT_VH`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitKind {
    /// 上下に分割
    Vertical,
    /// 左右に分割
    Horizontal,
    /// 縦横に分割
    Both,
}

/// `nLimit = DpiScaleX(32)` in `DoSplit`: a bar closer than this to an edge removes the split on
/// that side.
pub(crate) const LIMIT: f32 = 32.0;

/// How the active pane is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Active {
    Pane(usize),
    /// A split into rows from the unsplit window: pane 2 if the caret is below the top row of
    /// pane 2, else pane 0 (`GetViewTopLine() < GetCaretLayoutPos().y` of pane 2).
    ByCaret,
}

/// What `DoSplit` does.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Plan {
    pub grid: Grid,
    /// Where the bars are, from the left and from the top; 0 for a bar that is not there.
    pub h: f32,
    pub v: f32,
    /// `(from, to)`: `to` takes the view of `from` (`CopyViewStatus`: caret, selection, scroll).
    pub copies: Vec<(usize, usize)>,
    /// Pane 2 starts one view below pane 0 (`SetViewTopLine( top + m_nViewRowNum )`).
    pub stagger: bool,
    pub active: Active,
}

/// The place of the bars that a split command asks for (`VSplitOnOff`, `HSplitOnOff` and
/// `VHSplitOnOff`): `(h, v)`, 0 for no bar. `h` and `v` are the bars now; `width` and `height`
/// the size of the area.
pub(crate) fn request(
    kind: SplitKind,
    old: Grid,
    h: f32,
    v: f32,
    width: f32,
    height: f32,
) -> (f32, f32) {
    let rows = old.rows > 1;
    let cols = old.cols > 1;
    match kind {
        SplitKind::Vertical => match (rows, cols) {
            (false, false) => (0.0, height / 2.0),
            (false, true) => (h, height / 2.0),
            (true, false) => (0.0, 0.0),
            (true, true) => (h, 0.0),
        },
        SplitKind::Horizontal => match (rows, cols) {
            (false, false) => (width / 2.0, 0.0),
            (false, true) => (0.0, 0.0),
            (true, false) => (width / 2.0, v),
            (true, true) => (0.0, v),
        },
        SplitKind::Both => {
            if rows && cols {
                (0.0, 0.0)
            } else {
                let y = if rows { v } else { height / 2.0 };
                let x = if cols { h } else { width / 2.0 };
                (x, y)
            }
        }
    }
}

/// `CSplitterWnd::DoSplit( nHorizontal, nVertical )`: the split with bars at `h` (from the left)
/// and `v` (from the top), 0 for none, in an area of `width` by `height`. `old` and `active` are
/// the split and the active pane now.
pub(crate) fn do_split(old: Grid, active: usize, h: f32, v: f32, width: f32, height: f32) -> Plan {
    let (mut h, mut v) = (h, v);
    // A bar near an edge is not a split: the bar was dragged there. `h_up` and `v_up` say that it
    // went to the left or top edge, where the right or lower pane stays.
    let mut h_up = false;
    let mut v_up = false;
    if h < LIMIT {
        h_up = h > 0.0;
        h = 0.0;
    }
    if h > width - LIMIT * 2.0 {
        h = 0.0;
    }
    if v < LIMIT {
        v_up = v > 0.0;
        v = 0.0;
    }
    if v > height - LIMIT * 2.0 {
        v = 0.0;
    }

    let old_rows = old.rows > 1;
    let old_cols = old.cols > 1;
    let mut copies = Vec::new();
    let mut stagger = false;
    let (grid, new_active) = if v == 0.0 && h == 0.0 {
        let grid = Grid::ONE;
        // Pane 0 stays. It takes the view of the pane the user was in, unless a bar went to the
        // top or left edge: then the view of the pane that was left of or below that bar.
        let from_active = (active != 0).then_some((active, 0));
        match (old_rows, old_cols) {
            (false, false) => {}
            (true, false) => {
                if v_up {
                    copies.push((2, 0));
                } else {
                    copies.extend(from_active);
                }
            }
            (false, true) => {
                if h_up {
                    copies.push((1, 0));
                } else {
                    copies.extend(from_active);
                }
            }
            (true, true) => match (v_up, h_up) {
                (false, false) => copies.extend(from_active),
                (true, false) => copies.push((2, 0)),
                (false, true) => copies.push((1, 0)),
                (true, true) => copies.push((3, 0)),
            },
        }
        (grid, Active::Pane(0))
    } else if v > 0.0 && h == 0.0 {
        let grid = Grid { rows: 2, cols: 1 };
        match (old_rows, old_cols) {
            (false, false) => {
                copies.push((0, 2));
                stagger = true;
            }
            (true, false) | (false, true) => {}
            (true, true) => {
                if h_up || (active != 0 && active != 2) {
                    copies.push((1, 0));
                    copies.push((3, 2));
                }
            }
        }
        let new_active = if active == 0 || active == 1 {
            // 2007.10.01 ryoji: only a split from "no split" chooses; otherwise pane 0.
            if !old_rows && !old_cols {
                Active::ByCaret
            } else {
                Active::Pane(0)
            }
        } else {
            Active::Pane(2)
        };
        (grid, new_active)
    } else if v == 0.0 && h > 0.0 {
        let grid = Grid { rows: 1, cols: 2 };
        match (old_rows, old_cols) {
            (false, false) => copies.push((0, 1)),
            (true, false) | (false, true) => {}
            (true, true) => {
                if v_up || (active != 0 && active != 1) {
                    copies.push((2, 0));
                    copies.push((3, 1));
                }
            }
        }
        (
            grid,
            Active::Pane(if active == 0 || active == 2 { 0 } else { 1 }),
        )
    } else {
        let grid = Grid { rows: 2, cols: 2 };
        match (old_rows, old_cols) {
            (false, false) => copies.extend([(0, 1), (0, 2), (0, 3)]),
            (true, false) => copies.extend([(0, 1), (2, 3)]),
            (false, true) => copies.extend([(0, 2), (1, 3)]),
            (true, true) => {}
        }
        (grid, Active::Pane(active))
    };
    Plan {
        grid,
        h,
        v,
        copies,
        stagger,
        active: new_active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 1000.0;
    const H: f32 = 600.0;

    fn grid(rows: u8, cols: u8) -> Grid {
        Grid { rows, cols }
    }

    /// A split command with the bars where the test says they are.
    fn command(kind: SplitKind, old: Grid, active: usize) -> Plan {
        let (rh, rv) = request(kind, old, 500.0, 300.0, W, H);
        do_split(old, active, rh, rv, W, H)
    }

    #[test]
    fn the_panes_of_a_grid_are_sakuras() {
        assert_eq!(Grid::ONE.panes(), &[0]);
        assert_eq!(grid(2, 1).panes(), &[0, 2]);
        assert_eq!(grid(1, 2).panes(), &[0, 1]);
        assert_eq!(grid(2, 2).panes(), &[0, 1, 2, 3]);
        assert!(grid(2, 1).shows(2));
        assert!(!grid(2, 1).shows(1));
    }

    #[test]
    fn splitting_up_and_down_puts_the_bar_in_the_middle() {
        let p = command(SplitKind::Vertical, Grid::ONE, 0);
        assert_eq!(p.grid, grid(2, 1));
        assert_eq!((p.h, p.v), (0.0, 300.0));
        // pane 2 starts as a copy of pane 0, one view further down
        assert_eq!(p.copies, vec![(0, 2)]);
        assert!(p.stagger);
        assert_eq!(p.active, Active::ByCaret);
    }

    #[test]
    fn splitting_left_and_right_copies_pane_0_and_keeps_it_active() {
        let p = command(SplitKind::Horizontal, Grid::ONE, 0);
        assert_eq!(p.grid, grid(1, 2));
        assert_eq!((p.h, p.v), (500.0, 0.0));
        assert_eq!(p.copies, vec![(0, 1)]);
        assert!(!p.stagger);
        assert_eq!(p.active, Active::Pane(0));
    }

    #[test]
    fn splitting_both_ways_makes_three_copies_of_pane_0() {
        let p = command(SplitKind::Both, Grid::ONE, 0);
        assert_eq!(p.grid, grid(2, 2));
        assert_eq!((p.h, p.v), (500.0, 300.0));
        assert_eq!(p.copies, vec![(0, 1), (0, 2), (0, 3)]);
        assert_eq!(p.active, Active::Pane(0));
    }

    #[test]
    fn the_same_command_again_removes_the_split() {
        for (kind, from) in [
            (SplitKind::Vertical, grid(2, 1)),
            (SplitKind::Horizontal, grid(1, 2)),
            (SplitKind::Both, grid(2, 2)),
        ] {
            let p = command(kind, from, 0);
            assert_eq!(p.grid, Grid::ONE, "{kind:?}");
            assert_eq!(p.active, Active::Pane(0));
        }
    }

    #[test]
    fn the_pane_that_stays_takes_the_view_of_the_active_pane() {
        let p = command(SplitKind::Vertical, grid(2, 1), 2);
        assert_eq!(p.copies, vec![(2, 0)]);
        let p = command(SplitKind::Horizontal, grid(1, 2), 1);
        assert_eq!(p.copies, vec![(1, 0)]);
        let p = command(SplitKind::Both, grid(2, 2), 3);
        assert_eq!(p.copies, vec![(3, 0)]);
        // nothing to copy when pane 0 was active
        let p = command(SplitKind::Both, grid(2, 2), 0);
        assert!(p.copies.is_empty());
    }

    #[test]
    fn from_four_panes_one_command_removes_one_direction() {
        // F_SPLIT_V: the rows go, the columns stay at the same bar
        let p = command(SplitKind::Vertical, grid(2, 2), 3);
        assert_eq!(p.grid, grid(1, 2));
        assert_eq!(p.h, 500.0);
        assert_eq!(p.copies, vec![(2, 0), (3, 1)]);
        assert_eq!(p.active, Active::Pane(1));
        // F_SPLIT_H: the columns go
        let p = command(SplitKind::Horizontal, grid(2, 2), 3);
        assert_eq!(p.grid, grid(2, 1));
        assert_eq!(p.v, 300.0);
        assert_eq!(p.copies, vec![(1, 0), (3, 2)]);
        assert_eq!(p.active, Active::Pane(2));
        // when the active pane is in the part that stays, no copy
        let p = command(SplitKind::Horizontal, grid(2, 2), 2);
        assert!(p.copies.is_empty());
        assert_eq!(p.active, Active::Pane(2));
        let p = command(SplitKind::Vertical, grid(2, 2), 1);
        assert!(p.copies.is_empty());
        assert_eq!(p.active, Active::Pane(1));
    }

    #[test]
    fn adding_the_other_direction_keeps_the_panes_and_the_active_one() {
        let p = command(SplitKind::Both, grid(2, 1), 2);
        assert_eq!(p.grid, grid(2, 2));
        assert_eq!(p.copies, vec![(0, 1), (2, 3)]);
        assert_eq!(p.active, Active::Pane(2));
        let p = command(SplitKind::Both, grid(1, 2), 1);
        assert_eq!(p.grid, grid(2, 2));
        assert_eq!(p.copies, vec![(0, 2), (1, 3)]);
        assert_eq!(p.active, Active::Pane(1));
        // F_SPLIT_V on columns and F_SPLIT_H on rows add the other direction as well
        let p = command(SplitKind::Vertical, grid(1, 2), 1);
        assert_eq!(p.grid, grid(2, 2));
        let p = command(SplitKind::Horizontal, grid(2, 1), 2);
        assert_eq!(p.grid, grid(2, 2));
        assert_eq!(p.active, Active::Pane(2));
    }

    #[test]
    fn a_bar_near_an_edge_removes_the_split() {
        // dragged to the top edge: the lower pane stays
        let p = do_split(grid(2, 1), 0, 0.0, 10.0, W, H);
        assert_eq!(p.grid, Grid::ONE);
        assert_eq!(p.copies, vec![(2, 0)]);
        // dragged to the bottom edge: the upper pane stays, with the view of the active pane
        let p = do_split(grid(2, 1), 2, 0.0, H - 10.0, W, H);
        assert_eq!(p.grid, Grid::ONE);
        assert_eq!(p.copies, vec![(2, 0)]);
        // the left edge: the right pane stays
        let p = do_split(grid(1, 2), 0, 10.0, 0.0, W, H);
        assert_eq!(p.copies, vec![(1, 0)]);
        // both edges of four panes: pane 3 stays
        let p = do_split(grid(2, 2), 0, 10.0, 10.0, W, H);
        assert_eq!(p.copies, vec![(3, 0)]);
        // one edge of four panes keeps two panes
        let p = do_split(grid(2, 2), 0, 10.0, 300.0, W, H);
        assert_eq!(p.grid, grid(2, 1));
        assert_eq!(p.copies, vec![(1, 0), (3, 2)]);
    }

    #[test]
    fn moving_a_bar_keeps_the_grid() {
        let p = do_split(grid(2, 1), 2, 0.0, 200.0, W, H);
        assert_eq!(p.grid, grid(2, 1));
        assert_eq!(p.v, 200.0);
        assert!(p.copies.is_empty());
        assert_eq!(p.active, Active::Pane(2));
        // from pane 0 the active pane is pane 0, not chosen by the caret
        let p = do_split(grid(2, 1), 0, 0.0, 200.0, W, H);
        assert_eq!(p.active, Active::Pane(0));
    }
}
