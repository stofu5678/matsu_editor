//! The replacement syntax of bregonig (the "pattern2" of Sakura's regular-expression replace).
//!
//! This module is an independent implementation. It was written from the user documentation of
//! bregonig (`bregonig.html`, section 5.3) and from Sakura's help, without looking at the source
//! code of bregonig, for ADR-0006. Where the documentation is silent it follows what the real
//! bregonig.dll 4.20 (Sakura's package) does, measured on 2026-10-16 with `tools/bregonig-compat`;
//! `tests/bregonig_compat.rs` replays those measurements.
//!
//! # Syntax
//!
//! | Text | Meaning |
//! |------|---------|
//! | `\t \n \r \b \f \a \e` | tab, LF, CR, backspace, form feed, bell, ESC |
//! | `\0nn` | octal character (at most 3 digits including the `0`) |
//! | `\xHH`, `\x{H...}` | hexadecimal character (1-2 digits, or up to 8 in braces; `\x{}` is U+0000) |
//! | `\o{n...}` | octal character in braces (`\o{}` is U+0000) |
//! | `\cX` | control character (`\c[` is ESC); `\c\X` is read as `\cX` |
//! | `\l \u` | the next character in lower / upper case |
//! | `\L \U \E` | lower / upper case until `\E` (or the end) |
//! | `\Q` | nothing (it is skipped; it does not quote) |
//! | `\1`..`\9`, `$1`.. | group by number |
//! | `\10` and more | the group if the pattern has that many, else 2-3 octal digits up to `\377` make a character, else the group (empty) |
//! | `${n}` | group by number, safe before digits (`${0}` is the whole match) |
//! | `$&` | the whole match |
//! | `$+` | the group with the highest number that took part in the match |
//! | `\k<name>`, `\k'name'`, `${name}`, `$+{name}` | the first group of that name (lowest number) |
//! | `$-{name}[n]` | the n-th group of that name (`0` first, `-1` last) |
//! | `\` + other | the character itself (`\\`, `\$`, `\/`, `\v`, ...) |
//!
//! Case conversion only touches ASCII letters, as the documentation says.
//!
//! # What bregonig.dll 4.20 does where the documentation is silent
//!
//! * A reference that cannot be resolved (a group number past the pattern's groups, an unknown
//!   name) gives an empty string and is listed in [`ReplacementTemplate::problems`].
//! * A `$` that does not start a reference is text together with the character after it, which
//!   is not read as anything: `$$1` is the text `$$1`, `$\n` the text `$\n`, `$0` the text `$0`.
//! * `${...}`: a form that starts with a digit has to be digits and `}` (`${1a}` and `${1 }` are
//!   text); any other form is a name up to the first `}`, whatever it holds (`${}`, `${a b}`,
//!   `${ 1}`, `${-1}` are names no group has, so they give nothing). Without a `}` it is text.
//!   `$+{...}`, `$-{...}[n]` and `\k<...>` read their names the same way; `\k<1>` is the name
//!   "1", so it gives nothing.
//! * `$-{name}[n]`: the index is `[]` (0) or white space, an optional sign and digits, then `]`;
//!   anything else makes the whole form text.
//! * A malformed backslash form (`\x{41`, `\k<x`, `\o`) drops the backslash and keeps the rest as
//!   text. `\x` with no hexadecimal digit, `\x{` with more than 8 digits or a non-digit are
//!   malformed.
//! * `\xHH` and the octal escapes give the code point with that number, not a byte of some
//!   encoding: Sakura uses the UTF-16 API of bregonig (`sakura_core/extmodule/CBregexpDll2.cpp`),
//!   so `\xC3\xA9` is two characters, not `é`.
//! * Several groups with one name: `\k<n>`, `${n}`, `$+{n}` use the group with the lowest number,
//!   even if it did not take part in the match (then they give nothing).
//! * `\L`/`\U` stay in effect over literal text, escapes and groups; `\l`/`\u` wait for the next
//!   character that is written (an empty group does not use it up), and win over `\L`/`\U`.
//! * A replacement that ends with an odd number of backslashes is an error
//!   ([`ReplacementError::UnmatchedSeparator`]), see [`ends_with_escape`].
//!
//! # Known differences from the DLL
//!
//! * The DLL turns `\x{D800}` (a surrogate) and `\x{110000}` and above into lone surrogates in
//!   its UTF-16 text; a Rust string cannot hold them, so they give nothing here.
//! * `$-{name}[n]` with an `n` past the groups reads memory it should not in the DLL (the 32-bit
//!   and the 64-bit DLL differ, and `-2` for a single group gives that group); here it is empty.
//! * A group number of 20 or more digits wraps around in the DLL and saturates here.

use std::fmt;
use std::ops::Range;

use crate::names::GroupNames;
use crate::prepare::ends_with_escape;

/// Why a replacement text cannot be used at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplacementError {
    /// The text ends with a single backslash, which escapes the delimiter bregonig expects after
    /// the replacement.
    UnmatchedSeparator,
}

impl fmt::Display for ReplacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedSeparator => {
                f.write_str("the replacement ends with a backslash that escapes its delimiter")
            }
        }
    }
}

impl std::error::Error for ReplacementError {}

