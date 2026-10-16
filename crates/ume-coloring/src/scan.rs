// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/view/colors/CColorStrategy.cpp, sakura_core/view/colors/CColorStrategy.h
//     (CheckChangeColor, CheckColorMODE, the order of the pool)
//   sakura_core/view/colors/CColor_Comment.cpp, sakura_core/view/colors/CColor_Quote.cpp,
//     sakura_core/view/colors/CColor_Heredoc.cpp, sakura_core/view/colors/CColor_Numeric.cpp,
//     sakura_core/view/colors/CColor_KeywordSet.cpp, sakura_core/view/colors/CColor_Url.cpp
//   sakura_core/doc/layout/CLayoutMgr_DoLayout.cpp (_MakeOneLine)
//   sakura_core/view/CEditView_Paint.cpp (DrawLogicLine)
//   sakura_core/doc/CBlockComment.cpp, sakura_core/doc/CLineComment.cpp (Match_CommentTo)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2002, YAZAKI, Moca
//   Copyright (C) 2005, D.S.Koba
//   Copyright (C) 2008, kobake
//   Copyright (C) 2011, Moca
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

//! Scanning one line: the color state at its end, and the color runs in it.
//!
//! Sakura colors a line with a set of *strategies* (`CColorStrategy`). At each character the
//! strategy that is active may end, and when none is active the strategies are tried in a fixed
//! order until one begins ([`SColorStrategyInfo::CheckChangeColor`], `CColorStrategyPool`). This
//! module does that in two passes, as Sakura does:
//!
//! * [`next_state`] is the pass of the layout (`CLayoutMgr::_MakeOneLine` with
//!   `CColorStrategyPool::CheckColorMODE`). It tries only the strategies that can go over the
//!   end of a line (here-documents, block comments, line comments, strings), and gives the state
//!   the next line starts in.
//! * [`color_runs`] is the pass of the painting (`CEditView::DrawLogicLine`). It tries every
//!   strategy and gives the runs.
//!
//! The pool order is: here-document, block comment 1, block comment 2, line comment, single
//! quote, double quote, URL, number, keyword. The first to begin wins, and nothing else begins
//! until it ends.
//!
//! # The line
//!
//! `line` is the text with its line ending, as Sakura's `GetStringRefWithEOL`; `text_len` is the
//! length of the text without it. Sakura visits the line ending too: it ends a line comment, a
//! here-document id has to be followed by it, and a string looks for its closing quote through
//! it. The positions are byte offsets.

use ume_config::settings::type_rules::string_type;

use crate::charkind::{
    CharKind, char_len_at, is_keyword_char, kind_at, next_word_break, prev_char_start,
    two_chars_kind,
};
use crate::keyword::Lookup;
use crate::kind::{ColorKind, ColorRun};
use crate::number::number_len;
use crate::quote::{
    Escape, escape_of, is_cpp_raw_string, match_quote, match_quote_str, tracks_escaped_line_end,
};
use crate::rules::ColorRules;
use crate::url::url_len;

/// The color state at the start of a line: which strategy is still active from the lines before.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ColorState {
    /// No strategy is active.
    #[default]
    Normal,
    /// Inside the block comment 0 or 1 (`COLORIDX_BLOCK1`, `COLORIDX_BLOCK2`).
    Block(u8),
    /// Inside a string that went over the end of a line.
    Quote { double: bool, kind: QuoteKind },
    /// Inside a here-document, which ends at a line that starts with the id.
    Heredoc(String),
}

/// What kind of string is open (`m_nColorTypeIndex` and `m_tag` of `CColor_Quote`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuoteKind {
    /// An ordinary string, closed by the quote.
    Plain,
    /// A C++ raw string `R"tag( ... )tag"`; the text is what closes it, `)tag"`.
    Raw(String),
    /// A C# verbatim string `@"..."`, where a doubled quote is a quote.
    Verbatim,
    /// A Python string in triple quotes.
    Triple,
}

/// A strategy that has begun, with where it ends.
#[derive(Clone, Debug)]
enum Active {
    Heredoc {
        id: String,
        end: usize,
    },
    /// `end` is `None` while the line has no end of the comment.
    Block {
        index: usize,
        end: Option<usize>,
    },
    LineComment,
    Quote {
        double: bool,
        kind: QuoteKind,
        end: usize,
    },
    Url {
        end: usize,
    },
    Number {
        end: usize,
    },
    Keyword {
        slot: u8,
        end: usize,
    },
}

