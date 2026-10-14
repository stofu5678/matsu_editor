// This file is ported from Sakura Editor (zlib License) and was modified.
//
// Original files:
//   sakura_core/agent/CSearchAgent.cpp (SearchString, CreateWordList, SearchStringWord,
//                                       SearchWord)
//
// Copyright (C) 2008, kobake
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
//
// Modifications: rewritten in Rust for UME Editor. The text is UTF-8 `char`s instead of
// UTF-16 units, lines come without their line ending, and the font-dependent width test
// is replaced by a fixed rule.

//! Matching inside one line: the [`Matcher`] trait and the plain-text matcher.

use std::fmt;
use std::ops::Range;

use super::chars::{Units, eq_folded, fold_char, is_word_delimiter};

/// One match inside a line. Offsets are bytes of the line text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub start: usize,
    pub end: usize,
    /// Capture groups 1, 2, ... (group 0 is `start..end`). `None` for a group that did not take
    /// part in the match. Empty for plain text.
    pub captures: Vec<Option<Range<usize>>>,
}

impl Match {
    /// A match without capture groups.
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            captures: Vec::new(),
        }
    }

    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// A matcher gave up, for example a regex that hit the engine's stack limit. This is not the same
/// as finding no match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatcherError {
    message: String,
}

impl MatcherError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for MatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the search failed: {}", self.message)
    }
}

impl std::error::Error for MatcherError {}

/// Finds matches in one line of decoded text, without its line ending.
///
/// Sakura matches line by line and never across lines (ADR-0006), so the document search only
/// needs this. The plain-text matcher implements it now; the regex crate will implement it
/// later (its `Regex::captures_at` has the same shape: a line, a byte offset, byte ranges).
///
/// A matcher is immutable and shared between threads, so a search can run off the UI thread.
/// `start`, `limit` and every returned offset are on char boundaries of `line`.
pub trait Matcher: Send + Sync + fmt::Debug {
    /// The first match at or after byte `start` (`start <= line.len()`). The line is given whole
    /// so that look-behind and word boundaries can see the text before `start`. A matcher may
    /// return a match that begins before `start` when `start` is inside a word, as Sakura's
    /// whole-word search does.
    fn find_at(&self, line: &str, start: usize) -> Result<Option<Match>, MatcherError>;

    /// Whether a match depends only on its own text, not on the text around it. Plain text
    /// search without whole words is; a whole-word or regex search (`\b`, look-behind) is not.
    /// "Replace all" can then look for all matches in the unchanged document, which is the same
    /// as looking after each replacement. The default is `false`, which is always correct.
    fn is_context_free(&self) -> bool {
        false
    }

    /// The last match of the left-to-right scan from the start of the line that begins before
    /// byte `limit` (`None`: no limit). Sakura searches backward this way: it scans the line
    /// forward and keeps the last hit before the caret, so that matches do not overlap
    /// (`CSearchAgent::SearchWord`, `sakura_core/agent/CSearchAgent.cpp`).
    fn find_last_before(
        &self,
        line: &str,
        limit: Option<usize>,
    ) -> Result<Option<Match>, MatcherError> {
        last_before_by_scan(self, line, limit)
    }

    /// All matches from byte `start`, one after the other without overlap. Matchers that have to
    /// prepare the line (cut it into words) override this to do it once.
    fn find_all_from(&self, line: &str, start: usize) -> Result<Vec<Match>, MatcherError> {
        let mut all = Vec::new();
        scan(self, line, start, |m| {
            all.push(m);
            true
        })?;
        Ok(all)
    }
}

/// The byte offset of the char after the one at `at`, or `None` at the end of the line.
pub(crate) fn next_char_boundary(line: &str, at: usize) -> Option<usize> {
    line[at..].chars().next().map(|c| at + c.len_utf8())
}

/// Rounds `col` up to a char boundary of `line`; `None` if it is past the end.
pub(crate) fn ceil_boundary(line: &str, col: usize) -> Option<usize> {
    if col > line.len() {
        return None;
    }
    let mut at = col;
    while !line.is_char_boundary(at) {
        at += 1;
    }
    Some(at)
}