/// A reference or escape that gives nothing or is not what it looks like. The replacement still
/// works; these are for showing a warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplacementProblem {
    /// A group number the pattern does not have.
    NoSuchGroup(usize),
    /// A group name the pattern does not have (or whose number could not be worked out).
    UnknownName(String),
    /// `$-{name}[n]` with an `n` past the groups of that name.
    NameIndexOutOfRange { name: String, index: i64 },
    /// Text that looks like an escape or reference but is malformed; the string says which.
    Malformed(String),
}

/// How a letter is converted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Lower,
    Upper,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    Text(String),
    /// Group by number; 0 is the whole match.
    Group(usize),
    /// `$+`
    LastGroup,
    /// `\l` or `\u`: the next written character.
    Next(Case),
    /// `\L` or `\U`.
    Mode(Case),
    /// `\E`
    EndMode,
}

/// A parsed replacement, ready to be expanded for any number of matches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplacementTemplate {
    parts: Vec<Part>,
    problems: Vec<ReplacementProblem>,
}

impl ReplacementTemplate {
    /// Parses `source` for a pattern with `capture_count` groups (not counting the whole match)
    /// whose named groups are `names`.
    pub fn parse(
        source: &str,
        names: &GroupNames,
        capture_count: usize,
    ) -> Result<Self, ReplacementError> {
        if ends_with_escape(source) {
            return Err(ReplacementError::UnmatchedSeparator);
        }
        let mut parser = Parser {
            chars: source.chars().collect(),
            pos: 0,
            names,
            capture_count,
            text: String::new(),
            parts: Vec::new(),
            problems: Vec::new(),
            seek: [None, None, None],
            index_failed_at: None,
        };
        parser.run();
        Ok(Self {
            parts: parser.parts,
            problems: parser.problems,
        })
    }

    /// References that give nothing and malformed escapes, in order of appearance.
    pub fn problems(&self) -> &[ReplacementProblem] {
        &self.problems
    }

    /// Whether every match gets the same text (no reference and no case conversion).
    pub fn is_constant(&self) -> bool {
        self.parts.iter().all(|p| matches!(p, Part::Text(_)))
    }

    /// The text for one match. `text` is the string the match was found in and `groups` the byte
    /// ranges in it: element 0 is the whole match, then the groups (`None` for a group that did
    /// not take part).
    pub fn expand(&self, text: &str, groups: &[Option<Range<usize>>]) -> String {
        let group_text = |n: usize| -> &str {
            groups
                .get(n)
                .and_then(|g| g.as_ref())
                .and_then(|r| text.get(r.clone()))
                .unwrap_or("")
        };
        let mut out = Emitter::default();
        for part in &self.parts {
            match part {
                Part::Text(s) => out.push_str(s),
                Part::Group(n) => out.push_str(group_text(*n)),
                Part::LastGroup => {
                    let last = (1..groups.len()).rev().find(|&n| groups[n].is_some());
                    if let Some(n) = last {
                        out.push_str(group_text(n));
                    }
                }
                Part::Next(case) => out.next = Some(*case),
                Part::Mode(case) => out.mode = Some(*case),
                Part::EndMode => out.mode = None,
            }
        }
        out.text
    }
}

/// Builds the result and applies the pending case conversions.
#[derive(Default)]
struct Emitter {
    text: String,
    /// `\L` / `\U` in effect.
    mode: Option<Case>,
    /// `\l` / `\u` waiting for a character.
    next: Option<Case>,
}

impl Emitter {
    fn push_str(&mut self, s: &str) {
        if self.mode.is_none() && self.next.is_none() {
            self.text.push_str(s);
            return;
        }
        for c in s.chars() {
            let case = self.next.take().or(self.mode);
            self.text.push(match case {
                Some(Case::Lower) => c.to_ascii_lowercase(),
                Some(Case::Upper) => c.to_ascii_uppercase(),
                None => c,
            });
        }
    }
}

/// The closing delimiter of a form that reads up to it.
#[derive(Clone, Copy)]
enum Close {
    Brace,
    Angle,
    Quote,
}

impl Close {
    fn index(self) -> usize {
        self as usize
    }

    fn char(self) -> char {
        match self {
            Self::Brace => '}',
            Self::Angle => '>',
            Self::Quote => '\'',
        }
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    names: &'a GroupNames,
    capture_count: usize,
    /// Literal text not yet pushed to `parts`.
    text: String,
    parts: Vec<Part>,
    problems: Vec<ReplacementProblem>,
    /// For each [`Close`], the index of the next such character at or after every position
    /// (`usize::MAX`: none). Built on first use, so that an unclosed form costs no scan.
    seek: [Option<Vec<usize>>; 3],
    /// The `}` after which an index failed to parse. Every `$-{` before one `}` sees the same
    /// text after it, so it is not read again.
    index_failed_at: Option<usize>,
}

impl Parser<'_> {
    fn run(&mut self) {
        while let Some(c) = self.bump() {
            match c {
                '\\' => self.escape(),
                '$' => self.dollar(),
                _ => self.text.push(c),
            }
        }
        self.flush();
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn flush(&mut self) {
        if !self.text.is_empty() {
            self.parts.push(Part::Text(std::mem::take(&mut self.text)));
        }
    }

    fn push_part(&mut self, part: Part) {
        self.flush();
        self.parts.push(part);
    }

    fn malformed(&mut self, what: &str) {
        self.problems
            .push(ReplacementProblem::Malformed(what.to_owned()));
    }

    /// Consumes a run of ASCII decimal digits and returns its value (saturating).
    fn decimal_run(&mut self) -> (usize, usize) {
        let mut value = 0usize;
        let mut len = 0;
        while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
            value = value.saturating_mul(10).saturating_add(d as usize);
            len += 1;
            self.pos += 1;
        }
        (value, len)
    }

