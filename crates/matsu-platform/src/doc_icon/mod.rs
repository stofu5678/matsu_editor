//! The icon of a window that follows its document (「文書アイコンを使う」, `bUseDocumentIcon` of
//! the type).
//!
//! Sakura: `CDocType::SetDocumentIcon` (`sakura_core/doc/CDocType.cpp`) gives the window the icon
//! the shell has for the file (`CEditWnd::GetRelatedIcon`: `SHGetFileInfo` with `SHGFI_ICON`, big
//! and small) when the type says so, and the icon of the application when it does not
//! (`GetDefaultIcon`), with `WM_SETICON` (`CEditWnd::SetWindowIcon`, which destroys the icon it
//! replaces).
//!
//! What each OS allows:
//!
//! * Windows: the same (`windows.rs`). One difference: Sakura gives the icon of the application
//!   when the extension has no `DefaultIcon` in the registry; here the shell's own icon for the
//!   extension is used then (its generic document icon).
//! * macOS: the icon of a window is the one of the application. The title bar shows the icon of
//!   the document only for a window made of `NSDocument`, which GPUI's windows are not.
//! * Linux: X11 has `_NET_WM_ICON`, which GPUI sets once when it makes the window and gives the
//!   program no way to change; Wayland has no window icon the program can set (the icon comes from
//!   the `.desktop` file).
//!
//! Both of the last two return [`Error::Unsupported`]. The calls into the OS are not run by CI
//! (it has no desktop); the user checks them.

use std::path::Path;

pub use raw_window_handle::RawWindowHandle;

#[cfg(not(windows))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
use unsupported as os;
#[cfg(windows)]
use windows as os;

/// Why the icon of the window was not changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// This OS cannot do it. The text says why.
    Unsupported(&'static str),
    /// The OS refused, or the handle is not a window of this OS.
    Failed(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unsupported(why) | Error::Failed(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Error {}

/// Why this OS cannot change the icon of a window, or `None` when it can.
pub fn unsupported() -> Option<&'static str> {
    os::unsupported()
}

/// Gives the window the icon of the shell for `file`, or the icon the window had when it was made
/// (the application's) for `None`.
pub fn set(window: RawWindowHandle, file: Option<&Path>) -> Result<(), Error> {
    os::set(window, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn where_the_os_cannot_the_icon_is_refused_with_a_reason() {
        assert!(unsupported().is_some());
        let handle = RawWindowHandle::Xlib(raw_window_handle::XlibWindowHandle::new(1));
        assert!(matches!(
            set(handle, Some(Path::new("a.txt"))),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(set(handle, None), Err(Error::Unsupported(_))));
    }
}
