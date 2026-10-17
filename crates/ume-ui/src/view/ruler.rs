// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/view/CRuler.cpp (DrawRulerBg, _DrawRulerCaret, DispRuler)
//
// Original copyright notice of that file:
//   Copyright (C) 2018-2022, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The ruler above the text (`RUL`): a scale of columns with a longer line every 5 columns, a
//! line and the number of tens every 10 columns, a mark at the wrap column, and a mark for the
//! column of the caret.
//!
//! This is the arithmetic of it in pixels, so that it is tested without a window. `x` is from
//! the left of the text area, `y` from the top of the ruler; the ruler is as high as
//! `nRulerHeight` (13 by default) and the scale sits on its bottom.

use super::lines::exor_color;

/// A line of the scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tick {
    pub x: f32,
    /// The first pixel row of the line; it runs down to [`Scale::base_y`] (not including it).
    pub top: f32,
    /// The column this line is at, counted from 0.
    pub column: u32,
    /// The number of tens, written right of the line, when the column is a multiple of 10.
    pub label: Option<u32>,
}

/// The scale of a ruler.
#[derive(Clone, Debug, PartialEq)]
pub struct Scale {
    /// The line under the scale: its y, and from x to x.
    pub base_y: f32,
    pub base_from: f32,
    pub base_to: f32,
    pub ticks: Vec<Tick>,
}

/// The scale for a ruler `height` pixels high over a text area `view_width` wide, scrolled left by
/// `scroll_x`, where a column is `cell` wide and the text wraps at `max_columns`
/// (`DrawRulerBg`).
pub fn scale(height: u32, cell: f32, scroll_x: f32, view_width: f32, max_columns: u32) -> Scale {
    // `nY = GetRulerHeight() - 2`: the foot of the lines; the base line is the row below it.
    let foot = height.saturating_sub(2) as f32;
    let base_y = foot + 1.0;
    // The line ends where the wrap column is (one pixel past it), or at the right edge.
    let base_to = view_width.min(max_columns as f32 * cell - scroll_x + 1.0);
    let mut ticks = Vec::new();
    // A column cut by the left edge is skipped.
    let mut column = (scroll_x / cell).ceil().max(0.0) as u32;
    while column <= max_columns {
        let x = column as f32 * cell - scroll_x;
        if x > view_width + 1.0 {
            break;
        }
        let tens = column.is_multiple_of(10);
        let end = if column == max_columns || tens {
            0.0
        } else if column.is_multiple_of(5) {
            (foot - 6.0).max(0.0)
        } else {
            (foot - 3.0).max(0.0)
        };
        // A line does not include the point it ends at.
        ticks.push(Tick {
            x,
            top: end + 1.0,
            column,
            label: (tens && column != max_columns).then_some(column / 10),
        });
        column += 1;
    }
    Scale {
        base_y,
        base_from: 0.0,
        base_to,
        ticks,
    }
}

/// The mark of the caret on the ruler: from x to x, and from y to y (`_DrawRulerCaret`: one pixel
/// right of the column, a column wide less that pixel, and the ruler's height less one). `None`
/// when the caret's column is not on the screen.
pub fn caret_mark(
    height: u32,
    caret_col: u32,
    cell: f32,
    scroll_x: f32,
    view_width: f32,
) -> Option<(f32, f32, f32, f32)> {
    let first = (scroll_x / cell).floor().max(0.0) as u32;
    let right = ((scroll_x + view_width) / cell).floor() as u32;
    if caret_col < first || caret_col > right + 2 {
        return None;
    }
    let x = caret_col as f32 * cell - scroll_x + 1.0;
    Some((x, x + cell - 1.0, 0.0, height.saturating_sub(1) as f32))
}

