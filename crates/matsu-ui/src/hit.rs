//! From a point in the text area to a position in the document, and from a position to `X`
//! on a row. The arithmetic of rows and pixels is in [`crate::geometry`]; this module asks the
//! layout what is at a row and an `X`.

use matsu_commands::context::cursor_x;
use matsu_commands::layout::{row_is_wrapped, wrapped_gap_before};
use matsu_core::buffer::{Document, Pos, Result};
use matsu_core::cursor::Cursor;
use matsu_layout::{DocumentLayout, RowInfo, RowPos, X};

use crate::geometry::{Metrics, XScale, row_at};
use crate::layout_adapter::RowRead;

/// The position that a click at `x` pixels right of the text origin (scroll already added) and
/// `y` pixels below the top of the text area puts the caret at.
///
/// The caret goes to the nearer edge of the character that was hit: the right half of a
/// character puts it after the character. Below the last row the caret goes to the last row, and
/// right of a row it goes to the end of the row.
pub fn position_at(
    layout: &DocumentLayout,
    doc: &Document,
    m: &Metrics,
    scroll_y: f32,
    x: f32,
    y: f32,
) -> Result<Pos> {
    let row = row_at(m, scroll_y, y, layout.row_count());
    position_in_row(layout, doc, m, row, x)
}

/// The `X` of a click to the right of the text of its row, or `None` when the click is on the
/// text. The free cursor mode puts the caret there (`CCaret::MoveCursorProperly`: the position of
/// the click, `ClientToLayout`). It is the edge of the column that the click is in (rounded
/// down), so that the caret of the free cursor stands on the grid of columns.
pub fn free_x_at(
    layout: &DocumentLayout,
    doc: &Document,
    m: &Metrics,
    scroll_y: f32,
    x: f32,
    y: f32,
) -> Result<Option<X>> {
    let row = row_at(m, scroll_y, y, layout.row_count());
    let scale = XScale::new(m.cell_width, layout.config().width.per_column());
    let at = scale.column_start(x);
    let target = layout.row_to_pos(doc, row, at)?;
    if target.extra_x > X::ZERO {
        return Ok(Some(at));
    }
    // The end of a wrapped row is a place of its own for the free caret (it goes on the row, not
    // on the start of the next one), so a column that is just the end of its text counts.
    Ok(ends_wrapped_row(layout, doc, row, target.pos)?.then_some(at))
}

/// The row and the `X` that the caret of `cursor` is drawn at: where the layout puts its position
/// (`DocumentLayout::pos_to_row`), except for a free caret in the gap after a wrapped row.
///
/// The position of such a caret is the end of the wrapped row, which the layout puts at the start
/// of the next row. The stored free column (`Cursor::free_col`) is the mark that it is on the row
/// before, at that column (`matsu_commands::context`, the free cursor mode). `free_cursor` is
/// whether the mode is on: with it off, only the end of the text of the row is a place of the
/// caret (`End`, a click right of the row), not the space right of it.
pub fn caret_row_pos(
    layout: &DocumentLayout,
    doc: &Document,
    cursor: &Cursor,
    free_cursor: bool,
) -> Result<RowPos> {
    let caret = cursor.caret();
    let at = layout.pos_to_row(doc, caret)?;
    let Some(col) = cursor.free_col() else {
        return Ok(at);
    };
    // The same rule as the commands use (`matsu_commands::layout::wrapped_gap_before`).
    let rows = RowRead(layout);
    match wrapped_gap_before(&rows, doc, caret)? {
        Some(gap) if gap.takes(cursor_x(col), free_cursor) => Ok(RowPos {
            row: layout.pos_to_row(doc, gap.last)?.row,
            x: gap.text_end,
        }),
        _ => Ok(at),
    }
}

/// Whether the stored column of `cursor` is the mark of the end of a wrapped row, the one place
/// right of the text that the caret keeps when the free cursor mode is off (the view drops every
/// other column then, so that it does not come back when the mode is turned on).
pub fn is_wrapped_row_end_mark(layout: &DocumentLayout, doc: &Document, cursor: &Cursor) -> bool {
    let Some(col) = cursor.free_col() else {
        return false;
    };
    let rows = RowRead(layout);
    matches!(
        wrapped_gap_before(&rows, doc, cursor.caret()),
        Ok(Some(gap)) if gap.takes(cursor_x(col), false)
    )
}

