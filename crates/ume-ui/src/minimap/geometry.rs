//! The arithmetic of the minimap, in plain numbers so that it is tested without a window.
//!
//! Sakura: `CViewFont::CreateFonts` (the font), `CTextArea::DetectWidthOfLineNumberArea` (the
//! left margin of 4 pixels instead of the line numbers), `CTextMetrics::Update` (the height of a
//! row is the height of the font and the line space), and `CEditView::OnMOUSEMOVE` (the drag).

/// The space left of the text (`nViewAlignLeftNew = 4` for a minimap).
pub(crate) const LEFT_MARGIN: f32 = 4.0;

/// The narrowest pane: the scroll bar and a little text.
pub(crate) const MIN_WIDTH: f32 = 24.0;

/// The line space of a type, which Sakura adds to the height of the font (`nLineSpace`, 1 by
/// default).
const LINE_SPACE: f32 = 1.0;

/// How far from the top or the bottom of the pane a drag starts to scroll it, in rows
/// (`nScrollMargin`).
const SCROLL_MARGIN: usize = 15;

/// The height of a character of the font, in pixels, for `nMiniMapFontSize`: the size of a
/// negative `lfHeight`, the height of a cell for a positive one (this program cannot tell the
/// internal leading apart, so it is the same), and 12 for 0, the default height of the system.
pub(crate) fn font_px(setting: i32) -> f32 {
    match setting {
        0 => 12.0,
        n => n.unsigned_abs() as f32,
    }
}

/// The height of a row: the font and the line space (`m_nDyBasis`).
pub(crate) fn row_height(setting: i32) -> f32 {
    font_px(setting) + LINE_SPACE
}

/// How many rows fit in a pane `height` pixels high, at least one.
pub(crate) fn view_rows(height: f32, row_height: f32) -> usize {
    ((height / row_height).floor() as usize).max(1)
}

/// The top row at which the last row can be: the same rule as the view's scroll range (the last
/// row ends at the bottom, and one row is left blank).
pub(crate) fn max_top(total_rows: usize, view_rows: usize) -> usize {
    total_rows.saturating_sub(view_rows.saturating_sub(1))
}

/// The row at `y` pixels below the top of the pane when its first row is `top`; a point above the
/// pane is never above the first row (`ptNew.y < 0`).
pub(crate) fn row_at(top: usize, y: f32, row_height: f32) -> usize {
    let row = top as f32 + (y / row_height).floor();
    row.max(0.0) as usize
}

/// While the button is down near the top or the bottom of the pane, the pane scrolls so that the
/// row under the pointer stays `SCROLL_MARGIN` rows (or half the pane) from the edge. Returns the
/// new top row (`CEditView::OnMOUSEMOVE`, `m_bMiniMapMouseDown`).
pub(crate) fn top_after_drag(
    top: usize,
    view_rows: usize,
    total_rows: usize,
    pointer_row: usize,
) -> usize {
    let margin = SCROLL_MARGIN.min(view_rows / 2);
    let new_top = if total_rows > view_rows && pointer_row + margin > top + view_rows {
        // below: the pointer row is `margin` rows above the bottom edge
        (pointer_row + margin).saturating_sub(view_rows)
    } else if top > 0 && pointer_row < top + margin {
        // above: the pointer row is `margin` rows below the top edge, or the top
        pointer_row.saturating_sub(margin)
    } else {
        top
    };
    new_top.min(max_top(total_rows, view_rows))
}

/// The top row after the wheel moved `rows` rows (positive: down).
pub(crate) fn top_after_wheel(top: usize, rows: f32, view_rows: usize, total_rows: usize) -> usize {
    let wanted = (top as f32 + rows).round().max(0.0) as usize;
    wanted.min(max_top(total_rows, view_rows))
}

/// The scroll bar's thumb: its top and height in the track of `track` pixels.
pub(crate) fn thumb(top: usize, view_rows: usize, total_rows: usize, track: f32) -> (f32, f32) {
    let max = max_top(total_rows, view_rows);
    let thumb_height = (track * view_rows as f32 / (max + view_rows) as f32)
        .max(24.0)
        .min(track);
    let ratio = if max > 0 {
        (top as f32 / max as f32).min(1.0)
    } else {
        0.0
    };
    ((track - thumb_height) * ratio, thumb_height)
}