/// The color of the caret mark over the ruler's background: the mark inverts what is under it
/// (`R2_NOTXORPEN`), with black, or with gray while the caret has no width (the view has not got
/// the focus).
pub fn caret_color(background: u32, focused: bool) -> u32 {
    exor_color(background, if focused { 0x000000 } else { 0x808080 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns(scale: &Scale) -> Vec<u32> {
        scale.ticks.iter().map(|t| t.column).collect()
    }

    #[test]
    fn the_scale_has_a_line_for_each_column_up_to_the_wrap_column() {
        let s = scale(13, 8.0, 0.0, 100.0, 10);
        assert_eq!(columns(&s), (0..=10).collect::<Vec<_>>());
        assert_eq!(s.ticks[3].x, 24.0);
    }

    #[test]
    fn lines_are_short_every_column_medium_every_5_and_tall_every_10() {
        // height 13: the lines end at y 11
        let s = scale(13, 8.0, 0.0, 400.0, 100);
        let top = |column: u32| s.ticks[column as usize].top;
        // short: from 11 - 3 = 8, not including 8
        assert_eq!(top(1), 9.0);
        // medium: 11 - 6 = 5
        assert_eq!(top(5), 6.0);
        // tall: down from 0
        assert_eq!(top(10), 1.0);
        assert_eq!(top(20), 1.0);
        assert_eq!(s.base_y, 12.0);
    }

    #[test]
    fn a_number_stands_at_each_10_columns() {
        let s = scale(13, 8.0, 0.0, 400.0, 100);
        let labels: Vec<(u32, u32)> = s
            .ticks
            .iter()
            .filter_map(|t| t.label.map(|l| (t.column, l)))
            .collect();
        assert_eq!(
            labels,
            [(0, 0), (10, 1), (20, 2), (30, 3), (40, 4), (50, 5)]
        );
    }

    #[test]
    fn the_wrap_column_has_a_tall_line_and_no_number() {
        let s = scale(13, 8.0, 0.0, 400.0, 12);
        let last = s.ticks.last().unwrap();
        assert_eq!((last.column, last.top, last.label), (12, 1.0, None));
        // and the scale ends there
        assert_eq!(s.base_to, 12.0 * 8.0 + 1.0);
        let s = scale(13, 8.0, 0.0, 400.0, 10);
        assert_eq!(s.ticks.last().unwrap().label, None);
    }

    #[test]
    fn the_scale_follows_the_scroll_and_skips_a_column_that_is_cut() {
        // scrolled by 20 pixels: column 2 (16) is cut, column 3 is at 4
        let s = scale(13, 8.0, 20.0, 100.0, 100);
        assert_eq!(s.ticks[0].column, 3);
        assert_eq!(s.ticks[0].x, 4.0);
        // scrolled by a whole number of columns
        let s = scale(13, 8.0, 16.0, 100.0, 100);
        assert_eq!((s.ticks[0].column, s.ticks[0].x), (2, 0.0));
    }

    #[test]
    fn the_scale_stops_at_the_right_edge() {
        let s = scale(13, 8.0, 0.0, 100.0, 1000);
        // 100 / 8 = 12.5: column 12 is at 96, column 13 at 104 (past 101)
        assert_eq!(s.ticks.last().unwrap().column, 12);
        assert_eq!(s.base_to, 100.0);
    }

    #[test]
    fn a_wide_scale_is_cheap_to_make() {
        let started = std::time::Instant::now();
        let s = scale(13, 8.0, 0.0, 2_000.0, 10_240);
        assert_eq!(s.ticks.len(), 251);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn a_ruler_that_is_very_low_still_has_a_scale() {
        let s = scale(1, 8.0, 0.0, 100.0, 10);
        assert_eq!(s.base_y, 1.0);
        assert_eq!(s.ticks.len(), 11);
        assert!(s.ticks.iter().all(|t| t.top >= 0.0));
    }

    #[test]
    fn the_caret_mark_is_a_column_wide_in_the_columns_of_the_caret() {
        let (x0, x1, y0, y1) = caret_mark(13, 3, 8.0, 0.0, 100.0).unwrap();
        assert_eq!((x0, x1, y0, y1), (25.0, 32.0, 0.0, 12.0));
        // scrolled
        assert_eq!(caret_mark(13, 3, 8.0, 8.0, 100.0).unwrap().0, 17.0);
    }

    #[test]
    fn the_caret_mark_is_not_drawn_off_the_screen() {
        assert_eq!(caret_mark(13, 3, 8.0, 80.0, 100.0), None);
        assert!(caret_mark(13, 12, 8.0, 0.0, 100.0).is_some());
        // up to two columns past the right edge (the caret at the end of a line)
        assert!(caret_mark(13, 14, 8.0, 0.0, 100.0).is_some());
        assert_eq!(caret_mark(13, 15, 8.0, 0.0, 100.0), None);
    }

    #[test]
    fn the_caret_mark_inverts_the_background() {
        assert_eq!(caret_color(0xefefef, true), 0x101010);
        assert_eq!(caret_color(0x000000, true), 0xffffff);
        // gray when the caret has no width
        assert_eq!(caret_color(0xefefef, false), 0x909090);
    }
}
