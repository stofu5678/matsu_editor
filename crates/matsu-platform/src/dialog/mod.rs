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

#[cfg(any(target_os = "macos", test))]
mod groups;
#[cfg(not(any(windows, target_os = "macos")))]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
mod mac_fields;
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
use std::sync::Arc;

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

    /// What a list of file types shows: the name and the patterns (`C/C++ (*.c;*.h)`), so that
    /// the extensions are seen whatever the OS shows (Windows and the portal show the name as it
    /// is). A filter without patterns shows its name alone.
    pub fn label(&self) -> String {
        if self.patterns.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, self.patterns.join(";"))
        }
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
    /// The list of file types of a save dialog or an open dialog; empty for no list (and for the dialog
    /// that picks a folder). Windows shows it as the list next to the name (`IFileDialog::SetFileTypes`), the
    /// portal as its filters (`filters`, `current_filter`). The panel of macOS has no list: it
    /// takes the extensions of the selected type (`allowedFileTypes`) and lets other names be
    /// typed (its open panel has no list at all). GPUI's dialog, the fallback, has none of this.
    pub filters: Vec<FileFilter>,
    /// The entry of [`filters`](Self::filters) shown first.
    pub selected_filter: usize,
    /// The extension (`txt`, no dot) added to a name typed without one (a save dialog;
    /// `IFileDialog::SetDefaultExtension`). Windows and macOS do it in the dialog; the portal
    /// does not, so [`show`] does it for the portal.
    pub default_extension: Option<String>,
    /// Fields the save dialog shows next to the name (Sakura's encoding, BOM and line ending,
    /// `CDlgOpenFile_CommonItemDialog::Customize`). The dialog does not know what they mean: it
    /// shows them, and [`Chosen::extras`] gives back what the user chose, in the same order.
    /// Windows shows them with `IFileDialogCustomize`, the portal as `choices`, the save panel of
    /// macOS as an accessory view (the lists side by side, at the right); the dialog of GPUI and
    /// the open panel of macOS have none, so [`Chosen::extras`] is empty there.
    pub extras: Vec<Extra>,
    /// A check box that follows a list while the dialog is open (Windows only).
    pub follow: Option<Follow>,
}

/// One field of [`Request::extras`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Extra {
    /// A drop-down list. `label` names its group.
    Combo {
        label: String,
        items: Vec<String>,
        selected: usize,
    },
    /// A check box. It sits in the group of the list before it.
    Check {
        label: String,
        checked: bool,
        enabled: bool,
    },
    /// A list of recent files or folders. Nothing is selected at first; choosing an item puts
    /// the file name in the name field, or shows the folder (Sakura's `COMBO_MRU` and
    /// `COMBO_OPENFOLDER`, `CDlgOpenFile_CommonItemDialog::OnItemSelected`). Only the dialog of
    /// Windows can do that while it is open: the portal's `choices` cannot change the name or the
    /// folder, and the panels of macOS are not given such a list. There it is left out
    /// ([`show_with_extras`]).
    Recent {
        label: String,
        /// The paths, as text.
        items: Vec<String>,
        kind: RecentKind,
    },
}

/// What choosing an item of [`Extra::Recent`] does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecentKind {
    /// The item is a file: its path becomes the file name of the dialog.
    Files,
    /// The item is a folder: the dialog shows it.
    Folders,
}

/// What the user chose for one [`Extra`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtraValue {
    /// The index of the item chosen.
    Combo(usize),
    Check(bool),
    /// A field that gives no value ([`Extra::Recent`]), or that the dialog did not show.
    Nothing,
}

/// What the user answered: the paths, and the value of every [`Request::extras`] field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chosen {
    pub paths: Vec<PathBuf>,
    /// Empty when the dialog showed no fields (its OS has none, or the request had none).
    pub extras: Vec<ExtraValue>,
}

/// The state of a check box as a function of the item chosen in a list: `(enabled, checked)`.
type FollowFn = dyn Fn(usize) -> (bool, bool) + Send + Sync;