/// The pass of the layout: the state at the end of `line` when it starts in `state`.
pub fn next_state(
    rules: &ColorRules,
    line: &str,
    text_len: usize,
    state: &ColorState,
) -> ColorState {
    if !rules.has_range_based() {
        return ColorState::Normal;
    }
    let scan = Scan::new(rules, line, text_len);
    let mut active = scan.resume(state);
    // The layout visits the first character of a line ending only (`nLength` in `_MakeOneLine`).
    let visit_end = if line.ends_with("\r\n") {
        line.len() - 1
    } else {
        line.len()
    };
    let mut pos = 0;
    while pos < visit_end {
        if active.as_ref().is_some_and(|a| scan.ends(a, pos)) {
            active = None;
        }
        if active.is_none() {
            active = scan.begin_ranged(pos);
        }
        pos = scan.advance(pos, active.as_ref(), visit_end);
    }
    scan.state_of(active.as_ref())
}

/// The pass of the painting: the runs of `line` from the start up to `upto` (a byte offset; the
/// last run is cut there), when it starts in `state`. The runs cover the text; when `upto` reaches
/// the end of the text the last run goes on through the line ending, so that its color is the
/// color at the line ending.
pub fn color_runs(
    rules: &ColorRules,
    line: &str,
    text_len: usize,
    state: &ColorState,
    upto: usize,
) -> Vec<ColorRun> {
    let scan = Scan::new(rules, line, text_len);
    let mut active = scan.resume(state);
    let visit_end = if upto >= text_len { line.len() } else { upto };
    let mut runs: Vec<ColorRun> = Vec::new();
    let mut run_start = 0;
    let mut current = ColorKind::Text;
    let mut pos = 0;
    while pos < visit_end {
        if active.as_ref().is_some_and(|a| scan.ends(a, pos)) {
            active = None;
        }
        if active.is_none() {
            active = scan.begin_any(pos);
        }
        let kind = active.as_ref().map_or(ColorKind::Text, kind_of);
        if kind != current {
            if run_start < pos {
                runs.push(ColorRun {
                    start: run_start,
                    end: pos,
                    kind: current,
                });
            }
            run_start = pos;
            current = kind;
        }
        pos = scan.advance(pos, active.as_ref(), visit_end);
    }
    if run_start < visit_end {
        runs.push(ColorRun {
            start: run_start,
            end: visit_end,
            kind: current,
        });
    }
    runs
}

fn kind_of(active: &Active) -> ColorKind {
    match active {
        Active::Heredoc { .. } => ColorKind::Heredoc,
        Active::Block { .. } | Active::LineComment => ColorKind::Comment,
        Active::Quote { double: false, .. } => ColorKind::SingleQuote,
        Active::Quote { double: true, .. } => ColorKind::DoubleQuote,
        Active::Url { .. } => ColorKind::Url,
        Active::Number { .. } => ColorKind::Number,
        Active::Keyword { slot, .. } => ColorKind::Keyword(*slot),
    }
}

struct Scan<'a> {
    rules: &'a ColorRules,
    line: &'a str,
    bytes: &'a [u8],
    text_len: usize,
}

impl<'a> Scan<'a> {
    fn new(rules: &'a ColorRules, line: &'a str, text_len: usize) -> Self {
        Self {
            rules,
            line,
            bytes: line.as_bytes(),
            text_len: text_len.min(line.len()),
        }
    }

    /// Whether `pos` is in the line ending (`WCODE::IsLineDelimiter`).
    fn in_line_ending(&self, pos: usize) -> bool {
        pos >= self.text_len && pos < self.line.len()
    }

