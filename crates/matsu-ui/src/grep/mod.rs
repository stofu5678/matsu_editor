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
mod replace_window;
pub mod settings;
pub mod stdout;
pub mod window;

/// The folder of a test as the Grep writes it. The Grep gives the long form of its folders
/// (`GetLongFileName`, which only Windows has), and a temporary folder there may be in its short
/// form (`RUNNER~1`): the paths that a test expects are built from the long form too.
#[cfg(test)]
fn long_temp_path(path: &std::path::Path) -> String {
    let path = path.to_string_lossy().into_owned();
    matsu_platform::long_path::long_path(&path).unwrap_or(path)
}

#[cfg(test)]
mod access_key_tests;
#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod group_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod huge_tests;
#[cfg(test)]
mod replace_tests;
#[cfg(test)]
mod this_text_tests;
#[cfg(test)]
mod window_tests;
