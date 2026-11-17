//! What commands need to know about how the text is laid out on screen.
//!
//! Up and down, the display column, and the start and end of a *row* depend on the layout (tabs,
//! full-width chars, wrapping). `matsu-ui` will implement [`Layout`] with the real layout of
//! `matsu-layout` (ADR-0013); until it exists, [`PlainLayout`] is the simple one that the tests use:
//! no wrapping, tab stops every 4 columns, full-width chars take 2 columns.
//!
//! A *row* is one line on the screen. With wrapping one line of the document has several rows;
//! without it a row is a line. Sakura calls them layout lines (`CLayout`) and logical lines
//! (`CDocLine`). Positions are always buffer positions ([`Pos`]); the horizontal position on a row
//! is an [`X`] (ADR-0034), the unit of the layout, which is Sakura's layout X (`CLayoutXInt`).
//! A half-width column is [`Layout::column_x`] of them. Code that moves "one column" adds
//! `column_x`, and code that counts characters multiplies; a bare number is never an `X`.

use matsu_core::buffer::{Document, Pos, Result};
pub use matsu_layout::X;

use crate::context::TextChange;

/// The layout of the document that the command runs on. The layout must match the document: the
/// methods are given the current [`Document`] on every call and may read it.
///
/// The layout is told about every edit right after the buffer changed ([`edited`](Self::edited)),
/// and about undo and redo ([`reset`](Self::reset)), so the methods below always see a layout
/// that matches the document, even in the middle of a command or a macro (ADR-0013 section 4).
pub trait Layout {
    /// The text changed as described by `change`; `doc` is the document after the edit. Called
    /// by [`CommandContext`](crate::context::CommandContext) after each edit, before the command
    /// goes on. A layout that keeps no state of its own can ignore it.
    fn edited(&mut self, doc: &Document, change: &TextChange);

    /// The document was replaced by an older or newer version (undo, redo) and the edits are not
    /// known: lay out `doc` again.
    fn reset(&mut self, doc: &Document);

    /// Makes the rows of line `line` exact. After an edit of very many lines at once a layout may
    /// only estimate their rows (one row each) until it has laid them out; the row functions of
    /// that line are not right before. A layout that always knows them has nothing to do.
    fn make_exact(&mut self, _doc: &Document, _line: usize) {}

    /// The horizontal position of `pos` counted from the left edge of the screen, so the indent
    /// of a wrapped row is included (Sakura: the layout X of the caret).
    fn display_x(&self, doc: &Document, pos: Pos) -> Result<X>;

    /// The first position of the row that contains `pos`.
    fn row_start(&self, doc: &Document, pos: Pos) -> Result<Pos>;

    /// The last position of the row that contains `pos`: before the line ending on the last row
    /// of a line, and at the wrap point on the others.
    fn row_end(&self, doc: &Document, pos: Pos) -> Result<Pos>;

    /// The first position of the row after the one that contains `pos`, or `None` on the last
    /// row. The empty row after a final line ending counts as a row.
    fn next_row_start(&self, doc: &Document, pos: Pos) -> Result<Option<Pos>>;

    /// The position on the row of `row_pos` that is at `x`: the char that covers it (the start
    /// of it), or the row end if `x` is past the text.
    fn pos_at_x(&self, doc: &Document, row_pos: Pos, x: X) -> Result<Pos>;

    /// Moves `rows` rows down (negative: up) from `pos`, stopping at the first and the last row,
    /// and returns the position at `goal` there. Sakura moves to the goal of the same row when it
    /// cannot move further (`CCaret::Cursor_UPDOWN`).
    fn move_rows(&self, doc: &Document, pos: Pos, rows: isize, goal: X) -> Result<Pos>;

    /// The width of a tab stop in columns, at least 1: the tab width of the document type that
    /// the layout uses (`CLayoutMgr::GetTabSpaceKetas`). It is the number of spaces that a soft
    /// tab is made of. [`tab_x`](Self::tab_x) is the same width as an `X`.
    fn tab_width(&self) -> usize;

    /// The width of a tab stop as an `X`: the stop that [`display_x`](Self::display_x) counts
    /// tabs with (`CLayoutMgr::GetTabSpace`).
    fn tab_x(&self) -> X;

    /// One half-width column as an `X`, the space that the caret moves over where there is no
    /// text (`CTextMetrics::GetLayoutXDefault`, ADR-0034 `per_column`).
    fn column_x(&self) -> X;

    /// The width of a row that the wrap column makes, the rightmost `X` of a box or a free caret
    /// (`CLayoutMgr::GetMaxLineLayout`). Without wrapping Sakura breaks a line at 10240 columns,
    /// which is the default here.
    fn max_line_x(&self) -> X {
        self.column_x()
            .saturating_mul(crate::context::MAX_BOX_COLUMNS)
    }