/// Whether `pos` is the end of the text of `row` and the row ends because the line wraps there.
fn ends_wrapped_row(layout: &DocumentLayout, doc: &Document, row: usize, pos: Pos) -> Result<bool> {
    // The same question as the commands ask (`matsu_commands::layout::row_is_wrapped`), of the
    // first position of the row, since the end of a wrapped row belongs to the next row.
    let info = layout.row(doc, row)?;
    Ok(info.end == pos && row_is_wrapped(&RowRead(layout), doc, info.start)?)
}

/// Like [`position_at`], on a given row.
pub fn position_in_row(
    layout: &DocumentLayout,
    doc: &Document,
    m: &Metrics,
    row: usize,
    x: f32,
) -> Result<Pos> {
    let scale = XScale::new(m.cell_width, layout.config().width.per_column());
    let target = layout.row_to_pos(doc, row, scale.x_at(x))?;
    if target.extra_x > X::ZERO || ends_wrapped_row(layout, doc, row, target.pos)? {
        return Ok(target.pos);
    }
    // Only the char at the position is needed: a very long line is not decoded as a whole.
    let text = if doc.is_long_line(target.pos.line) {
        doc.line_slice(target.pos.line, target.pos.col, target.pos.col + 4)?
            .map(|slice| slice.text)
    } else {
        doc.line(target.pos.line)?
            .and_then(|line| line.text.get(target.pos.col..).map(str::to_owned))
    };
    let Some(c) = text.and_then(|rest| rest.chars().next()) else {
        return Ok(target.pos);
    };
    let start_x = layout.pos_to_row(doc, target.pos)?.x;
    let width = layout.config().advance(c, start_x);
    // How far into the character the click is, in `X` (a fraction of one).
    let into = scale.x_f32(x) - start_x.raw() as f32;
    Ok(if into * 2.0 >= width.raw() as f32 {
        Pos::new(target.pos.line, target.pos.col + c.len_utf8())
    } else {
        target.pos
    })
}

/// The part of `row` that the selection `from..to` (ordered) covers, or `None`. A selection that
/// goes on past the end of the last row of a line also covers the line break, which is drawn as
/// one column wide.
pub fn selected_x(
    layout: &DocumentLayout,
    doc: &Document,
    row: &RowInfo,
    (from, to): (Pos, Pos),
) -> Result<Option<(X, X)>> {
    // The end of a wrapped row is the start of the next row (`DocumentLayout::pos_to_row`), so a
    // selection that starts there covers nothing on this row.
    if to <= row.start || from > row.end || (row.wrapped && from == row.end) || from == to {
        return Ok(None);
    }
    let first = if from <= row.start {
        row.start_x
    } else {
        layout.pos_to_row(doc, from)?.x
    };
    let breaks_line = !row.wrapped && !row.is_eof_row && to > row.end;
    let last = if to >= row.end {
        if breaks_line {
            row.end_x + layout.config().width.to_x(1)
        } else {
            row.end_x
        }
    } else {
        layout.pos_to_row(doc, to)?.x
    };
    Ok((last > first).then_some((first, last)))
}

/// The area of a box selection (Sakura's 矩形選択): rows `first..=last` of the layout, and the
/// span `from..to` of `X` on each of them. A corner can be right of the text of its row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoxArea {
    first_row: usize,
    last_row: usize,
    from: X,
    to: X,
}

impl BoxArea {
    /// The area between two corners, each given as a row and an `X`. `None` for a box that is
    /// not wide (it covers nothing).
    pub fn between((row_a, x_a): (usize, X), (row_b, x_b): (usize, X)) -> Option<Self> {
        (x_a != x_b).then(|| Self {
            first_row: row_a.min(row_b),
            last_row: row_a.max(row_b),
            from: x_a.min(x_b),
            to: x_a.max(x_b),
        })
    }

