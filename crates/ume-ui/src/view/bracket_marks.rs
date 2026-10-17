// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/view/CEditView_Paint_Bracket.cpp (SetBracketPairPos, DrawBracketPair)
//
// Original copyright notices of that file:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2022, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! The highlight of the bracket pair (`BRC`, 対括弧の強調表示): while the caret is at a bracket
//! (or right after one) and nothing is selected, the bracket and the one it pairs with are drawn
//! in the color `BRC`.
//!
//! This decides which characters to draw; `element.rs` draws them. The search is
//! `ume_commands::text::bracket` (`CEditView::SearchBracket`), here with the reach of the
//! highlight (`mode = 2`): only the lines that are on the screen are searched, and if the pair
//! is not on the screen there is no highlight at all, not even for the bracket at the caret
//! (`SetBracketPairPos`).

use ume_commands::text::bracket::{Reach, pair_of_start, start_of};
use ume_core::buffer::{Document, Pos};
use ume_layout::DocumentLayout;

/// A line longer than this many bytes is not searched: the highlight is made again at every
/// paint, and the search reads whole lines.
pub const MAX_LINE_BYTES: usize = 64 * 1024;

/// What is on the screen: the rows and the columns (both ends included).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Screen {
    pub first_row: usize,
    pub last_row: usize,
    pub first_col: u32,
    pub last_col: u32,
}

impl Screen {
    fn has(&self, row: usize, col: u32) -> bool {
        (self.first_row..=self.last_row).contains(&row)
            && (self.first_col..=self.last_col).contains(&col)
    }
}

/// A bracket to draw in the color `BRC`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark {
    /// The row of the layout, and the column where the bracket starts.
    pub row: usize,
    pub col: u32,
    /// How many columns the bracket takes (a full-width bracket takes 2).
    pub cols: u32,
    pub ch: char,
}

/// The brackets to draw for a caret at `caret`: the one that pairs with the bracket at the caret
/// (or before it), then that bracket; none if there is no pair on the screen. The caller has
/// checked that nothing is selected.
pub fn marks(doc: &Document, layout: &DocumentLayout, caret: Pos, screen: Screen) -> Vec<Mark> {
    marks_checked(doc, layout, caret, screen).unwrap_or_default()
}

fn marks_checked(
    doc: &Document,
    layout: &DocumentLayout,
    caret: Pos,
    screen: Screen,
) -> Option<Vec<Mark>> {
    // Finding the bracket reads the whole line: not one that is too long for that.
    if doc.line_len(caret.line).ok()?? > MAX_LINE_BYTES {
        return None;
    }
    let start = start_of(doc, caret).ok()??;
    let start_at = layout.pos_to_row(doc, start.pos).ok()?;
    // `nSearchNum`: the rows from the bracket to the bottom (down) or to the top (up).
    let reach = Reach {
        forward: screen.last_row.saturating_sub(start_at.row),
        backward: start_at.row.saturating_sub(screen.first_row),
        max_line_bytes: MAX_LINE_BYTES,
    };
    let other = pair_of_start(doc, start, Some(reach)).ok()??;
    let other_at = layout.pos_to_row(doc, other).ok()?;
    // The pair has to be on the screen, or nothing is drawn.
    if !screen.has(other_at.row, other_at.column) {
        return None;
    }
    let mut marks = Vec::with_capacity(2);
    for (pos, at) in [(other, other_at), (start.pos, start_at)] {
        if !screen.has(at.row, at.column) {
            continue;
        }
        let ch = doc
            .line(pos.line)
            .ok()??
            .text
            .get(pos.col..)?
            .chars()
            .next()?;
        marks.push(Mark {
            row: at.row,
            col: at.column,
            cols: layout.config().columns_of(ch, at.column),
            ch,
        });
    }
    Some(marks)
}

#[cfg(test)]
mod tests {
    use ume_layout::{LayoutConfig, Wrap};

    use super::*;

    fn setup(text: &str, config: LayoutConfig) -> (Document, DocumentLayout) {
        let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
        let layout = DocumentLayout::laid_out(&doc, config).unwrap();
        (doc, layout)
    }

    fn screen(rows: usize) -> Screen {
        Screen {
            first_row: 0,
            last_row: rows - 1,
            first_col: 0,
            last_col: 79,
        }
    }

    fn at(marks: &[Mark]) -> Vec<(usize, u32, char)> {
        marks.iter().map(|m| (m.row, m.col, m.ch)).collect()
    }

