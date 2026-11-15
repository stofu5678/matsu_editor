//! Running a one-liner (ADR-0029): a shell command line that gets the selection, or the whole
//! document, on its standard input; its output replaces the input, goes on the next line, or goes
//! to the output window. This crate is the parts that run it, with no window and no menu; the
//! flow in the window (the document, the undo step, the box that says it runs, the output
//! window) is `matsu-ui`.
//!
//! * [`shell`]: which shell runs the line and with which arguments. Pure, so it is tested on every
//!   OS. It is here and not in `matsu-terminal` because it only needs that crate's splitting of
//!   the `Shell=` setting, and the encodings and the PowerShell script are about one-liners, not
//!   about the tab. It is here and not in `matsu-core` because the core must not depend on the
//!   terminal crate.
//! * [`powershell`]: the script that wraps the line for PowerShell.
//! * [`start`]: starts the shell with the text on its standard input and returns a [`Run`], which
//!   gives the result when the process has ended.
//!
//! The text rules (the input in chunks, the output decoded and made LF, the cap) are
//! `matsu_core::oneliner`; the process is `matsu_platform::exec`.

pub mod powershell;
mod run;
pub mod shell;

pub use run::{Event, Finished, Input, OutputKind, Request, Run, RunError, StartError, start};
