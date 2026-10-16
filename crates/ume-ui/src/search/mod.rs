//! Find and replace in the window (Sakura's 検索 menu): the state the dialogs remember
//! ([`state`]), the highlight of the matches ([`highlight`]), the operations on the view
//! ([`ops`]) and the panel that stands for the dialogs ([`panel`]). The functions the keys and
//! menus run are the UI commands in `ui_commands/search/`; they only call into here.
//!
//! The matching rules are `ume_core::search` (ADR-0006), and `ume_regex` for the regular
//! expression option; the matcher is chosen in one place ([`state::build_matcher`]).

pub mod bookmarks;
pub mod highlight;
pub mod isearch;
pub mod modified_lines;
pub mod ops;
pub mod panel;
pub mod state;

pub use isearch::{ISearch, ISearchMode};
pub use ops::SearchOutcome;
pub use panel::{PanelMode, SearchPanel};
pub use state::{SearchOptions, SearchState};

#[cfg(test)]
mod bookmarks_tests;
#[cfg(test)]
mod isearch_tests;
#[cfg(test)]
mod modified_lines_tests;
#[cfg(test)]
mod regex_tests;
#[cfg(test)]
mod tests;
