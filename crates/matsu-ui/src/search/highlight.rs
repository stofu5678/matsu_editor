//! Which columns of the visible rows hold a match of the current pattern (Sakura's 検索文字列の
//! 強調表示, `sakura_core/view/CEditView_Search.cpp` and the search colour of
//! `CTypeSupport`/`CColor_Found`). Pure arithmetic over the layout, tested without a window.
//!
//! Sakura colors a line together with its line ending (`CEditView::IsSearchString` gets the line
//! with its `EOL` from `CColor_Found`), so a regular expression that matches the line break
//! (`\r\n`, `\s`, `[^a]`) colors the line ending mark too: one column after the end of the text of
//! the last row of the line.

use matsu_core::buffer::Document;
use matsu_core::search::Matcher;
use matsu_layout::{DocumentLayout, RowInfo};

/// Lines longer than this are not searched for the highlight: the highlight is redone on every
/// paint, and a line this long is searched whole (matchers see whole lines).
pub const MAX_HIGHLIGHT_LINE_BYTES: usize = 64 * 1024;

/// A match on a row: the columns it covers, and which of the search colors it is drawn in (0 is
/// the first, up to 4 for `FN5`, counted over the colors that are shown; see [`slot_of`]).
pub type RowMatch = (u32, u32, u8);

