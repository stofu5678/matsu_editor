//! `statvfs`.

// `statvfs` is `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::ffi::CString;
use std::io;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub(super) fn available(dir: &Path) -> io::Result<u64> {
    let c_path = CString::new(dir.as_os_str().as_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `c_path` is a NUL-terminated string and `stat` is a buffer for one `statvfs`.
    let result = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `statvfs` filled it when it returned 0.
    let stat = unsafe { stat.assume_init() };
    #[allow(clippy::unnecessary_cast)]
    let (blocks, size) = (stat.f_bavail as u64, stat.f_frsize as u64);
    Ok(blocks.saturating_mul(size))
}