    /// Where the scan goes after `pos`: to the next character, or, if the active strategy has a
    /// known end, straight there (nothing in between can end or begin anything).
    fn advance(&self, pos: usize, active: Option<&Active>, limit: usize) -> usize {
        let step = pos + char_len_at(self.line, pos).max(1);
        let jump = match active {
            Some(Active::Heredoc { end, .. })
            | Some(Active::Quote { end, .. })
            | Some(Active::Url { end })
            | Some(Active::Number { end })
            | Some(Active::Keyword { end, .. }) => *end,
            Some(Active::Block { end, .. }) => end.unwrap_or(limit),
            Some(Active::LineComment) => self.text_len,
            None => step,
        };
        step.max(jump.min(limit))
    }

    // ---- ending ----

    /// `EndColor` of every strategy.
    fn ends(&self, active: &Active, pos: usize) -> bool {
        match active {
            Active::Heredoc { end, .. }
            | Active::Quote { end, .. }
            | Active::Url { end }
            | Active::Number { end }
            | Active::Keyword { end, .. } => pos >= *end,
            Active::Block { end, .. } => end.is_some_and(|e| pos >= e),
            Active::LineComment => self.in_line_ending(pos) || pos >= self.line.len(),
        }
    }

    // ---- the state at the start of a line ----

    /// The strategy that is active at the start of a line in `state` (`GetStrategyByColor`,
    /// `InitStrategyStatus` and `SetStrategyColorInfo`), with the end it finds on this line the way
    /// the first `EndColor` call of the line does. `None` if that strategy is off.
    fn resume(&self, state: &ColorState) -> Option<Active> {
        match state {
            ColorState::Normal => None,
            ColorState::Block(index) => {
                let index = usize::from(*index);
                self.rules.blocks.get(index)?.as_ref()?;
                Some(Active::Block {
                    index,
                    end: self.block_end(index, 0),
                })
            }
            ColorState::Quote { double, kind } => {
                let on = if *double {
                    self.rules.double_quote
                } else {
                    self.rules.single_quote
                };
                if !on {
                    return None;
                }
                let quote = if *double { '"' } else { '\'' };
                let end = match kind {
                    QuoteKind::Plain => match_quote(
                        self.line,
                        self.text_len,
                        quote,
                        0,
                        escape_of(self.rules.string_type),
                        None,
                    ),
                    QuoteKind::Raw(tag) => match_quote_str(self.line, tag, 0, false),
                    QuoteKind::Verbatim => {
                        match_quote(self.line, self.text_len, quote, 0, Escape::Doubled, None)
                    }
                    QuoteKind::Triple => {
                        match_quote_str(self.line, &quote.to_string().repeat(3), 0, true)
                    }
                };
                Some(Active::Quote {
                    double: *double,
                    kind: kind.clone(),
                    end,
                })
            }
            ColorState::Heredoc(id) => {
                if !self.rules.heredoc {
                    return None;
                }
                Some(Active::Heredoc {
                    id: id.clone(),
                    end: self.heredoc_end(id),
                })
            }
        }
    }

    /// `CColor_Heredoc::EndColor` at the start of a line: where the here-document ends on this
    /// line. It ends after the id when the line is the id alone, or the id and `;`.
    fn heredoc_end(&self, id: &str) -> usize {
        let len = self.line.len();
        if self.line.starts_with(id) {
            if id.len() == len {
                return id.len();
            }
            let i = id.len();
            if i + 1 < len && self.bytes[i] == b';' && self.in_line_ending(i + 1) {
                return i;
            }
            if i < len && self.in_line_ending(i) {
                return i;
            }
        }
        len
    }

    /// The state a line ends in, from the strategy that is active at its end.
    fn state_of(&self, active: Option<&Active>) -> ColorState {
        match active {
            Some(Active::Heredoc { id, .. }) => ColorState::Heredoc(id.clone()),
            Some(Active::Block { index, .. }) => ColorState::Block(*index as u8),
            Some(Active::Quote { double, kind, .. }) => ColorState::Quote {
                double: *double,
                kind: kind.clone(),
            },
            // A line comment ends with the line, and the other strategies are not tried by the
            // pass of the layout.
            _ => ColorState::Normal,
        }
    }

    // ---- beginning ----

    /// `CheckColorMODE`: the strategies that can go over a line ending, in order.
    fn begin_ranged(&self, pos: usize) -> Option<Active> {
        self.begin_heredoc(pos)
            .or_else(|| self.begin_block(0, pos))
            .or_else(|| self.begin_block(1, pos))
            .or_else(|| self.begin_line_comment(pos))
            .or_else(|| self.begin_quote(false, pos))
            .or_else(|| self.begin_quote(true, pos))
    }

