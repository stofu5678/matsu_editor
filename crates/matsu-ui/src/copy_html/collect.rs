// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/cmd/CViewCommander_Clipboard.cpp (Command_COPY_COLOR_HTML)
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2000-2001, jepro, genta
// Copyright (C) 2001, novice
// Copyright (C) 2002, hor, genta, Azumaiya, すなふき
// Copyright (C) 2004, Moca
// Copyright (C) 2005, genta
// Copyright (C) 2007, ryoji
// Copyright (C) 2010, ryoji
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the C++ code (the layout objects
// and the color strategies that are asked at every character) was rewritten in Rust on the rows
// of `matsu-layout` and the runs of `matsu-coloring`; the search colors are not in the copy; and
// the three differences in the comments marked "Difference" were made on purpose.
//
// zlib License
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

//! What `Command_COPY_COLOR_HTML` copies: the rows of the selection, with the style of each
//! character from the colors of the document type, and the line numbers if asked.
//!
//! The rows are the rows of the layout, as in Sakura; a line that wraps is several rows, and a
//! box selection takes the columns of each row.

use matsu_coloring::{
    ColorKind, ColorRules, ColorRun, ColorState, ColorStates, line_end_state, line_runs,
};
use matsu_core::buffer::{Document, Pos};
use matsu_core::cursor::Cursor;
use matsu_layout::{DocumentLayout, RowInfo};

use super::builder::{Attr, Html, Row};
use crate::hit::BoxArea;
use crate::settings::view_settings::rgb_of;

/// What the copy reads.
pub(crate) struct Source<'a> {
    pub doc: &'a Document,
    pub layout: &'a DocumentLayout,
    pub rules: &'a ColorRules,
    pub states: &'a ColorStates,
    pub cursor: &'a Cursor,
    /// `bLineNumIsCRLF`: the line numbers count lines of the text, not rows of the layout.
    pub line_num_is_crlf: bool,
    /// The numbers of the rows of the layout are written before each row (the "with line number"
    /// function).
    pub line_numbers: bool,
}

/// The style of a character of the color kind `kind`.
fn attr_of(rules: &ColorRules, kind: ColorKind) -> Attr {
    let entry = rules.entry(kind);
    Attr {
        color: Some((rgb_of(entry.text), rgb_of(entry.back))),
        bold: entry.bold,
        underline: entry.underline,
    }
}

/// The kind of color at byte `at` of a line, from its runs.
fn kind_at(runs: &[ColorRun], at: usize) -> ColorKind {
    let i = runs.partition_point(|r| r.end <= at);
    runs.get(i)
        .filter(|r| r.start <= at)
        .map_or(ColorKind::Text, |r| r.kind)
}

/// The runs of the colors of a line, scanned from its start in `state`; the whole text and the
/// line ending of the line.
struct LineColors {
    line: usize,
    runs: Vec<ColorRun>,
}

/// Walks the lines of the selection in order, with the color state each starts in.
struct Colors<'a> {
    source: &'a Source<'a>,
    current: Option<LineColors>,
    /// The state that the line after `current` starts in.
    next: Option<(usize, ColorState)>,
}

impl<'a> Colors<'a> {
    fn new(source: &'a Source<'a>) -> Self {
        Self {
            source,
            current: None,
            next: None,
        }
    }

    /// The colors of `line`, scanned the first time it is asked for.
    fn line(&mut self, line: usize) -> Option<&LineColors> {
        if self.current.as_ref().map(|c| c.line) != Some(line) {
            let source = self.source;
            let layout = source.layout.line_layout(source.doc, line).ok()??;
            let rules = source.rules;
            let state = match self.next.take() {
                Some((at, state)) if at == line => state,
                _ => source.states.state_at(source.doc, rules, line).0,
            };
            let runs = if rules.colors_nothing() {
                Vec::new()
            } else {
                line_runs(rules, &layout.text, layout.eol, &state, layout.text.len())
            };
            self.next = Some((
                line + 1,
                line_end_state(rules, &layout.text, layout.eol, &state),
            ));
            self.current = Some(LineColors { line, runs });
        }
        self.current.as_ref()
    }
}

/// The number of characters of `text` before byte `at`; `at` is cut to the text.
fn chars_before(text: &str, at: usize) -> usize {
    let at = at.min(text.len());
    text.char_indices().take_while(|(i, _)| *i < at).count()
}

/// The characters of a row with the line ending that ends it, and the style of each.
fn row_chars(row: &RowInfo, colors: &LineColors, rules: &ColorRules) -> (Vec<char>, Vec<Attr>) {
    let mut text = String::from(row.text());
    if let Some(eol) = row.eol {
        text.push_str(eol.as_str());
    }
    let mut chars = Vec::with_capacity(text.len());
    let mut attrs = Vec::with_capacity(text.len());
    // The cache of the style of the last kind, which is the same for most characters.
    let mut cached: Option<(ColorKind, Attr)> = None;
    let mut byte = row.start.col;
    for c in text.chars() {
        let kind = kind_at(&colors.runs, byte);
        let attr = match cached {
            Some((k, attr)) if k == kind => attr,
            _ => {
                let attr = attr_of(rules, kind);
                cached = Some((kind, attr));
                attr
            }
        };
        chars.push(c);
        attrs.push(attr);
        byte += c.len_utf8();
    }
    (chars, attrs)
}

