//! The lock on the folder `unsaved/` (ADR-0022, section 8): the process that holds it is the only one that
//! writes or restores.
//!
//! It is `File::try_lock` on a file `lock` that is never deleted: an advisory `flock` on macOS and Linux,
//! `LockFileEx` on Windows. The OS releases it when the process ends, also when it crashes, so no lock is left
//! behind. Deleting the file while it is held would let another process create a new one and lock it, so
//! [`super::Store::clear`] keeps it.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

/// A held lock; dropping it lets go.
#[derive(Debug)]
pub(super) struct DirLock {
    _file: File,
}

/// Why the lock was not taken.
#[derive(Debug)]
pub(super) enum LockFailure {
    /// Another process (or another store in this process) holds it.
    Held,
    /// The file system refused: the file cannot be made, or it has no locks (some network drives).
    Io(io::Error),
}

impl DirLock {
    pub(super) fn acquire(path: &Path) -> Result<Self, LockFailure> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options.open(path).map_err(LockFailure::Io)?;
        match file.try_lock() {
            Ok(()) => Ok(DirLock { _file: file }),
            Err(TryLockError::WouldBlock) => Err(LockFailure::Held),
            Err(TryLockError::Error(e)) => Err(LockFailure::Io(e)),
        }
    }
}
