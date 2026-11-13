//! Selecting cells of the screen and reading the selected text (ADR-0028, the selection PR).
//!
//! A small API around `alacritty_terminal`'s `Selection`, which the terminal state keeps and moves: when
//! the output scrolls lines, `alacritty_terminal` scrolls the selection with them, and clears it when the
//! lines it is on are erased, when the screen is switched (the alternate screen) or when a line of it is
//! changed by a clear or an erase. This crate follows that and adds nothing of its own.
//!
//! Positions are on the view, as in [`crate::Snapshot::rows`]: `row` 0 is the first line of the view, which is
//! the live screen unless the view is scrolled back. Rows below 0 and rows below the screen are the
//! scrollback and are allowed while the view is scrolled back.

use std::cmp::min;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::Term;

/// What a selection selects as it grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    /// The cells between the start and the end (a drag).
    Cells,
    /// Whole words (a double click): the selection grows by words. Words end at the delimiters of
    /// [`WORD_DELIMITERS`].
    Words,
    /// Whole lines (a triple click): the selection grows by lines, and a line that wraps is one line.
    Lines,
}

/// Which half of a cell a position is in: a cell is selected by a drag when the pointer passed its middle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellSide {
    Left,
    Right,
}

impl From<CellSide> for Side {
    fn from(side: CellSide) -> Self {
        match side {
            CellSide::Left => Self::Left,
            CellSide::Right => Self::Right,
        }
    }
}

/// The characters that end a word when a word is selected: the delimiters of Windows Terminal's
/// `wordDelimiters` (space, `/ \ ( ) " ' - . , : ; < > ~ ! @ # $ % ^ & * | + = [ ] { } ?` and the box
/// drawing bar U+2502), plus the tab.
pub const WORD_DELIMITERS: &str = " \t/\\()\"'-.,:;<>~!@#$%^&*|+=[]{}?\u{2502}";

/// The selected cells, from the first to the last, both included, on the view. A line between the two is
/// selected whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionSpan {
    pub start_row: i32,
    pub start_column: usize,
    pub end_row: i32,
    pub end_column: usize,
}

impl SelectionSpan {
    /// Whether the cell at `row` and `column` is selected.
    pub fn contains(&self, row: i32, column: usize) -> bool {
        if row < self.start_row || row > self.end_row {
            return false;
        }
        (row != self.start_row || column >= self.start_column)
            && (row != self.end_row || column <= self.end_column)
    }
}

fn point<T>(term: &Term<T>, row: i32, column: usize) -> Point {
    let offset = i32::try_from(term.grid().display_offset()).unwrap_or(0);
    let last = term.columns().saturating_sub(1);
    Point::new(Line(row - offset), Column(min(column, last)))
}

/// Starts a selection at a cell. A cell selection is empty until [`update`] moves its end; words and lines
/// are selected at once.
pub(crate) fn start<T>(
    term: &mut Term<T>,
    mode: SelectionMode,
    row: i32,
    column: usize,
    side: CellSide,
) {
    let kind = match mode {
        SelectionMode::Cells => SelectionType::Simple,
        SelectionMode::Words => SelectionType::Semantic,
        SelectionMode::Lines => SelectionType::Lines,
    };
    let at = point(term, row, column);
    term.selection = Some(Selection::new(kind, at, side.into()));
}

/// Moves the end of the selection to a cell. Without a selection nothing happens.
pub(crate) fn update<T>(term: &mut Term<T>, row: i32, column: usize, side: CellSide) {
    let at = point(term, row, column);
    if let Some(selection) = term.selection.as_mut() {
        selection.update(at, side.into());
    }
}

pub(crate) fn clear<T>(term: &mut Term<T>) {
    term.selection = None;
}

/// Whether anything is selected (a selection that covers no cell is none).
pub(crate) fn is_selected<T>(term: &Term<T>) -> bool {
    term.selection
        .as_ref()
        .is_some_and(|selection| selection.to_range(term).is_some())
}

/// The selected cells, on the view.
pub(crate) fn span<T>(term: &Term<T>) -> Option<SelectionSpan> {
    let range = term.selection.as_ref()?.to_range(term)?;
    let offset = i32::try_from(term.grid().display_offset()).unwrap_or(0);
    Some(SelectionSpan {
        start_row: range.start.line.0 + offset,
        start_column: range.start.column.0,
        end_row: range.end.line.0 + offset,
        end_column: range.end.column.0,
    })
}

/// The selected text, with `\n` between lines (the UI turns that into the platform's line end).
///
/// - A line that wraps (the program wrote past the last column) is joined to the next without a line end.
/// - The blanks to the right of the last character of a line are not copied, as terminals do: those cells
///   are empty, there are no spaces the user typed. (`alacritty_terminal` stops at the last cell that holds a
///   character, whatever the background color of the blanks after it; this is the rule followed.) Spaces
///   before a character are kept, also when the selection ends among them.
/// - A wide character is one character, and combining characters stay with their base.
/// - Selecting lines (a triple click) gives the text of the lines without a line end after the last one.
pub(crate) fn text<T>(term: &Term<T>) -> Option<String> {
    let selected = term.selection_to_string()?;
    let by_lines = term
        .selection
        .as_ref()
        .is_some_and(|selection| selection.ty == SelectionType::Lines);
    Some(if by_lines {
        selected.strip_suffix('\n').unwrap_or(&selected).to_owned()
    } else {
        selected
    })
}

#[cfg(test)]
mod tests;
