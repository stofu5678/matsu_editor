//! The window of UME Editor, on GPUI (ADR-0004).
//!
//! This crate is the View of ADR-0013: it draws the text and takes keys, text, the IME and the
//! mouse, and hands everything that changes the document to the named commands of
//! `ume-commands`. The modules:
//!
//! * [`document`]: `Entity<SharedDocument>`, the document and its undo history, shared by views.
//! * [`view`]: `Entity<EditorView>`, the layout, cursor, scroll position and input of one view.
//! * [`app`]: the window, the menu and the status bar.
//! * [`mod@launch`]: starting, and handing a second launch over to the running process.
//! * [`menu`]: Sakura's main menu as a model (labels, shortcuts, what is enabled).
//! * [`ui_command`]: the commands that need the window (the size of the view, dialogs), one per file
//!   under `ui_commands/` like the core commands, found by the same Sakura function codes.
//! * [`macros`]: key macros (record, save, load, play) and macro files (`.mac`, `.mjs`, `.js`, `.py`,
//!   `.rb`, `.php`, `.lua`, `.mruby`, `.upy`).
//! * [`open`]: where a file is read into a document (encoding detection, BOM, size limit).
//! * `file_ops`: the flows of the file functions: open, save, close, reopen, and the questions
//!   they ask.
//! * the rest is the arithmetic and the glue that need no window and are tested without one:
//!   [`keys`] (GPUI keystroke to key map chord), [`layout_adapter`] (`ume-layout` as the commands'
//!   `Layout`, kept up to date after every edit), [`geometry`] and [`hit`] (pixels,
//!   rows and columns), [`segments`] (what to paint in a row), [`word`] and [`status`].

pub mod app;
mod bars;
mod clipboard;
mod command_line;
mod common_settings;
mod custom_menu;
mod diff;
mod doc_types;
pub mod document;
mod exec;
mod file_dialog;
mod file_ops;
pub mod geometry;
pub mod grep;
mod history;
pub mod hit;
pub mod keys;
pub mod launch;
pub mod layout_adapter;
pub mod macros;
pub mod menu;
pub mod menu_bar;
pub mod open;
mod outline;
mod panes;
pub mod print;
pub mod search;
pub mod segments;
pub mod settings;
pub mod status;
mod support;
mod tabs;
mod tags;
pub mod ui_command;
mod ui_commands;
pub mod view;
mod window_ops;
pub mod word;

pub use launch::launch;

// The commands register themselves with `inventory` when the crate is linked; this makes sure
// that the linker keeps it (ADR-0013, "守ること").
use ume_commands as _;
