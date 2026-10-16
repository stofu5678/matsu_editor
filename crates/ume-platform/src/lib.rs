//! What GPUI does not have and the OS does: printing, opening a shell in a folder, placing
//! windows, handing a second launch over to the running UME Editor, and file dialogs that start
//! in a given folder ([ADR-0011], [ADR-0013]).
//!
//! This crate is below `ume-ui` and knows nothing of GPUI. The pages come from `ume-print`
//! (the layout, the same for the preview and for every OS); this crate prints them: the print
//! dialog and GDI on Windows, AppKit's print panel and Core Text on macOS, and a PDF handed to
//! the print portal on Linux. `unsafe` code is in the OS modules `print::windows`, `print::mac`,
//! `window::windows`, `window::mac`, `shell::windows`, `instance::windows`, `instance::unix`,
//! `dialog::windows`, `dialog::mac` and `folders::windows` only.
//!
//! [ADR-0011]: https://github.com/stofu5678/ume_editor/blob/main/docs/adr/0011-printing.md
//! [ADR-0013]: https://github.com/stofu5678/ume_editor/blob/main/docs/adr/0013-module-structure.md

pub mod dialog;
pub mod folders;
pub mod instance;
#[cfg(not(any(windows, target_os = "macos")))]
mod portal;
pub mod print;
pub mod shell;
pub mod window;
