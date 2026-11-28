// Derived from Sakura Editor (zlib License):
//   sakura_core/doc/layout/CLayoutMgr_DoLayout.cpp (_MakeOneLine, _DoKinsokuSkip, _DoWordWrap,
//     _DoKutoBurasage, _DoGyotoKinsoku, _DoGyomatsuKinsoku, _OnLine1, _DoLayoutSub)
//   sakura_core/doc/layout/CLayoutMgr_New.cpp (getIndentOffset_Tx2x, getIndentOffset_LeftSpace)
//   sakura_core/charset/charcode.cpp (IS_KEYWORD_CHAR, gm_keyword_char)
// Modified: rewritten in Rust for one line of text at a time, measured in columns instead of
// pixels, without colouring, TSV/CSV mode, and the layout list of the whole document.
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

use std::ops::Range;

use crate::config::{LayoutConfig, WrapIndent};
use crate::x::X;

/// One row of a laid-out line (Sakura's `CLayout`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The bytes of the line on this row. A line ending is not in the text, so the row that
    /// holds only the line ending (see [`layout_line`]) has an empty range.
    pub range: Range<usize>,
    /// The position of the first character: the indent of a wrapped row (`CLayout::GetIndent`),
    /// 0 for the first row.
    pub start_x: X,
    /// The position after the last character, without the line ending
    /// (`CLayout::CalcLayoutWidth`). It can pass the wrap width by a hanging punctuation.
    pub end_x: X,
    /// The row ends because the line wraps, so another row of the same line follows.
    pub wrapped: bool,
}

/// What kind of kinsoku run is going (`EKinsokuType`, `CLayoutMgr.h`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Run {
    None,
    WordWrap,
    Head,
    Tail,
    Kuto,
}

struct State {
    /// The byte offset being placed (`nPos`).
    pos: usize,
    /// Where the row began (`nBgn`).
    bgn: usize,
    /// The position of `pos`, with the indent (`nPosX`).
    x: X,
    /// The indent of this row (`nIndent`).
    indent: X,
    run: Run,
    /// The run is `word_bgn..word_bgn + word_len` (`nWordBgn`, `nWordLen`).
    word_bgn: usize,
    word_len: usize,
}

/// The characters of an English word (`IS_KEYWORD_CHAR`, `gm_keyword_char` in `charcode.cpp`):
/// `0-9 A-Z a-z _` and `# $ @ \` (the user-defined ones, "Oct. 31, 2000 JEPRO" for TeX and
/// "Nov. 9, 2000 JEPRO" for HSP).
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'#' | b'$' | b'@' | b'\\')
}

