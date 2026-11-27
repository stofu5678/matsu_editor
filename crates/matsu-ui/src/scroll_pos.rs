//! The vertical position of a view: the row at the top and how many pixels of it are cut off.
//!
//! An `f32` has 24 bits of precision. A position kept as absolute pixels is exact only up to about
//! 2^24 px (some 500,000 rows); at 2x10^8 rows a step of the `f32` is about 16 rows, and the end of
//! such a file could not be reached (ADR-0020, chapter 6: the view keeps its place as a row, not as
//! pixels). So the owner of the position ([`VScroll`]) keeps the top row as a `usize`; only the
//! pixels inside that row, and pixels relative to it (a screen's height), are `f32`. Where a
//! fraction of the whole document is needed (the scroll bar), the rows are an `f64`, which is exact
//! far beyond any row count.

/// The top of the view: row `row`, scrolled `offset` pixels into it. The offset is kept from 0 up
/// to (not including) the height of a row, so that equal positions are equal values; whoever makes
/// a position gives the row height it was made for.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct VScroll {
    row: usize,
    offset: f32,
}

/// A row height that can divide: a broken metric must not make a position `NaN`.
fn height(line_height: f32) -> f32 {
    if line_height.is_finite() && line_height > 0.0 {
        line_height
    } else {
        1.0
    }
}

impl VScroll {
    /// The first row, not scrolled.
    pub const TOP: VScroll = VScroll {
        row: 0,
        offset: 0.0,
    };

    /// The top of the view is exactly at the start of `row`.
    pub fn at_row(row: usize) -> Self {
        Self { row, offset: 0.0 }
    }

    /// `offset` pixels below the start of `row`: an offset out of the row moves to the row it is
    /// in, and what is above the first row is the first row.
    pub fn new(row: usize, offset: f32, line_height: f32) -> Self {
        let lh = height(line_height);
        if offset.is_nan() {
            return Self::at_row(row);
        }
        let mut steps = (offset / lh).floor() as i64;
        let mut offset = offset - steps as f32 * lh;
        if !offset.is_finite() {
            offset = 0.0;
        }
        // (The subtraction can leave the offset a little out of its row.)
        if offset < 0.0 {
            offset += lh;
            steps = steps.saturating_sub(1);
        }
        if offset >= lh {
            offset = 0.0;
            steps = steps.saturating_add(1);
        }
        let row = if steps >= 0 {
            row.saturating_add(steps as usize)
        } else {
            match row.checked_sub(steps.unsigned_abs() as usize) {
                Some(row) => row,
                None => return Self::TOP,
            }
        };
        Self { row, offset }
    }

    /// `px` pixels from the top of a document that is small enough for pixels (a few hundred
    /// thousand rows): for the tests and for what is given in pixels by a person.
    pub fn from_px(px: f32, line_height: f32) -> Self {
        Self::new(0, px.max(0.0), line_height)
    }

    /// `rows` rows from the top, with the fraction of a row as pixels.
    pub fn from_rows(rows: f64, line_height: f32) -> Self {
        if rows.is_nan() || rows <= 0.0 {
            return Self::TOP;
        }
        let whole = rows.floor();
        let row = if whole >= usize::MAX as f64 {
            usize::MAX
        } else {
            whole as usize
        };
        Self::new(
            row,
            ((rows - whole) * f64::from(line_height)) as f32,
            line_height,
        )
    }

    /// The row at the top (the first one that is shown, at least in part).
    pub fn row(self) -> usize {
        self.row
    }

    /// The pixels of [`row`](Self::row) that are above the view.
    pub fn offset(self) -> f32 {
        self.offset
    }

    /// Whether the view is at the very top.
    pub fn is_top(self) -> bool {
        self.row == 0 && self.offset <= 0.0
    }

    /// The row nearest to the top (Sakura's `GetViewTopLine`): the next one from half a row on.
    pub fn nearest_row(self, line_height: f32) -> usize {
        if self.offset / height(line_height) >= 0.5 {
            self.row.saturating_add(1)
        } else {
            self.row
        }
    }

    /// The position as a number of rows, exact far beyond any row count.
    pub fn rows(self, line_height: f32) -> f64 {
        self.row as f64 + f64::from(self.offset) / f64::from(height(line_height))
    }

    /// The position in pixels, exact only for a document of few rows. Not for painting or for
    /// hit testing: those use [`y_of_row`](Self::y_of_row) and [`row_at`](Self::row_at).
    #[cfg(test)]
    pub fn px(self, line_height: f32) -> f32 {
        self.row as f32 * line_height + self.offset
    }

    /// The position in pixels as an `f64`: exact to the pixel for any document that has fewer than
    /// 2^53 pixels (the background image of the view is placed by it).
    pub fn px_f64(self, line_height: f32) -> f64 {
        self.row as f64 * f64::from(line_height) + f64::from(self.offset)
    }

