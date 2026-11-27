//! Scrolling and hit-testing arithmetic in plain `f32` pixels, so that it can be tested without a
//! window. `EditorView` calls these with the sizes it measured at the last paint.

use matsu_layout::X;

use crate::scroll_pos::VScroll;

/// The width of the vertical scroll bar.
pub const SCROLLBAR_WIDTH: f32 = 14.0;

/// The height of the horizontal scroll bar, and the side of its arrow buttons. The corner where
/// the two bars meet is a square of this size (where Sakura has the size box).
pub const HSCROLLBAR_HEIGHT: f32 = SCROLLBAR_WIDTH;

/// How many columns an arrow of the horizontal scroll bar moves (`CEditView::OnHScroll`:
/// `GetLayoutXDefault(CKetaXInt(4))`).
pub const HSCROLL_LINE_COLUMNS: f32 = 4.0;

/// The width of an arrow button of the horizontal scroll bar.
pub const HSCROLL_ARROW: f32 = HSCROLLBAR_HEIGHT;

/// The height of an arrow button of the vertical scroll bar (`SM_CYVSCROLL`).
pub const VSCROLL_ARROW: f32 = SCROLLBAR_WIDTH;

/// The width of a split box (`CSplitBoxWnd`: `cxEdge * 2 + cxBorder * 3`, with the 2 and 1 pixel
/// edges of 100% scaling), and the room it takes from the bar it sits on.
pub const SPLIT_BOX: f32 = 7.0;

/// What the last paint measured. Before the first paint [`Metrics::default`] stands in, so that
/// keys and the mouse work (and tests run) without a window that has been drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// The width of a half-width character, which is one column of the layout.
    pub cell_width: f32,
    pub line_height: f32,
    /// The size of the text area: the view without the line numbers and the scroll bar.
    pub view_width: f32,
    pub view_height: f32,
    /// The room under the vertical scroll bar that is not the text area: the corner square when
    /// the view has no horizontal bar (`CEditView::OnSize` puts the vertical bar `nCyVScroll`
    /// short of the bottom whatever the horizontal bar does).
    pub vbar_gap: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            cell_width: 8.0,
            line_height: 20.0,
            view_width: 800.0,
            view_height: 600.0,
            vbar_gap: 0.0,
        }
    }
}

impl Metrics {
    /// How many rows fit in the view, at least one.
    pub fn visible_rows(&self) -> usize {
        ((self.view_height / self.line_height).floor() as usize).max(1)
    }
}

/// How many `X` a half-width character without spacing is: the layout of the view counts in 1/64 of
/// a half-width column (ADR-0034, case C).
pub const X_PER_HALF_WIDTH: u32 = 64;

/// `X` in one column of a view whose half-width is `half` pixels and whose columns have `space`
/// pixels of character spacing: 64 plus the spacing in the same unit
/// (`space_units = round(space * 64 / half)`, ADR-0034, case C). It is exactly 64 without spacing.
pub fn per_column_for(half: f32, space: u32) -> u32 {
    if space == 0 || half <= 0.0 {
        return X_PER_HALF_WIDTH;
    }
    let units = (space as f32 * X_PER_HALF_WIDTH as f32 / half).round();
    X_PER_HALF_WIDTH.saturating_add(units.clamp(0.0, 1.0e6) as u32)
}

/// The pixels of the layout's `X`: a column is `cell` pixels and `per_column` `X`.
///
/// This is the one place where the pixels of the text area and `X` are converted. A position is
/// `(whole columns) * cell + (the rest) * (cell / per_column)`, so that a position on a column edge
/// is `columns * cell` as it was when the layout counted columns, whatever `per_column` is (a
/// product with the quotient `cell / per_column` would be a little off when `per_column` is not a
/// power of two, the layout with character spacing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XScale {
    cell: f32,
    per_column: u32,
}

impl XScale {
    pub fn new(cell: f32, per_column: u32) -> Self {
        Self {
            cell,
            per_column: per_column.max(1),
        }
    }

    /// The width of a column in pixels.
    pub fn cell(&self) -> f32 {
        self.cell
    }