    #[test]
    fn the_pair_and_the_bracket_at_the_caret_are_marked() {
        let (doc, layout) = setup("a(b)c", LayoutConfig::default());
        // caret before "(": the pair is drawn first
        let m = marks(&doc, &layout, Pos::new(0, 1), screen(5));
        assert_eq!(at(&m), [(0, 3, ')'), (0, 1, '(')]);
        // caret after ")": the bracket before the caret
        let m = marks(&doc, &layout, Pos::new(0, 4), screen(5));
        assert_eq!(at(&m), [(0, 1, '('), (0, 3, ')')]);
    }

    #[test]
    fn nothing_is_marked_away_from_brackets_or_without_a_pair() {
        let (doc, layout) = setup("a(b c", LayoutConfig::default());
        assert!(marks(&doc, &layout, Pos::new(0, 0), screen(5)).is_empty());
        assert!(marks(&doc, &layout, Pos::new(0, 1), screen(5)).is_empty());
    }

    #[test]
    fn brackets_nest_and_full_width_ones_take_two_columns() {
        let (doc, layout) = setup("「a(「b」)」", LayoutConfig::default());
        let m = marks(&doc, &layout, Pos::new(0, 0), screen(5));
        assert_eq!(at(&m), [(0, 10, '」'), (0, 0, '「')]);
        assert_eq!(m[0].cols, 2);
        let m = marks(&doc, &layout, Pos::new(0, 4), screen(5));
        assert_eq!(at(&m), [(0, 9, ')'), (0, 3, '(')]);
        assert_eq!((m[0].cols, m[1].cols), (1, 1));
    }

    #[test]
    fn quotes_do_not_matter() {
        // Sakura counts a bracket in a string
        let (doc, layout) = setup("(\"a)\")", LayoutConfig::default());
        let m = marks(&doc, &layout, Pos::new(0, 0), screen(5));
        assert_eq!(at(&m), [(0, 3, ')'), (0, 0, '(')]);
    }

    #[test]
    fn the_pair_has_to_be_on_the_screen() {
        let text = "{\n1\n2\n3\n4\n5\n}\n";
        let (doc, layout) = setup(text, LayoutConfig::default());
        // the brace is 6 rows below: not on a screen of 5 rows
        assert!(marks(&doc, &layout, Pos::new(0, 0), screen(5)).is_empty());
        assert_eq!(marks(&doc, &layout, Pos::new(0, 0), screen(7)).len(), 2);
        // from the bottom brace: the first row is off the screen
        let down = Screen {
            first_row: 2,
            last_row: 6,
            ..screen(7)
        };
        assert!(marks(&doc, &layout, Pos::new(6, 0), down).is_empty());
        let down = Screen {
            first_row: 0,
            ..down
        };
        assert_eq!(
            at(&marks(&doc, &layout, Pos::new(6, 0), down)),
            [(0, 0, '{'), (6, 0, '}')]
        );
    }

    #[test]
    fn a_pair_scrolled_away_to_the_side_is_not_drawn() {
        let (doc, layout) = setup(
            "(                                  )",
            LayoutConfig::default(),
        );
        let narrow = Screen {
            last_col: 20,
            ..screen(5)
        };
        assert!(marks(&doc, &layout, Pos::new(0, 0), narrow).is_empty());
        // the far bracket is on the screen but the near one is scrolled away
        let scrolled = Screen {
            first_col: 5,
            ..screen(5)
        };
        assert_eq!(
            at(&marks(&doc, &layout, Pos::new(0, 0), scrolled)),
            [(0, 35, ')')]
        );
    }

    #[test]
    fn rows_of_a_wrapped_line_count_for_the_reach() {
        let config = LayoutConfig {
            wrap: Wrap::Columns(4),
            ..LayoutConfig::default()
        };
        let (doc, layout) = setup("(abcdefgh)\nx", config);
        // the line takes 3 rows: the closing bracket is on the third
        let m = marks(&doc, &layout, Pos::new(0, 0), screen(3));
        assert_eq!(at(&m), [(2, 1, ')'), (0, 0, '(')]);
        assert!(marks(&doc, &layout, Pos::new(0, 0), screen(2)).is_empty());
    }

    #[test]
    fn a_huge_document_is_not_read_beyond_the_screen() {
        let text = "(\n".to_string() + &"x\n".repeat(300_000) + ")";
        let (doc, layout) = setup(&text, LayoutConfig::default());
        let started = std::time::Instant::now();
        assert!(marks(&doc, &layout, Pos::new(0, 0), screen(50)).is_empty());
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn a_line_that_is_very_long_is_not_searched() {
        let text = format!("({})", "x".repeat(MAX_LINE_BYTES + 10));
        let (doc, layout) = setup(&text, LayoutConfig::default());
        assert!(marks(&doc, &layout, Pos::new(0, 0), screen(5)).is_empty());
    }
}