/// The number of digits of the line numbers (`nLineNumberMaxLen` of the C++ code, without the
/// colon): the digits of `max`, at least 1 and at most 11. `max` counts from 0, so the last line
/// number can be one digit wider (Sakura's own arithmetic).
fn number_width(max: usize) -> usize {
    let mut width = 1;
    let mut limit: u64 = 10;
    while width < 12 && limit <= max as u64 {
        limit *= 10;
        width += 1;
    }
    width
}

/// The HTML for the selection of the cursor, or `None` when nothing is selected.
pub(crate) fn color_html(source: &Source<'_>) -> Option<String> {
    let (doc, layout, rules) = (source.doc, source.layout, source.rules);
    let (from, to) = source.cursor.selection()?;
    let is_box = source.cursor.is_box();

    // The rows of the selection (`rcSel`) and, for a box, its columns.
    let (top, bottom, area) = if is_box {
        let area = BoxArea::of_cursor(layout, doc, source.cursor)?;
        let ((a, _), (c, _)) = source.cursor.box_corners()?;
        let (row_a, row_c) = (
            layout.pos_to_row(doc, a).ok()?.row,
            layout.pos_to_row(doc, c).ok()?.row,
        );
        (row_a.min(row_c), row_a.max(row_c), Some(area))
    } else {
        (
            layout.pos_to_row(doc, from).ok()?.row,
            layout.pos_to_row(doc, to).ok()?.row,
            None,
        )
    };
    // One row more than the selection: whether the line goes on after the last row.
    let rows = layout.rows(doc, top, bottom - top + 2).ok()?;
    let last = rows.iter().position(|r| r.row == bottom)?;

    // The width of the line numbers (`nLineNumberMax`).
    let width = number_width(if source.line_num_is_crlf {
        rows[last].line
    } else {
        bottom
    });
    let blank_number = format!("{:width$}:", "");

    let back = rgb_of(rules.entry(ColorKind::Text).back);
    let mut html = Html::new(back);
    let mut colors = Colors::new(source);
    for (index, info) in rows.iter().enumerate().take(last + 1) {
        if info.is_eof_row {
            // the empty row after the last line ending is no row in Sakura
            break;
        }
        let Some(line) = colors.line(info.line) else {
            break;
        };
        let (chars, attrs) = row_chars(info, line, rules);
        let len = chars.len();
        let (from_index, to_index) = match &area {
            Some(area) => {
                // The characters at the left and the right column of the box; a row that is
                // shorter has its end for both. The line ending is not in it.
                let (left, right) = area.columns_on(info.row)?;
                let index_at = |column: u32| -> Option<usize> {
                    let at = layout.row_to_pos(doc, info.row, column).ok()?.pos;
                    let col = at.col.clamp(info.start.col, info.end.col);
                    Some(chars_before(info.text(), col - info.start.col))
                };
                (index_at(left)?, index_at(right)?)
            }
            None => {
                let index_of =
                    |pos: Pos| chars_before(info.text(), pos.col.saturating_sub(info.start.col));
                (
                    if info.row == top { index_of(from) } else { 0 },
                    if info.row == bottom {
                        index_of(to)
                    } else {
                        len
                    },
                )
            }
        };
        // Difference: Sakura also takes the rows of a wrapped line that come after the last row
        // of the selection (`nIdxTo = nLineLen` for them); here they are not in the copy.
        // Do not write the number of the line after the last line ending
        // ("最後の改行の次の行番号を表示しないように"). Difference: Sakura does it for a box too, and
        // loses the last row of a box that ends on an empty row; a box keeps all its rows here.
        if area.is_none() && to_index == 0 && info.row == bottom {
            break;
        }
        if source.line_numbers {
            if source.line_num_is_crlf {
                if info.row_in_line == 0 {
                    html.raw(&format!("{:width$}:", info.line + 1));
                } else if is_box {
                    html.raw(&blank_number);
                }
            } else if is_box || info.row_in_line == 0 {
                html.raw(&format!("{:width$}:", info.row + 1));
            }
        }
        let ends_with_delimiter = html.row(&Row {
            chars,
            attrs,
            from: Some(from_index),
            to: to_index,
        });
        if source.line_numbers {
            // The tags close at the end of a line, so that the number of the next one is outside.
            let next = rows.get(index + 1).filter(|r| !r.is_eof_row);
            let line_ends =
                next.is_none_or(|next| source.line_num_is_crlf || is_box || next.row_in_line == 0);
            if line_ends {
                html.close_tags();
            }
        }
        if is_box && !ends_with_delimiter {
            // `bLineNumLayout`: a row of a box ends with CR LF.
            html.raw("\r\n");
        }
    }
    Some(html.finish())
}
