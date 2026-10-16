//! Regular expressions for UME Editor.
//!
//! Sakura Editor searches with bregonig.dll, which is Onigmo with Perl-style
//! syntax. This crate vendors Onigmo ([ADR-0006]) and hides it behind the
//! [`RegexEngine`] and [`Regex`] traits, so search code never touches Onigmo
//! and another engine could replace it.
//!
//! Like Sakura, the engine matches one line at a time: compile a pattern once,
//! then call [`Regex::captures_at`] on each line. Positions are byte offsets
//! into the `&str` that is passed in (UTF-8).
//!
//! ```
//! use ume_regex::{Onigmo, Options, RegexEngine};
//!
//! let re = Onigmo.compile(r"(\d+)年", Options::default()).unwrap();
//! let caps = re.captures_at("令和７年と2026年", 0).unwrap().unwrap();
//! assert_eq!(caps.get(1), Some(6..9)); // the full-width "７": `\d` matches it
//! ```
//!
//! # Syntax and options
//!
//! The syntax is bregonig's: `ONIG_SYNTAX_PERL` plus `(?~...)`, `\g<name>`,
//! `[a&&b]` set operations and different-length alternatives in look-behind
//! (`bregonig.html`, sections 5.1, 5.1.6, 5.1.7 and 5.1.9). The options follow what
//! Sakura passes in `CBregexp::MakePatternSub` (`sakura_core/extmodule/CBregexp.cpp`):
//! the pattern is always compiled with the `m` modifier, which is
//! `ONIG_OPTION_NEGATE_SINGLELINE`. `\w`, `\d` and `\s` are Unicode-aware
//! unless [`Options::ascii_range`] is set, so `\d` matches full-width digits
//! (`help/sakura/res/HLP000089.html`).
//!
//! # Document search
//!
//! On top of the engine, this crate adapts it to the search of `ume-core`:
//!
//! * [`RegexMatcher`] implements `ume_core::search::Matcher` with Sakura's options and
//!   pre-processing of the pattern ([`make_pattern_alternate`]).
//! * [`ReplacementTemplate`] is bregonig's replacement syntax (`$1`, `${name}`, `\n`, `\U`...),
//!   written from bregonig's documentation (`bregonig.html`) without its source code, which is
//!   under the GPL or the Artistic License.
//! * [`RegexReplacer`] replaces in a `Document` (one match, the next one, all) with it.
//!
//! [ADR-0006]: https://github.com/stofu5678/ume_editor/blob/main/docs/adr/0006-regex-engine.md

mod ffi;
mod matcher;
mod names;
mod prepare;
mod replace;
mod replacement;

pub use matcher::{PatternError, RegexMatcher, RegexSearchOptions};
pub use names::GroupNames;
pub use prepare::{ends_with_escape, make_pattern_alternate};
pub use replace::{RegexReplacer, ReplaceAllMode};
pub use replacement::{ReplacementError, ReplacementProblem, ReplacementTemplate};

/// Tests take the read side; the Onigmo suite takes the write side while it
/// changes Onigmo's process-wide limits.
#[cfg(test)]
static LIMITS_LOCK: std::sync::RwLock<()> = std::sync::RwLock::new(());

#[cfg(test)]
mod onigmo_suite;

use std::fmt;
use std::ops::Range;

use ffi::{Compiled, OnigError, OptionBits, Syntax};

/// Compile options. The default is what Sakura's search dialog gives with no
/// box ticked: case-sensitive, `\w` and friends Unicode-aware.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Ignore case (Sakura: "大文字と小文字を区別する" unticked; the `i` modifier).
    pub ignore_case: bool,
    /// Extended syntax: whitespace and `#` comments in the pattern are ignored (the `x` modifier).
    pub extended: bool,
    /// Limit `\w`, `\d`, `\s` and POSIX brackets to ASCII (the `a` modifier).
    pub ascii_range: bool,
    /// Treat CR LF as one line terminator for `$` and `.` (the `R` modifier).
    pub crlf: bool,
}

impl Options {
    fn to_onigmo(self) -> u32 {
        // `m` is always on in Sakura (CBregexp.cpp: `*pPat++ = L'm'`).
        let mut bits = OptionBits::negate_singleline();
        if self.ignore_case {
            bits |= OptionBits::ignore_case();
        }
        if self.extended {
            bits |= OptionBits::extend();
        }
        if self.ascii_range {
            bits |= OptionBits::ascii_range();
        }
        if self.crlf {
            bits |= OptionBits::newline_crlf();
        }
        bits
    }
}

/// The pattern could not be compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileError {
    code: i32,
    message: String,
}

