//! Reads and writes UME Editor's settings file `ume.ini`, in the format of Sakura Editor's `sakura.ini`
//! (ADR-0010).
//!
//! - [`Profile`] is the file format (`sakura_core/env/CProfile.cpp`). It keeps every line, so reading a file and
//!   writing it back gives the same bytes, and entries UME Editor does not know are never lost.
//! - [`value`] is the typed access of `CDataProfile`.
//! - [`settings`] has typed structs for the settings the first editor window needs.
//! - [`location`] says where the file is on each OS.
//!
//! This crate has no UI and no OS-specific code, so the same tests run on every OS.

pub mod dialog_dir;
pub mod history;
pub mod location;
pub mod profile;
pub mod save_filter;
pub mod settings;
mod sync;
pub mod value;

pub use profile::{FileEncoding, LineEnding, Profile, ProfileError};