    /// The number of columns of a char other than a tab: 1 or 2 (`CNativeW::GetKetaOfChar`). The
    /// default is the short table of [`PlainLayout`]; a real layout answers with its own rule.
    fn char_columns(&self, c: char) -> usize {
        char_width(c)
    }

    /// The width of a char other than a tab as an `X` (`CNativeW::GetColmOfChar`): the columns
    /// times [`column_x`](Self::column_x) on a fixed grid; a layout that measures each char
    /// answers with the width of the char.
    fn char_x(&self, c: char) -> X {
        self.column_x().saturating_mul(self.char_columns(c) as u32)
    }

    /// The index of the row that contains `pos`, counted from 0 over the whole document (Sakura:
    /// the layout line number, `CLayout` y). The default walks the rows from the start, which is
    /// slow on a long document; a layout that keeps an index overrides it.
    fn row_index(&self, doc: &Document, pos: Pos) -> Result<usize> {
        let target = self.row_start(doc, pos)?;
        let mut at = Pos::new(0, 0);
        let mut index = 0;
        while at < target {
            match self.next_row_start(doc, at)? {
                Some(next) => at = next,
                None => break,
            }
            index += 1;
        }
        Ok(index)
    }

    /// The first position of row `row` (from 0), or `None` if the document has fewer rows. The
    /// default walks the rows from the start, like [`row_index`](Self::row_index).
    fn row_start_at(&self, doc: &Document, row: usize) -> Result<Option<Pos>> {
        let mut at = Pos::new(0, 0);
        for _ in 0..row {
            match self.next_row_start(doc, at)? {
                Some(next) => at = next,
                None => return Ok(None),
            }
        }
        Ok(Some(at))
    }
}

/// A layout without wrapping: a row is a line. Tabs go to the next multiple of 4; chars of East
/// Asian full width take 2 cells; the rest take 1.
///
/// The width table is a short approximation of Sakura's, which the real layout will replace
/// (`sakura_core/charset/charcode.cpp`, `CharWidthCache`).
#[derive(Clone, Copy, Debug)]
pub struct PlainLayout {
    pub tab_width: usize,
    /// `X` in one column. 1 gives the layout that counts in columns; a larger one is the same
    /// layout in finer units (ADR-0034: the tests run both and expect the same text).
    pub per_column: u32,
}

impl Default for PlainLayout {
    fn default() -> Self {
        Self {
            tab_width: 4,
            per_column: 1,
        }
    }
}

/// The cells of a char other than a tab (an approximation, see [`PlainLayout`]).
fn char_width(c: char) -> usize {
    match u32::from(c) {
        0x1100..=0x115F
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1F64F
        | 0x20000..=0x3FFFD => 2,
        _ => 1,
    }
}

impl PlainLayout {
    /// The layout with `per_column` `X` in a column.
    pub fn scaled(per_column: u32) -> Self {
        Self {
            per_column,
            ..Self::default()
        }
    }

    /// The position after `c` when it starts at `x`.
    fn advance(self, x: X, c: char) -> X {
        if c == '\t' {
            let w = self.tab_x();
            x + (w - x % w)
        } else {
            x + self.char_x(c)
        }
    }

    fn line_text(doc: &Document, line: usize) -> Result<String> {
        Ok(doc.line(line)?.map(|l| l.text).unwrap_or_default())
    }

    /// The last row: the last line, or the empty line after a final line ending.
    fn last_row(doc: &Document) -> Result<usize> {
        Ok(doc.end_pos()?.line)
    }
}

impl Layout for PlainLayout {
    // A row is a line: the row functions are direct (the defaults walk the rows from the start of
    // the document, which makes a loop over many lines quadratic).
    fn row_index(&self, doc: &Document, pos: Pos) -> Result<usize> {
        Ok(doc.clamp_pos(pos)?.line)
    }

    fn row_start_at(&self, doc: &Document, row: usize) -> Result<Option<Pos>> {
        Ok((row <= Self::last_row(doc)?).then(|| Pos::new(row, 0)))
    }

    fn tab_width(&self) -> usize {
        self.tab_width.max(1)
    }

    fn tab_x(&self) -> X {
        self.column_x().saturating_mul(self.tab_width() as u32)
    }

    fn column_x(&self) -> X {
        X::from_raw(self.per_column.max(1))
    }

    // Rows are computed from the document on every call, so there is nothing to update.
    fn edited(&mut self, _doc: &Document, _change: &TextChange) {}

    fn reset(&mut self, _doc: &Document) {}

    fn display_x(&self, doc: &Document, pos: Pos) -> Result<X> {
        let pos = doc.clamp_pos(pos)?;
        let text = Self::line_text(doc, pos.line)?;
        let upto = text.get(..pos.col).unwrap_or(&text);
        Ok(upto.chars().fold(X::ZERO, |x, c| self.advance(x, c)))
    }

    fn row_start(&self, doc: &Document, pos: Pos) -> Result<Pos> {
        Ok(Pos::new(doc.clamp_pos(pos)?.line, 0))
    }