/// For each row, the matches on it, in order. Empty matches have no width and are left out. A
/// match that wraps onto the next row is cut at the row end. A match that covers the line ending
/// adds one column for it after the text of the last row of the line. `slot_of` says which of
/// the search colors a matched text is drawn in.
pub fn highlight_columns(
    layout: &DocumentLayout,
    doc: &Document,
    rows: &[RowInfo],
    matcher: &dyn Matcher,
    slot_of: &dyn Fn(&str) -> u8,
) -> Vec<Vec<RowMatch>> {
    let mut cached: Option<(usize, LineMatches)> = None;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        if row.is_eof_row {
            out.push(Vec::new());
            continue;
        }
        if cached.as_ref().is_none_or(|(line, _)| *line != row.line) {
            cached = Some((row.line, matches_of_line(doc, row.line, matcher, slot_of)));
        }
        let line_matches = cached.as_ref().map(|(_, m)| m);
        let matches = line_matches.map_or(&[][..], |m| m.text.as_slice());
        let mut walk = ColumnWalk::new(layout, row);
        let mut cols = Vec::new();
        for (m, slot) in matches {
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
                cols.push((from, to, *slot));
            }
        }
        if row.eol.is_some()
            && let Some(slot) = line_matches.and_then(|m| m.eol)
        {
            cols.push((row.end_column, row.end_column + 1, slot));
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

/// The color of a match of a whole-word search (`CEditView::IsSearchString`): the number of the
/// word of the key it is, counting from 0, and round and round over the colors that are shown
/// (`CColor_Found::GetStrategyColor`: `(result - 1) % validColorNum`). Any other search has the
/// first color.
pub fn slot_of(word_index: Option<usize>, shown: usize) -> u8 {
    match (word_index, shown) {
        (Some(i), n) if n > 0 => (i % n) as u8,
        _ => 0,
    }
}

/// The matches of one line: the byte ranges in its text, and the color of the first match that
/// covers the line ending.
#[derive(Default)]
struct LineMatches {
    text: Vec<(Range, u8)>,
    eol: Option<u8>,
}

/// The non-empty matches of one line, as byte ranges. A line that cannot be read, is too long, or
/// makes the matcher fail has none.
fn matches_of_line(
    doc: &Document,
    line: usize,
    matcher: &dyn Matcher,
    slot_of: &dyn Fn(&str) -> u8,
) -> LineMatches {
    // the length first: a huge line must not be decoded on every paint
    if !matches!(doc.line_len(line), Ok(Some(len)) if len <= MAX_HIGHLIGHT_LINE_BYTES) {
        return LineMatches::default();
    }
    let Ok(Some(l)) = doc.line(line) else {
        return LineMatches::default();
    };
    let body = l.text.len();
    let eol = l.eol.filter(|_| matcher.sees_line_ending());
    let mut subject = l.text;
    if let Some(eol) = eol {
        subject.push_str(eol.as_str());
    }
    let mut out = LineMatches::default();
    let all = matcher.find_all_from(&subject, body, 0).unwrap_or_default();
    for m in all.into_iter().filter(|m| !m.is_empty()) {
        let slot = slot_of(&subject[m.start.min(body)..m.end.min(body)]);
        if m.start < body {
            out.text.push((m.start..m.end.min(body), slot));
        }
        if m.end > body && out.eol.is_none() {
            out.eol = Some(slot);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use matsu_core::search::{PlainMatcher, PlainOptions};
    use matsu_layout::{LayoutConfig, Wrap};

    use super::*;

    fn columns(text: &str, pattern: &str, cfg: LayoutConfig) -> Vec<Vec<(u32, u32)>> {
        slots(text, pattern, cfg, &|_| 0)
            .into_iter()
            .map(|row| row.into_iter().map(|(a, b, _)| (a, b)).collect())
            .collect()
    }

    fn slots(
        text: &str,
        pattern: &str,
        cfg: LayoutConfig,
        slot_of: &dyn Fn(&str) -> u8,
    ) -> Vec<Vec<RowMatch>> {
        let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
        let layout = DocumentLayout::laid_out(&doc, cfg).unwrap();
        let rows = layout.rows(&doc, 0, 100).unwrap();
        let matcher = PlainMatcher::new(pattern, PlainOptions::default());
        highlight_columns(&layout, &doc, &rows, &matcher, slot_of)
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
    fn a_regex_that_matches_the_line_break_colors_its_mark() {
        use matsu_regex::{RegexMatcher, RegexSearchOptions};
        let doc = Document::from_utf8(b"ab\r\ncd\r\nef".to_vec()).unwrap();
        let layout = DocumentLayout::laid_out(&doc, LayoutConfig::default()).unwrap();
        let rows = layout.rows(&doc, 0, 100).unwrap();
        let matcher = RegexMatcher::new(r"b\r\n|f", RegexSearchOptions::default()).unwrap();
        let cols = highlight_columns(&layout, &doc, &rows, &matcher, &|_| 0);
        // "b" and the mark after it (column 2); the last line has no line ending
        assert_eq!(cols[0], [(1, 2, 0), (2, 3, 0)]);
        assert!(cols[1].is_empty());
        assert_eq!(cols[2], [(1, 2, 0)]);
    }

    #[test]
    fn a_very_long_line_is_not_searched() {
        let text = "a".repeat(MAX_HIGHLIGHT_LINE_BYTES + 1);
        let rows = columns(&text, "a", LayoutConfig::default());
        assert!(rows[0].is_empty());
    }

    #[test]
    fn each_match_gets_the_slot_the_caller_gives_its_text() {
        let rows = slots("ab xx ab", "ab", LayoutConfig::default(), &|text| {
            if text == "ab" { 3 } else { 0 }
        });
        assert_eq!(rows[0], [(0, 2, 3), (6, 8, 3)]);
    }

    #[test]
    fn the_slot_of_a_word_goes_round_the_colors_that_are_shown() {
        // Sakura: `highlightColors[(m_nSearchResult - 1) % validColorNum]`
        assert_eq!(slot_of(Some(0), 5), 0);
        assert_eq!(slot_of(Some(4), 5), 4);
        assert_eq!(slot_of(Some(5), 5), 0);
        assert_eq!(slot_of(Some(3), 2), 1);
        // not a whole-word search, or no color is shown
        assert_eq!(slot_of(None, 5), 0);
        assert_eq!(slot_of(Some(2), 0), 0);
    }
}
