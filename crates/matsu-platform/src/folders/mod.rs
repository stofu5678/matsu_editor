//! The folders of the user that Sakura's meta folders name (`%MYDOC%`, `%Personal%`, `%Desktop%` ...,
//! `CFileNameManager::ExpandMetaToFolder`, which reads them from the registry key
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders`).
//!
//! * Windows: the known folders of the shell (`SHGetKnownFolderPath`), by the names of that key.
//! * macOS and Linux: the folders of the home directory (`~/Documents`, `~/Desktop`, ...; on Linux
//!   those of `~/.config/user-dirs.dirs`). Folders for all users (`Common ...`) and the folders of
//!   Windows programs (`AppData` ...) do not exist there.
//!
//! [`known_folder`] is the only function the callers need; a name is compared without regard to
//! case, as the registry does.

#[cfg(not(windows))]
mod unix;
#[cfg(windows)]
mod windows;

use std::path::PathBuf;

/// The folder the registry key `Shell Folders` calls `name` (`Personal`, `Desktop`, `My Pictures`,
/// `My Music`, `Video`, `Favorites`, `AppData`, `Local AppData`, `Common Desktop`,
/// `Common Documents`, `CommonPictures`, `CommonMusic`, `CommonVideo`), or `None` if this OS has
/// no such folder.
pub fn known_folder(name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        windows::known_folder(name)
    }
    #[cfg(not(windows))]
    {
        unix::known_folder(name)
    }
}
