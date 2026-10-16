//! The functions that only macros call and that need the window: the output panel (`TraceOut`),
//! the status bar (`StatusMsg`), the sound (`MsgBeep`), the scroll position (`SetViewTop`,
//! `SetViewLeft`), the way the text wraps (`TextWrapMethod`), files with an encoding (`PutFile`,
//! `InsFile`, `FileOpen`), and the window itself (`WinMaximize`, `WinMinimize`, `WinRestore`).
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp` reads the arguments of each.
//! The ones that do not need the window are core commands (`ume_commands::commands::macro_only`).
//!
//! A script reaches these through `crate::macros` (a call to a function that no core command
//! implements goes to the UI command of that function), a key macro through `run_function`.

pub mod fileopen2;
pub mod insfile;
pub mod msgbeep;
pub mod putfile;
pub mod setviewleft;
pub mod setviewtop;
pub mod statusmsg;
pub mod textwrapmethod;
pub mod traceout;
pub mod winmaximize;
pub mod winminimize;
pub mod winrestore;

#[cfg(test)]
mod tests;

use gpui::Window;
use ume_commands::command::{Arg, CommandError};

/// What the window functions ask of the window. GPUI's headless test window cannot minimize or
/// zoom (it stops with `unimplemented!`), so the rules are written on this and tested with a
/// fake.
pub(crate) trait WindowChrome {
    fn is_maximized(&self) -> bool;
    /// GPUI only toggles the zoom of a window.
    fn toggle_zoom(&self);
    fn minimize(&self);
}

impl WindowChrome for Window {
    fn is_maximized(&self) -> bool {
        Window::is_maximized(self)
    }

    fn toggle_zoom(&self) {
        self.zoom_window();
    }

    fn minimize(&self) {
        self.minimize_window();
    }
}

/// `ShowWindow(SW_MAXIMIZE)`: a window that is maximized stays so (a toggle would restore it).
pub(crate) fn maximize(window: &dyn WindowChrome) {
    if !window.is_maximized() {
        window.toggle_zoom();
    }
}

/// `ShowWindow(SW_MINIMIZE)`.
pub(crate) fn minimize(window: &dyn WindowChrome) {
    window.minimize();
}

/// What `ShowWindow(SW_RESTORE)` does where the OS cannot be asked (GPUI): a maximized window
/// goes back; a minimized one cannot be told.
pub(crate) fn zoom_back(window: &dyn WindowChrome) {
    if window.is_maximized() {
        window.toggle_zoom();
    }
}

/// The text argument `i`, or the mistake that Sakura reports for a missing one
/// (`CMacro::HandleCommand` shows a message and stops the macro).
pub(crate) fn text_arg<'a>(name: &str, args: &'a [Arg], i: usize) -> Result<&'a str, CommandError> {
    match args.get(i) {
        Some(Arg::Text(text)) => Ok(text),
        _ => Err(CommandError::BadArgs(format!(
            "{name} needs the text as argument {}",
            i + 1
        ))),
    }
}

/// The number argument `i`, or `default` when the macro left it out (Sakura reads a missing
/// number as the default: `Argument[i] != nullptr ? _wtoi(Argument[i]) : default`).
pub(crate) fn int_arg(
    name: &str,
    args: &[Arg],
    i: usize,
    default: i64,
) -> Result<i64, CommandError> {
    match args.get(i) {
        None => Ok(default),
        Some(Arg::Int(n)) => Ok(*n),
        Some(_) => Err(CommandError::BadArgs(format!(
            "{name} needs a number as argument {}",
            i + 1
        ))),
    }
}
