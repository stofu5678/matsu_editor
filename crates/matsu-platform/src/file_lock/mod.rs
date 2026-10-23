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
//!   [`ShareMode::DenyReadWrite`]). It stops only the programs that ask for a lock too (another MATSU
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

/// Whether this program can write `path` now: the check `CSaveAgent::OnCheckSave` makes before a
/// save (`sakura_core/agent/CSaveAgent.cpp`). Sakura opens the file for appending, which keeps its
/// content, and deletes a file that was not there. The caller lets go of its own lock first.
///
/// * Windows: the open fails while another program holds the file with a share mode that
///   refuses writing.
/// * macOS and Linux: there is no share mode, so after the open an exclusive `flock` is tried
///   and given back at once. It fails (`WouldBlock`) while another program, another MATSU Editor
///   for example, holds the file locked in either mode. A program that does not lock is not seen.
pub fn check_writable(path: &Path) -> io::Result<()> {
    // Only a regular file is probed. Opening a FIFO for appending blocks until a reader comes, and
    // a dangling symlink would get its target made; the save itself reports what it cannot write.
    match std::fs::metadata(path) {
        Ok(meta) if !meta.is_file() => return Ok(()),
        Ok(_) => {}
        Err(_) if std::fs::symlink_metadata(path).is_ok() => return Ok(()),
        Err(_) => {}
    }
    // `create_new` tells a file made here from one that was there, also when another program
    // makes it meanwhile or the path is a dangling symlink (which `create` would follow and a
    // later delete would remove).
    let (file, created) = match File::options().append(true).create_new(true).open(path) {
        Ok(file) => (file, true),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            (File::options().append(true).open(path)?, false)
        }
        Err(error) => return Err(error),
    };
    #[cfg(not(windows))]
    let locked = {
        use std::fs::TryLockError;
        match file.try_lock() {
            Ok(()) => Ok(()),
            Err(TryLockError::WouldBlock) => Err(io::Error::from(io::ErrorKind::WouldBlock)),
            Err(TryLockError::Error(error)) => Err(error),
        }
    };
    #[cfg(windows)]
    let locked = Ok(());
    drop(file);
    if created {
        // `::DeleteFile`: the file was only made for the check.
        let _ = std::fs::remove_file(path);
    }
    locked
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

    #[test]
    fn a_free_file_can_be_written_and_is_left_as_it_was() {
        let (_dir, path) = file();
        check_writable(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "abc");
    }

    #[test]
    fn a_file_that_is_not_there_is_not_left_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.txt");
        check_writable(&path).unwrap();
        assert!(!path.exists());
    }

    /// A dangling symlink is left as it is, and its target is not made.
    #[cfg(unix)]
    #[test]
    fn a_dangling_symlink_stays() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(dir.path().join("target"), &link).unwrap();
        check_writable(&link).unwrap();
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        assert!(!dir.path().join("target").exists());
    }

    #[test]
    fn a_path_in_a_missing_folder_cannot_be_written() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check_writable(&dir.path().join("no").join("a.txt")).is_err());
    }

    /// Another program that holds the lock stops the save, in either mode; Windows refuses the
    /// append for the same reason through the share mode.
    #[test]
    fn a_file_another_lock_holds_cannot_be_written() {
        for mode in [ShareMode::DenyWrite, ShareMode::DenyReadWrite] {
            let (_dir, path) = file();
            let lock = FileLock::acquire(&path, mode).unwrap();
            assert!(check_writable(&path).is_err(), "{mode:?}");
            drop(lock);
            check_writable(&path).unwrap();
        }
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