    /// The pixels from the text origin to `x`, or the width in pixels of `x`.
    pub fn px(&self, x: X) -> f32 {
        let raw = x.raw();
        let per = self.per_column;
        (raw / per) as f32 * self.cell + (raw % per) as f32 * (self.cell / per as f32)
    }

    /// The `X` of the column edge at or before the pixel `px` (a negative pixel is the start).
    pub fn column_start(&self, px: f32) -> X {
        let columns = (px / self.cell).floor().max(0.0);
        X::from_raw((columns as u32).saturating_mul(self.per_column))
    }

    /// The `X` of the nearest column edge to the pixel `px`: where a box selection puts a corner.
    pub fn nearest_column_edge(&self, px: f32) -> X {
        let columns = (px / self.cell + 0.5).floor().max(0.0);
        X::from_raw((columns as u32).saturating_mul(self.per_column))
    }

    /// The `X` at the pixel `px`, rounded down (a negative pixel is the start).
    pub fn x_at(&self, px: f32) -> X {
        let cells = (px / self.cell).max(0.0);
        let columns = cells.floor();
        let rest = ((cells - columns) * self.per_column as f32).floor() as u32;
        X::from_raw(
            (columns as u32)
                .saturating_mul(self.per_column)
                .saturating_add(rest.min(self.per_column - 1)),
        )
    }

    /// `px` as a fractional number of `X` (for finding the nearer edge of a character).
    pub fn x_f32(&self, px: f32) -> f32 {
        (px / self.cell).max(0.0) * self.per_column as f32
    }

    /// `columns` columns in pixels (the settings of the type count columns: the ruler, the lines).
    pub fn columns_px(&self, columns: u32) -> f32 {
        columns as f32 * self.cell
    }
}

/// The width of a column: the half-width of the font and the spacing of the type
/// (`CTextMetrics::Update`: `m_nDxBasis = m_nCharWidth + nColmSpace`).
pub fn column_width(half_width: f32, column_space: u32) -> f32 {
    half_width + column_space as f32
}

/// The height of a row: the height of the font's line and the spacing of the type, which can be
/// negative (`CTextMetrics::Update`: `m_nDyBasis = max(1, sz.cy + nLineSpace)`).
pub fn row_height(font_height: f32, line_space: i32) -> f32 {
    (font_height + line_space as f32).max(1.0)
}

/// The lowest top of the view: the last row ends at the bottom of the view, plus one blank row (the
/// PoC's rule, so that the last row is never hidden under the edge of the window).
pub fn max_scroll(m: &Metrics, total_rows: usize) -> VScroll {
    VScroll::at_row(total_rows.saturating_sub(m.visible_rows() - 1))
}

/// [`max_scroll`] in pixels, for the tests of small documents.
#[cfg(test)]
pub fn max_scroll_y(m: &Metrics, total_rows: usize) -> f32 {
    max_scroll(m, total_rows).px(m.line_height)
}

/// `scroll` kept between the first row and [`max_scroll`].
pub fn clamp_scroll(m: &Metrics, total_rows: usize, scroll: VScroll) -> VScroll {
    let max = max_scroll(m, total_rows);
    if scroll > max { max } else { scroll }
}

/// The top of the view that shows `row` completely, changing `scroll` as little as possible.
pub fn scroll_to_show_row(m: &Metrics, scroll: VScroll, row: usize) -> VScroll {
    if row < scroll.row() || (row == scroll.row() && scroll.offset() > 0.0) {
        // the row starts above the view
        return VScroll::at_row(row);
    }
    // The row ends below the view: its end is more than `offset` below the last full row.
    let visible = m.visible_rows();
    if row.saturating_add(1) > scroll.row().saturating_add(visible) {
        return VScroll::at_row((row + 1).saturating_sub(visible));
    }
    scroll
}

/// The horizontal scroll offset that shows the pixels `from..to` (relative to the text origin),
/// changing `scroll_x` as little as possible. The caret is `cell_width` wide, so a caret at the
/// end of a line does not touch the edge.
pub fn scroll_x_to_show(m: &Metrics, scroll_x: f32, from: f32, to: f32) -> f32 {
    let to = to + m.cell_width;
    if from < scroll_x {
        from.max(0.0)
    } else if to > scroll_x + m.view_width {
        (to - m.view_width).max(0.0)
    } else {
        scroll_x
    }
}

