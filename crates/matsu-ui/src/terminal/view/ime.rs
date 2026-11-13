//! The text input of the terminal (ADR-0028, "IME"): GPUI's input handler for the view.
//!
//! What the platform delivers as text comes here and not as keys: the text of the keys (the key handler leaves a
//! key with text unhandled, see `input::Translated::Text`), what an IME commits, what an on-screen keyboard
//! or `SendInput`'s Unicode input (`VK_PACKET`) types, and the dead keys (a dead key's accent is not text
//! until the next key says which character it makes).
//!
//! * Committed text ([`EntityInputHandler::replace_text_in_range`]) is written to the program as UTF-8, as
//!   typed text; it is not a paste, so no bracketed paste markers go around it.
//! * Composition (`replace_and_mark_text_in_range`, `unmark_text`) is the preedit: the view keeps the text,
//!   `TerminalElement` draws it at the terminal's cursor, underlined, over the grid, and nothing is sent to
//!   the program until it is committed. The terminal has no text of its own for the IME to read back, so the
//!   text of the input handler is the preedit alone.
//! * While the preedit exists the keys are the IME's (see `TerminalView::on_key_down`).
//! * `bounds_for_range` is the cell at the cursor (past the preedit text the IME asks about), so that the
//!   candidate window opens there.

use std::ops::Range;

use gpui::{Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window, px, size};
use unicode_segmentation::UnicodeSegmentation as _;
use unicode_width::UnicodeWidthStr as _;

use super::TerminalView;

/// The text being composed: nothing of it is written until the IME commits it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Marked {
    pub text: String,
    /// The IME's selection (a caret when it is empty) in `text`, in UTF-16 units.
    pub selection: Range<usize>,
}

/// A character of the preedit on the grid: a grapheme cluster, the columns it covers and where it is, as a
/// distance from the cell of the cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreeditCell {
    pub text: String,
    /// Rows below the cursor's row (the preedit wraps at the last column).
    pub row: usize,
    pub column: usize,
    /// The columns the cluster covers: 1 or 2.
    pub span: usize,
    /// Where the cluster starts in the preedit, in UTF-16 units.
    pub utf16: usize,
}

/// Puts the preedit on the grid from `column` on, `columns` wide. A cluster that does not fit at the end of a
/// row goes to the start of the next, as a program's wide character does.
pub(crate) fn preedit_cells(text: &str, column: usize, columns: usize) -> Vec<PreeditCell> {
    let columns = columns.max(2);
    let (mut row, mut at) = (0, column.min(columns - 1));
    let mut utf16 = 0;
    let mut cells = Vec::new();
    for cluster in text.graphemes(true) {
        // A cluster of no width (a lone combining mark) takes a cell, as the IME's text shows it.
        let span = cluster.width().clamp(1, 2);
        if at + span > columns {
            row += 1;
            at = 0;
        }
        cells.push(PreeditCell {
            text: cluster.to_owned(),
            row,
            column: at,
            span,
            utf16,
        });
        at += span;
        utf16 += cluster.encode_utf16().count();
    }
    cells
}