/// Calls `f` with each match from `start` on until `f` returns false. After a match the next
/// search starts at its end; after an empty match it starts one char later, so that the same
/// empty match is not found again (Sakura: `CSearchAgent::SearchWord` and
/// `CViewCommander::Command_SEARCH_NEXT`, "無限マッチ対策").
pub(crate) fn scan<M: Matcher + ?Sized>(
    matcher: &M,
    line: &str,
    start: usize,
    mut f: impl FnMut(Match) -> bool,
) -> Result<(), MatcherError> {
    let Some(mut pos) = ceil_boundary(line, start) else {
        return Ok(());
    };
    let mut floor = pos;
    while let Some(m) = matcher.find_at(line, pos)? {
        // A matcher may report a match that starts before `pos` (whole-word search); skip those
        // that overlap what was already reported.
        if m.start < floor {
            let Some(next) = next_char_boundary(line, pos) else {
                break;
            };
            pos = next;
            continue;
        }
        let end = m.end;
        let empty = m.is_empty();
        floor = end;
        if !f(m) {
            break;
        }
        pos = if empty {
            match next_char_boundary(line, end) {
                Some(next) => next,
                None => break,
            }
        } else {
            end
        };
    }
    Ok(())
}

/// The default of [`Matcher::find_last_before`]: scan forward and keep the last match that
/// begins before `limit`.
pub(crate) fn last_before_by_scan<M: Matcher + ?Sized>(
    matcher: &M,
    line: &str,
    limit: Option<usize>,
) -> Result<Option<Match>, MatcherError> {
    let mut last = None;
    scan(matcher, line, 0, |m| {
        if limit.is_some_and(|l| m.start >= l) {
            return false;
        }
        last = Some(m);
        true
    })?;
    Ok(last)
}

/// The options of plain-text search that Sakura's search dialog has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlainOptions {
    /// 英大文字と小文字を区別する (`SSearchOption::bLoHiCase`). Off by default, as in Sakura.
    pub match_case: bool,
    /// 単語単位で探す (`SSearchOption::bWordOnly`).
    pub whole_word: bool,
}

/// Plain-text search with Sakura's rules.
///
/// * Without `whole_word`: the pattern is found anywhere in the line. An empty pattern never
///   matches (`CSearchAgent::SearchString` returns null for it).
/// * With `whole_word`: Sakura cuts the *pattern* into words (`CSearchAgent::CreateWordList`,
///   with the word rules of `CWordParse`) and finds any one of them as a whole word of the line
///   (`CSearchAgent::SearchStringWord`). So `foo,bar` finds `foo`, `,` and `bar` separately.
///   Spaces, tabs and ideographic spaces in the pattern only separate words.
/// * Without `match_case`, letters are compared after [`fold_char`].
#[derive(Clone, Debug)]
pub struct PlainMatcher {
    pattern: Needle,
    options: PlainOptions,
    /// For `whole_word`: the words of the pattern.
    words: Vec<Needle>,
}

/// Text to look for, kept as given and, when case is ignored, folded.
#[derive(Clone, Debug)]
struct Needle {
    text: String,
    /// The chars of `text`, folded unless the search matches case.
    chars: Vec<char>,
}

impl Needle {
    fn new(text: &str, match_case: bool) -> Self {
        Self {
            text: text.to_owned(),
            chars: text
                .chars()
                .map(|c| if match_case { c } else { fold_char(c) })
                .collect(),
        }
    }

    /// The first place in `line` at or after `start` where the needle is, as a byte range.
    fn find(&self, line: &str, start: usize, match_case: bool) -> Option<Range<usize>> {
        let first = *self.chars.first()?;
        if match_case {
            let at = line[start..].find(&self.text)? + start;
            return Some(at..at + self.text.len());
        }
        for (i, c) in line[start..].char_indices() {
            if fold_char(c) != first {
                continue;
            }
            let from = start + i;
            let mut chars = line[from..].char_indices();
            let mut end = from;
            let all = self.chars.iter().all(|&p| {
                chars.next().is_some_and(|(j, ch)| {
                    end = from + j + ch.len_utf8();
                    fold_char(ch) == p
                })
            });
            if all {
                return Some(from..end);
            }
        }
        None
    }

    /// Whether `text` is the needle (comparing folded chars unless the search matches case).
    fn is(&self, text: &str, match_case: bool) -> bool {
        if match_case {
            text == self.text
        } else {
            eq_folded(text, &self.chars)
        }
    }
}