    fn group(&mut self, n: usize) {
        if n > self.capture_count {
            self.problems.push(ReplacementProblem::NoSuchGroup(n));
        }
        self.push_part(Part::Group(n));
    }

    /// The group called `name`. With several groups of that name it is the one with the lowest
    /// number, whether or not it took part in the match (bregonig.dll 4.20).
    fn named(&mut self, name: &str) {
        match self.names.numbers(name).first().copied() {
            Some(n) => self.push_part(Part::Group(n)),
            None => self
                .problems
                .push(ReplacementProblem::UnknownName(name.to_owned())),
        }
    }

    /// The index of the next `close` at or after the current position.
    fn next_close(&mut self, close: Close) -> Option<usize> {
        let chars = &self.chars;
        let table = self.seek[close.index()].get_or_insert_with(|| {
            let mut next = vec![usize::MAX; chars.len() + 1];
            for i in (0..chars.len()).rev() {
                next[i] = if chars[i] == close.char() {
                    i
                } else {
                    next[i + 1]
                };
            }
            next
        });
        table.get(self.pos).copied().filter(|&i| i != usize::MAX)
    }

    /// Consumes the text up to the next `close` and the `close` itself, and returns the text,
    /// whatever it holds. `None` (and nothing consumed) when there is no `close` later on.
    fn up_to(&mut self, close: Close) -> Option<String> {
        let at = self.next_close(close)?;
        let text = self.chars[self.pos..at].iter().collect();
        self.pos = at + 1;
        Some(text)
    }

    /// Reads a run of characters that satisfy `accept` (possibly none, if `allow_empty`), then
    /// `close`, and consumes both. `None` (and nothing consumed) otherwise. The scan stops at the
    /// first character that does not fit, so a failed attempt costs only its valid prefix and the
    /// whole parse stays linear.
    fn until(
        &mut self,
        close: char,
        accept: impl Fn(char) -> bool,
        allow_empty: bool,
    ) -> Option<String> {
        let rest = &self.chars[self.pos..];
        let len = rest.iter().take_while(|&&c| accept(c)).count();
        if (len == 0 && !allow_empty) || rest.get(len) != Some(&close) {
            return None;
        }
        let label: String = rest[..len].iter().collect();
        self.pos += len + 1;
        Some(label)
    }

    /// After a `$`. A `$` that does not start a reference is text together with the character
    /// after it, which is never read as the start of anything: `$$1` is the four characters
    /// `$$1` and `$\n` is `$\n` (bregonig.dll 4.20).
    fn dollar(&mut self) {
        let done = match self.peek() {
            Some('&') => {
                self.pos += 1;
                self.push_part(Part::Group(0));
                true
            }
            Some('1'..='9') => {
                let (n, _) = self.decimal_run();
                self.group(n);
                true
            }
            Some('{') => self.braced(),
            Some('+') => self.plus(),
            Some('-') => self.indexed_name(),
            _ => false,
        };
        if !done {
            self.text.push('$');
            if let Some(c) = self.bump() {
                self.text.push(c);
            }
        }
    }

    /// `${n}` or `${name}`, at the `{`. A form that starts with a digit has to be digits and `}`
    /// (`${1a}` is text); otherwise everything up to the first `}` is a name.
    fn braced(&mut self) -> bool {
        let start = self.pos;
        self.pos += 1;
        if self.peek().is_some_and(|c| c.is_ascii_digit()) {
            let (n, _) = self.decimal_run();
            if self.eat('}') {
                self.group(n);
                return true;
            }
        } else if let Some(name) = self.up_to(Close::Brace) {
            self.named(&name);
            return true;
        }
        self.pos = start;
        self.malformed("${...} is not complete");
        false
    }

    /// `$+` or `$+{name}`, at the `+`.
    fn plus(&mut self) -> bool {
        if self.peek_at(1) != Some('{') {
            self.pos += 1;
            self.push_part(Part::LastGroup);
            return true;
        }
        let start = self.pos;
        self.pos += 2;
        if let Some(name) = self.up_to(Close::Brace) {
            self.named(&name);
            return true;
        }
        self.pos = start;
        self.malformed("$+{name} is not complete");
        false
    }

    /// `$-{name}[n]`, at the `-`. The name is everything up to the first `}`.
    fn indexed_name(&mut self) -> bool {
        if self.peek_at(1) != Some('{') {
            return false;
        }
        let start = self.pos;
        self.pos += 2;
        let name_start = self.pos;
        let parsed = self.next_close(Close::Brace).and_then(|at| {
            if self.index_failed_at == Some(at) {
                return None;
            }
            self.pos = at + 1;
            let index = self.index();
            if index.is_none() {
                self.index_failed_at = Some(at);
            }
            index.map(|index| (at, index))
        });
        let Some((at, index)) = parsed else {
            self.pos = start;
            self.malformed("$-{name}[n] is not complete");
            return false;
        };
        let name: String = self.chars[name_start..at].iter().collect();
        let numbers = self.names.numbers(&name);
        if numbers.is_empty() {
            self.problems.push(ReplacementProblem::UnknownName(name));
            return true;
        }
        let from_start = if index < 0 {
            usize::try_from(index.unsigned_abs())
                .ok()
                .and_then(|back| numbers.len().checked_sub(back))
        } else {
            usize::try_from(index).ok()
        };
        match from_start.and_then(|i| numbers.get(i).copied()) {
            Some(n) => self.push_part(Part::Group(n)),
            None => self
                .problems
                .push(ReplacementProblem::NameIndexOutOfRange { name, index }),
        }
        true
    }