    fn row_end(&self, doc: &Document, pos: Pos) -> Result<Pos> {
        let line = doc.clamp_pos(pos)?.line;
        Ok(Pos::new(line, doc.line_len(line)?.unwrap_or(0)))
    }

    fn next_row_start(&self, doc: &Document, pos: Pos) -> Result<Option<Pos>> {
        let line = doc.clamp_pos(pos)?.line;
        Ok((line < Self::last_row(doc)?).then(|| Pos::new(line + 1, 0)))
    }

    fn pos_at_x(&self, doc: &Document, row_pos: Pos, x: X) -> Result<Pos> {
        let line = doc.clamp_pos(row_pos)?.line;
        let text = Self::line_text(doc, line)?;
        let mut at = X::ZERO;
        for (i, c) in text.char_indices() {
            let next = self.advance(at, c);
            if next > x {
                return Ok(Pos::new(line, i));
            }
            at = next;
        }
        Ok(Pos::new(line, text.len()))
    }

    fn move_rows(&self, doc: &Document, pos: Pos, rows: isize, goal: X) -> Result<Pos> {
        let line = doc.clamp_pos(pos)?.line;
        let target = line.saturating_add_signed(rows).min(Self::last_row(doc)?);
        self.pos_at_x(doc, Pos::new(target, 0), goal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(s: &str) -> Document {
        Document::from_utf8(s.as_bytes().to_vec()).unwrap()
    }

    /// The layouts that must agree in columns: one `X` for a column, and the finer unit that the
    /// screen will use (ADR-0034).
    const PER_COLUMN: [u32; 2] = [1, 64];

    fn x(l: &PlainLayout, columns: u32) -> X {
        X::from_raw(columns * l.per_column)
    }

    #[test]
    fn display_columns_count_tabs_and_full_width_chars() {
        let d = doc("a\tb日c");
        for per_column in PER_COLUMN {
            let l = PlainLayout::scaled(per_column);
            let cols: Vec<_> = [0, 1, 2, 3, 6, 7]
                .iter()
                .map(|&c| l.display_x(&d, Pos::new(0, c)).unwrap())
                .collect();
            let want: Vec<_> = [0, 1, 4, 5, 7, 8].iter().map(|&c| x(&l, c)).collect();
            assert_eq!(cols, want, "per_column {per_column}");
        }
    }

    #[test]
    fn pos_at_x_picks_the_char_that_covers_the_position() {
        let d = doc("a\tb日c");
        for per_column in PER_COLUMN {
            let l = PlainLayout::scaled(per_column);
            let at = |col| l.pos_at_x(&d, Pos::new(0, 0), x(&l, col)).unwrap().col;
            assert_eq!(at(0), 0);
            assert_eq!(at(1), 1); // the tab covers columns 1..4
            assert_eq!(at(3), 1);
            assert_eq!(at(4), 2);
            assert_eq!(at(5), 3); // the full-width char covers columns 5..7
            assert_eq!(at(6), 3);
            assert_eq!(at(7), 6);
            assert_eq!(at(99), 7);
        }
    }

    #[test]
    fn a_position_inside_a_column_belongs_to_the_char_that_covers_it() {
        let d = doc("a\tb日c");
        let l = PlainLayout::scaled(64);
        // the last unit of the tab (columns 1..4) and the first of the full-width char (5..7)
        let at = |raw| {
            l.pos_at_x(&d, Pos::new(0, 0), X::from_raw(raw))
                .unwrap()
                .col
        };
        assert_eq!(at(64 * 4 - 1), 1);
        assert_eq!(at(64 * 5 + 1), 3);
        assert_eq!(at(64 * 7 - 1), 3);
    }

    #[test]
    fn rows_include_the_empty_line_after_a_final_line_ending() {
        let d = doc("ab\n");
        for per_column in PER_COLUMN {
            let l = PlainLayout::scaled(per_column);
            assert_eq!(
                l.next_row_start(&d, Pos::new(0, 1)).unwrap(),
                Some(Pos::new(1, 0))
            );
            assert_eq!(l.next_row_start(&d, Pos::new(1, 0)).unwrap(), None);
            assert_eq!(
                l.move_rows(&d, Pos::new(0, 2), 5, x(&l, 2)).unwrap(),
                Pos::new(1, 0)
            );
            assert_eq!(
                l.move_rows(&d, Pos::new(1, 0), -9, x(&l, 1)).unwrap(),
                Pos::new(0, 1)
            );
        }
    }

    #[test]
    fn row_ends_before_the_line_ending() {
        let d = doc("ab\r\ncd");
        let l = PlainLayout::default();
        assert_eq!(l.row_end(&d, Pos::new(0, 0)).unwrap(), Pos::new(0, 2));
        assert_eq!(l.row_end(&d, Pos::new(1, 0)).unwrap(), Pos::new(1, 2));
        assert_eq!(l.row_start(&d, Pos::new(1, 2)).unwrap(), Pos::new(1, 0));
    }
}
