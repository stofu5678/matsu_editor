//! The commands, one per file, in groups.
//!
//! To add a command: create `<group>/<command>.rs` (the README of this crate has a template),
//! add one `pub mod <command>;` line to the `mod.rs` of the group (in alphabetical order), and
//! write its tests in the same file. Nothing else lists commands: `register_command!` in the
//! file registers it.
//!
//! The groups follow Sakura's: editing, cursor movement, selection, conversion, insertion.

pub mod convert;
pub mod cursor;
pub mod edit;
pub mod insert;
pub mod select;
