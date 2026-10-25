//! The text of the view for screen readers (UI Automation, AX, AT-SPI through AccessKit).
//!
//! The view is one node (a multi-line text field, `a11y::editor`) whose children are text runs:
//! one run for each row on the screen, with the line break of a line at the end of its last row,
//! and the caret and the selection as the text selection of the node. Only the rows that are
//! drawn are exposed, never the whole document (a huge file stays cheap), and the rows are the
//! ones the paint has already laid out: the paint leaves a [`Snapshot`] (the rows it moves, no
//! text is copied), and the strings of the runs are built only when a screen reader is connected
//! (GPUI calls the hook of the node only then).
//!
//! The run of a row has: the text of the row (a line break is one `\n` whatever the file has: CR,
//! LF, CRLF, NEL, LS or PS), the length of each character in UTF-8, the starts of words, the
//! position and the width of each character on the screen (a screen magnifier follows the caret
//! with them), and the bounds of the row. A wrapped line is several runs, as it is several rows
//! on the screen.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::A11ySubtreeBuilder;
use gpui::accesskit::{Node, Rect, Role, TextDirection, TextPosition, TextSelection};
use matsu_commands::text::word::search_next_word;
use matsu_core::buffer::Pos;
use matsu_layout::{LayoutConfig, RowInfo};

/// What the last paint left for the screen reader, shared by the view and its node.
pub(crate) type Shared = Rc<RefCell<Option<Snapshot>>>;

/// A place in the text on the screen: the row (counted from the top of the document) and the
/// position in the document.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Spot {
    pub(crate) row: usize,
    pub(crate) pos: Pos,
}

/// The rows on the screen and where they are, as the last paint saw them.
pub(crate) struct Snapshot {
    /// The rows that are drawn, from the first (partly) visible one.
    pub(crate) rows: Vec<RowInfo>,
    /// The fixed end of the selection (the caret when nothing is selected) and the caret.
    pub(crate) anchor: Spot,
    pub(crate) focus: Spot,
    /// For the width of tabs and wide characters (`LayoutConfig::columns_of`); the rest of the
    /// configuration is not used.
    pub(crate) columns: LayoutConfig,
    /// The left edge of column 0 of the text and the top edge of the first row, in pixels of the
    /// window (before the scale), with the horizontal scroll already taken off.
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) cell: f32,
    pub(crate) line_height: f32,
    /// Physical pixels per pixel of the window.
    pub(crate) scale: f32,
}

/// One text run: a row.
#[derive(Debug, PartialEq)]
pub(crate) struct Run {
    /// The number of the row in the document; the id of the node is made from it.
    pub(crate) row: usize,
    pub(crate) value: String,
    pub(crate) char_lengths: Vec<u8>,
    pub(crate) word_starts: Vec<usize>,
    pub(crate) positions: Vec<f32>,
    pub(crate) widths: Vec<f32>,
    pub(crate) bounds: [f64; 4],
}

/// A place in the runs: the index in `runs` and the character index in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct At {
    pub(crate) run: usize,
    pub(crate) index: usize,
}

/// The runs of the rows on the screen and the selection (the caret when `anchor == focus`).
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Text {
    pub(crate) runs: Vec<Run>,
    pub(crate) selection: Option<(At, At)>,
}

/// The index (in characters) where each word of `text` starts, by the rules of the word keys
/// (`matsu_commands::text::word::search_next_word`: Sakura's `SearchNextWordPosition`), so that
/// the screen reader moves by word where Ctrl+Right does. Trailing spaces belong to the word
/// before; leading spaces of a line are a word of their own. `before` is the character just
/// before the text when it goes on from a soft wrap (the row before it): a word that starts right
/// at the wrap then starts at 0. At the start of a line it is `None` and the first character is
/// implicitly the start of a word (AccessKit). `line_break` is whether the row ends with a line
/// break (the run holds one `\n`): the key stops before it, so the break is a start too.
/// `stops_both_ends` is the common setting of the same name (the key stops at the end of a word
/// as well, before the spaces after it).
fn word_starts(
    text: &str,
    before: Option<char>,
    line_break: bool,
    stops_both_ends: bool,
) -> Vec<usize> {
    let skip = usize::from(before.is_some());
    let line: Vec<char> = before
        .into_iter()
        .chain(text.chars())
        .chain(line_break.then_some('\n'))
        .collect();
    let mut starts = Vec::new();
    let mut at = 0;
    while let Some(next) = search_next_word(&line, usize::from(line_break), at, stops_both_ends) {
        // The end of the text is where the key stops when no word follows: not a start. (`next`
        // is never 0, and the prepended character is not part of the text.)
        if next >= line.len() {
            break;
        }
        starts.push(next - skip);
        at = next;
    }
    starts
}

impl Snapshot {
    /// The runs and the selection. This is the part that costs, so it runs only when a screen
    /// reader asks for the tree.
    pub(crate) fn text(&self) -> Text {
        let stops_both_ends = matsu_commands::edit_options::get().stops_both_ends_word;
        let mut runs = Vec::with_capacity(self.rows.len());
        for (i, row) in self.rows.iter().enumerate() {
            runs.push(self.run(i, row, stops_both_ends));
        }
        // A caret in an invisible place is no selection (it would sit on the wrong text).
        let selection = self
            .place(self.focus, false)
            .zip(self.place(self.anchor, true))
            .map(|(focus, anchor)| (anchor, focus));
        Text { runs, selection }
    }

