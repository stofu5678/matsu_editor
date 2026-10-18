//! Keeping an open file locked against other programs (Sakura's 排他制御, `nFileShareMode`).
//!
//! Sakura opens the file once more and keeps that handle as long as the document is open
//! (`CFile::FileLock`, `sakura_core/io/CFile.cpp`): `SHAREMODE_DENY_WRITE` opens it with
//! `FILE_SHARE_READ` (others may read, nobody may write), `SHAREMODE_DENY_READWRITE` with no
//! sharing at all. [`FileLock`] is that handle; dropping it is `FileUnlock`.
//!
//! * Windows: the same, through the share mode of `OpenOptions`. The OS enforces it against every
//!   program. A locked file cannot be renamed over or deleted either.
//! * macOS and Linux: there is no mandatory share mode. The lock is an advisory `flock`
//!   (`File::try_lock_shared` for [`ShareMode::DenyWrite`], `File::try_lock` for
//!   [`ShareMode::DenyReadWrite`]). It stops only the programs that ask for a lock too (another UME
//!   Editor, `flock(1)`); a program that just opens and writes the file is not stopped. The shared
//!   lock is the "others may read" mode: it refuses any exclusive lock and allows other shared
//!   locks. The exclusive lock refuses every other lock.
//!
//! There is no `unsafe` here: the standard library has both APIs.
//!
//! A save that replaces the file (write a temporary file, rename it over, ADR-0005) ends the lock
//! on Unix, where the lock belongs to the old file, and is refused on Windows while the lock is
//! held. So the caller releases the lock around a save and takes it again afterwards, as Sakura
//! does.

use std::fs::File;
use std::io;
use std::path::Path;

/// How the file is shared with other programs while it is locked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShareMode {
    /// `SHAREMODE_DENY_WRITE`: others may read the file but not write it.
    DenyWrite,
    /// `SHAREMODE_DENY_READWRITE`: others may neither read nor write it.
    DenyReadWrite,
}

/// A held lock. The file is unlocked when this is dropped.
#[derive(Debug)]
pub struct FileLock {
    // Only held: closing the handle releases the lock.
    _file: File,
    mode: ShareMode,
}

impl FileLock {
    /// Locks the file at `path`. Fails if it does not exist (`OPEN_EXISTING`) or another program
    /// holds a conflicting lock or has the file open in a way that conflicts (Windows).
    pub fn acquire(path: &Path, mode: ShareMode) -> io::Result<Self> {
        let file = open(path, mode)?;
        Ok(Self { _file: file, mode })
    }

    /// The mode the file is locked in.
    pub fn mode(&self) -> ShareMode {
        self.mode
    }
}

#[cfg(windows)]
fn open(path: &Path, mode: ShareMode) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    /// `FILE_SHARE_READ`
    const FILE_SHARE_READ: u32 = 1;
    let share = match mode {
        ShareMode::DenyWrite => FILE_SHARE_READ,
        ShareMode::DenyReadWrite => 0,
    };
    // `GENERIC_READ`, `OPEN_EXISTING`, as `CFile::FileLock`.
    File::options().read(true).share_mode(share).open(path)
}

#[cfg(not(windows))]
fn open(path: &Path, mode: ShareMode) -> io::Result<File> {
    use std::fs::TryLockError;
    let file = File::open(path)?;
    let locked = match mode {
        ShareMode::DenyWrite => file.try_lock_shared(),
        ShareMode::DenyReadWrite => file.try_lock(),
    };
    match locked {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "abc").unwrap();
        (dir, path)
    }

    #[test]
    fn a_file_that_is_not_there_cannot_be_locked() {
        let dir = tempfile::tempdir().unwrap();
        let error =
            FileLock::acquire(&dir.path().join("missing"), ShareMode::DenyWrite).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn the_lock_remembers_its_mode() {
        let (_dir, path) = file();
        let lock = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        assert_eq!(lock.mode(), ShareMode::DenyReadWrite);
    }

    #[test]
    fn two_locks_that_deny_writing_share_the_file() {
        let (_dir, path) = file();
        let _a = FileLock::acquire(&path, ShareMode::DenyWrite).unwrap();
        let _b = FileLock::acquire(&path, ShareMode::DenyWrite).unwrap();
    }

    #[test]
    fn a_lock_that_denies_reading_refuses_every_other_lock() {
        let (_dir, path) = file();
        let _a = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        assert!(FileLock::acquire(&path, ShareMode::DenyWrite).is_err());
        assert!(FileLock::acquire(&path, ShareMode::DenyReadWrite).is_err());
    }

    #[test]
    fn a_lock_that_denies_writing_refuses_a_lock_that_denies_reading() {
        let (_dir, path) = file();
        let _a = FileLock::acquire(&path, ShareMode::DenyWrite).unwrap();
        assert!(FileLock::acquire(&path, ShareMode::DenyReadWrite).is_err());
    }

    #[test]
    fn dropping_the_lock_releases_the_file() {
        let (_dir, path) = file();
        let a = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        assert!(FileLock::acquire(&path, ShareMode::DenyReadWrite).is_err());
        drop(a);
        FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
    }

    /// Only Windows enforces the lock against a program that does not lock.
    #[cfg(windows)]
    #[test]
    fn windows_refuses_writing_to_a_file_that_denies_writing() {
        let (_dir, path) = file();
        let _lock = FileLock::acquire(&path, ShareMode::DenyWrite).unwrap();
        assert!(File::options().write(true).open(&path).is_err());
        assert!(File::open(&path).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn windows_refuses_reading_a_file_that_denies_reading() {
        let (_dir, path) = file();
        let _lock = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        assert!(File::open(&path).is_err());
        assert!(File::options().append(true).open(&path).is_err());
    }

    /// The lock is advisory on macOS and Linux: a program that does not lock still writes.
    #[cfg(not(windows))]
    #[test]
    fn unix_does_not_stop_a_program_that_does_not_lock() {
        let (_dir, path) = file();
        let _lock = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        assert!(File::open(&path).is_ok());
        assert!(File::options().append(true).open(&path).is_ok());
    }
}
