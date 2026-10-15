//! Named commands of UME Editor.
//!
//! * [`command`]: the [`Command`](command::Command) trait and `register_command!`. Every command
//!   is a unit struct in its own file under `commands/`, with its ID and the Sakura function it
//!   implements.
//! * [`registry`]: all commands, by ID and by Sakura's function code.
//! * [`context`]: [`CommandContext`](context::CommandContext), the document, the cursor and the
//!   layout that a command works on, with the methods that edit and move.
//! * [`convert`]: the text conversions of Sakura (case, width, kana, tabs) and the date and time
//!   pictures, as functions on strings.
//! * [`layout`]: the small trait through which commands ask the layout (`ume-layout` will
//!   implement it), and a simple layout for tests.
//! * [`keymap`]: keys, modifiers and the key map (Sakura's default keys, the macOS rules, user bindings).
//! * [`sakura`]: Sakura's function codes, default key bindings, default main menu and macro
//!   function tables.
//! * [`macro_names`]: Sakura's macro names and their function codes.
//! * [`key_macro`]: recording, saving, loading and playing key macros (Sakura's `.mac` files).
//!
//! The README of this crate explains how to add a command and how to write its test.

pub mod command;
pub mod commands;
pub mod context;
pub mod convert;
pub mod key_macro;
pub mod keymap;
pub mod layout;
pub mod macro_names;
pub mod registry;
pub mod sakura;
pub mod text;

#[cfg(test)]
pub(crate) mod testing;

// `register_command!` refers to `inventory` through this path.
#[doc(hidden)]
pub use inventory;
