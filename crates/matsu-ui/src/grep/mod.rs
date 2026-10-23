//! Sakura's Grep (検索 menu, `F_GREP_DIALOG` and `F_GREP`): searches the files of folders and
//! writes the hits to a window, in the form that the tag jump reads.
//!
//! * [`keys`], [`enumerate`], [`mod@format`] and [`engine`] are the search without any window. They
//!   follow `sakura_core/grep/` and `sakura_core/agent/CGrepAgent.cpp`, and are derived from
//!   them (the notices are at the top of the files). The engine runs on any thread; it writes to a
//!   [`Sink`](engine::Sink) and reads a flag to stop.
//! * [`settings`] holds what the dialog holds, with its defaults, its checks and the arguments of
//!   the macro `Grep`.
//! * Grep 置換 (`F_GREP_REPLACE_DLG` and `F_GREP_REPLACE`) is the same search with
//!   [`GrepInfo::replace`](engine::GrepInfo::replace) set: [`engine`] rewrites the files that have
//!   a hit, and the dialog is the panel with the boxes 置換後 and the like.
//! * [`panel`] is the dialog, a panel in the editor window. [`window`] runs a Grep in a window: it
//!   picks the window for the result, runs the engine on a background task, writes the pieces to
//!   the text, and keeps what makes the window a Grep window (its title, no question about saving,
//!   no closing while it searches).
//!
//! The matching is the one of the search panel ([`crate::search::state::build_matcher`]).

pub mod engine;
pub mod enumerate;
pub mod format;
pub mod keys;
pub mod panel;
pub mod settings;
pub mod window;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod replace_tests;
#[cfg(test)]
mod window_tests;
