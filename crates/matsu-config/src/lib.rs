//! Reads and writes MATSU Editor's settings file `matsu.ini`, in the format of Sakura Editor's `sakura.ini`
//! (ADR-0010).
//!
//! - [`Profile`] is the file format (`sakura_core/env/CProfile.cpp`). It keeps every line, so reading a file and
//!   writing it back gives the same bytes, and entries MATSU Editor does not know are never lost.
//! - [`value`] is the typed access of `CDataProfile`.
//! - [`settings`] has typed structs for the settings the first editor window needs.
//! - [`location`] says where the file is on each OS.
//!
//! This crate has no UI and no OS-specific code, so the same tests run on every OS.

pub mod databases;
pub mod dialog_dir;
pub mod history;
pub mod location;
/// Temporary (ADR-0021): takes the settings of the old name over. Delete at 1.0.
pub mod migrate_old_name;
pub mod plugins;
pub mod profile;
pub mod save_filter;
pub mod settings;
mod sync;
pub mod unsaved;
pub mod value;

pub use profile::{FileEncoding, LineEnding, Profile, ProfileError};