/// Lays out one line (its text without the line ending) into rows, as
/// `CLayoutMgr::_MakeOneLine` does, and puts them in `rows` (cleared first). There is at least one
/// row, even for an empty line. (Sakura makes no row for an empty last line without a line
/// ending; the document-wide layout decides that.)
///
/// `has_eol` says the line ends with a line ending. It takes one column (of `X`:
/// `Widths::per_column`), as Sakura counts it
/// (`CNativeW::GetHabaOfChar`: "HACK: returns 1 for a line ending"), so a line that fills the
/// wrap column puts its line ending on a row of its own, unless [`crate::Kinsoku::hang_eol`]
/// lets it hang. The row that holds only the line ending has an empty range at `text.len()`.
///
/// It allocates nothing but the growth of `rows`.
pub fn layout_line(text: &str, has_eol: bool, cfg: &LayoutConfig, rows: &mut Vec<Row>) {
    let mut measure = matsu_core::measure::span("layout.layout_line");
    measure.bytes(text.len());
    rows.clear();
    let len = text.len();
    let max = cfg.max_x();
    let one = cfg.width.to_x(1);
    let two = cfg.width.to_x(2);
    let four = cfg.width.to_x(4);
    // Sakura looks for hanging punctuation and kinsoku only when fewer than 2 (4) columns are
    // left (`_DoKutoBurasage`: `GetMaxLineLayout() - nPosX < 2 * GetWidthPerKeta()`;
    // `_DoGyotoKinsoku` and `_DoGyomatsuKinsoku`: `< 4 * GetWidthPerKeta()`,
    // `sakura_core/doc/layout/CLayoutMgr_DoLayout.cpp`). The bounds are made for characters of at
    // most 2 columns, which the grid has. A character of a table can be wider, so then the rules
    // are asked at every position and decide by the real widths (`rest < w`, `rest < w1 + w2`).
    let table = cfg.width.is_table();
    let k = &cfg.kinsoku;
    let kinsoku_enabled = cfg.word_wrap || k.any();
    let mut st = State {
        pos: 0,
        bgn: 0,
        x: X::ZERO,
        indent: X::ZERO,
        run: Run::None,
        word_bgn: 0,
        word_len: 0,
    };
    let mut eol_placed = false;

    // _MakeOneLine: "while( pWork->nPos < nLength )", where the length includes the line ending.
    loop {
        let at_eol = st.pos >= len;
        if at_eol && (!has_eol || eol_placed) {
            break;
        }
        let ch = if at_eol {
            None
        } else {
            text[st.pos..].chars().next()
        };

        // "bKinsokuEnabled && !_DoKinsokuSkip": while a run goes, no new rule starts.
        if kinsoku_enabled {
            if st.run != Run::None {
                // _DoKinsokuSkip: the run ends when `pos` reaches its end.
                if st.pos >= st.word_bgn + st.word_len {
                    if st.run == Run::Kuto
                        && st.pos == st.word_bgn + st.word_len
                        && !(k.hang_eol && at_eol)
                    {
                        // The hanging punctuation is the last of its row.
                        break_row(text, cfg, max, &mut st, rows);
                    }
                    st.word_len = 0;
                    st.run = Run::None;
                }
            } else if let Some(c) = ch {
                let rest = i64::from(max.raw()) - i64::from(st.x.raw());

                // _DoWordWrap: at the head of an English word, measure it and break before it
                // if it would reach the last column.
                if cfg.word_wrap && is_word_byte_at(text, st.pos) {
                    let n = text.as_bytes()[st.pos..]
                        .iter()
                        .take_while(|&&b| is_word_byte(b))
                        .count();
                    st.word_bgn = st.pos;
                    st.word_len = n;
                    st.run = Run::WordWrap;
                    // Every character of a word is ASCII: one column each on the grid, and the
                    // widths of the characters added up in a table (`_GetKeywordLength` adds up
                    // `GetLayoutXOfChar`).
                    let word = cfg.width.ascii_width(&text.as_bytes()[st.pos..st.pos + n]);
                    if st.x + word >= max && st.pos > st.bgn {
                        break_row(text, cfg, max, &mut st, rows);
                    }
                }

                // _DoKutoBurasage: a punctuation that does not fit hangs over the edge. The
                // width is the character's own, not the tab's (GetLayoutXOfChar).
                if k.kuto && st.run == Run::None && (table || rest < i64::from(two.raw())) {
                    let w = i64::from(cfg.width.width_of(c).raw());
                    if rest < w && k.is_kuto(c) {
                        st.word_bgn = st.pos;
                        st.word_len = c.len_utf8();
                        st.run = Run::Kuto;
                    }
                }

                // _DoGyotoKinsoku and _DoGyomatsuKinsoku. "nPos+1 < length" means that a
                // character (or the line ending) follows.
                let after = st.pos + c.len_utf8();
                let has_next = after < len || has_eol;
                if (k.head || k.tail)
                    && st.run == Run::None
                    && has_next
                    && (table || rest < i64::from(four.raw()))
                    && st.x > st.indent
                {
                    let next = text[after..].chars().next();
                    let w1 = i64::from(cfg.width.width_of(c).raw());
                    // The line ending is one column; beyond it there is nothing.
                    let w2 = i64::from(next.map_or(one, |n| cfg.width.width_of(n)).raw());
                    if k.head
                        && rest < w1 + w2
                        && next.is_some_and(|n| k.is_head(n))
                        && !k.is_head(c)
                        && !k.is_kuto(c)
                    {
                        st.word_bgn = st.pos;
                        st.word_len = c.len_utf8() + next.map_or(0, char::len_utf8);
                        st.run = Run::Head;
                        break_row(text, cfg, max, &mut st, rows);
                    } else if k.tail && rest < w1 + w2 && k.is_tail(c) {
                        st.word_bgn = st.pos;
                        st.word_len = c.len_utf8();
                        st.run = Run::Tail;
                        break_row(text, cfg, max, &mut st, rows);
                    }
                }
            }
        }

        // Place the character; break the row first if it does not fit. The row is never left
        // empty for this (Sakura cannot meet that case, the indent is kept small).
        let w = ch.map_or(one, |c| cfg.advance(c, st.x));
        if st.x + w > max && st.run != Run::Kuto && st.pos > st.bgn && !(k.hang_eol && at_eol) {
            break_row(text, cfg, max, &mut st, rows);
            continue;
        }
        st.x += w;
        match ch {
            Some(c) => st.pos += c.len_utf8(),
            None => eol_placed = true,
        }
    }

    // "if( pWork->nPos - pWork->nBgn > 0 )": the last row.
    if st.pos > st.bgn || eol_placed || rows.is_empty() {
        rows.push(Row {
            range: st.bgn..st.pos,
            start_x: st.indent,
            end_x: if eol_placed { st.x - one } else { st.x },
            wrapped: false,
        });
    }
}