/// The row at `y` pixels below the top of the text area, clamped to the existing rows.
pub fn row_at(m: &Metrics, scroll: VScroll, y: f32, total_rows: usize) -> usize {
    let row = scroll.row_at(y, m.line_height).unwrap_or(0);
    row.min(total_rows.saturating_sub(1))
}

/// The column that contains `x` pixels right of the text origin (`scroll_x` already added by the
/// caller), and how far into the column `x` is, from 0 up to 1.
pub fn column_at(m: &Metrics, x: f32) -> (u32, f32) {
    let cells = (x / m.cell_width).max(0.0);
    (cells.floor() as u32, cells.fract())
}

/// The parts of a scroll bar that a press can land on, in the order along the bar (`SB_LINEUP`,
/// `SB_PAGEUP`, the thumb, `SB_PAGEDOWN`, `SB_LINEDOWN`; the left and right ones of the horizontal
/// bar are the same with `LEFT` and `RIGHT`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarPart {
    LineBack,
    PageBack,
    Thumb,
    PageForward,
    LineForward,
}

impl BarPart {
    /// The part at `pos` pixels from the start of a bar `len` long whose arrows are `arrow` long,
    /// with the thumb at `thumb_start` for `thumb_len` along the track between the arrows.
    pub fn at(pos: f32, len: f32, arrow: f32, thumb_start: f32, thumb_len: f32) -> BarPart {
        if pos < arrow {
            return BarPart::LineBack;
        }
        if pos >= len - arrow {
            return BarPart::LineForward;
        }
        let on_track = pos - arrow;
        if on_track < thumb_start {
            BarPart::PageBack
        } else if on_track <= thumb_start + thumb_len {
            BarPart::Thumb
        } else {
            BarPart::PageForward
        }
    }
}

/// The vertical scroll bar: an arrow button at each end, between them the track, and on it the
/// thumb that shows how much of the document is in view. The track is the bar without its two
/// arrows, and `thumb_top` is counted from its top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scrollbar {
    pub track_height: f32,
    pub thumb_top: f32,
    pub thumb_height: f32,
}

impl Scrollbar {
    /// The length of the whole bar, the arrows included.
    pub fn bar_length(m: &Metrics) -> f32 {
        (m.view_height - m.vbar_gap).max(0.0)
    }

    fn track_for(m: &Metrics) -> f32 {
        (Self::bar_length(m) - 2.0 * VSCROLL_ARROW).max(0.0)
    }

    /// The part of the bar at `pos` pixels from its top.
    pub fn part_at(&self, m: &Metrics, pos: f32) -> BarPart {
        BarPart::at(
            pos,
            Self::bar_length(m),
            VSCROLL_ARROW,
            self.thumb_top,
            self.thumb_height,
        )
    }

    pub fn new(m: &Metrics, total_rows: usize, scroll: VScroll) -> Self {
        let track = Self::track_for(m);
        // In rows and as `f64`: pixels of a document of 10^8 rows do not fit an `f32`.
        let max_rows = max_scroll(m, total_rows).row() as f64;
        let max = max_rows * f64::from(m.line_height);
        let thumb_height = (f64::from(track)
            * (f64::from(m.view_height) / (max + f64::from(m.view_height))))
            as f32;
        let thumb_height = thumb_height.max(24.0).min(track);
        // The top may be past the range (`EditorView::move_view_top_line`).
        let ratio = if max_rows > 0.0 {
            (scroll.rows(m.line_height) / max_rows).min(1.0)
        } else {
            0.0
        };
        Self {
            track_height: track,
            thumb_top: (f64::from(track - thumb_height) * ratio) as f32,
            thumb_height,
        }
    }