    fn run(&self, index: usize, row: &RowInfo, stops_both_ends: bool) -> Run {
        let mut value = String::from(row.text());
        let mut char_lengths: Vec<u8> = value.chars().map(|c| c.len_utf8() as u8).collect();
        let mut x = row.start_column;
        let mut positions = Vec::with_capacity(char_lengths.len() + 1);
        let mut widths = Vec::with_capacity(char_lengths.len() + 1);
        let unit = self.cell * self.scale;
        let origin = x;
        for c in value.chars() {
            let w = self.columns.columns_of(c, x);
            positions.push((x - origin) as f32 * unit);
            widths.push(w as f32 * unit);
            x += w;
        }
        if row.eol.is_some() {
            // One character for every kind of line break (a CRLF counts as one: the AccessKit
            // rule), after the last character, one cell wide as its mark.
            value.push('\n');
            char_lengths.push(1);
            positions.push((x - origin) as f32 * unit);
            widths.push(unit);
            x += 1;
        }
        // A row that goes on from a soft wrap continues the word before it unless the characters
        // around the wrap say otherwise. Without the row before it (it is above the screen),
        // the wrap counts as a space.
        let before = (row.row_in_line > 0).then(|| {
            index
                .checked_sub(1)
                .and_then(|i| self.rows[i].text().chars().next_back())
                .unwrap_or(' ')
        });
        // AccessKit reads the starts as `u8` and goes wrong on a run with more characters than
        // that (the line break counts): such a row is one word (no starts).
        let words = if char_lengths.len() > usize::from(u8::MAX) {
            Vec::new()
        } else {
            word_starts(row.text(), before, row.eol.is_some(), stops_both_ends)
        };
        // The width counts the cell of the mark of the end of the text, where the caret can be.
        let width = (x - origin + u32::from(row.eol.is_none())) as f32 * unit;
        let x0 = f64::from((self.left + row.start_column as f32 * self.cell) * self.scale);
        let y0 = f64::from((self.top + index as f32 * self.line_height) * self.scale);
        Run {
            row: row.row,
            value,
            char_lengths,
            word_starts: words,
            positions,
            widths,
            bounds: [
                x0,
                y0,
                x0 + f64::from(width),
                y0 + f64::from(self.line_height * self.scale),
            ],
        }
    }

    /// Where `spot` is in the runs. A place above the first row is the start of it and one below
    /// the last row is the end of that, when `clamp` allows (the other end of a selection that
    /// goes off the screen); the caret itself has to be on a row.
    fn place(&self, spot: Spot, clamp: bool) -> Option<At> {
        let first = self.rows.first()?;
        let last = self.rows.last()?;
        if spot.row < first.row {
            return clamp.then_some(At { run: 0, index: 0 });
        }
        if spot.row > last.row {
            return clamp.then(|| At {
                run: self.rows.len() - 1,
                index: self.run_len(self.rows.len() - 1),
            });
        }
        let run = spot.row - first.row;
        let row = self.rows.get(run)?;
        let text = row.text();
        let upto = spot.pos.col.saturating_sub(row.start.col).min(text.len());
        // A position in the middle of a character (it should not be) is its start.
        let upto = (0..=upto).rev().find(|i| text.is_char_boundary(*i))?;
        Some(At {
            run,
            index: text[..upto].chars().count(),
        })
    }

    /// The characters of a run, with the line break.
    fn run_len(&self, run: usize) -> usize {
        let row = &self.rows[run];
        row.text().chars().count() + usize::from(row.eol.is_some())
    }
}

/// Writes the runs and the selection into the node of the view. The hook of GPUI calls this
/// after the paint, only while a screen reader is connected.
pub(crate) fn write(shared: &Shared, builder: &mut A11ySubtreeBuilder) {
    let guard = shared.borrow();
    let Some(snapshot) = guard.as_ref() else {
        return;
    };
    let text = snapshot.text();
    let mut ids = Vec::with_capacity(text.runs.len());
    for run in &text.runs {
        let id = builder.synthetic_node_id(run.row);
        let mut node = Node::new(Role::TextRun);
        node.set_value(run.value.as_str());
        node.set_character_lengths(run.char_lengths.as_slice());
        node.set_word_starts(
            run.word_starts
                .iter()
                // The list holds `u8`; `run` leaves it empty for a longer row.
                .filter_map(|i| u8::try_from(*i).ok())
                .collect::<Vec<_>>(),
        );
        node.set_character_positions(run.positions.as_slice());
        node.set_character_widths(run.widths.as_slice());
        node.set_text_direction(TextDirection::LeftToRight);
        node.set_bounds(Rect {
            x0: run.bounds[0],
            y0: run.bounds[1],
            x1: run.bounds[2],
            y1: run.bounds[3],
        });
        builder.push_child(id, node);
        ids.push(id);
    }
    if let Some((anchor, focus)) = text.selection {
        let position = |at: At| TextPosition {
            node: ids[at.run],
            character_index: at.index,
        };
        builder.parent_node().set_text_selection(TextSelection {
            anchor: position(anchor),
            focus: position(focus),
        });
    }
}

#[cfg(test)]
mod tests;