/// Makes the check box `check` (an index into [`Request::extras`]) follow the list `combo`: when
/// the user chooses an item, the dialog asks `rule` for the state of the check box (Sakura's
/// `OnItemSelected` for the BOM). It runs on the thread of the dialog.
#[derive(Clone)]
pub struct Follow {
    pub combo: usize,
    pub check: usize,
    pub rule: Arc<FollowFn>,
}

impl Follow {
    pub fn new(
        combo: usize,
        check: usize,
        rule: impl Fn(usize) -> (bool, bool) + Send + Sync + 'static,
    ) -> Self {
        Self {
            combo,
            check,
            rule: Arc::new(rule),
        }
    }
}

impl fmt::Debug for Follow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Follow({} -> {})", self.combo, self.check)
    }
}

impl PartialEq for Follow {
    fn eq(&self, other: &Self) -> bool {
        self.combo == other.combo
            && self.check == other.check
            && Arc::ptr_eq(&self.rule, &other.rule)
    }
}

impl Eq for Follow {}

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
    Ok(os::show(owner, request).await?.map(|chosen| chosen.paths))
}

/// [`show`] with the values of the fields of [`Request::extras`].
pub async fn show_with_extras(owner: Owner, request: Request) -> Result<Option<Chosen>, Error> {
    #[cfg(windows)]
    {
        os::show(owner, request).await
    }
    #[cfg(not(windows))]
    {
        // The dialogs of the other OSes cannot change the name or the folder while they are open:
        // the lists of recent paths are left out, and their places in the answer are kept.
        let (request, shown) = without_recent(request);
        let chosen = os::show(owner, request).await?;
        Ok(chosen.map(|chosen| with_recent_places(chosen, &shown)))
    }
}

/// `request` without its [`Extra::Recent`] fields, and for each field of the original whether it
/// is shown.
#[cfg_attr(windows, allow(dead_code))]
fn without_recent(mut request: Request) -> (Request, Vec<bool>) {
    let shown: Vec<bool> = request
        .extras
        .iter()
        .map(|extra| !matches!(extra, Extra::Recent { .. }))
        .collect();
    if shown.iter().all(|shown| *shown) {
        return (request, shown);
    }
    // `Follow` indexes the fields before the lists of recent paths, which come last: but any
    // index that moves is mapped.
    let new_index = |old: usize| shown[..old].iter().filter(|shown| **shown).count();
    if let Some(follow) = &mut request.follow {
        follow.combo = new_index(follow.combo);
        follow.check = new_index(follow.check);
    }
    let mut kept = shown.iter();
    request.extras.retain(|_| *kept.next().unwrap_or(&true));
    (request, shown)
}

/// The values the dialog gave for the fields it showed, with [`ExtraValue::Nothing`] where a field
/// was left out. No values (the dialog showed no fields) stay none.
#[cfg_attr(windows, allow(dead_code))]
fn with_recent_places(mut chosen: Chosen, shown: &[bool]) -> Chosen {
    if shown.iter().all(|shown| *shown) || chosen.extras.is_empty() {
        return chosen;
    }
    let mut values = chosen.extras.into_iter();
    chosen.extras = shown
        .iter()
        .map(|shown| {
            if *shown {
                values.next().unwrap_or(ExtraValue::Nothing)
            } else {
                ExtraValue::Nothing
            }
        })
        .collect();
    chosen
}

/// The default extension for a dialog that does not do it itself: the dialog of Windows changes
/// it to that of the file type the user chooses (`IFileDialog::SetDefaultExtension`), so this is
/// the first extension the chosen type names (`*.c;*.h` gives `c`), else `fallback`.
pub fn extension_for_filter(filter: Option<&FileFilter>, fallback: &str) -> String {
    filter
        .and_then(|filter| filter.extensions().first().map(|ext| (*ext).to_owned()))
        .unwrap_or_else(|| fallback.to_owned())
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
#[cfg(test)]
mod recent_tests;
#[cfg(all(test, unix))]
mod tests;