fn is_word_byte_at(text: &str, pos: usize) -> bool {
    text.as_bytes().get(pos).is_some_and(|&b| is_word_byte(b))
}

/// `_OnLine1`: closes the row at `pos` and starts the next one at its indent.
fn break_row(text: &str, cfg: &LayoutConfig, max: X, st: &mut State, rows: &mut Vec<Row>) {
    rows.push(Row {
        range: st.bgn..st.pos,
        start_x: st.indent,
        end_x: st.x,
        wrapped: true,
    });
    st.bgn = st.pos;
    st.indent = next_indent(text, cfg, max, rows.last().expect("a row was just pushed"));
    st.x = st.indent;
}

/// The indent of the row after `prev` (`m_getIndentOffset`).
fn next_indent(text: &str, cfg: &LayoutConfig, max: X, prev: &Row) -> X {
    match &cfg.indent {
        WrapIndent::None => X::ZERO,
        // getIndentOffset_Tx2x
        WrapIndent::AfterLastTab => {
            // A wrapped row keeps the indent of the row before it.
            if prev.range.start > 0 {
                return prev.start_x;
            }
            let mut col = prev.start_x;
            let mut indent = prev.start_x;
            for c in text[prev.range.clone()].chars() {
                col += cfg.advance(c, col);
                if c == '\t' {
                    indent = col;
                }
            }
            clamp_indent(indent, max, cfg)
        }
        // getIndentOffset_LeftSpace
        WrapIndent::LeadingWhiteSpace {
            full_width_space,
            extra,
        } => {
            if prev.range.start > 0 {
                return prev.start_x;
            }
            let mut col = prev.start_x;
            for c in text[prev.range.clone()].chars() {
                // WCODE::IsIndentChar, then the "other characters to indent".
                let is_indent = c == '\t'
                    || c == ' '
                    || (*full_width_space && c == '\u{3000}')
                    || extra.contains(c);
                if !is_indent {
                    break;
                }
                col += cfg.advance(c, col);
            }
            clamp_indent(col, max, cfg)
        }
    }
}

/// "2010.07.06 Moca": when less than the tab width plus 2 columns remain, the indent is cut
/// back, so that a tab always fits and the wrapping never loops.
fn clamp_indent(indent: X, max: X, cfg: &LayoutConfig) -> X {
    let need = cfg.tab_x() + cfg.width.to_x(2);
    if max.saturating_sub(indent) < need {
        max.saturating_sub(need)
    } else {
        indent
    }
}

impl Row {
    /// The text of the row, out of the line it was laid out from.
    pub fn text<'a>(&self, text: &'a str) -> &'a str {
        &text[self.range.clone()]
    }
}
