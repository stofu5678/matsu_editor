//! The UI commands (`crate::ui_command`), one per file, in Sakura's groups.
//!
//! To add one: create `<group>/<id>.rs` with a type that implements `UiCommand` and
//! `register_ui_command!(Type);`, add one `pub mod <id>;` line to the group's `mod.rs` (in
//! alphabetical order), and write its tests. Nothing else lists them.

pub mod box_select;
pub mod cursor;
pub mod file;
pub mod macros;
pub mod search;
pub mod select;
pub mod settings;
pub mod support;
