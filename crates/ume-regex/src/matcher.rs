//! The regular expression [`Matcher`] for the document search.
//!
//! It lives here and not in `ume-core` because the dependency goes one way: `ume-regex` knows
//! `ume-core`'s `Matcher` trait (for which ADR-0006 made the search work line by line), and
//! `ume-core` never learns about the regex engine.

use std::fmt;

use ume_core::search::{Match, Matcher, MatcherError};

use crate::names::GroupNames;
use crate::prepare::{ends_with_escape, make_pattern_alternate};
use crate::{CompileError, Onigmo, Options, Regex, RegexEngine};

/// The search options that go with a regular expression in Sakura's search dialog.
///
/// 単語単位 (`bWordOnly`) is not here: Sakura greys the box out when 正規表現 is on, and
/// `CSearchAgent::SearchWord` takes the regex branch before it looks at `bWordOnly`
/// (`sakura_core/dlg/CDlgFind.cpp`, `sakura_core/agent/CSearchAgent.cpp`). Use `\b` in the
/// pattern for words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegexSearchOptions {
    /// 英大文字と小文字を区別する (`bLoHiCase`). Off by default, as in Sakura: then the pattern
    /// is compiled with the `i` modifier (`CBregexp::MakePatternSub`).
    pub match_case: bool,
}

/// The pattern could not be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternError {
    /// The pattern ends with a backslash that is not escaped. It escapes the delimiter that Sakura
    /// puts after the pattern (`CBregexp::MakePatternSub` builds `m<D>pattern<D>...`), so the
    /// pattern has no end for bregonig.dll. That a backslash escapes the delimiter is the Perl-style
    /// reading; not in bregonig.html (which only says the delimiter may be any character, 5.0);
    /// to check against the real bregonig.dll, including what it reports.
    UnmatchedSeparator,
    /// Onigmo rejected the pattern.
    Compile(CompileError),
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedSeparator => f.write_str("the pattern ends with a backslash"),
            Self::Compile(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for PatternError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnmatchedSeparator => None,
            Self::Compile(e) => Some(e),
        }
    }
}

impl From<CompileError> for PatternError {
    fn from(e: CompileError) -> Self {
        Self::Compile(e)
    }
}

/// A compiled search pattern, as Sakura searches with one.
///
/// * The pattern goes through Sakura's pre-processing ([`make_pattern_alternate`]) and is
///   compiled with the syntax and options of bregonig.dll ([`Onigmo`]).
/// * Matching is line by line, on the line without its line ending. Sakura matches the line with
///   its line ending, so a pattern that can match the ending (`\s+` at the end of a line, `[^a]`,
///   `\r`, `\n`) reaches over it there and stops at the end of the text here; see the PR text.
/// * Backward search and "find all" use the default scans of [`Matcher`], which are Sakura's
///   loops (`CSearchAgent::SearchWord`): after a match the next search starts at its end, after
///   an empty match one char later.
///
/// To replace with a regex, use [`RegexReplacer`](crate::RegexReplacer): the generic replace
/// functions of `ume-core` insert one fixed text, but `$1` and friends differ per match.
#[derive(Debug)]
pub struct RegexMatcher {
    regex: Box<dyn Regex>,
    names: GroupNames,
    pattern: String,
    options: RegexSearchOptions,
}

impl RegexMatcher {
    /// Compiles `pattern`.
    pub fn new(pattern: &str, options: RegexSearchOptions) -> Result<Self, PatternError> {
        if ends_with_escape(pattern) {
            return Err(PatternError::UnmatchedSeparator);
        }
        let prepared = make_pattern_alternate(pattern);
        let engine_options = Options {
            ignore_case: !options.match_case,
            ..Options::default()
        };
        let regex = Onigmo.compile(&prepared, engine_options)?;
        let names = GroupNames::scan(&prepared, engine_options.extended, regex.capture_count());
        Ok(Self {
            regex,
            names,
            pattern: pattern.to_owned(),
            options,
        })
    }

    /// The pattern as it was given.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn options(&self) -> RegexSearchOptions {
        self.options
    }

    /// The number of capture groups, not counting the whole match.
    pub fn capture_count(&self) -> usize {
        self.regex.capture_count()
    }

    /// The names of the groups, for compiling a replacement.
    pub fn names(&self) -> &GroupNames {
        &self.names
    }
}

impl Matcher for RegexMatcher {
    fn find_at(&self, line: &str, start: usize) -> Result<Option<Match>, MatcherError> {
        if start > line.len() {
            return Ok(None);
        }
        let found = self
            .regex
            .captures_at(line, start)
            .map_err(|e| MatcherError::new(e.to_string()))?;
        Ok(found.map(|caps| {
            let whole = caps.whole();
            Match {
                start: whole.start,
                end: whole.end,
                captures: caps.iter().skip(1).collect(),
            }
        }))
    }
}
