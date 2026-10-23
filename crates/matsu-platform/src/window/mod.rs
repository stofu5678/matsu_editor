//! Moving, sizing and raising the windows of the OS: what Sakura's window functions (重ねて表示,
//! 並べて表示, 縦方向に最大化, 常に手前に表示 ...) need and GPUI does not have.
//!
//! GPUI can make a window, resize it (not move it), minimize it and toggle zoom, but it cannot
//! place an existing window, keep it on top or tell the work area of its screen. The OS can, with
//! the handle of the window ([`RawWindowHandle`]):
//!
//! * Windows: `SetWindowPos`, `GetWindowRect`, `GetMonitorInfoW` (`windows`).
//! * macOS: `NSWindow` and `NSScreen` (`mac`).
//! * Linux: nothing. A Wayland compositor decides where windows go, and a program cannot move
//!   its own windows or keep them on top; for X11 GPUI gives no way to reach the window
//!   from here. Every function returns [`Error::Unsupported`].
//!
//! The arithmetic (where each window goes) is in [`arrange`] and is tested on every OS. The
//! calls into the OS are not run by CI (it has no desktop); the user checks them.
//!
//! All rectangles are in the units of the OS (pixels on Windows, points on macOS) with the y axis
//! pointing down, the origin being the top left of the primary screen.

pub mod arrange;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(not(any(windows, target_os = "macos")))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
use mac as os;
#[cfg(not(any(windows, target_os = "macos")))]
use unsupported as os;
#[cfg(windows)]
use windows as os;

pub use arrange::Rect;
pub use raw_window_handle::RawWindowHandle;

use std::fmt;

/// Why a window could not be moved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// This OS (or this kind of window) cannot do it. The text says what.
    Unsupported(&'static str),
    /// The OS refused, or the handle is not a window of this OS.
    Failed(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(what) => write!(f, "{what}: この環境では使えない"),
            Self::Failed(what) => write!(f, "{what}に失敗した"),
        }
    }
}

impl std::error::Error for Error {}

/// Why this OS cannot place windows, or `None` when it can. Ask before queueing a change that
/// would only fail later.
pub fn unsupported() -> Option<&'static str> {
    os::unsupported()
}

/// The area of the screen the window is on that programs may use: without the task bar, the
/// menu bar and the dock (Sakura's `GetMonitorWorkRect`).
pub fn work_area(window: RawWindowHandle) -> Result<Rect, Error> {
    os::work_area(window)
}

/// The rectangle of the window including its frame.
pub fn frame(window: RawWindowHandle) -> Result<Rect, Error> {
    os::frame(window)
}

/// Whether the window is minimized (Sakura's `IsIconic`).
pub fn is_minimized(window: RawWindowHandle) -> Result<bool, Error> {
    os::is_minimized(window)
}

/// Puts the window at `rect`. A maximized or minimized window is restored first
/// (`ShowWindow(SW_RESTORE)` before `SetWindowPos` in `Command_TILE_V` and the others), and
/// the z order is left alone.
pub fn place(window: RawWindowHandle, rect: Rect) -> Result<(), Error> {
    os::place(window, rect)
}

/// How far a cascade shifts each window: the width and the height of the frame and the title
/// bar (`SM_CXSIZEFRAME + SM_CXSIZE`, `SM_CYSIZEFRAME + SM_CYSIZE`).
pub fn cascade_step() -> (i32, i32) {
    os::cascade_step()
}

/// Whether the window stays above the others.
pub fn is_topmost(window: RawWindowHandle) -> Result<bool, Error> {
    os::is_topmost(window)
}

/// Keeps the window above the others, or not (`WindowTopMost`: `HWND_TOPMOST`,
/// `HWND_NOTOPMOST`), without activating it.
pub fn set_topmost(window: RawWindowHandle, topmost: bool) -> Result<(), Error> {
    os::set_topmost(window, topmost)
}

/// Brings a minimized or maximized window back to its normal size and place (Sakura's
/// `ShowWindow(SW_RESTORE)`, `F_WINRESTORE`). A window that is normal already stays as it is.
pub fn restore(window: RawWindowHandle) -> Result<(), Error> {
    os::restore(window)
}