    /// `[n]` after `$-{name}`: `[]` (0), or white space, an optional sign and at least one digit,
    /// then `]` (bregonig.dll 4.20). Anything else makes the whole form text.
    fn index(&mut self) -> Option<i64> {
        if !self.eat('[') {
            return None;
        }
        if self.eat(']') {
            return Some(0);
        }
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
        let negative = self.eat('-');
        if !negative {
            self.eat('+');
        }
        let (n, len) = self.decimal_run();
        if len == 0 || !self.eat(']') {
            return None;
        }
        let n = i64::try_from(n).unwrap_or(i64::MAX);
        Some(if negative { -n } else { n })
    }

    /// After a `\`. (A trailing lone backslash was rejected before parsing.)
    fn escape(&mut self) {
        let Some(c) = self.bump() else {
            return;
        };
        match c {
            't' => self.text.push('\t'),
            'n' => self.text.push('\n'),
            'r' => self.text.push('\r'),
            'b' => self.text.push('\u{8}'),
            'f' => self.text.push('\u{c}'),
            'a' => self.text.push('\u{7}'),
            'e' => self.text.push('\u{1b}'),
            'l' => self.push_part(Part::Next(Case::Lower)),
            'u' => self.push_part(Part::Next(Case::Upper)),
            'L' => self.push_part(Part::Mode(Case::Lower)),
            'U' => self.push_part(Part::Mode(Case::Upper)),
            'E' => self.push_part(Part::EndMode),
            // bregonig.dll 4.20 skips `\Q` (it does not quote anything).
            'Q' => {}
            '0' => self.octal_after_zero(),
            '1'..='9' => {
                self.pos -= 1;
                self.number_escape();
            }
            'x' => self.hex_escape(),
            'o' => self.brace_octal_escape(),
            'c' => self.control_escape(),
            'k' => self.named_escape(),
            other => self.text.push(other),
        }
    }

    fn push_code(&mut self, code: u32, what: &str) {
        match char::from_u32(code) {
            Some(c) => self.text.push(c),
            None => self.malformed(&format!("{what} is not a character")),
        }
    }

    /// `\0nn`: up to 3 octal digits counting the leading 0.
    fn octal_after_zero(&mut self) {
        let mut value = 0;
        for _ in 0..2 {
            match self.peek().and_then(|c| c.to_digit(8)) {
                Some(d) => {
                    value = value * 8 + d;
                    self.pos += 1;
                }
                None => break,
            }
        }
        self.push_code(value, "octal escape");
    }

    /// `\1`..`\9...`: the digits are read as one decimal number. It is a group if the pattern has
    /// that many groups. Otherwise, if it is two or three octal digits with a value up to `\377`,
    /// it is that character; and in every other case it is still a group, which gives nothing
    /// (`\7`, `\18`, `\400`, `\1111`) (bregonig.dll 4.20).
    fn number_escape(&mut self) {
        let start = self.pos;
        let (n, len) = self.decimal_run();
        if n > self.capture_count && (2..=3).contains(&len) {
            let digits = &self.chars[start..start + len];
            if digits.iter().all(|c| ('0'..='7').contains(c)) {
                let value = digits
                    .iter()
                    .fold(0, |v, c| v * 8 + c.to_digit(8).unwrap_or(0));
                if value <= 0o377 {
                    self.push_code(value, "octal escape");
                    return;
                }
            }
        }
        self.group(n);
    }

    /// `\xHH` or `\x{H...}`.
    ///
    /// bregonig.html 5.3 calls `\xHH` and the unbraced octal forms "encoded byte values". Sakura
    /// calls the wide API of bregonig (`BMatchW`, `BSubstW`, `BSubstExW` in
    /// `sakura_core/extmodule/CBregexpDll2.cpp`), where the text is UTF-16, so a "byte value" is
    /// one UTF-16 code unit: `\xC3` is U+00C3, and `\xC3\xA9` is two characters, not the UTF-8
    /// bytes of `é`. bregonig.dll 4.20 does so.
    ///
    /// `\x{}` is U+0000, and the braces take at most 8 digits (more is text, `x{...`). The DLL
    /// also turns a value of 0x10000 or more into a surrogate pair by a formula that is wrong
    /// above U+10FFFF, and passes a value in the surrogate range through as a lone surrogate; a
    /// Rust string holds neither, so those give nothing here.
    fn hex_escape(&mut self) {
        if self.eat('{') {
            let start = self.pos;
            if let Some(digits) = self.until('}', |c| c.is_ascii_hexdigit(), true)
                && digits.len() <= 8
            {
                let value = u32::from_str_radix(&digits, 16).unwrap_or(0);
                self.push_code(value, "hexadecimal escape");
                return;
            }
            self.pos = start;
            self.malformed(r"\x{...} is not complete");
            self.text.push_str("x{");
            return;
        }
        let mut value = 0;
        let mut len = 0;
        while len < 2 {
            let Some(d) = self.peek().and_then(|c| c.to_digit(16)) else {
                break;
            };
            value = value * 16 + d;
            len += 1;
            self.pos += 1;
        }
        if len == 0 {
            self.malformed(r"\x without hexadecimal digits");
            self.text.push('x');
        } else {
            self.push_code(value, "hexadecimal escape");
        }
    }

