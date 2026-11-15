//! What a confirmation window shows before text is sent to a generative AI (ADR-0030, 案 2 and 7):
//! the head and the tail of the text, its size, and the lines that look like secrets.
//!
//! The text is read from a [`Document`](crate::buffer::Document) by range, never as one string
//! (`CLAUDE.md`). [`secrets`] is a pure scanner over one line; [`preview`] walks a range of the
//! document with a byte budget, so that a huge selection cannot freeze the window.

pub mod preview;
pub mod secrets;

pub use preview::{
    LARGE_TEXT_BYTES, PreviewLimits, PreviewLine, SecretLine, TextPreview, preview_range,
};
pub use secrets::{SecretKind, SecretMatch, find_secrets, mask_line};
