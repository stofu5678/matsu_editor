// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/view/CEditView_Paint.cpp (CEditView::DrawBackImage)
//
// Original copyright notice of those files:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2022, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2008, kobake; 2018-2022, Sakura Editor Organization
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

//! Where the tiles of the background image go: `CEditView::DrawBackImage`
//! (`sakura_core/view/CEditView_Paint.cpp`).
//!
//! Everything is in pixels of the window, with the origin at the top left of the view. The
//! numbers are `i64` because the offset comes from the settings file and may be any `int`.

use matsu_config::settings::types::back_img_pos as pos;

/// A rectangle: `left` and `top` are inside, `right` and `bottom` are outside (a `RECT`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

impl Rect {
    pub(crate) fn new(left: i64, top: i64, right: i64, bottom: i64) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub(crate) fn width(&self) -> i64 {
        self.right - self.left
    }

    pub(crate) fn height(&self) -> i64 {
        self.bottom - self.top
    }

    /// Whether the two share at least one pixel (`IntersectRect` is not empty).
    pub(crate) fn meets(&self, other: &Rect) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }
}

/// What decides where the image is.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Layout {
    /// The text area (`CTextArea::GetAreaLeft` and the like): the position is relative to it.
    pub area: Rect,
    /// What is painted: the image is cut to it.
    pub clip: Rect,
    /// The size of the image in pixels.
    pub image: (i64, i64),
    /// How many times the picture handed to the painter holds the image across and down (1 where
    /// the image does not repeat). A small repeating image is made into a bigger picture of
    /// copies of itself, so that a view does not need thousands of tiles; the copies are the
    /// same picture, so the grid is the same.
    pub copies: (i64, i64),
    /// `bgImgPos` (`EBackgroundImagePos`).
    pub position: i32,
    /// `bgImgPosOffsetX` and `Y`.
    pub offset: (i64, i64),
    /// `bgImgRepeartX` and `Y`.
    pub repeat: (bool, bool),
    /// `bgImgScrollX` and `Y`: the image moves with the text.
    pub scroll: (bool, bool),
    /// How far the text is scrolled, in pixels.
    pub scrolled: (i64, i64),
}

/// The rectangles of the tiles that touch the painted part, in the order Sakura blits them (rows
/// from the top, left to right in each).
pub(crate) fn tiles(layout: &Layout) -> Vec<Rect> {
    let Layout {
        area,
        clip,
        image: (w, h),
        copies,
        ..
    } = *layout;
    if w <= 0 || h <= 0 {
        return Vec::new();
    }

    // Where the first tile is for the position. (A number that is no position gets the top left;
    // Sakura leaves the point as it was made, zero.)
    let (mut left, mut top) = (0, 0);
    let (horizontal, vertical) = anchors(layout.position);
    left += match horizontal {
        Anchor::Start => area.left,
        Anchor::End => area.right - w,
        Anchor::Middle => area.left + area.width() / 2 - w / 2,
        Anchor::None => 0,
    };
    top += match vertical {
        Anchor::Start => area.top,
        Anchor::End => area.bottom - h,
        Anchor::Middle => area.top + area.height() / 2 - h / 2,
        Anchor::None => 0,
    };
    left += layout.offset.0;
    top += layout.offset.1;

    // Scrolling moves the image the way it moves the text. `tile` is the size of the image where
    // it repeats (the scroll wraps there) and without end where it does not.
    if layout.scroll.0 {
        let tile = if layout.repeat.0 { w } else { i64::MAX };
        left -= layout.scrolled.0 % tile;
    }
    if layout.scroll.1 {
        let tile = if layout.repeat.1 { h } else { i64::MAX };
        top -= layout.scrolled.1 % tile;
    }
    // From here a tile is the picture handed to the painter.
    let (w, h) = (w * copies.0.max(1), h * copies.1.max(1));
    // A repeating image starts left of and above the painted part.
    if layout.repeat.0 && left > 0 {
        left = left % w - w;
    }
    if layout.repeat.1 && top > 0 {
        top = top % h - h;
    }
    // Skip the tiles that end before the painted part (the same tiles, without counting them).
    if layout.repeat.0 && left + w <= clip.left {
        left += (clip.left - left) / w * w;
    }
    if layout.repeat.1 && top + h <= clip.top {
        top += (clip.top - top) / h * h;
    }

    let mut out = Vec::new();
    let mut y = top;
    while y <= area.bottom {
        let mut x = left;
        while x <= area.right {
            let tile = Rect::new(x, y, x + w, y + h);
            if tile.meets(&clip) {
                out.push(tile);
            }
            x += w;
            if !layout.repeat.0 {
                break;
            }
        }
        y += h;
        if !layout.repeat.1 {
            break;
        }
    }
    out
}

#[derive(Clone, Copy)]
enum Anchor {
    Start,
    End,
    Middle,
    None,
}

