//! Which columns of the visible rows hold a match of the current pattern (Sakura's 検索文字列の
//! 強調表示, `sakura_core/view/CEditView_Search.cpp` and the search colour of
//! `CTypeSupport`/`CColor_Found`). Pure arithmetic over the layout, tested without a window.

use ume_core::buffer::Document;
use ume_core::search::Matcher;
use ume_layout::{DocumentLayout, RowInfo};

/// Lines longer than this are not searched for the highlight: the highlight is redone on every
/// paint, and a line this long is searched whole (matchers see whole lines).
pub const MAX_HIGHLIGHT_LINE_BYTES: usize = 64 * 1024;

/// For each row, the column ranges `(from, to)` of the matches on it, in order. Empty matches
/// have no width and are left out. A match that wraps onto the next row is cut at the row end.
pub fn highlight_columns(
    layout: &DocumentLayout,
    doc: &Document,
    rows: &[RowInfo],
    matcher: &dyn Matcher,
) -> Vec<Vec<(u32, u32)>> {
    let mut cached: Option<(usize, Vec<Range>)> = None;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        if row.is_eof_row {
            out.push(Vec::new());
            continue;
        }
        if cached.as_ref().is_none_or(|(line, _)| *line != row.line) {
            cached = Some((row.line, matches_of_line(doc, row.line, matcher)));
        }
        let matches = cached.as_ref().map_or(&[][..], |(_, m)| m.as_slice());
        let mut walk = ColumnWalk::new(layout, row);
        let mut cols = Vec::new();
        for m in matches {
            if m.end <= row.start.col {
                continue;
            }
            if m.start >= row.end.col {
                break;
            }
            // the part of the match on this row, in columns
            let from = walk.column_at(m.start.max(row.start.col));
            let to = walk.column_at(m.end.min(row.end.col));
            if to > from {
                cols.push((from, to));
            }
        }
        out.push(cols);
    }
    out
}

type Range = std::ops::Range<usize>;

/// Turns the byte offsets of a row, asked for in increasing order, into columns in one pass over
/// the row's text (the matches of a long line must not each rescan the row).
struct ColumnWalk<'a> {
    layout: &'a DocumentLayout,
    chars: std::str::CharIndices<'a>,
    /// The line offset and the column at the start of the chars not yet read.
    at: usize,
    col: u32,
    /// The offset of the row in its line.
    base: usize,
}

impl<'a> ColumnWalk<'a> {
    fn new(layout: &'a DocumentLayout, row: &'a RowInfo) -> Self {
        Self {
            layout,
            chars: row.text().char_indices(),
            at: row.start.col,
            col: row.start_column,
            base: row.start.col,
        }
    }

    /// The column of the line offset `target` (at the row's end if it is past it).
    fn column_at(&mut self, target: usize) -> u32 {
        while self.at < target {
            let Some((i, c)) = self.chars.next() else {
                break;
            };
            self.col += self.layout.config().columns_of(c, self.col);
            self.at = self.base + i + c.len_utf8();
        }
        self.col
    }
}

/// The non-empty matches of one line, as byte ranges. A line that cannot be read, is too long, or
/// makes the matcher fail has none.
fn matches_of_line(doc: &Document, line: usize, matcher: &dyn Matcher) -> Vec<Range> {
    let Ok(Some(text)) = doc.line(line).map(|l| l.map(|l| l.text)) else {
        return Vec::new();
    };
    if text.len() > MAX_HIGHLIGHT_LINE_BYTES {
        return Vec::new();
    }
    matcher
        .find_all_from(&text, 0)
        .unwrap_or_default()
        .into_iter()
        .filter(|m| !m.is_empty())
        .map(|m| m.range())
        .collect()
}

#[cfg(test)]
mod tests {
    use ume_core::search::{PlainMatcher, PlainOptions};
    use ume_layout::{LayoutConfig, Wrap};

    use super::*;

    fn columns(text: &str, pattern: &str, cfg: LayoutConfig) -> Vec<Vec<(u32, u32)>> {
        let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
        let layout = DocumentLayout::laid_out(&doc, cfg).unwrap();
        let rows = layout.rows(&doc, 0, 100).unwrap();
        let matcher = PlainMatcher::new(pattern, PlainOptions::default());
        highlight_columns(&layout, &doc, &rows, &matcher)
    }

    #[test]
    fn matches_are_columns_of_their_row() {
        let rows = columns("abab\nxab", "ab", LayoutConfig::default());
        assert_eq!(rows[0], [(0, 2), (2, 4)]);
        assert_eq!(rows[1], [(1, 3)]);
    }

    #[test]
    fn columns_count_wide_chars_and_tabs() {
        let rows = columns("日本\tab", "ab", LayoutConfig::default());
        // 日本 is 4 columns, the tab goes to the next stop (column 8)
        assert_eq!(rows[0], [(8, 10)]);
        let rows = columns("日本語", "本", LayoutConfig::default());
        assert_eq!(rows[0], [(2, 4)]);
    }

    #[test]
    fn a_match_on_a_wrapped_line_is_cut_at_the_row() {
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(4),
            ..LayoutConfig::default()
        };
        let rows = columns("xxabcd", "abcd", cfg);
        assert_eq!(rows[0], [(2, 4)]);
        assert_eq!(rows[1], [(0, 2)]);
    }

    #[test]
    fn no_match_no_columns() {
        let rows = columns("abc\n", "zz", LayoutConfig::default());
        assert!(rows.iter().all(Vec::is_empty));
    }

    #[test]
    fn many_matches_on_a_long_line_take_one_pass() {
        // 60000 matches on one row; a rescan of the row per match would be 3.6 billion steps
        let text = "a".repeat(MAX_HIGHLIGHT_LINE_BYTES - 4);
        let rows = columns(&text, "a", LayoutConfig::default());
        // Sakura breaks a line at 10240 columns even without wrapping
        let total: usize = rows.iter().map(Vec::len).sum();
        assert_eq!(total, text.len());
    }

    #[test]
    fn a_very_long_line_is_not_searched() {
        let text = "a".repeat(MAX_HIGHLIGHT_LINE_BYTES + 1);
        let rows = columns(&text, "a", LayoutConfig::default());
        assert!(rows[0].is_empty());
    }
}
