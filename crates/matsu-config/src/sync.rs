//! Flushing a file to the disk before it replaces another (ADR-0005).
//!
//! This is a private copy of `crates/matsu-core/src/file/sync.rs`: `matsu-config` does not depend on
//! `matsu-core`, and a new dependency edge is not worth one function. Change both together.
//!
//! On Apple targets Rust's [`File::sync_all`] is `fcntl(fd, F_FULLFSYNC)` with no fallback. Not
//! every file system supports that command: virtiofs (the shared folders of a macOS VM) answers
//! `ENOTTY`, and network file systems (SMB, NFS, AFP) are expected to refuse it the same way (not
//! verified here). Plain `fsync(2)` works on them, so [`sync_file`] falls back to it. SQLite does
//! the same (`full_fsync` in `os_unix.c`). Other errors, such as `EIO`, are real write failures and
//! are returned as they are.

use std::fs::File;
use std::io;

/// Writes the content of `file` to the disk. See the module comment for the macOS fallback.
pub(crate) fn sync_file(file: &File) -> io::Result<()> {
    sync_with_fallback(|| file.sync_all(), || fsync(file))
}

/// Does `full`; when it fails with an error that means "this file system has no full sync", does
/// `plain` and returns its result. Split out so that the decision can be tested on every OS.
#[cfg_attr(not(target_vendor = "apple"), allow(dead_code))]
fn sync_with_fallback(
    full: impl FnOnce() -> io::Result<()>,
    plain: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    match full() {
        Err(e) if is_full_sync_unsupported(&e) => plain(),
        other => other,
    }
}

/// The errors `F_FULLFSYNC` gives when the file system does not know the command. These are the
/// values on Apple targets (checked against `libc` by a test there).
///
/// * `ENOTTY` (25): measured on virtiofs; the usual answer of `fcntl`/`ioctl` to an unknown command.
/// * `ENOTSUP` (45) and `EOPNOTSUPP` (102): the file system says it cannot. They differ on Apple.
/// * `EINVAL` (22): some file systems reject an unknown `fcntl` command this way.
#[cfg_attr(not(target_vendor = "apple"), allow(dead_code))]
fn is_full_sync_unsupported(err: &io::Error) -> bool {
    matches!(err.raw_os_error(), Some(25 | 45 | 102 | 22))
}

#[cfg(target_vendor = "apple")]
#[allow(unsafe_code)]
fn fsync(file: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: the descriptor is open for as long as `file` is borrowed.
    if unsafe { libc::fsync(file.as_raw_fd()) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(target_vendor = "apple"))]
fn fsync(file: &File) -> io::Result<()> {
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn run(errno: Option<i32>) -> (io::Result<()>, bool) {
        let plain_called = Cell::new(false);
        let result = sync_with_fallback(
            || errno.map_or(Ok(()), |n| Err(io::Error::from_raw_os_error(n))),
            || {
                plain_called.set(true);
                Ok(())
            },
        );
        (result, plain_called.get())
    }

    #[test]
    fn success_does_not_fall_back() {
        assert!(!run(None).1);
    }

    #[test]
    fn unsupported_errors_fall_back_to_plain_fsync() {
        for errno in [25, 45, 102, 22] {
            let (result, plain_called) = run(Some(errno));
            assert!(result.is_ok(), "errno {errno}");
            assert!(plain_called, "errno {errno}");
        }
    }

    #[test]
    fn other_errors_are_returned() {
        // EIO (5), ENOSPC (28), EBADF (9), EDQUOT-ish and a missing errno.
        for errno in [5, 28, 9] {
            let (result, plain_called) = run(Some(errno));
            assert_eq!(result.unwrap_err().raw_os_error(), Some(errno));
            assert!(!plain_called);
        }
        let called = Cell::new(false);
        let r = sync_with_fallback(
            || Err(io::Error::other("x")),
            || {
                called.set(true);
                Ok(())
            },
        );
        assert!(r.is_err() && !called.get());
    }

    #[test]
    fn a_failing_plain_fsync_is_returned() {
        let r = sync_with_fallback(
            || Err(io::Error::from_raw_os_error(25)),
            || Err(io::Error::from_raw_os_error(5)),
        );
        assert_eq!(r.unwrap_err().raw_os_error(), Some(5));
    }

    #[test]
    fn syncs_a_real_file() {
        let path = std::env::temp_dir().join(format!("matsu-config-sync-{}", std::process::id()));
        let file = File::create(&path).unwrap();
        let result = sync_file(&file);
        let _ = std::fs::remove_file(&path);
        result.unwrap();
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn the_numbers_are_the_libc_constants() {
        assert_eq!(libc::ENOTTY, 25);
        assert_eq!(libc::ENOTSUP, 45);
        assert_eq!(libc::EOPNOTSUPP, 102);
        assert_eq!(libc::EINVAL, 22);
    }
}
