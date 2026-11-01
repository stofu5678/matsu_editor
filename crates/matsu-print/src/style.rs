//! Where color printing gets its colors from.

use std::ops::Range;

use crate::page::TextStyle;

/// A piece of a line in one style. The offsets are bytes of the line text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleRun {
    pub range: Range<usize>,
    pub style: TextStyle,
}

/// The colors of the text of a document, asked for one line at a time. The caller adapts the
/// coloring of its window to this (`matsu-print` knows neither the color rules nor the
/// document), so that the print is colored by the same code as the screen.
///
/// The layout asks only when the setting prints in color (`color_print`), and may ask for a line
/// more than once (a line that wraps over two pages), in any order of pages.
pub trait LineStyler {
    /// The style of text that no run covers, and of what Sakura draws in the text color of the
    /// type: the line numbers, the header and the footer (the device context keeps the color the
    /// last line was reset to, `Print_DrawLine`).
    fn plain(&self) -> TextStyle;

    /// Replaces `out` with the runs of line `line` (from 0), whose text without its line ending is
    /// `text`. Runs are sorted and do not overlap; what they leave out is [`plain`](Self::plain).
    fn style_line(&self, line: usize, text: &str, has_eol: bool, out: &mut Vec<StyleRun>);
}

/// The style at byte `at` of a line, for `at` that only grows: `next` is the index of the first
/// run that may still cover it, and moves on.
pub(crate) fn style_at(runs: &[StyleRun], next: &mut usize, at: usize) -> Option<TextStyle> {
    while runs.get(*next).is_some_and(|r| r.range.end <= at) {
        *next += 1;
    }
    runs.get(*next)
        .filter(|r| r.range.start <= at)
        .map(|r| r.style)
}