    /// The same offset into another row.
    pub fn with_row(self, row: usize, line_height: f32) -> Self {
        Self::new(row, self.offset, line_height)
    }

    /// `dy` pixels further down (up if negative), not above the first row. For a step of the
    /// wheel or of a drag: `dy` is at most some screens.
    pub fn shifted_px(self, dy: f32, line_height: f32) -> Self {
        Self::new(self.row, self.offset + dy, line_height)
    }

    /// `rows` rows further down (up if negative), not above the first row.
    pub fn shifted_rows(self, rows: isize) -> Self {
        let row = if rows >= 0 {
            self.row.saturating_add(rows as usize)
        } else {
            match self.row.checked_sub(rows.unsigned_abs()) {
                Some(row) => row,
                None => return Self::TOP,
            }
        };
        Self { row, ..self }
    }

    /// The row at `y` pixels below the top of the text area, or `None` above the first row.
    pub fn row_at(self, y: f32, line_height: f32) -> Option<usize> {
        let steps = ((self.offset + y) / height(line_height)).floor();
        if steps.is_nan() {
            return None;
        }
        let steps = steps as i64;
        if steps >= 0 {
            Some(self.row.saturating_add(steps as usize))
        } else {
            self.row.checked_sub(steps.unsigned_abs() as usize)
        }
    }

    /// Where the top of `row` is, in pixels below the top of the text area (negative above it).
    /// Only the rows near the top are given to anything that paints, so the `f32` is exact enough.
    pub fn y_of_row(self, row: usize, line_height: f32) -> f32 {
        let rows = if row >= self.row {
            (row - self.row) as f32
        } else {
            -((self.row - row) as f32)
        };
        rows * line_height - self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LH: f32 = 20.0;

    #[test]
    fn an_offset_that_leaves_its_row_moves_to_the_row_it_is_in() {
        assert_eq!(VScroll::new(3, 45.0, LH), VScroll::new(5, 5.0, LH));
        assert_eq!(VScroll::new(3, -5.0, LH), VScroll::new(2, 15.0, LH));
        assert_eq!(VScroll::new(0, -5.0, LH), VScroll::TOP);
        assert_eq!(VScroll::new(2, -100.0, LH), VScroll::TOP);
        assert_eq!(VScroll::new(3, 20.0, LH).row(), 4);
        assert_eq!(VScroll::from_px(70.0, LH), VScroll::new(3, 10.0, LH));
        assert_eq!(VScroll::from_px(-70.0, LH), VScroll::TOP);
    }

    #[test]
    fn the_nearest_row_rounds_half_up() {
        assert_eq!(VScroll::new(7, 9.9, LH).nearest_row(LH), 7);
        assert_eq!(VScroll::new(7, 10.0, LH).nearest_row(LH), 8);
    }

    #[test]
    fn rows_stay_exact_at_two_hundred_million() {
        let n = 200_000_000usize;
        let at = VScroll::at_row(n - 1);
        assert_eq!(at.shifted_rows(1).row(), n);
        assert_eq!(at.shifted_rows(-1).row(), n - 2);
        // the pixel of a row near the top is relative to it, whatever the row
        assert_eq!(at.y_of_row(n + 3, 31.0), 4.0 * 31.0);
        assert_eq!(at.y_of_row(n - 2, 31.0), -31.0);
        let half = VScroll::new(n, 7.0, 31.0);
        assert_eq!(half.row_at(31.0 * 2.0, 31.0), Some(n + 2));
        assert_eq!(half.row_at(-8.0, 31.0), Some(n - 1));
        assert_eq!(VScroll::from_rows(n as f64 + 0.5, 30.0).row(), n);
        assert_eq!(VScroll::from_rows(n as f64 + 0.5, 30.0).offset(), 15.0);
    }

    #[test]
    fn rows_above_the_first_are_none() {
        assert_eq!(VScroll::TOP.row_at(-1.0, LH), None);
        assert_eq!(VScroll::TOP.row_at(0.0, LH), Some(0));
        // the top is 25 pixels down: 6 above the area is still row 0, 26 above is none
        assert_eq!(VScroll::new(1, 5.0, LH).row_at(-6.0, LH), Some(0));
        assert_eq!(VScroll::new(1, 5.0, LH).row_at(-26.0, LH), None);
        assert_eq!(VScroll::new(1, 5.0, LH).row_at(-5.0, LH), Some(1));
    }

    #[test]
    fn order_follows_the_position() {
        assert!(VScroll::new(2, 5.0, LH) > VScroll::at_row(2));
        assert!(VScroll::at_row(3) > VScroll::new(2, 19.0, LH));
    }
}
