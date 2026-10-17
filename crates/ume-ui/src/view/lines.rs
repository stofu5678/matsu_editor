// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/view/CTextDrawer.cpp (DispVerticalLines, DispNoteLine)
//   sakura_core/view/CEditView.cpp (CaretUnderLineON, IsDrawCursorVLinePos)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, genta
//   Copyright (C) 2002, YAZAKI, aroka, MIK
//   Copyright (C) 2005, Moca
//   Copyright (C) 2006, ryoji, Moca
//   Copyright (C) 2007, kobake, ryoji
//   Copyright (C) 2008, ryoji, nasukoji
//   Copyright (C) 2009, ryoji
//   Copyright (C) 2010, ryoji
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

//! The lines Sakura draws over the text area, as arithmetic on pixels so that it is tested
//! without a window: the vertical lines at given columns (`VER`, 指定桁縦線), the note lines
//! (`NOT`, ノート線), the vertical line of the caret column (`CVL`) and the underline of the caret
//! line (`UND`).
//!
//! Every position is in pixels from the top left of the text area (right of the line numbers,
//! below the ruler); `scroll_x` is how far the text is scrolled left.

use ume_config::settings::type_rules::MAX_VERT_LINES;

/// The columns (counted from 1) of the vertical lines `idx` that can be on the screen: between
/// `first_col` and `last_col`, and not right of `max_columns`, the wrap column
/// (`DispVerticalLines`).
///
/// A positive entry is one line. A negative entry starts a repeating line: its absolute value is
/// the first column, the next entry is the last column and the one after it the step. The
/// entries after the first 0 are not used (the caller cuts the list there).
pub fn vertical_columns(idx: &[i32], first_col: u32, last_col: u32, max_columns: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut k = 0;
    while k < idx.len() {
        let entry = idx[k];
        let (start, end, step) = if entry < 0 {
            if k >= MAX_VERT_LINES - 2 {
                k += 3;
                continue;
            }
            let end = idx.get(k + 1).copied().unwrap_or(0);
            let step = idx.get(k + 2).copied().unwrap_or(0);
            k += 2;
            if end < -entry || step <= 0 {
                k += 1;
                continue;
            }
            (-entry, end, step)
        } else {
            (entry, entry, 1)
        };
        k += 1;
        let (start, end, step) = (start as u32, end as u32, step as u32);
        // Skip to the first line that is not left of the screen.
        let mut column = if start < first_col {
            start + (first_col - start).div_ceil(step) * step
        } else {
            start
        };
        while column <= end && column <= max_columns && column <= last_col {
            if column >= first_col {
                out.push(column);
            }
            column += step;
        }
    }
    out
}

/// The x of the vertical lines, from the left of the text area: the lines of
/// [`vertical_columns`] for the screen, each at the left of its column moved by `offset` pixels
/// (`m_nVertLineOffset`, -1 by default: the last pixel of the column before), cut at the edges of
/// the text area and at the wrap column. A bold line has a second line left of the first.
pub fn vertical_xs(
    idx: &[i32],
    cell: f32,
    scroll_x: f32,
    view_width: f32,
    max_columns: u32,
    offset: i32,
    bold: bool,
) -> Vec<f32> {
    let first = (scroll_x / cell).floor().max(0.0) as u32;
    let last = ((scroll_x + view_width) / cell).ceil() as u32 + 1;
    // `nPosXRight`: the text area's right edge, or the wrap column.
    let right = view_width.min(max_columns as f32 * cell - scroll_x);
    let mut out = Vec::new();
    for column in vertical_columns(idx, first, last, max_columns) {
        let x = (offset as f32 + (column - 1) as f32 * cell - scroll_x).round();
        let bold_x = if bold { x - 1.0 } else { x };
        if right <= bold_x {
            break;
        }
        if x >= 0.0 {
            if x < right {
                out.push(x);
            }
            if bold && bold_x >= 0.0 {
                out.push(bold_x);
            }
        }
    }
    out
}