    /// `\o{n...}`. `\o{}` is U+0000 and there is no limit on the digits; the value wraps at 32
    /// bits like the DLL's.
    fn brace_octal_escape(&mut self) {
        if self.peek() == Some('{') {
            let start = self.pos;
            self.pos += 1;
            if let Some(digits) = self.until('}', |c| c.is_digit(8), true) {
                let value = digits.chars().fold(0u32, |v, c| {
                    v.wrapping_mul(8).wrapping_add(c.to_digit(8).unwrap_or(0))
                });
                self.push_code(value, "octal escape");
                return;
            }
            self.pos = start;
        }
        self.malformed(r"\o{...} is not complete");
        self.text.push('o');
    }

    /// `\cX`. A backslash after the `c` is skipped when a character follows (`\c\[` is `\c[`).
    ///
    /// An ASCII letter is upper-cased first; any other character just has bit 6 flipped
    /// (`\cé` is U+00A9, `\c1` is `q`). At the end of the text the DLL reads the delimiter that
    /// Sakura puts after the replacement (U+FFFF) and gives U+FFBF; that is repeated here.
    fn control_escape(&mut self) {
        if self.peek() == Some('\\') && self.peek_at(1).is_some() {
            self.pos += 1;
        }
        match self.peek() {
            Some(x) if x.is_ascii() => {
                self.pos += 1;
                self.text
                    .push(char::from(x.to_ascii_uppercase() as u8 ^ 0x40));
            }
            Some(x) if u32::from(x) <= 0xFFFF => {
                self.pos += 1;
                self.push_code(u32::from(x) ^ 0x40, "control escape");
            }
            Some(_) => {
                self.malformed(r"\c with a character outside the BMP");
                self.text.push('c');
            }
            None => self.push_code(0xFFFF ^ 0x40, "control escape"),
        }
    }

