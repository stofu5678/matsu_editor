// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/cmd/CViewCommander_Window.cpp (Command_CASCADE, Command_TILE_V, Command_TILE_H,
//   Command_MAXIMIZE_V, Command_MAXIMIZE_H)
//   sakura_core/_main/CControlTray.cpp (ActiveNextWindow, ActivePrevWindow)
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2000-2001, jepro
// Copyright (C) 2001, MIK
// Copyright (C) 2002, YAZAKI, genta, MIK
// Copyright (C) 2003, MIK, genta
// Copyright (C) 2004, Moca, genta, crayonzen, Kazika
// Copyright (C) 2006, genta, ryoji, maru
// Copyright (C) 2007, ryoji, genta
// Copyright (C) 2008, syat
// Copyright (C) 2009, syat
// Copyright (C) 2010, Moca
// Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the Win32 calls (`SetWindowPos`,
// `GetMonitorWorkRect`) were taken out, so that only the arithmetic is left, written in Rust on
// plain rectangles; the tests are new.
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

//! Where Sakura puts the editor windows: cascade, tile and maximize in one direction, and which
//! window is next. Plain functions on rectangles, so that every OS tests them.
//!
//! The y axis points down and `right` / `bottom` are exclusive, as in Win32's `RECT`.

/// A rectangle on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// The rectangle at `(left, top)` that is `width` wide and `height` high.
    pub const fn at(left: i32, top: i32, width: i32, height: i32) -> Self {
        Self::new(left, top, left + width, top + height)
    }

    pub const fn width(&self) -> i32 {
        self.right - self.left
    }

    pub const fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// Which window comes after the current one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// `F_NEXTWINDOW`.
    Next,
    /// `F_PREVWINDOW`.
    Previous,
}

/// The index of the window that `direction` goes to from `current`, among `len` windows in
/// Sakura's order (the order the windows were opened in; `GetOpenedWindowArr` sorted by the
/// window number). It wraps around. With one window it is that window.
///
/// Sakura: `F_NEXTWINDOW` calls `CControlTray::ActivePrevWindow`, which looks for the entry
/// *after* the current one and starts again from the first (`CEditDoc::HandleCommand`; the names
/// of the two are the other way round). `F_PREVWINDOW` calls `ActiveNextWindow`, the entry
/// *before*, and starts again from the last.
pub fn neighbor(len: usize, current: usize, direction: Direction) -> usize {
    if len == 0 {
        return 0;
    }
    match direction {
        Direction::Next => (current + 1) % len,
        Direction::Previous => (current + len - 1) % len,
    }
}

/// Moves the window at `current` to the end of `windows`: the order of the cascade, where the
/// current window is placed last and so ends up on top (`Command_CASCADE`, Mar. 20, 2004 genta).
pub fn cascade_order<T>(mut windows: Vec<T>, current: usize) -> Vec<T> {
    if current < windows.len() {
        let window = windows.remove(current);
        windows.push(window);
    }
    windows
}

/// The order of the tiles: the current window first, and the window that was first in its
/// place (`Command_TILE_V`, Jul. 28, 2002 genta: when the loop reaches the current window it
/// copies the first entry to the next free place and puts the current window first).
pub fn tile_order<T>(windows: Vec<T>, current: usize) -> Vec<T> {
    let count = windows.len();
    let mut order = vec![0; count];
    for i in 0..count {
        if i == current {
            order[i] = order[0];
            order[0] = i;
        } else {
            order[i] = i;
        }
    }
    let mut slots: Vec<Option<T>> = windows.into_iter().map(Some).collect();
    order.into_iter().filter_map(|i| slots[i].take()).collect()
}