    /// A scroll bar that covers a whole file the scan has not finished (ADR-0020, chapter 2):
    /// the thumb is `visible` (0 to 1) of the track long and sits `top` (0 to 1) of the way down,
    /// both worked out from byte positions rather than rows.
    pub fn spanning(m: &Metrics, visible: f32, top: f32) -> Self {
        let track = Self::track_for(m);
        let thumb_height = (track * visible.clamp(0.0, 1.0)).max(24.0).min(track);
        Self {
            track_height: track,
            thumb_top: (track - thumb_height) * top.clamp(0.0, 1.0),
            thumb_height,
        }
    }

    /// How far down the file (0 to 1) a thumb is whose top is `thumb_top` pixels from the top of
    /// the track: the inverse of the `top` of [`Scrollbar::spanning`].
    pub fn position_of(&self, thumb_top: f32) -> f32 {
        let travel = self.track_height - self.thumb_height;
        if travel <= 0.0 {
            0.0
        } else {
            (thumb_top / travel).clamp(0.0, 1.0)
        }
    }

    /// The top of the view for a thumb dragged by `dy` pixels from where it was at `start`.
    pub fn scroll_for_drag(
        &self,
        m: &Metrics,
        total_rows: usize,
        start: VScroll,
        dy: f32,
    ) -> VScroll {
        let travel = self.track_height - self.thumb_height;
        if travel <= 0.0 {
            return start;
        }
        let max_rows = max_scroll(m, total_rows).row() as f64;
        let rows = start.rows(m.line_height) + max_rows * (f64::from(dy) / f64::from(travel));
        clamp_scroll(m, total_rows, VScroll::from_rows(rows, m.line_height))
    }
}

/// The horizontal scroll bar: two arrow buttons around a track, and a thumb on the track. It is
/// worked out in columns, as Sakura's `SetScrollInfo` call is (`CEditView::AdjustScrollBars`):
/// the range is `0..range`, the thumb is `page` of them wide, and its position is the left
/// column of the view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HScrollbar {
    /// The right edge of what can be scrolled to, in columns (`GetRightEdgeForScrollBar`).
    pub range: u32,
    /// The columns in view (`m_nViewColNum`), at most `range`.
    pub page: u32,
    /// The left column of the view (`GetViewLeftCol`), at most the last position.
    pub pos: f32,
    /// The room for the thumb: the bar without its two arrow buttons.
    pub track_width: f32,
    /// The thumb, from the start of the track.
    pub thumb_left: f32,
    pub thumb_width: f32,
}

impl HScrollbar {
    /// The smallest thumb, as the vertical bar has.
    const MIN_THUMB: f32 = 24.0;

    /// `bar_width` is the width of the whole bar.
    pub fn new(range: u32, page: u32, left_col: f32, bar_width: f32) -> Self {
        // `setScrollInfoIfNeeded`: the page and the position are kept inside the range.
        let range = range.max(1);
        let page = page.min(range);
        let track = (bar_width - 2.0 * HSCROLL_ARROW).max(0.0);
        let travel_cols = (range - page) as f32;
        let thumb_width = (track * (page as f32 / range as f32))
            .max(Self::MIN_THUMB)
            .min(track);
        let pos = left_col.clamp(0.0, travel_cols);
        let ratio = if travel_cols > 0.0 {
            pos / travel_cols
        } else {
            0.0
        };
        Self {
            range,
            page,
            pos,
            track_width: track,
            thumb_left: (track - thumb_width) * ratio,
            thumb_width,
        }
    }

    /// There is something to scroll to (`m_nViewColNum < GetRightEdgeForScrollBar()`); if not, the
    /// bar is drawn without a thumb.
    pub fn enabled(&self) -> bool {
        self.page < self.range
    }

    /// The part of the bar at `pos` pixels from its left end.
    pub fn part_at(&self, pos: f32) -> BarPart {
        BarPart::at(
            pos,
            self.track_width + 2.0 * HSCROLL_ARROW,
            HSCROLL_ARROW,
            self.thumb_left,
            self.thumb_width,
        )
    }

    /// The left column for a thumb dragged by `dx` pixels from where it was at `start_col`
    /// (`SB_THUMBTRACK`: a whole column).
    pub fn col_for_drag(&self, start_col: f32, dx: f32) -> f32 {
        let travel = self.track_width - self.thumb_width;
        if travel <= 0.0 || !self.enabled() {
            return start_col;
        }
        (start_col + (self.range - self.page) as f32 * (dx / travel))
            .round()
            .clamp(0.0, (self.range - self.page) as f32)
    }
}