    /// `CColorStrategyPool` in order, as far as this module has them.
    fn begin_any(&self, pos: usize) -> Option<Active> {
        self.begin_ranged(pos)
            .or_else(|| self.begin_url(pos))
            .or_else(|| self.begin_number(pos))
            .or_else(|| self.begin_keyword(pos))
    }

    /// `_IsPosKeywordHead`: the character before is not a keyword character.
    fn is_keyword_head(&self, pos: usize) -> bool {
        self.line[..pos]
            .chars()
            .next_back()
            .is_none_or(|c| !is_keyword_char(c))
    }

    /// `CColor_Heredoc::BeginColor`: `<<<ID` or `<<<'ID'` or `<<<"ID"` at the end of the line.
    fn begin_heredoc(&self, pos: usize) -> Option<Active> {
        if !self.rules.heredoc
            || self.bytes[pos] != b'<'
            || pos + 3 >= self.line.len()
            || !self.bytes[pos + 1..].starts_with(b"<<")
        {
            return None;
        }
        let b = self.bytes;
        let len = b.len();
        let mut id_start = pos + 3;
        while id_start < len && (b[id_start] == b'\t' || b[id_start] == b' ') {
            id_start += 1;
        }
        if id_start >= len {
            return None;
        }
        let quote = (b[id_start] == b'\'' || b[id_start] == b'"').then(|| b[id_start]);
        if quote.is_some() {
            id_start += 1;
        }
        let mut i = id_start;
        while i < len && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
            i += 1;
        }
        if i == id_start {
            return None;
        }
        let id_end = i;
        if let Some(q) = quote {
            if i < len && b[i] == q {
                i += 1;
            } else {
                return None;
            }
        }
        if i < len && self.in_line_ending(i) {
            return Some(Active::Heredoc {
                id: self.line[id_start..id_end].to_owned(),
                end: len,
            });
        }
        None
    }

    /// `CBlockComment::Match_CommentTo`: where the block comment `index` ends, searching from
    /// `from`; `None` if the line has no end of it.
    fn block_end(&self, index: usize, from: usize) -> Option<usize> {
        let block = self.rules.blocks.get(index)?.as_ref()?;
        let to = block.to.as_bytes();
        let last = self.bytes.len().checked_sub(to.len())?;
        (from..=last)
            .find(|&i| self.bytes[i..i + to.len()].eq_ignore_ascii_case(to))
            .map(|i| i + to.len())
    }

    /// `CColor_BlockComment::BeginColor`.
    fn begin_block(&self, index: usize, pos: usize) -> Option<Active> {
        let block = self.rules.blocks[index].as_ref()?;
        if !block.matches_start(self.line, pos) {
            return None;
        }
        Some(Active::Block {
            index,
            end: self.block_end(index, pos + block.from.len()),
        })
    }

    /// `CColor_LineComment::BeginColor` (`CLineComment::Match`).
    fn begin_line_comment(&self, pos: usize) -> Option<Active> {
        if self.rules.line_comments.is_empty() {
            return None;
        }
        // Columns are counted in UTF-16 units.
        let column = if self.rules.line_comment_columns {
            self.line[..pos].encode_utf16().count()
        } else {
            0
        };
        self.rules
            .line_comments
            .iter()
            .any(|c| c.matches(self.line, pos, column))
            .then_some(Active::LineComment)
    }

    /// `CColor_Quote::BeginColor`.
    fn begin_quote(&self, double: bool, pos: usize) -> Option<Active> {
        let on = if double {
            self.rules.double_quote
        } else {
            self.rules.single_quote
        };
        let quote = if double { '"' } else { '\'' };
        if !on || self.bytes[pos] != quote as u8 {
            return None;
        }
        let rules = self.rules;
        let len = self.line.len();
        let opened = |kind: QuoteKind, end: usize| Active::Quote { double, kind, end };
        let mut is_string = true;
        match rules.string_type {
            string_type::CPP => {
                if is_cpp_raw_string(self.line, pos)
                    && let Some(open) = self.line[pos + 1..].find('(').map(|i| pos + 1 + i)
                {
                    let delimiter = &self.line[pos + 1..open];
                    let tag = format!("){delimiter}\"");
                    let end = match_quote_str(self.line, &tag, open + 1, false);
                    return Some(opened(QuoteKind::Raw(tag), end));
                }
            }
            string_type::HTML => {
                // Only a value after `=` is a string.
                let before = self.line[..pos].trim_end_matches([' ', '\t']);
                if !before.ends_with('=') {
                    is_string = false;
                }
            }
            string_type::CSHARP => {
                if pos > 0 && self.bytes[pos - 1] == b'@' && quote == '"' {
                    let end = match_quote(
                        self.line,
                        self.text_len,
                        quote,
                        pos + 1,
                        Escape::Doubled,
                        None,
                    );
                    return Some(opened(QuoteKind::Verbatim, end));
                }
            }
            string_type::PYTHON => {
                let triple = pos + 2 < len
                    && self.bytes[pos + 1] == quote as u8
                    && self.bytes[pos + 2] == quote as u8;
                if triple {
                    let closing = quote.to_string().repeat(3);
                    let end = match_quote_str(self.line, &closing, pos + 3, true);
                    return Some(opened(QuoteKind::Triple, end));
                }
            }
            _ => {}
        }
        let mut escaped_end = false;
        let mut end = None;
        if is_string {
            end = Some(match_quote(
                self.line,
                self.text_len,
                quote,
                pos + 1,
                escape_of(rules.string_type),
                tracks_escaped_line_end(rules.string_type).then_some(&mut escaped_end),
            ));
        }
        // "Strings in one line only": a string without a closing quote is not a string, or is
        // colored to the end of the line.
        if rules.string_line_only && !escaped_end && end == Some(len + 1) {
            if rules.string_end_line {
                let end = if self.text_len < len {
                    self.text_len
                } else {
                    len + 1
                };
                return Some(opened(QuoteKind::Plain, end));
            }
            return None;
        }
        end.filter(|e| *e > 0)
            .map(|end| opened(QuoteKind::Plain, end))
    }

    /// `CColor_Url::BeginColor`.
    fn begin_url(&self, pos: usize) -> Option<Active> {
        if !self.rules.url || !self.is_keyword_head(pos) {
            return None;
        }
        let before = pos.checked_sub(1).map(|i| self.bytes[i]);
        match url_len(&self.bytes[pos..], before) {
            0 => None,
            n => Some(Active::Url { end: pos + n }),
        }
    }

    /// `CColor_Numeric::BeginColor`.
    fn begin_number(&self, pos: usize) -> Option<Active> {
        if !self.rules.number || !self.is_keyword_head(pos) {
            return None;
        }
        match number_len(&self.bytes[pos..]) {
            0 => None,
            n => Some(Active::Number { end: pos + n }),
        }
    }

    /// `CColor_KeywordSet::BeginColor`: the longest keyword of the first set (in the order of the
    /// slots) that has one at `pos`.
    fn begin_keyword(&self, pos: usize) -> Option<Active> {
        if self.rules.keywords.is_empty() {
            return None;
        }
        let line = self.line;
        let kind = kind_at(line, pos);
        if kind <= CharKind::Space {
            return None;
        }
        if pos > 0 {
            let prev = prev_char_start(line, pos);
            if two_chars_kind(kind_at(line, prev), kind) != CharKind::Null {
                return None;
            }
        }
        let first_break = next_word_break(line, pos).unwrap_or(pos);
        for (slot, set) in &self.rules.keywords {
            let mut word_end = pos;
            let mut candidate = first_break;
            while let Some(word) = line.get(pos..candidate) {
                match set.lookup(word) {
                    Lookup::Exact => {
                        word_end = candidate;
                        break;
                    }
                    Lookup::ExactAndLonger => word_end = candidate,
                    Lookup::Prefix => {}
                    Lookup::No => break,
                }
                if candidate >= line.len() {
                    break;
                }
                match next_word_break(line, candidate) {
                    Some(next) if next > candidate => candidate = next,
                    _ => break,
                }
            }
            if pos < word_end {
                return Some(Active::Keyword {
                    slot: *slot,
                    end: word_end,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests;
