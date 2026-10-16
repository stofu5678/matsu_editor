// Parts of this file are derived from Sakura Editor (https://github.com/sakura-editor/sakura,
// commit 48db6c7c25e2):
//   sakura_core/docplus/CDiffManager.cpp (CDiffLineGetter::DrawDiffMark)
//
// Copyright (C) 2002, MIK
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the strokes that Sakura draws with
// `MoveToEx` and `LineTo` of GDI are worked out as the pixels they cover, so that the view can
// paint them as small squares, and the tests are new.
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

//! The glyphs of the DIFF marks in the line number area: a plus for an added line, a short dashed
//! stroke for a changed one, and a little arrow for the place where lines of the other text are
//! missing (pointing down; up from the bottom of the last line when they were after it).
//!
//! GDI draws a stroke from the first point up to, but not including, the last one, with a pen
//! one pixel wide; the pixels here are the same.

use ume_core::diff::DiffMark;

/// A stroke through points (x relative to the left edge of the area, y absolute).
type Stroke = Vec<(i32, i32)>;

/// The strokes of a mark on a row whose vertical center is `cy` (`DrawDiffMark`).
fn strokes(mark: DiffMark, cy: i32) -> Vec<Stroke> {
    match mark {
        DiffMark::Append => vec![vec![(3, cy), (6, cy)], vec![(4, cy - 2), (4, cy + 3)]],
        DiffMark::Change => vec![vec![(3, cy - 4), (3, cy)], vec![(3, cy + 2), (3, cy + 3)]],
        DiffMark::Delete => {
            let cy = cy - 3;
            vec![vec![(3, cy), (5, cy), (3, cy + 2), (3, cy), (7, cy + 4)]]
        }
        DiffMark::DeleteEx => {
            let cy = cy + 3;
            vec![vec![(3, cy), (5, cy), (3, cy - 2), (3, cy), (7, cy - 4)]]
        }
    }
}

/// The pixels of the line from `a` up to, not including, `b` (Bresenham's algorithm).
fn line_pixels(a: (i32, i32), b: (i32, i32), out: &mut Vec<(i32, i32)>) {
    let (mut x, mut y) = a;
    let (dx, dy) = ((b.0 - a.0).abs(), -(b.1 - a.1).abs());
    let (sx, sy) = ((b.0 - a.0).signum(), (b.1 - a.1).signum());
    let mut err = dx + dy;
    while (x, y) != b {
        out.push((x, y));
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// The pixels of a mark: `(x, y)` with `x` from the left edge of the area and `y` from the top of
/// the row. `row_height` is the height of the row.
pub(super) fn pixels(mark: DiffMark, row_height: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for stroke in strokes(mark, row_height / 2) {
        for pair in stroke.windows(2) {
            line_pixels(pair[0], pair[1], &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(mark: DiffMark, height: i32) -> Vec<(i32, i32)> {
        let mut p = pixels(mark, height);
        p.sort_unstable();
        p.dedup();
        p
    }

    #[test]
    fn an_added_line_has_a_plus() {
        // center 10: the bar (3..6, 10) and the stem (4, 8..13)
        let p = sorted(DiffMark::Append, 20);
        assert_eq!(
            p,
            [(3, 10), (4, 8), (4, 9), (4, 10), (4, 11), (4, 12), (5, 10)]
        );
    }

    #[test]
    fn a_changed_line_has_a_dashed_stroke() {
        let p = sorted(DiffMark::Change, 20);
        assert_eq!(p, [(3, 6), (3, 7), (3, 8), (3, 9), (3, 12)]);
    }

    #[test]
    fn missing_lines_have_an_arrow_pointing_at_the_gap() {
        // an arrow head and a diagonal, above the center for a gap before a line
        let p = sorted(DiffMark::Delete, 20);
        assert!(p.contains(&(3, 7)) && p.contains(&(4, 7)));
        assert!(p.iter().all(|&(_, y)| (7..=11).contains(&y)), "{p:?}");
        // below the center, and upside down, for lines missing after the last line
        let q = sorted(DiffMark::DeleteEx, 20);
        assert!(q.contains(&(3, 13)) && q.contains(&(4, 13)));
        assert!(q.iter().all(|&(_, y)| (9..=13).contains(&y)), "{q:?}");
        assert_eq!(p.len(), q.len());
    }

    #[test]
    fn the_marks_stay_in_the_first_eight_pixels() {
        for mark in [
            DiffMark::Append,
            DiffMark::Change,
            DiffMark::Delete,
            DiffMark::DeleteEx,
        ] {
            assert!(pixels(mark, 20).iter().all(|&(x, _)| (3..8).contains(&x)));
        }
    }
}
