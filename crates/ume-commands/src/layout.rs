//! What commands need to know about how the text is laid out on screen.
//!
//! Up and down, the display column, and the start and end of a *row* depend on the layout (tabs,
//! full-width chars, wrapping). `ume-ui` will implement [`Layout`] with the real layout of
//! `ume-layout` (ADR-0013); until it exists, [`PlainLayout`] is the simple one that the tests use:
//! no wrapping, tab stops every 4 columns, full-width chars take 2 columns.
//!
//! A *row* is one line on the screen. With wrapping one line of the document has several rows;
//! without it a row is a line. Sakura calls them layout lines (`CLayout`) and logical lines
//! (`CDocLine`). Positions are always buffer positions ([`Pos`]); only *display columns* are in
//! cells.

use ume_core::buffer::{Document, Pos, Result};

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

    /// The display column of `pos` counted from the start of its row, in cells.
    fn display_col(&self, doc: &Document, pos: Pos) -> Result<usize>;

    /// The first position of the row that contains `pos`.
    fn row_start(&self, doc: &Document, pos: Pos) -> Result<Pos>;

    /// The last position of the row that contains `pos`: before the line ending on the last row
    /// of a line, and at the wrap point on the others.
    fn row_end(&self, doc: &Document, pos: Pos) -> Result<Pos>;

    /// The first position of the row after the one that contains `pos`, or `None` on the last
    /// row. The empty row after a final line ending counts as a row.
    fn next_row_start(&self, doc: &Document, pos: Pos) -> Result<Option<Pos>>;

    /// The position on the row of `row_pos` that is at display column `col`: the char that
    /// covers the column (the start of it), or the row end if the column is past the text.
    fn pos_at_col(&self, doc: &Document, row_pos: Pos, col: usize) -> Result<Pos>;

    /// Moves `rows` rows down (negative: up) from `pos`, stopping at the first and the last row,
    /// and returns the position at display column `goal_col` there. Sakura moves to the goal
    /// column of the same row when it cannot move further (`CCaret::Cursor_UPDOWN`).
    fn move_rows(&self, doc: &Document, pos: Pos, rows: isize, goal_col: usize) -> Result<Pos>;

    /// The width of a tab stop in display columns, at least 1: the tab width of the document
    /// type that the layout uses (`CLayoutMgr::GetTabSpaceKetas`). It must be the width that
    /// [`display_col`](Self::display_col) counts tabs with.
    fn tab_width(&self) -> usize;
}

/// A layout without wrapping: a row is a line. Tabs go to the next multiple of 4; chars of East
/// Asian full width take 2 cells; the rest take 1.
///
/// The width table is a short approximation of Sakura's, which the real layout will replace
/// (`sakura_core/charset/charcode.cpp`, `CharWidthCache`).
#[derive(Clone, Copy, Debug)]
pub struct PlainLayout {
    pub tab_width: usize,
}

impl Default for PlainLayout {
    fn default() -> Self {
        Self { tab_width: 4 }
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
    /// The column after `c` when it starts at column `col`.
    fn advance(&self, col: usize, c: char) -> usize {
        if c == '\t' {
            let w = self.tab_width.max(1);
            (col / w + 1) * w
        } else {
            col + char_width(c)
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
    fn tab_width(&self) -> usize {
        self.tab_width.max(1)
    }

    // Rows are computed from the document on every call, so there is nothing to update.
    fn edited(&mut self, _doc: &Document, _change: &TextChange) {}

    fn reset(&mut self, _doc: &Document) {}

    fn display_col(&self, doc: &Document, pos: Pos) -> Result<usize> {
        let pos = doc.clamp_pos(pos)?;
        let text = Self::line_text(doc, pos.line)?;
        let upto = text.get(..pos.col).unwrap_or(&text);
        Ok(upto.chars().fold(0, |col, c| self.advance(col, c)))
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

    fn pos_at_col(&self, doc: &Document, row_pos: Pos, col: usize) -> Result<Pos> {
        let line = doc.clamp_pos(row_pos)?.line;
        let text = Self::line_text(doc, line)?;
        let mut at = 0;
        for (i, c) in text.char_indices() {
            let next = self.advance(at, c);
            if next > col {
                return Ok(Pos::new(line, i));
            }
            at = next;
        }
        Ok(Pos::new(line, text.len()))
    }

    fn move_rows(&self, doc: &Document, pos: Pos, rows: isize, goal_col: usize) -> Result<Pos> {
        let line = doc.clamp_pos(pos)?.line;
        let target = line.saturating_add_signed(rows).min(Self::last_row(doc)?);
        self.pos_at_col(doc, Pos::new(target, 0), goal_col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(s: &str) -> Document {
        Document::from_utf8(s.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn display_columns_count_tabs_and_full_width_chars() {
        let d = doc("a\tb日c");
        let l = PlainLayout::default();
        let cols: Vec<_> = [0, 1, 2, 3, 6, 7]
            .iter()
            .map(|&c| l.display_col(&d, Pos::new(0, c)).unwrap())
            .collect();
        assert_eq!(cols, [0, 1, 4, 5, 7, 8]);
    }

    #[test]
    fn pos_at_col_picks_the_char_that_covers_the_column() {
        let d = doc("a\tb日c");
        let l = PlainLayout::default();
        let at = |col| l.pos_at_col(&d, Pos::new(0, 0), col).unwrap().col;
        assert_eq!(at(0), 0);
        assert_eq!(at(1), 1); // the tab covers columns 1..4
        assert_eq!(at(3), 1);
        assert_eq!(at(4), 2);
        assert_eq!(at(5), 3); // the full-width char covers columns 5..7
        assert_eq!(at(6), 3);
        assert_eq!(at(7), 6);
        assert_eq!(at(99), 7);
    }

    #[test]
    fn rows_include_the_empty_line_after_a_final_line_ending() {
        let d = doc("ab\n");
        let l = PlainLayout::default();
        assert_eq!(
            l.next_row_start(&d, Pos::new(0, 1)).unwrap(),
            Some(Pos::new(1, 0))
        );
        assert_eq!(l.next_row_start(&d, Pos::new(1, 0)).unwrap(), None);
        assert_eq!(
            l.move_rows(&d, Pos::new(0, 2), 5, 2).unwrap(),
            Pos::new(1, 0)
        );
        assert_eq!(
            l.move_rows(&d, Pos::new(1, 0), -9, 1).unwrap(),
            Pos::new(0, 1)
        );
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
