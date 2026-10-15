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
//! * [`sakura`]: Sakura's function codes, default key bindings and default main menu.
//!
//! The README of this crate explains how to add a command and how to write its test.

pub mod command;
pub mod commands;
pub mod context;
pub mod convert;
pub mod keymap;
pub mod layout;
pub mod registry;
pub mod sakura;
pub mod text;

#[cfg(test)]
pub(crate) mod testing;

// `register_command!` refers to `inventory` through this path.
#[doc(hidden)]
pub use inventory;
