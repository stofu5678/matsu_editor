//! Positional reads, the only OS-specific part of [`super::RangedFile`].
//!
//! `read_at` (Unix) and `seek_read` (Windows) read at an offset without a shared cursor, so
//! several threads can read one open file. Both are in the standard library: no `unsafe`.

use std::fs::File;
use std::io;

/// Reads into `buf` from `offset`, like `read`: it may return less than `buf.len()`, and 0 at the
/// end of the file. Retries when interrupted.
pub(super) fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    loop {
        match read_at_once(file, buf, offset) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            other => return other,
        }
    }
}

#[cfg(unix)]
fn read_at_once(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buf, offset)
}

#[cfg(windows)]
fn read_at_once(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buf, offset)
}

#[cfg(not(any(unix, windows)))]
fn read_at_once(_file: &File, _buf: &mut [u8], _offset: u64) -> io::Result<usize> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "positional reads are not supported on this platform",
    ))
}
