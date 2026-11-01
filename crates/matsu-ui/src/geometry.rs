//! Scrolling and hit-testing arithmetic in plain `f32` pixels, so that it can be tested without a
//! window. `EditorView` calls these with the sizes it measured at the last paint.

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
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            cell_width: 8.0,
            line_height: 20.0,
            view_width: 800.0,
            view_height: 600.0,
        }
    }
}

impl Metrics {
    /// How many rows fit in the view, at least one.
    pub fn visible_rows(&self) -> usize {
        ((self.view_height / self.line_height).floor() as usize).max(1)
    }
}

/// The largest vertical scroll offset: the last row ends at the bottom of the view, plus one blank
/// row (the PoC's rule, so that the last row is never hidden under the edge of the window).
pub fn max_scroll_y(m: &Metrics, total_rows: usize) -> f32 {
    let visible = m.visible_rows() as f32;
    (m.line_height * total_rows as f32 - m.line_height * (visible - 1.0)).max(0.0)
}

pub fn clamp_scroll_y(m: &Metrics, total_rows: usize, scroll_y: f32) -> f32 {
    scroll_y.clamp(0.0, max_scroll_y(m, total_rows))
}

/// The scroll offset that shows `row` completely, changing `scroll_y` as little as possible.
pub fn scroll_y_to_show_row(m: &Metrics, scroll_y: f32, row: usize) -> f32 {
    let top = m.line_height * row as f32;
    let bottom = top + m.line_height;
    let visible_height = m.line_height * m.visible_rows() as f32;
    if top < scroll_y {
        top
    } else if bottom > scroll_y + visible_height {
        bottom - visible_height
    } else {
        scroll_y
    }
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
pub fn row_at(m: &Metrics, scroll_y: f32, y: f32, total_rows: usize) -> usize {
    let row = ((y + scroll_y) / m.line_height).floor().max(0.0) as usize;
    row.min(total_rows.saturating_sub(1))
}

/// The column that contains `x` pixels right of the text origin (`scroll_x` already added by the
/// caller), and how far into the column `x` is, from 0 up to 1.
pub fn column_at(m: &Metrics, x: f32) -> (u32, f32) {
    let cells = (x / m.cell_width).max(0.0);
    (cells.floor() as u32, cells.fract())
}

/// The vertical scroll bar: the track is the full height of the view and the thumb shows how much
/// of the document is in view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scrollbar {
    pub track_height: f32,
    pub thumb_top: f32,
    pub thumb_height: f32,
}

impl Scrollbar {
    pub fn new(m: &Metrics, total_rows: usize, scroll_y: f32) -> Self {
        let track = m.view_height;
        let max = max_scroll_y(m, total_rows);
        let thumb_height = (track * (m.view_height / (max + m.view_height)))
            .max(24.0)
            .min(track);
        // The top may be past the range (`EditorView::move_view_top_line`).
        let ratio = if max > 0.0 {
            (scroll_y / max).min(1.0)
        } else {
            0.0
        };
        Self {
            track_height: track,
            thumb_top: (track - thumb_height) * ratio,
            thumb_height,
        }
    }

    /// The scroll offset for a thumb dragged by `dy` pixels from where it was at `start_scroll`.
    pub fn scroll_for_drag(
        &self,
        m: &Metrics,
        total_rows: usize,
        start_scroll: f32,
        dy: f32,
    ) -> f32 {
        let travel = self.track_height - self.thumb_height;
        if travel <= 0.0 {
            return start_scroll;
        }
        clamp_scroll_y(
            m,
            total_rows,
            start_scroll + max_scroll_y(m, total_rows) * (dy / travel),
        )
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
        }
    }

    #[test]
    fn the_scroll_stops_one_row_after_the_last() {
        assert_eq!(max_scroll_y(&m(), 100), 20.0 * 96.0);
        assert_eq!(max_scroll_y(&m(), 3), 0.0);
        assert_eq!(clamp_scroll_y(&m(), 100, 1e9), 20.0 * 96.0);
        assert_eq!(clamp_scroll_y(&m(), 100, -5.0), 0.0);
    }

    #[test]
    fn showing_a_row_moves_the_scroll_as_little_as_possible() {
        let m = m();
        assert_eq!(scroll_y_to_show_row(&m, 0.0, 2), 0.0);
        assert_eq!(scroll_y_to_show_row(&m, 0.0, 5), 20.0); // bottom edge to 120
        assert_eq!(scroll_y_to_show_row(&m, 200.0, 3), 60.0); // above: row on top
        assert_eq!(scroll_y_to_show_row(&m, 200.0, 12), 200.0); // rows 10..=14 are in view
        assert_eq!(scroll_y_to_show_row(&m, 200.0, 15), 220.0);
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
        assert_eq!(row_at(&m, 0.0, 0.0, 10), 0);
        assert_eq!(row_at(&m, 0.0, 39.9, 10), 1);
        assert_eq!(row_at(&m, 30.0, 10.0, 10), 2);
        assert_eq!(row_at(&m, 0.0, -50.0, 10), 0);
        assert_eq!(row_at(&m, 0.0, 9999.0, 10), 9);
        assert_eq!(column_at(&m, 25.0), (2, 0.5));
        assert_eq!(column_at(&m, -3.0), (0, 0.0));
    }

    #[test]
    fn the_thumb_follows_the_scroll() {
        let m = m();
        let top = Scrollbar::new(&m, 100, 0.0);
        assert_eq!(top.thumb_top, 0.0);
        let max = max_scroll_y(&m, 100);
        let bottom = Scrollbar::new(&m, 100, max);
        assert!((bottom.thumb_top + bottom.thumb_height - 100.0).abs() < 1e-3);
        assert_eq!(top.thumb_height, 24.0f32.max(top.thumb_height));
        // dragging the thumb all the way down scrolls to the end
        assert!((top.scroll_for_drag(&m, 100, 0.0, 1e6) - max).abs() < 1e-3);
        // a document that fits has a thumb as long as the track
        let fit = Scrollbar::new(&m, 3, 0.0);
        assert_eq!(fit.thumb_height, 100.0);
        assert_eq!(fit.scroll_for_drag(&m, 3, 0.0, 50.0), 0.0);
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
}
