//! Find and replace in the window (Sakura's 検索 menu): the state the dialogs remember
//! ([`state`]), the highlight of the matches ([`highlight`]), the operations on the view
//! ([`ops`]) and the panel that stands for the dialogs ([`panel`]). The functions the keys and
//! menus run are the UI commands in `ui_commands/search/`; they only call into here.
//!
//! The matching rules are `ume_core::search` (ADR-0006); the matcher is chosen in one place
//! ([`state::build_matcher`]) so that the regular expression option can come later without
//! touching the rest.

pub mod bookmarks;
pub mod highlight;
pub mod modified_lines;
pub mod ops;
pub mod panel;
pub mod state;

pub use ops::SearchOutcome;
pub use panel::{PanelMode, SearchPanel};
pub use state::{SearchOptions, SearchState};

#[cfg(test)]
mod bookmarks_tests;
#[cfg(test)]
mod modified_lines_tests;
#[cfg(test)]
mod tests;