/// The side of the area the position puts the image at, across and down.
fn anchors(position: i32) -> (Anchor, Anchor) {
    use Anchor::{End, Middle, None, Start};
    match position {
        pos::TOP_LEFT => (Start, Start),
        pos::TOP_RIGHT => (End, Start),
        pos::BOTTOM_LEFT => (Start, End),
        pos::BOTTOM_RIGHT => (End, End),
        pos::CENTER => (Middle, Middle),
        pos::TOP_CENTER => (Middle, Start),
        pos::BOTTOM_CENTER => (Middle, End),
        pos::CENTER_LEFT => (Start, Middle),
        pos::CENTER_RIGHT => (End, Middle),
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A view 400 wide and 300 high with a text area that starts at (40, 20) and ends at
    /// (380, 280); the painted part is the whole view below the ruler (20).
    fn layout(position: i32) -> Layout {
        Layout {
            area: Rect::new(40, 20, 380, 280),
            clip: Rect::new(0, 20, 380, 280),
            image: (100, 50),
            copies: (1, 1),
            position,
            offset: (0, 0),
            repeat: (false, false),
            scroll: (true, true),
            scrolled: (0, 0),
        }
    }

    fn one(layout: &Layout) -> (i64, i64) {
        let t = tiles(layout);
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!((t[0].width(), t[0].height()), layout.image);
        (t[0].left, t[0].top)
    }

    #[test]
    fn the_nine_positions() {
        // `CEditView::DrawBackImage`: the left of the area, its right edge minus the image, or
        // the middle; the same down.
        let expect = [
            (pos::TOP_LEFT, (40, 20)),
            (pos::TOP_RIGHT, (280, 20)),
            (pos::BOTTOM_LEFT, (40, 230)),
            (pos::BOTTOM_RIGHT, (280, 230)),
            (pos::CENTER, (160, 125)),
            (pos::TOP_CENTER, (160, 20)),
            (pos::BOTTOM_CENTER, (160, 230)),
            (pos::CENTER_LEFT, (40, 125)),
            (pos::CENTER_RIGHT, (280, 125)),
        ];
        for (position, at) in expect {
            assert_eq!(one(&layout(position)), at, "position {position}");
        }
    }

    #[test]
    fn the_middle_rounds_down_like_integer_division() {
        let mut l = layout(pos::CENTER);
        l.image = (101, 51);
        // 40 + 340/2 - 101/2 = 40 + 170 - 50; 20 + 260/2 - 51/2 = 20 + 130 - 25
        assert_eq!(one(&l), (160, 125));
    }

    #[test]
    fn a_position_that_is_no_position_puts_the_image_at_the_origin() {
        let mut l = layout(99);
        l.clip = Rect::new(0, 0, 380, 280);
        assert_eq!(one(&l), (0, 0));
    }

    #[test]
    fn the_offset_moves_the_image() {
        let mut l = layout(pos::TOP_LEFT);
        l.offset = (7, -3);
        // -3 puts the top above the ruler: the tile still touches the painted part
        assert_eq!(one(&l), (47, 17));
        let mut l = layout(pos::BOTTOM_RIGHT);
        l.offset = (-10, -10);
        assert_eq!(one(&l), (270, 220));
    }

    #[test]
    fn an_image_out_of_the_painted_part_is_not_drawn() {
        let mut l = layout(pos::TOP_LEFT);
        l.offset = (10_000, 0);
        assert!(tiles(&l).is_empty());
        l.offset = (0, -10_000);
        assert!(tiles(&l).is_empty());
        // beyond the text area, even if the painted part reached there, nothing starts there
        let mut l = layout(pos::TOP_LEFT);
        l.offset = (341, 0);
        assert!(tiles(&l).is_empty());
    }

    #[test]
    fn repeating_fills_the_painted_part_from_the_edge_of_the_area() {
        let mut l = layout(pos::TOP_LEFT);
        l.repeat = (true, true);
        let t = tiles(&l);
        // The grid starts left of the area at -60 (40 % 100 - 100) and above it at -30; the
        // tile at -30 ends at the ruler (20), so the first that is painted is the next.
        assert_eq!((t[0].left, t[0].top), (-60, 20));
        // Every pixel of the painted part is under a tile, and the tiles are a grid.
        for y in 20..280 {
            for x in (0..380).step_by(7) {
                let hits = t
                    .iter()
                    .filter(|r| r.left <= x && x < r.right && r.top <= y && y < r.bottom)
                    .count();
                assert_eq!(hits, 1, "({x}, {y})");
            }
        }
        for r in &t {
            assert_eq!((r.left + 60) % 100, 0);
            assert_eq!((r.top + 30) % 50, 0);
        }
    }

    #[test]
    fn repeating_one_way_stays_one_row_or_one_column() {
        let mut l = layout(pos::TOP_LEFT);
        l.repeat = (true, false);
        let t = tiles(&l);
        assert!(t.iter().all(|r| r.top == 20));
        assert!(t.len() > 3);
        l.repeat = (false, true);
        let t = tiles(&l);
        assert!(t.iter().all(|r| r.left == 40));
        assert!(t.len() > 3);
    }

    #[test]
    fn scrolling_moves_the_image_with_the_text() {
        let mut l = layout(pos::TOP_LEFT);
        l.scrolled = (15, 12);
        assert_eq!(one(&l), (25, 8));
        // The text scrolled by more than the room above the image: it is partly gone.
        l.scrolled = (0, 40);
        assert_eq!(one(&l), (40, -20));
        // (the tile at -20 + 50 = 30 still touches the painted part)
        l.scrolled = (0, 70);
        assert!(tiles(&l).is_empty(), "the image scrolled out of the view");
    }

    #[test]
    fn a_fixed_image_does_not_move_when_the_text_does() {
        let mut l = layout(pos::TOP_LEFT);
        l.scroll = (false, false);
        l.scrolled = (500, 900);
        assert_eq!(one(&l), (40, 20));
    }

    #[test]
    fn each_direction_scrolls_or_stays_by_itself() {
        let mut l = layout(pos::TOP_LEFT);
        l.scroll = (true, false);
        l.scrolled = (10, 10);
        assert_eq!(one(&l), (30, 20));
        l.scroll = (false, true);
        assert_eq!(one(&l), (40, 10));
    }

    #[test]
    fn a_repeating_image_scrolls_around_by_its_own_size() {
        let mut l = layout(pos::TOP_LEFT);
        l.repeat = (true, true);
        l.scrolled = (0, 0);
        let first = tiles(&l);
        // scrolled by a whole image, the grid is where it was
        l.scrolled = (100, 50);
        assert_eq!(tiles(&l), first);
        // scrolled by a part, the grid moved by that part
        l.scrolled = (130, 20);
        let t = tiles(&l);
        assert_eq!((t[0].left, t[0].top), (-90, 0));
        assert!(
            t.iter()
                .all(|r| (r.left + 90) % 100 == 0 && (r.top + 50) % 50 == 0)
        );
    }

    #[test]
    fn the_right_and_bottom_positions_repeat_from_the_same_grid() {
        let mut l = layout(pos::BOTTOM_RIGHT);
        l.repeat = (true, true);
        let t = tiles(&l);
        // One tile ends at the right edge of the area and at the bottom of it.
        assert!(t.iter().any(|r| r.right == 380 && r.bottom == 280));
    }

    #[test]
    fn a_far_offset_does_not_make_the_loop_long() {
        let mut l = layout(pos::TOP_LEFT);
        l.repeat = (true, true);
        l.offset = (i64::from(i32::MIN), i64::from(i32::MIN));
        let t = tiles(&l);
        assert!(!t.is_empty() && t.len() < 100, "{}", t.len());
        l.offset = (i64::from(i32::MAX), i64::from(i32::MAX));
        let t = tiles(&l);
        assert!(!t.is_empty() && t.len() < 100, "{}", t.len());
    }

    #[test]
    fn copies_of_the_image_make_the_same_grid_with_fewer_tiles() {
        // A centered image, repeating: the grid has a line in the middle of the area minus half
        // an image. With the picture three times as wide the grid is the same.
        for position in [pos::CENTER, pos::TOP_LEFT, pos::BOTTOM_RIGHT] {
            let mut l = layout(position);
            l.image = (30, 10);
            l.repeat = (true, true);
            l.scrolled = (17, 23);
            let single = tiles(&l);
            l.copies = (4, 3);
            let grouped = tiles(&l);
            assert!(grouped.len() < single.len());
            // each pixel of the painted part is covered once by either
            for y in (20..280).step_by(3) {
                for x in (0..380).step_by(5) {
                    let at = |t: &[Rect]| {
                        t.iter()
                            .position(|r| r.left <= x && x < r.right && r.top <= y && y < r.bottom)
                            .map(|i| ((x - t[i].left) % 30, (y - t[i].top) % 10))
                    };
                    assert_eq!(at(&single), at(&grouped), "({x}, {y}) at {position}");
                }
            }
        }
    }

    #[test]
    fn an_empty_image_has_no_tiles() {
        let mut l = layout(pos::TOP_LEFT);
        l.image = (0, 10);
        assert!(tiles(&l).is_empty());
    }

    #[test]
    fn the_painted_part_cuts_the_tiles_it_does_not_touch() {
        let mut l = layout(pos::TOP_LEFT);
        l.repeat = (true, true);
        l.clip = Rect::new(100, 100, 130, 130);
        let t = tiles(&l);
        assert!(!t.is_empty());
        assert!(t.iter().all(|r| r.meets(&l.clip)));
    }
}