/// The top row for a thumb dragged by `dy` pixels from where the drag began at `start_top`.
pub(crate) fn top_for_thumb_drag(
    start_top: usize,
    dy: f32,
    view_rows: usize,
    total_rows: usize,
    track: f32,
) -> usize {
    let max = max_top(total_rows, view_rows);
    let (_, thumb_height) = thumb(start_top, view_rows, total_rows, track);
    let travel = track - thumb_height;
    if travel <= 0.0 || max == 0 {
        return start_top.min(max);
    }
    let wanted = start_top as f32 + max as f32 * (dy / travel);
    wanted.round().clamp(0.0, max as f32) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_font_is_the_size_of_the_setting() {
        // the default of Sakura is -2: two pixels, in rows of three
        assert_eq!(font_px(-2), 2.0);
        assert_eq!(row_height(-2), 3.0);
        assert_eq!(font_px(4), 4.0);
        assert_eq!(font_px(0), 12.0);
    }

    #[test]
    fn a_pointer_above_the_pane_is_never_above_the_first_row() {
        assert_eq!(row_at(10, 0.0, 3.0), 10);
        assert_eq!(row_at(10, 7.0, 3.0), 12);
        assert_eq!(row_at(0, -20.0, 3.0), 0);
        assert_eq!(row_at(10, -4.0, 3.0), 8);
    }

    #[test]
    fn the_last_row_can_come_to_the_bottom() {
        assert_eq!(max_top(1000, 100), 901);
        assert_eq!(max_top(50, 100), 0);
        assert_eq!(view_rows(0.0, 3.0), 1);
        assert_eq!(view_rows(300.0, 3.0), 100);
    }

    #[test]
    fn a_drag_near_the_bottom_scrolls_until_the_row_is_15_rows_from_the_edge() {
        // 100 rows in the pane, the top is 0, the pointer at row 90 is 10 rows from the bottom
        assert_eq!(top_after_drag(0, 100, 1000, 90), 5);
        // inside the margins: no scroll
        assert_eq!(top_after_drag(0, 100, 1000, 85), 0);
        assert_eq!(top_after_drag(50, 100, 1000, 100), 50);
        // past the end of the document: the scroll stops at the last row
        assert_eq!(top_after_drag(900, 100, 1000, 999), 901);
    }

    #[test]
    fn a_drag_near_the_top_scrolls_up_and_stops_at_the_first_row() {
        assert_eq!(top_after_drag(100, 100, 1000, 105), 90);
        assert_eq!(top_after_drag(10, 100, 1000, 0), 0);
        // already at the top: nothing to do
        assert_eq!(top_after_drag(0, 100, 1000, 3), 0);
    }

    #[test]
    fn a_document_that_fits_does_not_scroll() {
        assert_eq!(top_after_drag(0, 100, 50, 49), 0);
        assert_eq!(top_after_wheel(0, 30.0, 100, 50), 0);
    }

    #[test]
    fn a_small_pane_uses_half_of_its_rows_as_the_margin() {
        // 10 rows: the margin is 5
        assert_eq!(top_after_drag(0, 10, 1000, 8), 3);
    }

    #[test]
    fn the_wheel_moves_whole_rows_inside_the_range() {
        assert_eq!(top_after_wheel(100, 30.0, 100, 1000), 130);
        assert_eq!(top_after_wheel(10, -30.0, 100, 1000), 0);
        assert_eq!(top_after_wheel(900, 30.0, 100, 1000), 901);
    }

    #[test]
    fn the_thumb_follows_the_top_row() {
        let (top, height) = thumb(0, 100, 1000, 300.0);
        assert_eq!(top, 0.0);
        assert!((24.0..300.0).contains(&height));
        let (end_top, end_height) = thumb(max_top(1000, 100), 100, 1000, 300.0);
        assert!((end_top + end_height - 300.0).abs() < 0.01);
        // a document that fits fills the track
        assert_eq!(thumb(0, 100, 50, 300.0), (0.0, 300.0));
    }

    #[test]
    fn dragging_the_thumb_to_the_end_reaches_the_last_top_row() {
        let top = top_for_thumb_drag(0, 1e6, 100, 1000, 300.0);
        assert_eq!(top, max_top(1000, 100));
        assert_eq!(top_for_thumb_drag(500, -1e6, 100, 1000, 300.0), 0);
        assert_eq!(top_for_thumb_drag(0, 10.0, 100, 50, 300.0), 0);
    }
}