impl PlainMatcher {
    pub fn new(pattern: &str, options: PlainOptions) -> Self {
        let mut words = Vec::new();
        if options.whole_word {
            let units = Units::new(pattern);
            let mut i = 0;
            while i < units.len() {
                let end = units.word_end_from(i);
                if !is_word_delimiter(units.char_at(i)) {
                    let word = &pattern[units.byte(i)..units.byte(end)];
                    words.push(Needle::new(word, options.match_case));
                }
                i = end;
            }
        }
        Self {
            pattern: Needle::new(pattern, options.match_case),
            options,
            words,
        }
    }

    pub fn options(&self) -> PlainOptions {
        self.options
    }

    /// The pattern as it was given.
    pub fn pattern(&self) -> &str {
        &self.pattern.text
    }

    fn find_plain(&self, line: &str, start: usize) -> Option<Match> {
        let r = self.pattern.find(line, start, self.options.match_case)?;
        Some(Match::new(r.start, r.end))
    }

    /// False if no word of the pattern can be in `line`: a cheap test before the line is cut
    /// into words.
    fn may_contain_word(&self, line: &str) -> bool {
        self.words
            .iter()
            .any(|w| w.find(line, 0, self.options.match_case).is_some())
    }

    fn is_word_of_pattern(&self, text: &str) -> bool {
        self.words
            .iter()
            .any(|w| w.is(text, self.options.match_case))
    }

    /// `CSearchAgent::SearchStringWord`: starting at the word that holds `start`, test one word
    /// after the other.
    fn words_from(
        &self,
        line: &str,
        units: &Units,
        start: usize,
        mut f: impl FnMut(Match) -> bool,
    ) {
        let mut idx = units.unit_at_or_after(start);
        let mut floor = 0;
        while let Some(word) = units.current_word(idx) {
            let (a, b) = (units.byte(word.start), units.byte(word.end));
            if a >= floor && self.is_word_of_pattern(&line[a..b]) {
                floor = b;
                if !f(Match::new(a, b)) {
                    return;
                }
                idx = word.end;
                continue;
            }
            match units.next_word(idx) {
                Some(next) => idx = next,
                None => return,
            }
        }
    }
}

impl Matcher for PlainMatcher {
    fn is_context_free(&self) -> bool {
        !self.options.whole_word
    }

    fn find_at(&self, line: &str, start: usize) -> Result<Option<Match>, MatcherError> {
        let Some(start) = ceil_boundary(line, start) else {
            return Ok(None);
        };
        if !self.options.whole_word {
            return Ok(self.find_plain(line, start));
        }
        if !self.may_contain_word(line) {
            return Ok(None);
        }
        let units = Units::new(line);
        let mut found = None;
        self.words_from(line, &units, start, |m| {
            found = Some(m);
            false
        });
        Ok(found)
    }

    /// Sakura's backward whole-word search walks the words to the left of the caret
    /// (`CSearchAgent::SearchWord`, `PrevOrNextWord` with `CWordParse::SearchPrevWordPosition`).
    fn find_last_before(
        &self,
        line: &str,
        limit: Option<usize>,
    ) -> Result<Option<Match>, MatcherError> {
        if !self.options.whole_word {
            return last_before_by_scan(self, line, limit);
        }
        if !self.may_contain_word(line) {
            return Ok(None);
        }
        let units = Units::new(line);
        let mut idx = match limit {
            Some(col) => units.unit_at_or_after(col),
            None => units.len(),
        };
        while let Some(prev) = units.prev_word(idx) {
            idx = prev;
            let Some(word) = units.current_word(idx) else {
                break;
            };
            let (a, b) = (units.byte(word.start), units.byte(word.end));
            if self.is_word_of_pattern(&line[a..b]) {
                return Ok(Some(Match::new(a, b)));
            }
        }
        Ok(None)
    }

    fn find_all_from(&self, line: &str, start: usize) -> Result<Vec<Match>, MatcherError> {
        if !self.options.whole_word {
            let mut all = Vec::new();
            scan(self, line, start, |m| {
                all.push(m);
                true
            })?;
            return Ok(all);
        }
        let Some(start) = ceil_boundary(line, start) else {
            return Ok(Vec::new());
        };
        if !self.may_contain_word(line) {
            return Ok(Vec::new());
        }
        let units = Units::new(line);
        let mut all = Vec::new();
        self.words_from(line, &units, start, |m| {
            all.push(m);
            true
        });
        Ok(all)
    }
}