/// The y of a note line inside a row, from the top of the row: the lines go every row, and
/// `offset` (`nNoteLineOffset`) moves them down. With 0 the line is on the last pixel of the row
/// (`DispNoteLine`: the lines are `areaTop + offset - 1` plus whole rows).
pub fn note_line_y(offset: i32, line_height: f32) -> f32 {
    let height = line_height.round().max(1.0);
    ((offset - 1) as f32).rem_euclid(height)
}

/// The x of the vertical line of the caret column, from the left of the text area, if it is
/// drawn (`CaretUnderLineON`, `IsDrawCursorVLinePos`): one pixel left of the caret so that it
/// does not cover it. Not drawn in the line number area or right of the text area.
pub fn cursor_vline_x(caret_col: u32, cell: f32, scroll_x: f32, view_width: f32) -> Option<f32> {
    let x = (caret_col as f32 * cell - scroll_x - 1.0).round();
    in_text_area(x, view_width).then_some(x)
}

/// `IsDrawCursorVLinePos`: the line has to be right of the left edge of the text area
/// (`posX > areaLeft - m_nLineNumRightSpace`, the space being 0 by default, which also covers
/// `posX >= areaLeft - 2`) and not past the right edge.
fn in_text_area(x: f32, view_width: f32) -> bool {
    x > 0.0 && x <= view_width
}

/// The x of the second pixel of a bold caret column line (left of the first), if it is drawn.
pub fn cursor_vline_bold_x(x: f32, view_width: f32) -> Option<f32> {
    in_text_area(x - 1.0, view_width).then_some(x - 1.0)
}

/// The y of the underline of the caret row, inside the row: its last pixel
/// (`areaTop + row * lineHeight + fontHeight`, where the font is one pixel shorter than the row).
pub fn underline_y(line_height: f32) -> f32 {
    (line_height.round() - 1.0).max(0.0)
}

