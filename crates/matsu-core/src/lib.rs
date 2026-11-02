//! UI-independent core of MATSU Editor.
//!
//! The buffer, the cursor, undo, encodings, search, and editing commands live here. This crate
//! must not depend on any UI crate (see `CLAUDE.md`).

pub mod bookmarks;
pub mod buffer;
pub mod cursor;
pub mod diff;
pub mod encoding;
pub mod exec;
pub mod file;
pub mod indent_style;
pub mod jump_history;
pub mod model;
pub mod modified;
pub mod search;
pub mod sql_error;
pub mod tag_file;
pub mod tag_jump;
pub mod tags_make;
pub mod undo;