/// How many rows the preedit is moved up so that its last row is on the screen: a preedit that begins near the
/// right end of the bottom row wraps below the screen, and it is drawn over the rows above instead.
pub(crate) fn rows_up(cells: &[PreeditCell], cursor_row: usize, rows: usize) -> usize {
    let last = cells.last().map_or(0, |cell| cell.row);
    (cursor_row + last + 1).saturating_sub(rows)
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The part of `text` in `range` (UTF-16 units), the range pulled to the character boundaries inside the
/// text, and that range.
fn slice_utf16(text: &str, range: &Range<usize>) -> (String, Range<usize>) {
    let mut start = None;
    let mut end = None;
    let mut offset = 0;
    let mut slice = String::new();
    for c in text.chars() {
        if start.is_none() && offset >= range.start {
            start = Some(offset);
        }
        if end.is_none() && offset >= range.end {
            end = Some(offset);
        }
        if start.is_some() && end.is_none() {
            slice.push(c);
        }
        offset += c.len_utf16();
    }
    let total = offset;
    let start = start.unwrap_or(total);
    let end = end.unwrap_or(total).max(start);
    (slice, start..end)
}

/// The preedit after `text` takes the place of `range` (UTF-16 units) of the preedit there is, and where `text`
/// starts in it (UTF-16 units). Without a preedit, or without a range, `text` is all of it.
fn replaced(old: Option<&str>, range: Option<&Range<usize>>, text: &str) -> (String, usize) {
    let (Some(old), Some(range)) = (old, range) else {
        return (text.to_owned(), 0);
    };
    let (_, actual) = slice_utf16(old, range);
    let (before, _) = slice_utf16(old, &(0..actual.start));
    let (after, _) = slice_utf16(old, &(actual.end..usize::MAX));
    (format!("{before}{text}{after}"), actual.start)
}

/// The text of a composition as the terminal keeps it: a line break would run a command, so none.
fn composition_text(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

impl TerminalView {
    /// The text being composed, if the IME is composing.
    pub(crate) fn preedit(&self) -> Option<&str> {
        self.marked.as_ref().map(|marked| marked.text.as_str())
    }

    /// The text being composed and the IME's selection in it (UTF-16 units), for drawing.
    pub(crate) fn preedit_with_selection(&self) -> Option<(String, Range<usize>)> {
        self.marked
            .as_ref()
            .map(|marked| (marked.text.clone(), marked.selection.clone()))
    }

    /// Whether the IME is composing: the keys are its.
    pub(crate) fn is_composing(&self) -> bool {
        self.marked.is_some()
    }

    /// Writes typed or committed text to the program, as UTF-8 (see `send_typed`).
    pub(super) fn send_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.send_typed(text.as_bytes());
    }

    /// The preedit is over (committed or cancelled).
    pub(super) fn end_composition(&mut self) {
        self.marked = None;
    }

    /// Where the cell of the cursor is, in the window, and the size of a cell.
    fn cursor_origin(&self) -> Option<(Point<Pixels>, f32, f32)> {
        let layout = self.layout?;
        let info = self.terminal.info();
        let row = i64::from(info.cursor.row) + info.display_offset as i64;
        let x = layout.bounds.left() + px(info.cursor.column as f32 * layout.cell.width);
        let y = layout.bounds.top() + px(row as f32 * layout.cell.height);
        Some((Point { x, y }, layout.cell.width, layout.cell.height))
    }
}

impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let marked = self.marked.as_ref()?;
        let (slice, actual) = slice_utf16(&marked.text, &range_utf16);
        *actual_range = Some(actual);
        Some(slice)
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = self
            .marked
            .as_ref()
            .map_or(0..0, |marked| marked.selection.clone());
        Some(UTF16Selection {
            range,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked
            .as_ref()
            .map(|marked| 0..utf16_len(&marked.text))
    }

    /// The platform ends the composition and keeps the text as it is (macOS, when the user clicks away): it is
    /// committed.
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(marked) = self.marked.take() {
            self.send_text(&marked.text);
        }
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The terminal has no text for a range to point into but the preedit: what is committed takes the
        // place of the range of it (all of it without a range) and ends it. Without a preedit the text is
        // what is typed next.
        let (text, _) = replaced(self.preedit(), range_utf16.as_ref(), text);
        self.end_composition();
        self.send_text(&text);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = composition_text(text);
        let added = utf16_len(&text);
        let (text, base) = replaced(self.preedit(), range_utf16.as_ref(), &text);
        self.marked = (!text.is_empty()).then(|| {
            let len = utf16_len(&text);
            // The IME's selection is relative to the text it gave, which starts at `base` in the preedit.
            let selection = new_selected_range_utf16.map_or(base + added..base + added, |range| {
                (base + range.start).min(len)..(base + range.end).min(len)
            });
            Marked { text, selection }
        });
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let (origin, width, height) = self.cursor_origin()?;
        let columns = usize::from(self.size.columns);
        let info = self.terminal.info();
        let cells = preedit_cells(
            self.preedit().unwrap_or_default(),
            info.cursor.column,
            columns,
        );
        // The cell the range starts in, and the cells up to its end; past the text, the cell after it.
        let at = |utf16: usize| {
            cells
                .iter()
                .find(|cell| cell.utf16 >= utf16)
                .map(|cell| (cell.row, cell.column))
                .or_else(|| {
                    cells.last().map(|last| {
                        if last.column + last.span >= columns {
                            (last.row + 1, 0)
                        } else {
                            (last.row, last.column + last.span)
                        }
                    })
                })
                .unwrap_or((0, info.cursor.column))
        };
        let (row, column) = at(range_utf16.start);
        // A preedit that wraps below the last row is drawn higher (`rows_up`): the window opens there too.
        let screen_rows = usize::from(self.size.rows);
        let cursor_row =
            usize::try_from(i64::from(info.cursor.row) + info.display_offset as i64).unwrap_or(0);
        let up = rows_up(&cells, cursor_row, screen_rows);
        let shown = (cursor_row + row)
            .saturating_sub(up)
            .min(screen_rows.saturating_sub(1));
        let origin = Point {
            x: origin.x + px((column as f32 - info.cursor.column as f32) * width),
            y: origin.y + px((shown as f32 - cursor_row as f32) * height),
        };
        // A range on one row is as wide as its cells; one over several rows is one cell (the candidate
        // window opens at the start).
        let (end_row, end_column) = at(range_utf16.end);
        let span = if end_row == row && end_column > column {
            end_column - column
        } else {
            1
        };
        Some(Bounds::new(
            origin,
            size(px(span as f32 * width), px(height)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }

    /// A shell that has ended takes no text: an IME is not offered to a window that cannot use it.
    fn accepts_text_input(&self, _: &mut Window, _: &mut Context<Self>) -> bool {
        self.ended().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preedit_goes_on_the_cells_from_the_cursor() {
        let cells = preedit_cells("aあb", 3, 20);
        let at: Vec<_> = cells
            .iter()
            .map(|c| (c.text.as_str(), c.row, c.column, c.span, c.utf16))
            .collect();
        assert_eq!(
            at,
            [("a", 0, 3, 1, 0), ("あ", 0, 4, 2, 1), ("b", 0, 6, 1, 2)]
        );
    }

    #[test]
    fn a_preedit_that_wraps_below_the_screen_is_moved_up() {
        let cells = preedit_cells("aあい", 3, 5);
        // it takes two rows (0 and 1): from the last row of 24 it needs one row more than there is
        assert_eq!(rows_up(&cells, 23, 24), 1);
        assert_eq!(rows_up(&cells, 22, 24), 0);
        // one row only: nothing to move
        assert_eq!(rows_up(&preedit_cells("a", 0, 5), 23, 24), 0);
        assert_eq!(rows_up(&[], 23, 24), 0);
    }

    #[test]
    fn a_wide_character_that_does_not_fit_goes_to_the_next_row() {
        // 5 columns, starting at column 3: "a" at 3, "あ" needs 2 and 4+2 > 5
        let cells = preedit_cells("aあい", 3, 5);
        let at: Vec<_> = cells.iter().map(|c| (c.row, c.column)).collect();
        assert_eq!(at, [(0, 3), (1, 0), (1, 2)]);
    }

    #[test]
    fn a_combining_mark_stays_with_its_base_and_utf16_counts_units() {
        let cells = preedit_cells("e\u{301}😀", 0, 10);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].text, "e\u{301}");
        assert_eq!(cells[0].span, 1);
        // the emoji starts after two UTF-16 units, and covers two columns
        assert_eq!((cells[1].utf16, cells[1].span, cells[1].column), (2, 2, 1));
    }

    #[test]
    fn utf16_ranges_are_cut_at_characters() {
        let (slice, actual) = slice_utf16("あ😀い", &(1..3));
        assert_eq!((slice.as_str(), actual), ("😀", 1..3));
        // inside a surrogate pair: pulled to the next boundary
        let (slice, actual) = slice_utf16("あ😀い", &(2..4));
        assert_eq!((slice.as_str(), actual), ("い", 3..4));
        // past the end
        let (slice, actual) = slice_utf16("あ", &(5..9));
        assert_eq!((slice.as_str(), actual), ("", 1..1));
        let (slice, actual) = slice_utf16("abc", &(0..99));
        assert_eq!((slice.as_str(), actual), ("abc", 0..3));
    }

    #[test]
    fn a_range_of_the_preedit_is_replaced_and_the_rest_stays() {
        // no preedit, or no range: the text is all of it
        assert_eq!(replaced(None, Some(&(0..2)), "xy"), ("xy".to_owned(), 0));
        assert_eq!(replaced(Some("abcd"), None, "xy"), ("xy".to_owned(), 0));
        // a range of it
        assert_eq!(
            replaced(Some("abcd"), Some(&(1..3)), "X"),
            ("aXd".to_owned(), 1)
        );
        // an empty range inserts; a range past the end appends
        assert_eq!(
            replaced(Some("abcd"), Some(&(2..2)), "X"),
            ("abXcd".to_owned(), 2)
        );
        assert_eq!(
            replaced(Some("abcd"), Some(&(9..12)), "X"),
            ("abcdX".to_owned(), 4)
        );
        // UTF-16 units: the emoji is two
        assert_eq!(
            replaced(Some("a😀b"), Some(&(1..3)), "-"),
            ("a-b".to_owned(), 1)
        );
    }

    #[test]
    fn a_composition_has_no_control_characters() {
        assert_eq!(composition_text("a\r\nb\u{1b}c"), "abc");
        assert_eq!(composition_text("かな"), "かな");
    }
}