/// `R2_NOTXORPEN` with `pen` over `dest`: the color of a line drawn with a color equal to the
/// background of the text, so that it shows (`bExorPen` in `DispVerticalLines`). Colors are
/// `0xRRGGBB`.
pub fn exor_color(dest: u32, pen: u32) -> u32 {
    !(dest ^ pen) & 0x00ff_ffff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_column_is_a_line_when_it_is_on_the_screen() {
        assert_eq!(vertical_columns(&[10, 20], 0, 100, 100), [10, 20]);
        // left of the screen, right of it, past the wrap column
        assert_eq!(vertical_columns(&[10, 20, 30], 15, 25, 100), [20]);
        assert_eq!(vertical_columns(&[10, 200], 0, 300, 100), [10]);
    }

    #[test]
    fn a_negative_column_repeats_up_to_the_end_by_the_step() {
        // -10, 40, 10: columns 10, 20, 30, 40
        assert_eq!(
            vertical_columns(&[-10, 40, 10], 0, 100, 100),
            [10, 20, 30, 40]
        );
        // the entries after it are lines of their own
        assert_eq!(
            vertical_columns(&[-10, 30, 10, 50], 0, 100, 100),
            [10, 20, 30, 50]
        );
        // a repeat that starts left of the screen keeps its steps
        assert_eq!(vertical_columns(&[-10, 60, 10], 25, 45, 100), [30, 40]);
        assert_eq!(vertical_columns(&[-3, 20, 7], 5, 100, 100), [10, 17]);
    }

    #[test]
    fn a_repeat_that_is_not_complete_is_left_out() {
        // end before start, no step, a step of 0, a list that ends too early
        assert_eq!(vertical_columns(&[-10, 5, 1, 50], 0, 100, 100), [50]);
        assert_eq!(vertical_columns(&[-10, 50, 0, 60], 0, 100, 100), [60]);
        assert!(vertical_columns(&[-10], 0, 100, 100).is_empty());
        assert!(vertical_columns(&[-10, 50], 0, 100, 100).is_empty());
    }

    #[test]
    fn a_repeat_stops_at_the_wrap_column() {
        assert_eq!(vertical_columns(&[-1, 10, 1], 0, 100, 4), [1, 2, 3, 4]);
    }

    #[test]
    fn a_repeat_over_a_wide_range_is_quick() {
        let started = std::time::Instant::now();
        let columns = vertical_columns(&[-1, 10_240, 1], 5_000, 5_100, 10_240);
        assert_eq!(columns.len(), 101);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn the_line_is_on_the_last_pixel_of_the_column_before_by_default() {
        // column 5 starts at 32 (cell 8): the line is at 31
        assert_eq!(vertical_xs(&[5], 8.0, 0.0, 400.0, 100, -1, false), [31.0]);
        // moved right by 2
        assert_eq!(vertical_xs(&[5], 8.0, 0.0, 400.0, 100, 2, false), [34.0]);
        // scrolled left by 16 pixels
        assert_eq!(vertical_xs(&[5], 8.0, 16.0, 400.0, 100, -1, false), [15.0]);
    }

    #[test]
    fn a_line_left_of_the_text_area_or_past_its_right_edge_is_not_drawn() {
        // column 1 is at -1 with the default offset
        assert!(vertical_xs(&[1], 8.0, 0.0, 400.0, 100, -1, false).is_empty());
        assert!(vertical_xs(&[60], 8.0, 0.0, 400.0, 100, -1, false).is_empty());
        // the wrap column: a line at 40 + 1 columns
        assert_eq!(vertical_xs(&[10], 8.0, 0.0, 400.0, 10, -1, false), [71.0]);
        assert!(vertical_xs(&[12], 8.0, 0.0, 400.0, 10, -1, false).is_empty());
    }

    #[test]
    fn a_bold_line_has_a_second_pixel_on_its_left() {
        assert_eq!(
            vertical_xs(&[5], 8.0, 0.0, 400.0, 100, -1, true),
            [31.0, 30.0]
        );
        // at the left edge the second pixel is outside
        assert_eq!(vertical_xs(&[1], 8.0, 0.0, 400.0, 100, 1, true), [1.0, 0.0]);
        assert_eq!(vertical_xs(&[1], 8.0, 0.0, 400.0, 100, 0, true), [0.0]);
    }

    #[test]
    fn note_lines_are_on_the_last_pixel_of_a_row_unless_moved() {
        assert_eq!(note_line_y(0, 18.0), 17.0);
        // moved down by 1: the first pixel of the next row's... the first pixel of the row
        assert_eq!(note_line_y(1, 18.0), 0.0);
        assert_eq!(note_line_y(5, 18.0), 4.0);
        // a negative offset moves up, round the rows
        assert_eq!(note_line_y(-3, 18.0), 14.0);
        assert_eq!(note_line_y(18, 18.0), 17.0);
    }

    #[test]
    fn the_caret_column_line_is_left_of_the_caret() {
        // caret at column 3 (cell 8): the caret is at 24, the line at 23
        assert_eq!(cursor_vline_x(3, 8.0, 0.0, 400.0), Some(23.0));
        assert_eq!(cursor_vline_x(3, 8.0, 8.0, 400.0), Some(15.0));
    }

    #[test]
    fn the_caret_column_line_is_not_drawn_at_the_first_column_or_off_the_view() {
        // column 0: x = -1, the gap of the line numbers
        assert_eq!(cursor_vline_x(0, 8.0, 0.0, 400.0), None);
        // scrolled away to the left
        assert_eq!(cursor_vline_x(3, 8.0, 100.0, 400.0), None);
        // right of the text area
        assert_eq!(cursor_vline_x(100, 8.0, 0.0, 400.0), None);
        assert_eq!(cursor_vline_bold_x(1.0, 400.0), None);
        assert_eq!(cursor_vline_bold_x(23.0, 400.0), Some(22.0));
    }

    #[test]
    fn the_underline_is_on_the_last_pixel_of_the_row() {
        assert_eq!(underline_y(18.0), 17.0);
        assert_eq!(underline_y(18.4), 17.0);
    }

    #[test]
    fn a_line_in_the_color_of_the_background_shows_by_xor() {
        // NOT (dest XOR pen)
        assert_eq!(exor_color(0xfffbf0, 0xc0c0c0), 0xc0c4cf);
        assert_eq!(exor_color(0xffffff, 0xffffff), 0xffffff);
        assert_eq!(exor_color(0xffffff, 0x000000), 0x000000);
    }
}
