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
///   A refused try is repeated for a short time first (see `try_lock_retrying`), so that a child
///   process of this program does not look like another program.
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
    let locked = try_lock_retrying(&file, LOCK_RETRY_FOR, |file| file.try_lock());
    #[cfg(windows)]
    let locked = Ok(());
    drop(file);
    if created {
        // `::DeleteFile`: the file was only made for the check.
        let _ = std::fs::remove_file(path);
    }
    locked
}

/// Whether this program can read `path` now: the check `CLoadAgent::OnCheckLoad` makes before a
/// load (`CFile::IsFileReadable`, `sakura_core/io/CFile.cpp`). The caller lets go of its own lock
/// first, because a lock that denies reading would refuse its own check.
///
/// * Windows: the open shares reading and writing (`FILE_SHARE_READ | FILE_SHARE_WRITE`), so only
///   a program that denies reading, or a missing permission, refuses it.
/// * macOS and Linux: a plain open for reading; the advisory locks are not looked at (they never
///   refuse a read).
///
/// Only a regular file is probed (the metadata decides, following symbolic links). Opening a FIFO
/// for reading blocks until a writer comes, and the caller is the UI thread; `open_file_with`
/// refuses what is not a regular file by the same metadata, with its own message.
pub fn check_readable(path: &Path) -> io::Result<()> {
    if std::fs::metadata(path).is_ok_and(|meta| !meta.is_file()) {
        return Ok(());
    }
    #[cfg(windows)]
    let file = {
        use std::os::windows::fs::OpenOptionsExt as _;
        File::options().read(true).share_mode(1 | 2).open(path)
    };
    #[cfg(not(windows))]
    let file = File::open(path);
    file.map(drop)
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
    let file = File::open(path)?;
    try_lock_retrying(&file, LOCK_RETRY_FOR, |file| match mode {
        ShareMode::DenyWrite => file.try_lock_shared(),
        ShareMode::DenyReadWrite => file.try_lock(),
    })?;
    Ok(file)
}

/// How long a refused `flock` is tried again before it is reported.
#[cfg(not(windows))]
const LOCK_RETRY_FOR: std::time::Duration = std::time::Duration::from_millis(150);

/// The pause between two tries.
#[cfg(not(windows))]
const LOCK_RETRY_EVERY: std::time::Duration = std::time::Duration::from_millis(10);

/// Calls `try_lock` and, while it answers `WouldBlock`, again every [`LOCK_RETRY_EVERY`] for up to
/// `within`, then reports `WouldBlock`.
///
/// A `flock` belongs to the open file description, and a child that this process starts on another
/// thread (a plugin host, the shell of a terminal tab, an external command) holds a copy of every
/// descriptor from the fork until its exec closes the `O_CLOEXEC` ones. A lock this program has just
/// let go of is therefore still held for that short window, and would be taken for another
/// program's. A lock that another program really holds is still reported, only `within` later.
#[cfg(not(windows))]
fn try_lock_retrying(
    file: &File,
    within: std::time::Duration,
    try_lock: impl Fn(&File) -> Result<(), std::fs::TryLockError>,
) -> io::Result<()> {
    use std::fs::TryLockError;
    let start = std::time::Instant::now();
    loop {
        match try_lock(file) {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) => {
                if start.elapsed() >= within {
                    return Err(io::Error::from(io::ErrorKind::WouldBlock));
                }
                std::thread::sleep(LOCK_RETRY_EVERY);
            }
            Err(TryLockError::Error(error)) => return Err(error),
        }
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
    fn a_free_file_can_be_read() {
        let (_dir, path) = file();
        check_readable(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "abc");
    }

    /// A FIFO with no writer must not be opened (it would block for ever), also through a link.
    #[cfg(unix)]
    #[test]
    fn a_fifo_is_not_probed() {
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("pipe");
        let made = std::process::Command::new("mkfifo").arg(&fifo).status();
        if !made.is_ok_and(|status| status.success()) {
            return; // no mkfifo here
        }
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&fifo, &link).unwrap();
        for path in [fifo, link] {
            let (tx, rx) = std::sync::mpsc::channel();
            let probed = path.clone();
            // If the probe blocked, this thread would stay; the test gives up after a bound.
            std::thread::spawn(move || tx.send(check_readable(&probed).is_ok()).ok());
            let answer = rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("the probe of a FIFO blocked");
            assert!(answer, "{}", path.display());
        }
    }

    #[test]
    fn a_file_that_is_not_there_cannot_be_read() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check_readable(&dir.path().join("missing")).is_err());
    }

    /// A file without the permission to read (a program that runs as root reads it anyway).
    #[cfg(unix)]
    #[test]
    fn a_file_without_read_permission_cannot_be_read() {
        use std::os::unix::fs::PermissionsExt as _;
        let (_dir, path) = file();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        if File::open(&path).is_ok() {
            return; // root: the permission does not apply
        }
        assert!(check_readable(&path).is_err());
    }

    /// The lock of another program that denies reading refuses the check on Windows only.
    #[cfg(windows)]
    #[test]
    fn windows_refuses_the_check_while_another_lock_denies_reading() {
        let (_dir, path) = file();
        let lock = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        assert!(check_readable(&path).is_err());
        drop(lock);
        check_readable(&path).unwrap();
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

    /// A lock held through another open file description (what a forked child keeps for a moment)
    /// that is let go of within the bound does not refuse the check, nor a new lock.
    #[cfg(not(windows))]
    #[test]
    fn a_lock_let_go_of_soon_does_not_refuse_the_check() {
        let (_dir, path) = file();
        let lock = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(30));
            drop(lock);
        });
        check_writable(&path).unwrap();
        releaser.join().unwrap();
        let lock = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(30));
            drop(lock);
        });
        FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        releaser.join().unwrap();
    }

    /// A lock held longer than the bound is reported as `WouldBlock`, after the wait.
    #[cfg(not(windows))]
    #[test]
    fn a_lock_held_longer_than_the_bound_refuses_the_check() {
        let (_dir, path) = file();
        let _lock = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
        let start = std::time::Instant::now();
        let error = check_writable(&path).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert!(start.elapsed() >= LOCK_RETRY_FOR);
        let error = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
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
