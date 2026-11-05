//! The split boxes (分割ボックス): the small boxes at the end of the scroll bars of a window that
//! is not split enough, which are dragged to split the window.
//!
//! Sakura: `CSplitBoxWnd` (`sakura_core/window/CSplitBoxWnd.cpp`: the size, `OnPaint`, and the drag
//! that ends in `MYWM_DOSPLIT`), `CEditView::SplitBoxOnOff` and `CEditView::OnSize`
//! (`sakura_core/view/CEditView.cpp`, which gives the boxes their place and takes their room from
//! the bars), and which pane has which box (`CSplitterWnd::DoSplit`, `CSplitterWnd::OnSize`). The
//! vertical box sits on top of the vertical bar and splits the window into rows, the horizontal
//! one is left of the horizontal bar and splits it into columns.
//!
//! This file is the shapes and the rule of which pane has which box. The drag itself is the one of
//! the bars between the panes (`crate::panes`), which the box starts.

use crate::geometry::{HSCROLLBAR_HEIGHT, SCROLLBAR_WIDTH, SPLIT_BOX};

/// The face of the box (`COLOR_BTNFACE`).
pub(crate) const FACE: u32 = 0xf0f0f0;
/// Its edges (`COLOR_3DSHADOW` and `COLOR_BTNSHADOW`, which have the same value).
pub(crate) const SHADOW: u32 = 0xa0a0a0;

/// Which split boxes a pane has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Boxes {
    /// Above the vertical bar (`m_pcsbwVSplitBox`).
    pub vertical: bool,
    /// Left of the horizontal bar (`m_pcsbwHSplitBox`).
    pub horizontal: bool,
}

impl Boxes {
    pub(crate) const ALL: Boxes = Boxes {
        vertical: true,
        horizontal: true,
    };
}

/// Which box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoxKind {
    /// Above the vertical bar: drag it down to split into upper and lower panes.
    Vertical,
    /// Left of the horizontal bar: drag it right to split into left and right panes.
    Horizontal,
}

/// Where a box is, in window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoxRect {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl BoxRect {
    pub(crate) fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.left && x < self.left + self.width && y >= self.top && y < self.top + self.height
    }
}

/// The size of a box: 7 by the height of the horizontal bar, or the width of the vertical bar by
/// 7 (`CSplitBoxWnd::Create`).
pub(crate) fn size_of(kind: BoxKind) -> (f32, f32) {
    match kind {
        BoxKind::Vertical => (SCROLLBAR_WIDTH, SPLIT_BOX),
        BoxKind::Horizontal => (SPLIT_BOX, HSCROLLBAR_HEIGHT),
    }
}

/// The rectangles of a box and their colors, from its top left (`CSplitBoxWnd::OnPaint`): the
/// face, and the edges (`cxEdge` and `cyEdge`, 2 pixels) of the shadow around it. The face is
/// first.
pub(crate) fn parts(kind: BoxKind) -> Vec<(f32, f32, f32, f32, u32)> {
    let (w, h) = size_of(kind);
    let edge = 2.0;
    match kind {
        BoxKind::Vertical => vec![
            (edge, edge, w - 2.0 * edge, h - 2.0 * edge, FACE),
            (edge, h - edge, w - 2.0 * edge, edge, SHADOW),
            (w - edge, edge, edge, h - edge, SHADOW),
            (edge, 0.0, w - 2.0 * edge, edge, SHADOW),
            (0.0, 0.0, edge, h - edge, SHADOW),
            // The two corners that no edge covers are the face of the window behind
            // (the class brush `COLOR_3DFACE`).
            (w - edge, 0.0, edge, edge, FACE),
            (0.0, h - edge, edge, edge, FACE),
        ],
        BoxKind::Horizontal => vec![
            (edge, edge, w - 2.0 * edge, h - 2.0 * edge, FACE),
            (w - edge, edge, edge, h - edge, SHADOW),
            (0.0, h - edge, w - edge, edge, SHADOW),
            (0.0, 0.0, edge, h - edge, SHADOW),
            (edge, 0.0, w - edge, edge, SHADOW),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_is_seven_pixels_on_the_bar_it_sits_on() {
        assert_eq!(size_of(BoxKind::Horizontal), (7.0, 14.0));
        assert_eq!(size_of(BoxKind::Vertical), (14.0, 7.0));
    }

    #[test]
    fn the_parts_of_a_box_cover_it_without_overlap() {
        for kind in [BoxKind::Vertical, BoxKind::Horizontal] {
            let (w, h) = size_of(kind);
            let mut covered = vec![0u8; (w * h) as usize];
            for (x, y, pw, ph, _) in parts(kind) {
                for row in y as usize..(y + ph) as usize {
                    for col in x as usize..(x + pw) as usize {
                        covered[row * w as usize + col] += 1;
                    }
                }
            }
            assert!(covered.iter().all(|&n| n == 1), "{kind:?}: {covered:?}");
        }
    }

    #[test]
    fn a_point_is_in_the_box_inside_its_edges() {
        let b = BoxRect {
            left: 10.0,
            top: 20.0,
            width: 7.0,
            height: 14.0,
        };
        assert!(b.contains(10.0, 20.0));
        assert!(b.contains(16.9, 33.9));
        assert!(!b.contains(17.0, 20.0));
        assert!(!b.contains(10.0, 34.0));
    }
}
