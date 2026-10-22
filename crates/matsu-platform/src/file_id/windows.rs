//! The serial number of the volume and the index of the file. Not run by CI on the machine of the
//! author; the user checks it.

// `GetFileInformationByHandle` is `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::fs::File;
use std::os::windows::fs::OpenOptionsExt as _;
use std::os::windows::io::AsRawHandle as _;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
};

/// `FILE_READ_ATTRIBUTES`, and `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`.
const FILE_READ_ATTRIBUTES: u32 = 0x80;
const SHARE_ALL: u32 = 7;

/// `(volume serial number, file index)` of a file. The file is opened for its attributes only, so
/// that a file this editor has locked against reading (`file_lock`) can be asked too.
fn identity(path: &Path) -> Option<(u32, u64)> {
    let file = File::options()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(SHARE_ALL)
        .open(path)
        .ok()?;
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