/// The farthest the view can scroll right, as a left column (`CEditView::ScrollAtH`): the right
/// edge and the room for the hanging punctuation, less the columns in view. The bar itself stops
/// `overhang` columns short of this.
pub fn max_left_col(range: u32, overhang: u32, page: u32) -> f32 {
    (range + overhang).saturating_sub(page) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> Metrics {
        Metrics {
            cell_width: 10.0,
            line_height: 20.0,
            view_width: 200.0,
            view_height: 100.0, // 5 rows
            vbar_gap: 0.0,
        }
    }

    #[test]
    fn the_spacing_widens_columns_and_rows() {
        assert_eq!(column_width(8.0, 0), 8.0);
        assert_eq!(column_width(8.0, 3), 11.0);
        assert_eq!(row_height(20.0, 1), 21.0);
        assert_eq!(row_height(20.0, -5), 15.0);
        // the row is never less than one pixel high
        assert_eq!(row_height(20.0, -128), 1.0);
    }

    #[test]
    fn hit_testing_follows_the_spacing() {
        let spaced = Metrics {
            cell_width: column_width(8.0, 2),
            line_height: row_height(20.0, 4),
            ..m()
        };
        // columns are 10 pixels wide, so column 3 is 30 to 40 whatever the glyph
        assert_eq!(column_at(&spaced, 29.9).0, 2);
        assert_eq!(column_at(&spaced, 30.0).0, 3);
        assert_eq!(column_at(&spaced, 39.9).0, 3);
        assert_eq!(column_at(&spaced, 40.0).0, 4);
        // rows are 24 pixels high
        assert_eq!(row_at(&spaced, VScroll::TOP, 23.9, 10), 0);
        assert_eq!(row_at(&spaced, VScroll::TOP, 24.0, 10), 1);
        let tight = Metrics {
            line_height: row_height(20.0, -4),
            ..m()
        };
        assert_eq!(row_at(&tight, VScroll::TOP, 15.9, 10), 0);
        assert_eq!(row_at(&tight, VScroll::TOP, 16.0, 10), 1);
    }

    #[test]
    fn the_scroll_stops_one_row_after_the_last() {
        assert_eq!(max_scroll_y(&m(), 100), 20.0 * 96.0);
        assert_eq!(max_scroll_y(&m(), 3), 0.0);
        assert_eq!(
            clamp_scroll(&m(), 100, VScroll::from_px(1e9, 20.0)),
            max_scroll(&m(), 100)
        );
        assert_eq!(
            clamp_scroll(&m(), 100, VScroll::from_px(-5.0, 20.0)),
            VScroll::TOP
        );
    }

    #[test]
    fn showing_a_row_moves_the_scroll_as_little_as_possible() {
        let m = m();
        let show = |from: f32, row: usize| {
            scroll_to_show_row(&m, VScroll::from_px(from, 20.0), row).px(20.0)
        };
        assert_eq!(show(0.0, 2), 0.0);
        assert_eq!(show(0.0, 5), 20.0); // bottom edge to 120
        assert_eq!(show(200.0, 3), 60.0); // above: row on top
        assert_eq!(show(200.0, 12), 200.0); // rows 10..=14 are in view
        assert_eq!(show(200.0, 15), 220.0);
        // a row cut off at the top is brought to the top, one cut off at the bottom is not in view
        assert_eq!(show(205.0, 10), 200.0);
        assert_eq!(show(205.0, 14), 205.0); // its end is 5 pixels above the edge
        assert_eq!(show(205.0, 15), 220.0);
    }

    #[test]
    fn showing_a_column_keeps_the_caret_off_the_edge() {
        let m = m();
        assert_eq!(scroll_x_to_show(&m, 0.0, 50.0, 60.0), 0.0);
        // caret at pixel 190..200: needs one more cell to the right
        assert_eq!(scroll_x_to_show(&m, 0.0, 190.0, 190.0), 0.0);
        assert_eq!(scroll_x_to_show(&m, 0.0, 195.0, 195.0), 5.0);
        assert_eq!(scroll_x_to_show(&m, 100.0, 40.0, 50.0), 40.0);
        assert_eq!(scroll_x_to_show(&m, 100.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn hit_testing_maps_pixels_to_rows_and_columns() {
        let m = m();
        assert_eq!(row_at(&m, VScroll::TOP, 0.0, 10), 0);
        assert_eq!(row_at(&m, VScroll::TOP, 39.9, 10), 1);
        assert_eq!(row_at(&m, VScroll::from_px(30.0, 20.0), 10.0, 10), 2);
        assert_eq!(row_at(&m, VScroll::TOP, -50.0, 10), 0);
        assert_eq!(row_at(&m, VScroll::TOP, 9999.0, 10), 9);
        assert_eq!(column_at(&m, 25.0), (2, 0.5));
        assert_eq!(column_at(&m, -3.0), (0, 0.0));
    }

    #[test]
    fn a_bar_over_a_whole_file_maps_the_thumb_to_a_position_and_back() {
        let m = m();
        let bar = Scrollbar::spanning(&m, 0.001, 0.5);
        // a thumb that would be a sliver keeps its least length
        assert_eq!(bar.thumb_height, 24.0);
        assert!((bar.thumb_top - (bar.track_height - 24.0) / 2.0).abs() < 1e-3);
        assert!((bar.position_of(bar.thumb_top) - 0.5).abs() < 1e-5);
        assert_eq!(bar.position_of(-5.0), 0.0);
        assert_eq!(bar.position_of(1e6), 1.0);
        let top = Scrollbar::spanning(&m, 0.2, 0.0);
        assert_eq!((top.thumb_top, top.thumb_height), (0.0, 24.0f32.max(20.0)));
        let end = Scrollbar::spanning(&m, 0.2, 1.0);
        assert!((end.thumb_top + end.thumb_height - end.track_height).abs() < 1e-3);
    }

    #[test]
    fn the_thumb_follows_the_scroll() {
        let m = m();
        let top = Scrollbar::new(&m, 100, VScroll::TOP);
        assert_eq!(top.thumb_top, 0.0);
        let max = max_scroll(&m, 100);
        let bottom = Scrollbar::new(&m, 100, max);
        assert!((bottom.thumb_top + bottom.thumb_height - bottom.track_height).abs() < 1e-3);
        assert_eq!(top.thumb_height, 24.0f32.max(top.thumb_height));
        // dragging the thumb all the way down scrolls to the end
        assert_eq!(top.scroll_for_drag(&m, 100, VScroll::TOP, 1e6), max);
        // a document that fits has a thumb as long as the track
        let fit = Scrollbar::new(&m, 3, VScroll::TOP);
        assert_eq!(fit.thumb_height, fit.track_height);
        assert_eq!(fit.scroll_for_drag(&m, 3, VScroll::TOP, 50.0), VScroll::TOP);
    }

    /// 2x10^8 rows and more: the rows of the position stay exact where the pixels (6x10^9, a step of
    /// 512 as an `f32`) would not.
    #[test]
    fn the_vertical_position_is_exact_for_very_many_rows() {
        let m = Metrics {
            line_height: 31.0,
            view_height: 31.0 * 20.0 + 5.0, // 20 rows and a part
            ..m()
        };
        for total in [200_000_001usize, 1_000_000_001, 5_000_000_000] {
            let visible = m.visible_rows();
            assert_eq!(visible, 20);
            let last = total - 1;
            // Ctrl+End: the row of the end is the last of the rows in view
            let end = scroll_to_show_row(&m, VScroll::TOP, last);
            assert_eq!(end.row(), total - visible, "{total}");
            assert_eq!(end.y_of_row(last, 31.0), 31.0 * 19.0);
            assert!(clamp_scroll(&m, total, end) == end);
            // one row up and one row down are exactly one row
            assert_eq!(end.shifted_rows(-1).row(), total - visible - 1);
            assert_eq!(end.shifted_rows(-1).shifted_rows(1), end);
            assert_eq!(end.shifted_px(-31.0, 31.0).row(), total - visible - 1);
            assert_eq!(end.shifted_px(-31.0, 31.0).shifted_px(31.0, 31.0), end);
            // a page (a track click) is the rows of the view less one
            let before = VScroll::at_row(total / 3);
            assert_eq!(
                before.shifted_rows(visible as isize - 1).row(),
                total / 3 + 19
            );
            // the last position stops one row after the end of the text can be shown
            let max = max_scroll(&m, total);
            assert_eq!(max.row(), total - (visible - 1));
            assert_eq!(clamp_scroll(&m, total, VScroll::at_row(total + 100)), max);
            // the row under a point is relative to the top: the row of the end, and none below
            assert_eq!(row_at(&m, end, 31.0 * 19.0 + 1.0, total), last);
            assert_eq!(row_at(&m, end, 31.0 * 25.0, total), last);
            assert_eq!(row_at(&m, end, 0.0, total), total - visible);
            // the thumb at the start, at the end, and at the middle
            let bar = |at: VScroll| Scrollbar::new(&m, total, at);
            let travel = bar(VScroll::TOP).track_height - bar(VScroll::TOP).thumb_height;
            assert_eq!(bar(VScroll::TOP).thumb_top, 0.0);
            assert!((bar(max).thumb_top - travel).abs() < 0.01, "{total}");
            let middle = VScroll::at_row(max.row() / 2);
            assert!(
                (bar(middle).thumb_top - travel / 2.0).abs() < 0.01,
                "{total}"
            );
            // dragging the thumb to the middle of its track shows the rows around the middle
            let dragged = bar(VScroll::TOP).scroll_for_drag(&m, total, VScroll::TOP, travel / 2.0);
            assert!(dragged.row().abs_diff(max.row() / 2) <= 1, "{total}");
            // and to the end of it, the last position
            assert_eq!(
                bar(VScroll::TOP).scroll_for_drag(&m, total, VScroll::TOP, travel * 2.0),
                max
            );
            // dragging back from the end
            let back = bar(max).scroll_for_drag(&m, total, max, -travel);
            assert!(back.row() <= 1, "{total}");
        }
    }

    #[test]
    fn the_horizontal_thumb_follows_the_columns() {
        // 100 columns, 40 in view, a bar of 300 pixels: 272 for the track.
        let at_left = HScrollbar::new(100, 40, 0.0, 300.0);
        assert!(at_left.enabled());
        assert_eq!(at_left.track_width, 272.0);
        assert!((at_left.thumb_width - 108.8).abs() < 1e-3);
        assert_eq!(at_left.thumb_left, 0.0);
        let at_right = HScrollbar::new(100, 40, 60.0, 300.0);
        assert!((at_right.thumb_left + at_right.thumb_width - 272.0).abs() < 1e-3);
        // A position past the last one stops at the end of the track.
        assert_eq!(HScrollbar::new(100, 40, 90.0, 300.0), at_right);
    }

    #[test]
    fn the_horizontal_bar_is_off_when_everything_is_in_view() {
        assert!(!HScrollbar::new(40, 40, 0.0, 300.0).enabled());
        assert!(!HScrollbar::new(30, 40, 0.0, 300.0).enabled());
        assert!(HScrollbar::new(41, 40, 0.0, 300.0).enabled());
    }

    #[test]
    fn dragging_the_horizontal_thumb_moves_whole_columns() {
        let bar = HScrollbar::new(100, 40, 0.0, 300.0);
        let travel = bar.track_width - bar.thumb_width;
        assert_eq!(bar.col_for_drag(0.0, 0.0), 0.0);
        assert_eq!(bar.col_for_drag(0.0, travel), 60.0);
        assert_eq!(bar.col_for_drag(0.0, travel * 5.0), 60.0);
        assert_eq!(bar.col_for_drag(10.0, -travel), 0.0);
        let half = bar.col_for_drag(0.0, travel / 2.0);
        assert_eq!(half, 30.0);
    }

    #[test]
    fn the_view_can_scroll_the_overhang_past_the_bar() {
        // ScrollAtH: right edge + overhang - columns in view.
        assert_eq!(max_left_col(100, 3, 40), 63.0);
        assert_eq!(max_left_col(30, 3, 40), 0.0);
    }
    #[test]
    fn the_vertical_bar_has_an_arrow_at_each_end() {
        let m = m();
        let bar = Scrollbar::new(&m, 100, VScroll::TOP);
        // 100 pixels of bar: 14 for each arrow and 72 of track.
        assert_eq!(Scrollbar::bar_length(&m), 100.0);
        assert_eq!(bar.track_height, 72.0);
        assert_eq!(bar.part_at(&m, 0.0), BarPart::LineBack);
        assert_eq!(bar.part_at(&m, 13.9), BarPart::LineBack);
        // the thumb is at the top of the track
        assert_eq!(bar.part_at(&m, 14.0), BarPart::Thumb);
        assert_eq!(
            bar.part_at(&m, 14.0 + bar.thumb_height + 1.0),
            BarPart::PageForward
        );
        assert_eq!(bar.part_at(&m, 85.9), BarPart::PageForward);
        assert_eq!(bar.part_at(&m, 86.0), BarPart::LineForward);
        let at_end = Scrollbar::new(&m, 100, max_scroll(&m, 100));
        assert_eq!(at_end.part_at(&m, 20.0), BarPart::PageBack);
    }

    #[test]
    fn the_corner_takes_the_end_of_the_vertical_bar_without_a_horizontal_bar() {
        let with_corner = Metrics {
            vbar_gap: HSCROLLBAR_HEIGHT,
            ..m()
        };
        assert_eq!(Scrollbar::bar_length(&with_corner), 86.0);
        assert_eq!(
            Scrollbar::new(&with_corner, 100, VScroll::TOP).track_height,
            58.0
        );
    }

    #[test]
    fn the_horizontal_bar_has_the_same_parts() {
        let bar = HScrollbar::new(100, 40, 0.0, 300.0);
        assert_eq!(bar.part_at(5.0), BarPart::LineBack);
        assert_eq!(bar.part_at(HSCROLL_ARROW + 1.0), BarPart::Thumb);
        assert_eq!(bar.part_at(HSCROLL_ARROW + 200.0), BarPart::PageForward);
        assert_eq!(bar.part_at(299.0), BarPart::LineForward);
    }

    #[test]
    fn a_column_edge_is_columns_times_the_cell_whatever_the_unit() {
        for cell in [7.2_f32, 9.2, 8.0] {
            for per in [1, 64, 80, 85] {
                let scale = XScale::new(cell, per);
                for columns in [0_u32, 1, 3, 17, 1000, 10_240] {
                    let x = X::from_raw(columns * per);
                    assert_eq!(scale.px(x), columns as f32 * cell, "{cell} {per} {columns}");
                    // and back: the edge of the column, and a point a little inside it
                    assert_eq!(scale.column_start(columns as f32 * cell + 0.01), x);
                    assert_eq!(scale.nearest_column_edge(columns as f32 * cell + 0.01), x);
                }
            }
        }
    }

    #[test]
    fn a_point_in_a_column_is_a_fraction_of_its_unit() {
        let scale = XScale::new(8.0, 64);
        assert_eq!(scale.x_at(0.0), X::ZERO);
        assert_eq!(scale.x_at(4.0), X::from_raw(32));
        assert_eq!(scale.x_at(8.0), X::from_raw(64));
        assert_eq!(scale.x_at(-3.0), X::ZERO);
        assert_eq!(scale.px(X::from_raw(32)), 4.0);
        assert_eq!(scale.nearest_column_edge(3.9), X::ZERO);
        assert_eq!(scale.nearest_column_edge(4.0), X::from_raw(64));
    }

    #[test]
    fn the_unit_of_a_column_has_the_spacing_in_it() {
        assert_eq!(per_column_for(8.0, 0), 64);
        // 2 pixels of spacing on a half-width of 8: 16 units
        assert_eq!(per_column_for(8.0, 2), 80);
        assert_eq!(per_column_for(7.2, 0), 64);
    }
}
