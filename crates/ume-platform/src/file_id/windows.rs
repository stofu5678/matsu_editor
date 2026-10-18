//! The serial number of the volume and the index of the file. Not run by CI on the machine of the
//! author; the user checks it.

// `GetFileInformationByHandle` is `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::fs::File;
use std::os::windows::io::AsRawHandle as _;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
};

/// `(volume serial number, file index)` of a file.
fn identity(path: &Path) -> Option<(u32, u64)> {
    let file = File::open(path).ok()?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the handle is open for the call and `info` is a buffer for one structure.
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle().cast(), &mut info) };
    (ok != 0).then(|| {
        (
            info.dwVolumeSerialNumber,
            (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        )
    })
}

pub(super) fn same_file(a: &Path, b: &Path) -> bool {
    match (identity(a), identity(b)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}
