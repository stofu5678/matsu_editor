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

/// One entry of the list of file types of a save dialog (Sakura's `COMDLG_FILTERSPEC`), for
/// example "ユーザー指定" with `*.cpp` and `*.h`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    /// What the list shows.
    pub name: String,
    /// The wildcards, one by one (`*.txt`, `*.*`).
    pub patterns: Vec<String>,
}

impl FileFilter {
    /// The extensions of the patterns that name one (`*.txt` gives `txt`). A pattern with another
    /// wildcard in the extension (`*.t?t`) or without an extension (`Make*`) gives none.
    pub fn extensions(&self) -> Vec<&str> {
        self.patterns
            .iter()
            .filter_map(|pattern| pattern.strip_prefix("*."))
            .filter(|ext| !ext.is_empty() && !ext.contains(['*', '?', '.', '/', '\\']))
            .collect()
    }

    /// Whether the filter shows every file (`*.*` or `*`).
    pub fn matches_all(&self) -> bool {
        self.patterns.iter().any(|p| p == "*.*" || p == "*")
    }
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
    /// The list of file types of a save dialog; empty for no list (and for every other kind of
    /// dialog). Windows shows it as the list next to the name (`IFileDialog::SetFileTypes`), the
    /// portal as its filters (`filters`, `current_filter`). The panel of macOS has no list: it
    /// takes the extensions of the selected type (`allowedFileTypes`) and lets other names be
    /// typed. GPUI's dialog, the fallback, has none of this.
    pub filters: Vec<FileFilter>,
    /// The entry of [`filters`](Self::filters) shown first.
    pub selected_filter: usize,
    /// The extension (`txt`, no dot) added to a name typed without one (a save dialog;
    /// `IFileDialog::SetDefaultExtension`). Windows and macOS do it in the dialog; the portal
    /// does not, so [`show`] does it for the portal.
    pub default_extension: Option<String>,
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

/// `path` with `.extension` added when its file name has no extension: what the Windows dialog does
/// with the default extension. For a dialog that does not do it itself.
///
/// Such a dialog asked about overwriting the name as it was typed, not the one with the extension.
/// So a file that is there under the longer name is never taken: the name stays as it was typed
/// (the Windows dialog would ask).
pub fn with_default_extension(path: PathBuf, extension: &str) -> PathBuf {
    if path.extension().is_none()
        && !extension.is_empty()
        && let Some(name) = path.file_name()
    {
        let mut name = name.to_os_string();
        name.push(".");
        name.push(extension);
        let longer = path.with_file_name(name);
        if !longer.exists() {
            return longer;
        }
    }
    path
}

/// The path of a `file://` URI that a portal returns: the percent-escapes are decoded to bytes,
/// which need not be UTF-8 (a file name is any bytes on Unix). `None` for any other kind of URI,
/// or a host that is not local.
#[cfg(unix)]
#[cfg_attr(target_os = "macos", allow(dead_code))]
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
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(std::ffi::OsString::from_vec(out)))
}

#[cfg(test)]
mod filter_tests;
#[cfg(all(test, unix))]
mod tests;