    /// The area of the box selection of `cursor`, if it covers anything.
    pub fn of_cursor(
        layout: &DocumentLayout,
        doc: &Document,
        cursor: &matsu_core::cursor::Cursor,
    ) -> Option<Self> {
        let ((anchor, anchor_x), (caret, caret_x)) = cursor.box_corners()?;
        let corner = |pos, stored: usize| -> Option<(usize, X)> {
            let at = layout.pos_to_row(doc, pos).ok()?;
            Some((at.row, cursor_x(stored).max(at.x)))
        };
        Self::between(corner(anchor, anchor_x)?, corner(caret, caret_x)?)
    }

    /// The top row of the box and the span `from..to` it covers there.
    pub fn top_row(&self) -> (usize, X, X) {
        (self.first_row, self.from, self.to)
    }

    /// The span of the row `row` that the box covers, or `None`.
    pub fn range_on(&self, row: usize) -> Option<(X, X)> {
        (self.first_row..=self.last_row)
            .contains(&row)
            .then_some((self.from, self.to))
    }
}

#[cfg(test)]
mod tests {
    use matsu_layout::{LayoutConfig, WidthRule, Widths, Wrap};

    use super::*;

    /// `X` in a column: the grid of before (1), the view's (64), and two with character spacing
    /// (80 is a power of two apart from 5, 85 is not). The pixels hit are the same for all.
    const PER_COLUMN: [u32; 4] = [1, 64, 80, 85];

    fn doc(s: &str) -> Document {
        Document::from_utf8(s.as_bytes().to_vec()).unwrap()
    }

    fn config(per: u32) -> LayoutConfig {
        LayoutConfig {
            width: Widths::grid_scaled(WidthRule::default(), per),
            ..LayoutConfig::default()
        }
    }

    fn setup(text: &str, cfg: LayoutConfig) -> (Document, DocumentLayout, Metrics) {
        let d = doc(text);
        let l = DocumentLayout::laid_out(&d, cfg).unwrap();
        let m = Metrics {
            cell_width: 10.0,
            line_height: 20.0,
            view_width: 400.0,
            view_height: 200.0,
            vbar_gap: 0.0,
        };
        (d, l, m)
    }

    /// `n` columns as `X` when a column is `per`.
    fn col(per: u32, n: u32) -> X {
        X::from_raw(n * per)
    }

    #[test]
    fn a_click_picks_the_nearer_edge() {
        for per in PER_COLUMN {
            let (d, l, m) = setup("abc\ndef", config(per));
            let at = |x, y| position_at(&l, &d, &m, 0.0, x, y).unwrap();
            assert_eq!(at(0.0, 5.0), Pos::new(0, 0), "{per}");
            assert_eq!(at(4.9, 5.0), Pos::new(0, 0), "{per}");
            assert_eq!(at(5.0, 5.0), Pos::new(0, 1), "{per}");
            assert_eq!(at(14.0, 5.0), Pos::new(0, 1), "{per}");
            assert_eq!(at(16.0, 25.0), Pos::new(1, 2), "{per}");
            // right of the text: the end of the row; below the text: the last row
            assert_eq!(at(300.0, 5.0), Pos::new(0, 3), "{per}");
            assert_eq!(at(0.0, 500.0), Pos::new(1, 0), "{per}");
            assert_eq!(at(-30.0, -30.0), Pos::new(0, 0), "{per}");
        }
    }

    #[test]
    fn tabs_and_wide_characters_have_their_own_halves() {
        for per in PER_COLUMN {
            let (d, l, m) = setup("a\t日b", config(per));
            let at = |x| position_in_row(&l, &d, &m, 0, x).unwrap().col;
            // a: column 0, tab: columns 1..4, 日 (3 bytes): columns 4..6, b: column 6
            assert_eq!(at(14.0), 1, "{per}"); // first half of the tab
            assert_eq!(at(26.0), 2, "{per}"); // second half of the tab: after it
            assert_eq!(at(43.0), 2, "{per}"); // first half of 日
            assert_eq!(at(51.0), 5, "{per}"); // second half of 日: after its 3 bytes
            assert_eq!(at(61.0), 5, "{per}");
            assert_eq!(at(66.0), 6, "{per}");
        }
    }

