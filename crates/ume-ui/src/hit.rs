//! From a point in the text area to a position in the document, and from a position to columns
//! on a row. The arithmetic of rows and columns is in [`crate::geometry`]; this module asks the
//! layout what is at a row and a column.

use ume_core::buffer::{Document, Pos, Result};
use ume_layout::{DocumentLayout, RowInfo};

use crate::geometry::{Metrics, column_at, row_at};

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

/// Like [`position_at`], on a given row.
pub fn position_in_row(
    layout: &DocumentLayout,
    doc: &Document,
    m: &Metrics,
    row: usize,
    x: f32,
) -> Result<Pos> {
    let (col, _) = column_at(m, x);
    let target = layout.row_to_pos(doc, row, col)?;
    if target.extra_columns > 0 {
        return Ok(target.pos);
    }
    let Some(line) = doc.line(target.pos.line)? else {
        return Ok(target.pos);
    };
    let Some(c) = line
        .text
        .get(target.pos.col..)
        .and_then(|rest| rest.chars().next())
    else {
        return Ok(target.pos);
    };
    let start = layout.pos_to_row(doc, target.pos)?.column;
    let width = layout.config().columns_of(c, start);
    let into = (x / m.cell_width).max(0.0) - start as f32;
    Ok(if into * 2.0 >= width as f32 {
        Pos::new(target.pos.line, target.pos.col + c.len_utf8())
    } else {
        target.pos
    })
}

/// The columns of `row` that the selection `from..to` (ordered) covers, or `None`. A selection
/// that goes on past the end of the last row of a line also covers the line break, which is drawn
/// as one cell wide.
pub fn selected_columns(
    layout: &DocumentLayout,
    doc: &Document,
    row: &RowInfo,
    (from, to): (Pos, Pos),
) -> Result<Option<(u32, u32)>> {
    if to <= row.start || from > row.end || (from == to) {
        return Ok(None);
    }
    let first = if from <= row.start {
        row.start_column
    } else {
        layout.pos_to_row(doc, from)?.column
    };
    let breaks_line = !row.wrapped && !row.is_eof_row && to > row.end;
    let last = if to >= row.end {
        row.end_column + u32::from(breaks_line)
    } else {
        layout.pos_to_row(doc, to)?.column
    };
    Ok((last > first).then_some((first, last)))
}

#[cfg(test)]
mod tests {
    use ume_layout::{LayoutConfig, Wrap};

    use super::*;

    fn doc(s: &str) -> Document {
        Document::from_utf8(s.as_bytes().to_vec()).unwrap()
    }

    fn setup(text: &str, cfg: LayoutConfig) -> (Document, DocumentLayout, Metrics) {
        let d = doc(text);
        let l = DocumentLayout::laid_out(&d, cfg).unwrap();
        let m = Metrics {
            cell_width: 10.0,
            line_height: 20.0,
            view_width: 400.0,
            view_height: 200.0,
        };
        (d, l, m)
    }

    #[test]
    fn a_click_picks_the_nearer_edge() {
        let (d, l, m) = setup("abc\ndef", LayoutConfig::default());
        let at = |x, y| position_at(&l, &d, &m, 0.0, x, y).unwrap();
        assert_eq!(at(0.0, 5.0), Pos::new(0, 0));
        assert_eq!(at(4.9, 5.0), Pos::new(0, 0));
        assert_eq!(at(5.0, 5.0), Pos::new(0, 1));
        assert_eq!(at(14.0, 5.0), Pos::new(0, 1));
        assert_eq!(at(16.0, 25.0), Pos::new(1, 2));
        // right of the text: the end of the row; below the text: the last row
        assert_eq!(at(300.0, 5.0), Pos::new(0, 3));
        assert_eq!(at(0.0, 500.0), Pos::new(1, 0));
        assert_eq!(at(-30.0, -30.0), Pos::new(0, 0));
    }

    #[test]
    fn tabs_and_wide_characters_have_their_own_halves() {
        let (d, l, m) = setup("a\t日b", LayoutConfig::default());
        let at = |x| position_in_row(&l, &d, &m, 0, x).unwrap().col;
        // a: column 0, tab: columns 1..4, 日 (3 bytes): columns 4..6, b: column 6
        assert_eq!(at(14.0), 1); // first half of the tab
        assert_eq!(at(26.0), 2); // second half of the tab: after it
        assert_eq!(at(43.0), 2); // first half of 日
        assert_eq!(at(51.0), 5); // second half of 日: after its 3 bytes
        assert_eq!(at(61.0), 5);
        assert_eq!(at(66.0), 6);
    }

    #[test]
    fn a_wrapped_row_is_hit_by_its_own_columns() {
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(4),
            ..LayoutConfig::default()
        };
        let (d, l, m) = setup("abcdefgh", cfg);
        let pos = |row, x| position_in_row(&l, &d, &m, row, x).unwrap();
        assert_eq!(pos(1, 12.0), Pos::new(0, 5));
        assert_eq!(pos(0, 12.0), Pos::new(0, 1));
    }

    #[test]
    fn the_empty_last_row_is_a_target() {
        let (d, l, m) = setup("ab\n", LayoutConfig::default());
        assert_eq!(
            position_at(&l, &d, &m, 0.0, 30.0, 30.0).unwrap(),
            Pos::new(1, 0)
        );
    }

    #[test]
    fn selected_columns_cover_the_selection_on_each_row() {
        let (d, l, _) = setup("abc\nde\nf", LayoutConfig::default());
        let rows = l.rows(&d, 0, 3).unwrap();
        let sel = (Pos::new(0, 1), Pos::new(2, 1));
        let cols: Vec<_> = rows
            .iter()
            .map(|r| selected_columns(&l, &d, r, sel).unwrap())
            .collect();
        // the line breaks of rows 0 and 1 are selected and count one cell
        assert_eq!(cols, [Some((1, 4)), Some((0, 3)), Some((0, 1))]);
        let none = selected_columns(&l, &d, &rows[0], (Pos::new(1, 0), Pos::new(2, 0))).unwrap();
        assert_eq!(none, None);
        let empty = selected_columns(&l, &d, &rows[0], (Pos::new(0, 1), Pos::new(0, 1))).unwrap();
        assert_eq!(empty, None);
    }

    #[test]
    fn a_selection_ending_at_the_end_of_a_row_does_not_cover_the_break() {
        let (d, l, _) = setup("abc\nde", LayoutConfig::default());
        let rows = l.rows(&d, 0, 2).unwrap();
        let sel = (Pos::new(0, 1), Pos::new(0, 3));
        assert_eq!(
            selected_columns(&l, &d, &rows[0], sel).unwrap(),
            Some((1, 3))
        );
        assert_eq!(selected_columns(&l, &d, &rows[1], sel).unwrap(), None);
    }
}
