//! Moving a file to the recycle bin (Sakura's `SHFileOperation` with the flag for undo in
//! `CBackupAgent::MakeBackUp`, for 「バックアップファイルをごみ箱に放り込む」).
//!
//! * Windows: `SHFileOperationW` with the flag for undo (the recycle bin), without a dialog.
//! * macOS: `NSFileManager trashItemAtURL`.
//! * Linux: the freedesktop.org trash of the user (`$XDG_DATA_HOME/Trash`, else
//!   `~/.local/share/Trash`): the file goes to `files/` and its `.trashinfo` to `info/`.

#[cfg(target_os = "macos")]
mod mac;
#[cfg(not(any(windows, target_os = "macos")))]
mod unix;
#[cfg(windows)]
mod windows;

use std::io;
use std::path::Path;

/// Moves the file at `path` to the recycle bin.
pub fn move_to_trash(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        windows::move_to_trash(path)
    }
    #[cfg(target_os = "macos")]
    {
        mac::move_to_trash(path)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        unix::move_to_trash(path)
    }
}
