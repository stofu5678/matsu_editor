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

/// The class of a character for the starts of words. Space (and the line break) goes with the
/// word before it; the marks of punctuation are a word each; a run of the same class is a word.
fn class(c: char) -> u8 {
    match c {
        ' ' | '\t' | '\n' | '\u{3000}' => 0,
        'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => 1,
        '\u{3041}'..='\u{3096}' => 2,
        '\u{30A1}'..='\u{30FA}' | 'ー' => 3,
        '\u{4E00}'..='\u{9FFF}' | '々' => 4,
        c if c.is_alphanumeric() => 5,
        _ => 6,
    }
}

/// The index (in characters) where each word of `text` starts, except the first character (a run
/// starts a line, so it is implicitly the start of a word). Trailing spaces belong to the word
/// before; leading spaces of a line are a word of their own.
fn word_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut prev: Option<u8> = None;
    for (i, c) in text.chars().enumerate() {
        let k = class(c);
        if let Some(p) = prev
            && k != 0
            && (p == 0 || p != k || k == 6)
        {
            starts.push(i);
        }
        prev = Some(k);
    }
    starts
}

impl Snapshot {
    /// The runs and the selection. This is the part that costs, so it runs only when a screen
    /// reader asks for the tree.
    pub(crate) fn text(&self) -> Text {
        let mut runs = Vec::with_capacity(self.rows.len());
        for (i, row) in self.rows.iter().enumerate() {
            runs.push(self.run(i, row));
        }
        // A caret in an invisible place is no selection (it would sit on the wrong text).
        let selection = self
            .place(self.focus, false)
            .zip(self.place(self.anchor, true))
            .map(|(focus, anchor)| (anchor, focus));
        Text { runs, selection }
    }

    fn run(&self, index: usize, row: &RowInfo) -> Run {
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
        let words = word_starts(&value);
        if row.eol.is_some() {
            // One character for every kind of line break (a CRLF counts as one: the AccessKit
            // rule), after the last character, one cell wide as its mark.
            value.push('\n');
            char_lengths.push(1);
            positions.push((x - origin) as f32 * unit);
            widths.push(unit);
            x += 1;
        }
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
                // The list holds `u8`: a word that starts after the 255th character is left out.
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
