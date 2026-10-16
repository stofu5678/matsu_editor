//! What GPUI does not have and the OS does: printing, and opening a shell in a folder
//! ([ADR-0011], [ADR-0013]).
//!
//! This crate is below `ume-ui` and knows nothing of GPUI. The pages come from `ume-print`
//! (the layout, the same for the preview and for every OS); this crate prints them: the print
//! dialog and GDI on Windows, AppKit's print panel and Core Text on macOS, and a PDF handed to
//! the print portal on Linux. `unsafe` code is in the OS modules `print::windows` and
//! `print::mac` only.
//!
//! [ADR-0011]: https://github.com/stofu5678/ume_editor/blob/main/docs/adr/0011-printing.md
//! [ADR-0013]: https://github.com/stofu5678/ume_editor/blob/main/docs/adr/0013-module-structure.md

pub mod print;
pub mod shell;