    #[test]
    fn half_width_katakana_and_a_long_line_are_hit_as_they_were() {
        // Every click of the pixel row gives the same character for every `per_column`.
        let text = format!("ｱｲｳ日本{}", "ab\t".repeat(200));
        let hits = |per: u32| {
            let (d, l, m) = setup(&text, config(per));
            (0..400)
                .map(|px| position_in_row(&l, &d, &m, 0, px as f32 * 0.5).unwrap().col)
                .collect::<Vec<_>>()
        };
        let base = hits(1);
        for per in PER_COLUMN {
            assert_eq!(hits(per), base, "{per}");
        }
    }

    #[test]
    fn a_wrapped_row_is_hit_by_its_own_columns() {
        for per in PER_COLUMN {
            let cfg = LayoutConfig {
                wrap: Wrap::Columns(4),
                ..config(per)
            };
            let (d, l, m) = setup("abcdefgh", cfg);
            let pos = |row, x| position_in_row(&l, &d, &m, row, x).unwrap();
            assert_eq!(pos(1, 12.0), Pos::new(0, 5), "{per}");
            assert_eq!(pos(0, 12.0), Pos::new(0, 1), "{per}");
        }
    }

    #[test]
    fn the_empty_last_row_is_a_target() {
        for per in PER_COLUMN {
            let (d, l, m) = setup("ab\n", config(per));
            assert_eq!(
                position_at(&l, &d, &m, 0.0, 30.0, 30.0).unwrap(),
                Pos::new(1, 0)
            );
        }
    }

    #[test]
    fn a_click_right_of_the_text_is_the_edge_of_its_column_for_the_free_cursor() {
        for per in PER_COLUMN {
            let (d, l, m) = setup("ab\ncd", config(per));
            let at = |x, y| free_x_at(&l, &d, &m, 0.0, x, y).unwrap();
            // on the text and at its end (the end is not right of it)
            assert_eq!(at(5.0, 5.0), None, "{per}");
            assert_eq!(at(25.0, 5.0), None, "{per}");
            // two columns after the end, a click in the column: its left edge
            assert_eq!(at(47.5, 5.0), Some(col(per, 4)), "{per}");
            assert_eq!(at(1000.0, 25.0), Some(col(per, 100)), "{per}");
        }
    }

    #[test]
    fn selected_x_cover_the_selection_on_each_row() {
        for per in PER_COLUMN {
            let (d, l, _) = setup("abc\nde\nf", config(per));
            let rows = l.rows(&d, 0, 3).unwrap();
            let sel = (Pos::new(0, 1), Pos::new(2, 1));
            let spans: Vec<_> = rows
                .iter()
                .map(|r| selected_x(&l, &d, r, sel).unwrap())
                .collect();
            let span = |a, b| Some((col(per, a), col(per, b)));
            // the line breaks of rows 0 and 1 are selected and count one column
            assert_eq!(spans, [span(1, 4), span(0, 3), span(0, 1)], "{per}");
            let none = selected_x(&l, &d, &rows[0], (Pos::new(1, 0), Pos::new(2, 0))).unwrap();
            assert_eq!(none, None);
            let empty = selected_x(&l, &d, &rows[0], (Pos::new(0, 1), Pos::new(0, 1))).unwrap();
            assert_eq!(empty, None);
        }
    }

    #[test]
    fn a_selection_from_the_end_of_a_wrapped_row_starts_on_the_next_row() {
        for per in PER_COLUMN {
            let cfg = LayoutConfig {
                wrap: Wrap::Columns(4),
                ..config(per)
            };
            let (d, l, _) = setup("abcdefgh", cfg);
            let rows = l.rows(&d, 0, 2).unwrap();
            let sel = (Pos::new(0, 4), Pos::new(0, 6));
            assert_eq!(selected_x(&l, &d, &rows[0], sel).unwrap(), None);
            assert_eq!(
                selected_x(&l, &d, &rows[1], sel).unwrap(),
                Some((col(per, 0), col(per, 2)))
            );
        }
    }

    #[test]
    fn a_selection_ending_at_the_end_of_a_row_does_not_cover_the_break() {
        for per in PER_COLUMN {
            let (d, l, _) = setup("abc\nde", config(per));
            let rows = l.rows(&d, 0, 2).unwrap();
            let sel = (Pos::new(0, 1), Pos::new(0, 3));
            assert_eq!(
                selected_x(&l, &d, &rows[0], sel).unwrap(),
                Some((col(per, 1), col(per, 3)))
            );
            assert_eq!(selected_x(&l, &d, &rows[1], sel).unwrap(), None);
        }
    }

