//! Typed settings for the first editor window: the font, and the settings of a document type (tab width,
//! wrap, line numbers, what is shown, kinsoku, the encoding and line ending of new files).
//!
//! Everything else in the file stays in the [`Profile`](crate::Profile) as raw entries.

pub mod color;
pub mod font;
pub mod types;

pub use color::{ColorEntry, ColorSet};
pub use font::{FontSetting, LogFont, ViewFont};
pub use types::{NewFileKeys, TypeSettings, types_count};