/// The positions of `count` cascaded windows on `work`, and the size they all have
/// (`Command_CASCADE`).
///
/// `step` is the shift from one window to the next: Win32's `SM_CXSIZEFRAME + SM_CXSIZE` and
/// `SM_CYSIZEFRAME + SM_CYSIZE`, about the height of a title bar. The windows are four fifths of
/// the work area. When one would stick out at the right or at the bottom the next starts again at
/// the top, a little to the right of where the previous round started (so that the title bars
/// of the first round stay visible), and at the left again when there is no room for that.
pub fn cascade(work: Rect, count: usize, step: (i32, i32)) -> (Vec<(i32, i32)>, (i32, i32)) {
    let width = work.width() * 4 / 5;
    let height = work.height() * 4 / 5;
    let (w_delta, h_delta) = step;
    let mut w_offset = work.left;
    let mut h_offset = work.top;
    let mut roundtrip = 0;
    let mut positions = Vec::with_capacity(count);
    for _ in 0..count {
        if w_offset + width > work.right || h_offset + height > work.bottom {
            roundtrip += 1;
            if work.width() - w_delta * roundtrip < width {
                // No more room to move to the right: back to the top left.
                roundtrip = 0;
            }
            w_offset = work.left + w_delta * roundtrip;
            h_offset = work.top;
        }
        positions.push((w_offset, h_offset));
        w_offset += w_delta;
        h_offset += h_delta;
    }
    (positions, (width, height))
}

/// `count` windows one above the other, each the width of `work` (`Command_TILE_V`, 上下に並べて
/// 表示). Empty for no window.
pub fn tile_v(work: Rect, count: usize) -> Vec<Rect> {
    let Ok(n) = i32::try_from(count) else {
        return Vec::new();
    };
    if n == 0 {
        return Vec::new();
    }
    let height = work.height() / n;
    (0..n)
        .map(|i| Rect::at(work.left, work.top + height * i, work.width(), height))
        .collect()
}

/// `count` windows side by side, each the height of `work` (`Command_TILE_H`, 左右に並べて表示).
/// Empty for no window.
pub fn tile_h(work: Rect, count: usize) -> Vec<Rect> {
    let Ok(n) = i32::try_from(count) else {
        return Vec::new();
    };
    if n == 0 {
        return Vec::new();
    }
    let width = work.width() / n;
    (0..n)
        .map(|i| Rect::at(work.left + width * i, work.top, width, work.height()))
        .collect()
}

/// The window as high as the work area, where it is and as wide as it is
/// (`Command_MAXIMIZE_V`, 縦方向に最大化).
pub fn maximize_v(window: Rect, work: Rect) -> Rect {
    Rect::at(window.left, work.top, window.width(), work.height())
}

