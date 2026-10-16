//! The file dialogs of the OS, with a start folder: what GPUI's `prompt_for_paths` and
//! `prompt_for_new_path` cannot do (they cannot be told where to start), and Sakura's
//! 「ファイルダイアログの初期位置」 and 「最近使ったフォルダー」 need.
//!
//! * Windows: `IFileOpenDialog` / `IFileSaveDialog` and `SetFolder` (`windows`), the dialogs
//!   Sakura shows (`sakura_core/dlg/CDlgOpenFile_CommonItemDialog.cpp`). The dialog runs on a
//!   thread of its own, owned by the window.
//! * macOS: `NSOpenPanel` / `NSSavePanel` with `directoryURL`, as a sheet on the window (`mac`).
//! * Linux: the file chooser portal (`org.freedesktop.portal.FileChooser`) with `current_folder`
//!   (`linux`). Without a portal the call fails with [`Error::Unavailable`].
//!
//! The caller falls back to GPUI's dialog, without a start folder, when [`show`] fails. The calls
//! into the OS are not run by CI (it has no desktop); the user checks them.

#[cfg(not(any(windows, target_os = "macos")))]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod windows;

#[cfg(not(any(windows, target_os = "macos")))]
use linux as os;
#[cfg(target_os = "macos")]
use mac as os;
#[cfg(windows)]
use windows as os;

pub use os::Owner;
pub use raw_window_handle::RawWindowHandle;

use std::fmt;
use std::path::PathBuf;

/// Which dialog to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A file to open; `multiple` lets the user choose several.
    OpenFile { multiple: bool },
    /// A name to save under.
    SaveFile,
    /// A folder.
    Folder,
}

/// What to ask the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub kind: Kind,
    /// The folder the dialog shows first. `None`, or a folder that does not exist: where the OS
    /// wants.
    pub start: Option<PathBuf>,
    /// The file name offered (a save dialog, and an open dialog that has a file to select).
    pub name: Option<String>,
    /// The title of the dialog.
    pub title: String,
}

/// Why no dialog came up, or why it failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// This machine cannot show it (no portal, not the main thread ...). The caller may use
    /// another dialog.
    Unavailable(String),
    /// The OS refused.
    Failed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(why) => write!(f, "ファイルダイアログを使えません: {why}"),
            Self::Failed(why) => write!(f, "ファイルダイアログが失敗しました: {why}"),
        }
    }
}

impl std::error::Error for Error {}

/// Shows the dialog, owned by `owner`, and waits for the user: the paths chosen, or `None` when the
/// user cancels. It does not block the thread that polls it, except on macOS, where AppKit's panel
/// belongs to the main thread and the future must be polled there.
pub async fn show(owner: Owner, request: Request) -> Result<Option<Vec<PathBuf>>, Error> {
    os::show(owner, request).await
}

/// The path of a `file://` URI that a portal returns: the percent-escapes are decoded. `None` for
/// any other kind of URI, a host that is not local, or escapes that are not UTF-8.
#[cfg_attr(
    not(any(test, not(any(windows, target_os = "macos")))),
    allow(dead_code)
)]
pub(crate) fn path_from_file_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // `file://host/path`: only an empty host or `localhost` is this machine
    let path = match rest.find('/') {
        Some(0) => rest,
        Some(at) if rest[..at].eq_ignore_ascii_case("localhost") => &rest[at..],
        _ => return None,
    };
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(PathBuf::from(String::from_utf8(out).ok()?))
}

#[cfg(test)]
mod tests;
