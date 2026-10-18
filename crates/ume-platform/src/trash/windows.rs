//! `SHFileOperationW` with the flag for undo, as `CBackupAgent::MakeBackUp` calls it. Not run by CI;
//! the user checks it.

// `SHFileOperationW` is `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::UI::Shell::{
    FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW,
    SHFileOperationW,
};

pub(super) fn move_to_trash(path: &Path) -> io::Result<()> {
    // `pFrom` is a list of names that ends with two NULs.
    let mut from: Vec<u16> = std::path::absolute(path)?
        .as_os_str()
        .encode_wide()
        .collect();
    from.push(0);
    from.push(0);
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: from.as_ptr(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT) as _,
        ..Default::default()
    };
    // SAFETY: `operation` is filled in and `from` lives through the call.
    let code = unsafe { SHFileOperationW(&mut operation) };
    if code == 0 && operation.fAnyOperationsAborted == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!("SHFileOperation failed ({code})")))
    }
}