    #[test]
    fn a_box_covers_the_same_span_on_each_of_its_rows() {
        let x = X::from_raw;
        let area = BoxArea::between((4, x(7)), (2, x(3))).unwrap();
        assert_eq!(area.range_on(1), None);
        assert_eq!(area.range_on(2), Some((x(3), x(7))));
        assert_eq!(area.range_on(4), Some((x(3), x(7))));
        assert_eq!(area.range_on(5), None);
        // as wide as nothing: nothing covered
        assert_eq!(BoxArea::between((0, x(3)), (5, x(3))), None);
    }

    #[test]
    fn the_area_of_a_cursor_follows_the_corners_and_the_rows_of_the_layout() {
        use matsu_core::cursor::Cursor;
        for per in PER_COLUMN {
            let cfg = LayoutConfig {
                wrap: Wrap::Columns(4),
                ..config(per)
            };
            // rows: "abcd" "efg" "ij"
            let (d, l, _) = setup("abcdefg\nij", cfg);
            let mut c = Cursor::new(Pos::new(0, 5));
            c.begin_box((2 * per) as usize, true);
            // the caret is in the empty space of the last row: its position is the end of that row
            c.move_box_to(Pos::new(1, 2), (6 * per) as usize);
            let area = BoxArea::of_cursor(&l, &d, &c).unwrap();
            assert_eq!(area.range_on(0), None);
            assert_eq!(area.range_on(1), Some((col(per, 2), col(per, 6))));
            assert_eq!(area.range_on(2), Some((col(per, 2), col(per, 6))));
            assert_eq!(area.range_on(3), None);
            let none = BoxArea::of_cursor(&l, &d, &Cursor::new(Pos::new(0, 0)));
            assert_eq!(none, None);
        }
    }

    /// The pixels of every character, the caret, a selection and a box for a line with tabs,
    /// full-width characters, half-width katakana and a long run, for each `per_column` and for
    /// the cell of a font that has no round width (7.2) and one with character spacing (9.2).
    /// They are the same as with the layout counted in columns (`per_column` 1).
    #[test]
    fn the_pixels_of_a_line_do_not_depend_on_the_x_per_column() {
        use matsu_core::cursor::Cursor;
        let text = format!("a\t日ｱｲ\tb本{}z\n\tx", "ab日".repeat(300));
        for cell in [7.2_f32, 9.2, 10.0] {
            let pixels = |per: u32| {
                let (d, l, _) = setup(&text, config(per));
                let scale = XScale::new(cell, per);
                let rows = l.rows(&d, 0, 4).unwrap();
                let mut out: Vec<f32> = Vec::new();
                for row in &rows {
                    out.push(scale.px(row.start_x));
                    out.push(scale.px(row.end_x));
                    let mut at = row.start_x;
                    for c in row.text().chars() {
                        out.push(scale.px(at));
                        at += l.config().advance(c, at);
                    }
                }
                // a selection over two rows, and a box
                let sel = (Pos::new(0, 1), Pos::new(1, 1));
                for row in &rows {
                    if let Some((a, b)) = selected_x(&l, &d, row, sel).unwrap() {
                        out.extend([scale.px(a), scale.px(b)]);
                    }
                }
                let mut c = Cursor::new(Pos::new(0, 2));
                c.begin_box((3 * per) as usize, true);
                c.move_box_to(Pos::new(1, 0), (9 * per) as usize);
                let area = BoxArea::of_cursor(&l, &d, &c).unwrap();
                let (_, from, to) = area.top_row();
                out.extend([scale.px(from), scale.px(to)]);
                out
            };
            let base = pixels(1);
            for per in [64, 80, 85] {
                let got = pixels(per);
                assert_eq!(got.len(), base.len());
                for (i, (a, b)) in got.iter().zip(&base).enumerate() {
                    // a column edge is `columns * cell` exactly, whatever the unit
                    assert_eq!(a, b, "cell {cell} per_column {per} item {i}");
                }
            }
        }
    }
}
