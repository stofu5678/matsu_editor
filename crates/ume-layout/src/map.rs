// Derived from Sakura Editor (zlib License):
//   sakura_core/doc/layout/CLayoutMgr.cpp (LogicToLayout, LayoutToLogicEx)
// Modified: rewritten in Rust for the rows of one line, with byte offsets in place of UTF-16
// indexes, and measured in columns instead of pixels.
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

use crate::config::LayoutConfig;
use crate::wrap::Row;

/// A place on the screen: a row of the line and a column in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    /// The index into the rows of the line.
    pub row: usize,
    /// The display column, counted from the left edge (the indent is included).
    pub column: u32,
}

/// A place in the text, with the columns that were asked for beyond it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Located {
    /// The byte offset in the line, on a character boundary.
    pub offset: usize,
    /// The columns between the end of the row and the column that was asked for, when that is
    /// past the end (`CLogicPointEx::ext` in Sakura). 0 inside the row.
    pub extra_columns: u32,
}

/// The display position of the byte `offset` of the line (`CLayoutMgr::LogicToLayout`).
///
/// - The row is the last one that starts at or before `offset`, so an offset that is the start of
///   a wrapped row is at the left of that row, not at the right end of the row before it
///   (`pLayoutNext->GetLogicOffset() <= ptLogic.x`).
/// - An offset past the end of the line is the end of the line. An offset inside a character is
///   taken as the start of that character.
/// - The offset after a tab is the column after the tab; the offset of the tab is the column
///   where the tab starts.
///
/// `rows` is what [`crate::layout_line`] made of `text`.
pub fn offset_to_position(text: &str, rows: &[Row], cfg: &LayoutConfig, offset: usize) -> Position {
    let Some(last) = rows.last() else {
        return Position { row: 0, column: 0 };
    };
    let offset = offset.min(last.range.end);
    let row = rows
        .partition_point(|r| r.range.start <= offset)
        .saturating_sub(1);
    let r = &rows[row];
    let mut x = r.start_column;
    for (i, c) in text[r.range.clone()].char_indices() {
        // The offset in the middle of a character is the start of it.
        if r.range.start + i + c.len_utf8() > offset {
            break;
        }
        x += cfg.columns_of(c, x);
    }
    Position { row, column: x }
}

/// The byte offset at the display position (`row`, `column`) (`CLayoutMgr::LayoutToLogicEx`),
/// for a click or for moving the caret up and down.
///
/// - The column inside a full-width character or a tab snaps to the left edge of it
///   (`nX + nCharKetas > ptLayout.x` stops before the character).
/// - A column left of the first character, in the indent, is the start of the row.
/// - A column past the end of the row is the end of the row, and the columns past it are in
///   [`Located::extra_columns`]. Note that the end of a wrapped row is the start of the next
///   row, as in Sakura; the caller shows the caret at the left of the next row, or keeps it at
///   the end of this one, as it likes.
/// - A `row` past the last one is the last one.
pub fn position_to_offset(
    text: &str,
    rows: &[Row],
    cfg: &LayoutConfig,
    row: usize,
    column: u32,
) -> Located {
    let Some(last) = rows.last() else {
        return Located {
            offset: 0,
            extra_columns: column,
        };
    };
    let r = rows.get(row).unwrap_or(last);
    let mut x = r.start_column;
    for (i, c) in text[r.range.clone()].char_indices() {
        let w = cfg.columns_of(c, x);
        if x + w > column {
            return Located {
                offset: r.range.start + i,
                extra_columns: 0,
            };
        }
        x += w;
    }
    Located {
        offset: r.range.end,
        extra_columns: column - x,
    }
}