    /// `\k<name>` or `\k'name'`, after the `\k`. The name is everything up to the closing
    /// delimiter, and a number is a name that no group has (`\k<1>` gives nothing).
    fn named_escape(&mut self) {
        let close = match self.peek() {
            Some('<') => Close::Angle,
            Some('\'') => Close::Quote,
            _ => {
                self.malformed(r"\k without <name> or 'name'");
                self.text.push('k');
                return;
            }
        };
        let start = self.pos;
        self.pos += 1;
        match self.up_to(close) {
            Some(name) => self.named(&name),
            None => {
                self.pos = start;
                self.malformed(r"\k<name> is not complete");
                self.text.push('k');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expands `replacement` for the first match; the groups are given explicitly so that these
    /// tests do not need the engine. `names` is `(name, group numbers)`.
    fn run(
        replacement: &str,
        names: &[(&str, &[usize])],
        text: &str,
        groups: &[Option<Range<usize>>],
    ) -> (String, Vec<ReplacementProblem>) {
        let mut pattern = String::new();
        let mut order: Vec<(usize, &str)> = names
            .iter()
            .flat_map(|(n, v)| v.iter().map(move |&i| (i, *n)))
            .collect();
        order.sort_unstable();
        let count = groups.len().saturating_sub(1);
        for i in 1..=count {
            match order.iter().find(|(n, _)| *n == i) {
                Some((_, name)) => pattern.push_str(&format!("(?<{name}>.)")),
                None => pattern.push_str("(.)"),
            }
        }
        let names = GroupNames::scan(&pattern, false, count);
        let t = ReplacementTemplate::parse(replacement, &names, count).unwrap();
        (t.expand(text, groups), t.problems().to_vec())
    }

    fn plain(replacement: &str, text: &str, groups: &[Option<Range<usize>>]) -> String {
        run(replacement, &[], text, groups).0
    }

    #[test]
    fn numbers_and_whole_match() {
        // bregonig.html 5.3: \n, $n, ${n}, $&
        let g = [Some(0..7), Some(0..3), Some(4..7)];
        assert_eq!(plain("$2-$1", "foo bar", &g), "bar-foo");
        assert_eq!(plain(r"\2-\1", "foo bar", &g), "bar-foo");
        assert_eq!(plain("[$&]", "foo bar", &g), "[foo bar]");
        assert_eq!(plain("${1}2", "foo bar", &g), "foo2");
        assert_eq!(plain("<${0}>", "foo bar", &g), "<foo bar>");
    }

    #[test]
    fn dollar_forms_that_are_text() {
        // bregonig.html 5.3: $0 and $var are plain strings.
        let g = [Some(0..1)];
        assert_eq!(plain("$0 $var a$ $", "x", &g), "$0 $var a$ $");
        // bregonig.dll 4.20: the character after a `$` that is not a reference is text too.
        let one = [Some(0..1), Some(0..1)];
        assert_eq!(plain("$$1", "x", &one), "$$1");
        assert_eq!(plain("$$$1", "x", &one), "$$x");
        assert_eq!(plain(r"$\n", "x", &one), r"$\n");
        assert_eq!(plain("$ $1", "x", &one), "$ x");
        assert_eq!(plain("$", "x", &g), "$");
    }

    #[test]
    fn groups_that_did_not_take_part_or_do_not_exist_are_empty() {
        let g = [Some(1..2), None];
        let (out, problems) = run("[$1][$2]", &[], "abc", &g);
        assert_eq!(out, "[][]");
        assert_eq!(problems, vec![ReplacementProblem::NoSuchGroup(2)]);
    }

    #[test]
    fn last_matched_group() {
        // bregonig.html 5.3: with (a)|([^a]), $+ is whichever of $1 and $2 matched.
        let a = [Some(0..1), Some(0..1), None];
        let b = [Some(0..1), None, Some(0..1)];
        assert_eq!(plain("<$+>", "x", &a), "<x>");
        assert_eq!(plain("<$+>", "x", &b), "<x>");
        assert_eq!(plain("<$+>", "x", &[Some(0..1), None, None]), "<>");
        assert_eq!(plain("<$+>", "x", &[Some(0..1)]), "<>");
    }

    #[test]
    fn named_references() {
        // bregonig.html 5.3: \k<name>, \k'name', ${name}, $+{name}
        let g = [Some(0..7), Some(0..4), Some(5..7)];
        let names: &[(&str, &[usize])] = &[("y", &[1]), ("m", &[2])];
        for r in ["$+{m}/$+{y}", "${m}/${y}", r"\k<m>/\k'y'"] {
            assert_eq!(run(r, names, "2026-10", &g).0, "10/2026", "{r}");
        }
        assert_eq!(run("$-{y}[0]", names, "2026-10", &g).0, "2026");
        // A number in ${} is a group; in \k<> it is a name, which no group has.
        assert_eq!(run(r"\k<2>${2}", names, "2026-10", &g).0, "10");
    }

    #[test]
    fn a_name_used_twice() {
        // bregonig.html 5.3: $+{name} is the leftmost; $-{name}[n]: 0 leftmost, -1 rightmost.
        let g = [Some(0..2), Some(0..1), Some(1..2)];
        let names: &[(&str, &[usize])] = &[("n", &[1, 2])];
        assert_eq!(run("$+{n}", names, "ab", &g).0, "a");
        assert_eq!(run("$-{n}[0]", names, "ab", &g).0, "a");
        assert_eq!(run("$-{n}[1]", names, "ab", &g).0, "b");
        assert_eq!(run("$-{n}[-1]", names, "ab", &g).0, "b");
        assert_eq!(run("$-{n}[-2]", names, "ab", &g).0, "a");
        let (out, problems) = run("$-{n}[2]$-{n}[-3]", names, "ab", &g);
        assert_eq!(out, "");
        assert_eq!(problems.len(), 2);
        assert!(matches!(
            problems[0],
            ReplacementProblem::NameIndexOutOfRange { index: 2, .. }
        ));
        // The lowest number, even if it did not take part (bregonig.dll 4.20).
        let g = [Some(0..1), None, Some(0..1)];
        assert_eq!(run("$+{n}", names, "b", &g).0, "");
    }

    #[test]
    fn unknown_names_are_listed() {
        let (out, problems) = run("[$+{x}${y}\\k<z>$-{w}[0]]", &[], "a", &[Some(0..1)]);
        assert_eq!(out, "[]");
        assert_eq!(
            problems,
            ["x", "y", "z", "w"]
                .map(|n| ReplacementProblem::UnknownName(n.to_owned()))
                .to_vec()
        );
    }

    #[test]
    fn control_character_escapes() {
        // bregonig.html 5.3: \t \n \r \b \f \a \e
        assert_eq!(
            plain(r"\t\n\r\b\f\a\e", "x", &[Some(0..1)]),
            "\t\n\r\u{8}\u{c}\u{7}\u{1b}"
        );
    }

    #[test]
    fn numeric_escapes() {
        let g = [Some(0..1)];
        // \xHH, \x{HHHH} (a code point), \o{nnn}, \0nn
        assert_eq!(
            plain(r"\x41\x{3042}\x{1F600}\o{101}\o{30102}", "x", &g),
            "Aあ\u{1F600}Aあ"
        );
        assert_eq!(plain(r"\x4a\x4A\x4", "x", &g), "JJ\u{4}");
        assert_eq!(plain(r"\012\0\00\0101", "x", &g), "\n\0\0\u{8}1");
        assert_eq!(plain(r"\xe9", "x", &g), "\u{e9}");
    }

    #[test]
    fn control_letters() {
        // bregonig.html 5.3: \cx; 5.4: \c\x is \cx (so \c\[ is ESC).
        let g = [Some(0..1)];
        assert_eq!(
            plain(r"\c[\ca\cA\c@\c?", "x", &g),
            "\u{1b}\u{1}\u{1}\0\u{7f}"
        );
        assert_eq!(plain(r"\c\[", "x", &g), "\u{1b}");
    }

    #[test]
    fn numbers_past_the_groups_may_be_octal() {
        // bregonig.html 5.3 (\nnn): like Perl; \18 is always ${18}.
        let g = [Some(0..1)];
        assert_eq!(plain(r"\12", "x", &g), "\n");
        assert_eq!(plain(r"\101", "x", &g), "A");
        // Four digits are never octal.
        assert_eq!(plain(r"\1014", "x", &g), "");
        assert_eq!(plain(r"\400", "x", &g), "");
        assert_eq!(plain(r"\377", "x", &g), "\u{ff}");
        assert_eq!(plain(r"\18", "x", &g), "");
        // A digit 8 or 9 anywhere in the run makes it a group.
        assert_eq!(
            run(r"\1238", &[], "x", &g).1,
            vec![ReplacementProblem::NoSuchGroup(1238)]
        );
        assert_eq!(plain(r"\1", "x", &g), "");
        // `$12` is never octal.
        assert_eq!(plain("$12", "x", &g), "");
        // With the groups there, a group wins.
        let mut many = vec![Some(0..1)];
        many.extend((1..=12).map(|_| Some(0..1)));
        assert_eq!(plain(r"\12|\101", "x", &many), "x|A");
    }

    #[test]
    fn problems_for_octal_and_groups() {
        let (_, p) = run(r"\12 \3 \18", &[], "x", &[Some(0..1), Some(0..1)]);
        assert_eq!(
            p,
            vec![
                ReplacementProblem::NoSuchGroup(3),
                ReplacementProblem::NoSuchGroup(18),
            ]
        );
    }

    #[test]
    fn other_escapes_are_the_character() {
        let g = [Some(0..1)];
        assert_eq!(plain(r"\\\$\/\.\v\&", "x", &g), "\\$/.v&");
        // `\Q` is skipped.
        assert_eq!(plain(r"[\Qa$&\E]", "x", &g), "[ax]");
    }

    #[test]
    fn malformed_forms_keep_their_text() {
        let g = [Some(0..1)];
        for (src, out) in [
            ("${1", "${1"),
            ("$+{x", "$+{x"),
            ("${1a}", "${1a}"),
            ("${a", "${a"),
            (r"\x{4 1}", "x{4 1}"),
            ("$-{x}", "$-{x}"),
            ("$-{x}[a]", "$-{x}[a]"),
            (r"\k<x", "k<x"),
            (r"\k", "k"),
            (r"\x{41", "x{41"),
            (r"\x{000000041}", "x{000000041}"),
            (r"\xg", "xg"),
            (r"\x{110000}", ""),
            (r"\o", "o"),
            (r"\o{9}", "o{9}"),
        ] {
            let (got, problems) = run(src, &[], "x", &g);
            assert_eq!(got, out, "{src}");
            assert!(
                problems
                    .iter()
                    .any(|p| matches!(p, ReplacementProblem::Malformed(_))),
                "{src}: {problems:?}"
            );
        }
    }

    #[test]
    fn braces_and_names_read_up_to_the_first_close() {
        // bregonig.dll 4.20: a form that does not start with a digit is a name, whatever it holds.
        let g = [Some(0..1), Some(0..1)];
        for src in [
            "${}",
            "${a b}",
            "${ 1}",
            "${-1}",
            "${+1}",
            "$+{ }",
            "$-{n x}[0]",
            r"\k<a b>",
        ] {
            let (out, problems) = run(&format!("[{src}]"), &[], "x", &g);
            assert_eq!(out, "[]", "{src}");
            assert!(
                matches!(problems[..], [ReplacementProblem::UnknownName(_)]),
                "{src}: {problems:?}"
            );
        }
        // Up to the first `}`.
        assert_eq!(plain("${1}2}", "x", &g), "x2}");
        // Leading zeros are decimal.
        assert_eq!(plain("${01}", "x", &g), "x");
    }

    #[test]
    fn the_index_of_an_indexed_name() {
        // bregonig.dll 4.20: `[]` is 0; else white space, an optional sign and digits, then `]`.
        let g = [Some(0..2), Some(0..1), Some(1..2)];
        let names: &[(&str, &[usize])] = &[("n", &[1, 2])];
        for (src, out) in [
            ("$-{n}[ 1]", "b"),
            ("$-{n}[+1]", "b"),
            ("$-{n}[ -1]", "b"),
            ("$-{n}[]", "a"),
            ("$-{n}[01]", "b"),
            ("$-{n}[1 ]", "$-{n}[1 ]"),
            ("$-{n}[--1]", "$-{n}[--1]"),
            ("$-{n}[x]", "$-{n}[x]"),
            ("$-{n}[ ]", "$-{n}[ ]"),
            ("$-{n}[+]", "$-{n}[+]"),
            ("$-{n}[\t1]", "b"),
        ] {
            assert_eq!(run(src, names, "ab", &g).0, out, "{src}");
        }
    }

    #[test]
    fn hexadecimal_and_octal_in_braces() {
        let g = [Some(0..1)];
        // Up to 8 digits in `\x{}`; empty is U+0000; `\o{}` has no limit on the digits.
        assert_eq!(
            plain(r"\x{00000041}\x{}\o{0000000101}\o{}", "x", &g),
            "A\0A\0"
        );
        assert_eq!(plain(r"\x{000000041}", "x", &g), "x{000000041}");
        assert_eq!(
            plain(r"\x{10000}\x{10FFFF}", "x", &g),
            "\u{10000}\u{10FFFF}"
        );
        // A surrogate, or a value past U+10FFFF, is not a character here (the DLL gives lone surrogates).
        assert_eq!(
            plain(r"[\x{D800}\x{FFFFFFFF}\o{37777777777}]", "x", &g),
            "[]"
        );
    }

    #[test]
    fn control_escape_beyond_ascii_and_at_the_end() {
        let g = [Some(0..1)];
        // Bit 6 is flipped; only ASCII letters are upper-cased first.
        assert_eq!(plain("\\c\u{e9}\\cあ\\c1", "x", &g), "\u{a9}\u{3002}q");
        // The DLL reads the delimiter U+FFFF that Sakura puts after the replacement.
        assert_eq!(plain(r"a\c", "x", &g), "a\u{ffbf}");
    }

    #[test]
    fn many_unterminated_forms_parse_quickly() {
        let n = GroupNames::default();
        // Each kind repeated, with one closing delimiter at the very end.
        for unit in [
            "${", "$+{", "$-{", "\\k<", "\\k'", "\\x{", "\\o{", "${a", "\\x{4",
        ] {
            let spaces = format!("}}[{}x", " ".repeat(50_000));
            for close in ["}", ">", "'", spaces.as_str()] {
                let src = format!("{}{close}", unit.repeat(50_000));
                let start = std::time::Instant::now();
                assert!(ReplacementTemplate::parse(&src, &n, 0).is_ok());
                assert!(start.elapsed().as_secs() < 5, "{unit:?} is too slow");
            }
        }
    }

    #[test]
    fn a_form_after_a_malformed_one_still_works() {
        assert_eq!(plain("${1 $1", "ab", &[Some(0..2), Some(0..1)]), "${1 a");
    }

    #[test]
    fn case_conversion() {
        // bregonig.html 5.3: \l \u \L \U \E, ASCII only.
        let g = [Some(0..11), Some(0..5), Some(6..11)];
        let t = "hello WORLD";
        assert_eq!(plain(r"\u$1 \L$2\E!", t, &g), "Hello world!");
        assert_eq!(plain(r"\U$1\E $2", t, &g), "HELLO WORLD");
        assert_eq!(plain(r"\l$2", t, &g), "wORLD");
        assert_eq!(plain(r"\Uabc\Edef", t, &g), "ABCdef");
        assert_eq!(plain(r"\L\u$2", t, &g), "World");
        assert_eq!(plain(r"\U\x61\n", t, &g), "A\n");
        // The last one wins; \E ends only the mode.
        assert_eq!(plain(r"\Ua\Lb\Ec", t, &g), "Abc");
        // Not ASCII: left as it is.
        assert_eq!(plain(r"\Ustraße é あ", t, &g), "STRAßE é あ");
        assert_eq!(plain(r"\L$&", "ÀB", &[Some(0..3)]), "Àb");
    }

    #[test]
    fn a_pending_one_char_conversion_waits_for_a_character() {
        let g = [Some(0..1), Some(0..0)];
        assert_eq!(plain(r"\u$1z", "y", &g), "Z");
        assert_eq!(plain(r"\ux", "y", &g), "X");
        assert_eq!(plain(r"a\u", "y", &g), "a");
    }

    #[test]
    fn japanese_text_and_line_breaks() {
        // Group ranges are byte ranges in the line.
        let text = "東京都 港区";
        let g = [Some(0..text.len()), Some(0..9), Some(10..text.len())];
        assert_eq!(plain("$2\\n$1", text, &g), "港区\n東京都");
        assert_eq!(plain("[$&]\r\n", text, &g), "[東京都 港区]\r\n");
        // A replacement with an actual line break works as text.
        assert_eq!(plain("$1\n$2", text, &g), "東京都\n港区");
    }

    #[test]
    fn empty_groups_and_line_edges() {
        let text = "abc";
        // ^ matched at the start: an empty whole match and an empty group.
        let g = [Some(0..0), Some(0..0)];
        assert_eq!(plain(">$1<$&|", text, &g), "><|");
        // $ at the end.
        let g = [Some(3..3)];
        assert_eq!(plain("[$&]x", text, &g), "[]x");
        // A range that is not valid in the text gives nothing instead of panicking.
        assert_eq!(plain("$1", "あ", &[Some(0..3), Some(1..2)]), "");
        assert_eq!(plain("$1", "a", &[Some(0..1), Some(5..9)]), "");
    }

    #[test]
    fn trailing_backslash_is_an_error() {
        let n = GroupNames::default();
        for src in ["abc\\", "\\", "a\\\\\\"] {
            assert_eq!(
                ReplacementTemplate::parse(src, &n, 0).unwrap_err(),
                ReplacementError::UnmatchedSeparator
            );
        }
        assert!(ReplacementTemplate::parse("abc\\\\", &n, 0).is_ok());
        assert!(
            ReplacementError::UnmatchedSeparator
                .to_string()
                .contains("backslash")
        );
    }

    #[test]
    fn constant_templates() {
        let n = GroupNames::default();
        let c = |s: &str| ReplacementTemplate::parse(s, &n, 0).unwrap().is_constant();
        assert!(c(""));
        assert!(c("plain\\n$"));
        assert!(!c("$&"));
        assert!(!c(r"\U"));
        assert!(!c(r"\E"));
    }
}
