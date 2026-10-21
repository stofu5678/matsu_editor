//! Searching and replacing text, with Sakura's rules.
//!
//! # Design
//!
//! * **One line at a time.** Sakura matches line by line and never across lines, and the regex
//!   engine stays behind a trait (ADR-0006). A [`Matcher`] finds matches in one line of decoded
//!   text; [`PlainMatcher`] is the plain-text one, and the regex crate implements the trait too
//!   (nothing here depends on it). A regular expression gets each line *with its own line
//!   ending*, as in Sakura (`GetDocLineStrWithEOL`), so it can match the line break
//!   (`\r\n`, `\s`, a negated class) but never the next line ([`Matcher::sees_line_ending`],
//!   [`Found`]).
//! * **The document driver.** [`find`], [`find_all`] and the replace functions read the
//!   [`Document`](crate::buffer::Document) by line, each line once, and call the matcher. They
//!   take a `&Document`; a clone (cheap) can be searched on another thread while the UI keeps
//!   editing ([`SearchJob`]). A [`CancelToken`] is checked between lines and progress is
//!   reported in lines ([`Control`]).
//! * **Edits come back, not undo.** Replacing returns every edit with the old and new text, so
//!   that the command that calls it can record one undo step.
//!
//! Sakura's sources for each rule are cited where the rule is implemented:
//! `sakura_core/agent/CSearchAgent.cpp` (matching), `sakura_core/parse/CWordParse.cpp` (word
//! boundaries), `sakura_core/cmd/CViewCommander_Search.cpp` (next, previous, wrap-around,
//! replace).

mod chars;
mod find;
mod matcher;
mod replace;

#[cfg(test)]
mod tests;

pub use chars::fold_char;
pub use find::{
    CancelToken, Control, Direction, FindAll, Found, Progress, SearchError, SearchJob, find,
    find_all, find_in_line, start_after,
};
pub use matcher::{Match, Matcher, MatcherError, PlainMatcher, PlainOptions};
pub use replace::{
    AppliedEdit, ReplaceAllResult, ReplaceStep, ReplaceTarget, Replacement, apply_replacements,
    replace_all, replace_and_find_next, replace_one, replacement_for,
};