/// The window as wide as the work area, where it is and as high as it is
/// (`Command_MAXIMIZE_H`, 横方向に最大化).
pub fn maximize_h(window: Rect, work: Rect) -> Rect {
    Rect::at(work.left, window.top, work.width(), window.height())
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect::new(0, 0, 1000, 800);

    #[test]
    fn next_and_previous_wrap_around() {
        assert_eq!(neighbor(3, 0, Direction::Next), 1);
        assert_eq!(neighbor(3, 2, Direction::Next), 0);
        assert_eq!(neighbor(3, 0, Direction::Previous), 2);
        assert_eq!(neighbor(3, 2, Direction::Previous), 1);
        assert_eq!(neighbor(1, 0, Direction::Next), 0);
        assert_eq!(neighbor(1, 0, Direction::Previous), 0);
        assert_eq!(neighbor(0, 0, Direction::Next), 0);
    }

    #[test]
    fn the_current_window_is_last_in_a_cascade() {
        assert_eq!(
            cascade_order(vec!['a', 'b', 'c', 'd'], 1),
            ['a', 'c', 'd', 'b']
        );
        assert_eq!(cascade_order(vec!['a', 'b'], 1), ['a', 'b']);
        assert_eq!(cascade_order(vec!['a'], 0), ['a']);
        assert!(cascade_order(Vec::<char>::new(), 0).is_empty());
    }

    #[test]
    fn the_current_window_is_first_in_a_tile() {
        assert_eq!(tile_order(vec!['a', 'b', 'c'], 2), ['c', 'b', 'a']);
        assert_eq!(tile_order(vec!['a', 'b', 'c'], 1), ['b', 'a', 'c']);
        assert_eq!(tile_order(vec!['a', 'b', 'c'], 0), ['a', 'b', 'c']);
        assert_eq!(
            tile_order(vec!['a', 'b', 'c', 'd'], 2),
            ['c', 'b', 'a', 'd']
        );
        assert_eq!(tile_order(vec!['a'], 0), ['a']);
        assert!(tile_order(Vec::<char>::new(), 0).is_empty());
    }

    #[test]
    fn a_cascade_moves_by_the_title_bar() {
        let (positions, size) = cascade(WORK, 3, (30, 40));
        assert_eq!(size, (800, 640));
        assert_eq!(positions, [(0, 0), (30, 40), (60, 80)]);
    }

    #[test]
    fn a_cascade_starts_a_new_round_to_the_right() {
        // The windows are 800 x 640 in a 1000 x 800 area: a window at y = 160 still fits
        // (160 + 640 = 800), one at y = 200 does not.
        let (positions, _) = cascade(WORK, 6, (30, 40));
        assert_eq!(positions[4], (120, 160));
        // The second round starts one step to the right.
        assert_eq!(positions[5], (30, 0));
    }

    #[test]
    fn a_cascade_goes_back_to_the_top_left_when_there_is_no_room_on_the_right() {
        // A step of 150: the window at (150, 150) fits, the one at (300, 300) does not, so the
        // second round starts at x = 150 (1000 - 150 >= 800). The next window at x = 300 does
        // not fit either; a third round would start at x = 300, but 1000 - 300 < 800, so it goes
        // back to the corner.
        let (positions, _) = cascade(WORK, 5, (150, 150));
        assert_eq!(
            positions,
            [(0, 0), (150, 150), (150, 0), (0, 0), (150, 150)]
        );
    }

    #[test]
    fn a_cascade_follows_the_work_area_origin() {
        let work = Rect::new(100, 50, 1100, 850);
        let (positions, _) = cascade(work, 2, (30, 40));
        assert_eq!(positions, [(100, 50), (130, 90)]);
    }

    #[test]
    fn no_windows_make_no_cascade() {
        assert!(cascade(WORK, 0, (30, 40)).0.is_empty());
    }

    #[test]
    fn tiles_share_the_work_area() {
        assert_eq!(
            tile_v(WORK, 3),
            [
                Rect::at(0, 0, 1000, 266),
                Rect::at(0, 266, 1000, 266),
                Rect::at(0, 532, 1000, 266),
            ]
        );
        assert_eq!(
            tile_h(WORK, 3),
            [
                Rect::at(0, 0, 333, 800),
                Rect::at(333, 0, 333, 800),
                Rect::at(666, 0, 333, 800),
            ]
        );
        assert_eq!(tile_v(WORK, 1), [WORK]);
        assert_eq!(tile_h(WORK, 1), [WORK]);
        assert!(tile_v(WORK, 0).is_empty());
        assert!(tile_h(WORK, 0).is_empty());
    }

    #[test]
    fn tiles_follow_the_work_area_origin() {
        let work = Rect::new(-1920, 30, 0, 1110);
        assert_eq!(tile_h(work, 2)[1], Rect::at(-960, 30, 960, 1080));
        assert_eq!(tile_v(work, 2)[1], Rect::at(-1920, 570, 1920, 540));
    }

    #[test]
    fn maximizing_in_one_direction_keeps_the_other() {
        let window = Rect::at(200, 100, 500, 400);
        assert_eq!(maximize_v(window, WORK), Rect::at(200, 0, 500, 800));
        assert_eq!(maximize_h(window, WORK), Rect::at(0, 100, 1000, 400));
    }
}