impl CompileError {
    /// The engine's own message, for example `end pattern with unmatched parenthesis`.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The engine's error code (Onigmo: a negative `ONIGERR_*`). Not portable
    /// across engines; use it for diagnostics only.
    pub fn code(&self) -> i32 {
        self.code
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CompileError {}

impl From<OnigError> for CompileError {
    fn from(e: OnigError) -> Self {
        Self {
            code: e.code,
            message: e.message,
        }
    }
}

/// A search failed (not the same as finding no match).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchError {
    /// `start` is beyond the end of the text or inside a multi-byte character.
    InvalidStart { start: usize, len: usize },
    /// The engine gave up, for example because the match stack limit was exceeded.
    Engine { code: i32, message: String },
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStart { start, len } => write!(
                f,
                "start offset {start} is not on a character boundary of a text of {len} bytes"
            ),
            Self::Engine { message, .. } => f.write_str(message),
        }
    }
}

impl std::error::Error for SearchError {}

/// The capture groups of one match, as byte ranges into the searched text.
/// Group 0 is the whole match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captures {
    groups: Vec<Option<Range<usize>>>,
}

impl Captures {
    /// The whole match.
    pub fn whole(&self) -> Range<usize> {
        // Group 0 always participates in a match.
        self.groups[0].clone().unwrap_or(0..0)
    }

    /// Group `index` (0 is the whole match), or `None` if it did not take part
    /// in the match or does not exist.
    pub fn get(&self, index: usize) -> Option<Range<usize>> {
        self.groups.get(index).cloned().flatten()
    }

    /// The number of groups, including group 0.
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Always `false`: a match has at least group 0.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// All groups in order, starting with the whole match.
    pub fn iter(&self) -> impl Iterator<Item = Option<Range<usize>>> + '_ {
        self.groups.iter().cloned()
    }
}

/// A compiled pattern. It is immutable and can be shared between threads.
pub trait Regex: Send + Sync + fmt::Debug {
    /// The number of capture groups, not counting the whole match.
    fn capture_count(&self) -> usize;

    /// Finds the first match that starts at or after byte offset `start` of
    /// `text`. `text` is one whole line: `^`, `\G` (which is `start`) and
    /// look-behind see the part before `start`.
    fn captures_at(&self, text: &str, start: usize) -> Result<Option<Captures>, SearchError>;

    /// Like [`captures_at`](Self::captures_at), but `\G` is `gpos` instead of `start`. The `g`
    /// option of bregonig.dll keeps `\G` at the end of the last match while it steps over an
    /// empty one, so that `\G` matches only there (measured on bregonig.dll 4.20).
    fn captures_from(
        &self,
        text: &str,
        start: usize,
        gpos: usize,
    ) -> Result<Option<Captures>, SearchError>;

    /// Like [`captures_at`](Self::captures_at), but only the range of the whole match.
    fn find_at(&self, text: &str, start: usize) -> Result<Option<Range<usize>>, SearchError> {
        Ok(self.captures_at(text, start)?.map(|c| c.whole()))
    }
}

/// A regular expression engine.
pub trait RegexEngine {
    /// Compiles `pattern` once; the result is used for many searches.
    fn compile(&self, pattern: &str, options: Options) -> Result<Box<dyn Regex>, CompileError>;
}

/// The Onigmo engine, with the syntax and options of bregonig.dll.
#[derive(Clone, Copy, Debug, Default)]
pub struct Onigmo;

impl RegexEngine for Onigmo {
    fn compile(&self, pattern: &str, options: Options) -> Result<Box<dyn Regex>, CompileError> {
        let compiled = Compiled::new(pattern.as_bytes(), options.to_onigmo(), Syntax::Bregonig)?;
        Ok(Box::new(OnigmoRegex { compiled }))
    }
}

struct OnigmoRegex {
    compiled: Compiled,
}

impl fmt::Debug for OnigmoRegex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnigmoRegex")
            .field("capture_count", &self.compiled.capture_count())
            .finish_non_exhaustive()
    }
}

impl Regex for OnigmoRegex {
    fn capture_count(&self) -> usize {
        self.compiled.capture_count()
    }

    fn captures_at(&self, text: &str, start: usize) -> Result<Option<Captures>, SearchError> {
        self.search(text, None, start)
    }

    fn captures_from(
        &self,
        text: &str,
        start: usize,
        gpos: usize,
    ) -> Result<Option<Captures>, SearchError> {
        self.search(text, Some(gpos), start)
    }
}

impl OnigmoRegex {
    fn search(
        &self,
        text: &str,
        gpos: Option<usize>,
        start: usize,
    ) -> Result<Option<Captures>, SearchError> {
        if !text.is_char_boundary(start) || gpos.is_some_and(|g| !text.is_char_boundary(g)) {
            return Err(SearchError::InvalidStart {
                start,
                len: text.len(),
            });
        }
        let found = self
            .compiled
            .search(text.as_bytes(), gpos, start, text.len(), 0)
            .map_err(|e| SearchError::Engine {
                code: e.code,
                message: e.message,
            })?;
        Ok(found.map(|groups| Captures {
            groups: groups
                .into_iter()
                .map(|g| g.map(|(beg, end)| beg..end))
                .collect(),
        }))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod search_tests;
