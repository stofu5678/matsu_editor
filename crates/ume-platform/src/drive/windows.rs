//! `GetDriveType`, as `IsLocalDrive` of Sakura (`sakura_core/util/file.cpp`) uses it.

// `GetDriveTypeW` is `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_REMOTE: u32 = 4;
const DRIVE_CDROM: u32 = 5;

pub(super) fn is_local(path: &Path) -> bool {
    let text = path.to_string_lossy();
    let mut chars = text.chars();
    match chars.next() {
        Some(letter) if letter.is_alphabetic() => {
            let root: Vec<u16> = format!("{}:\\", letter.to_uppercase())
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            // SAFETY: `root` is a NUL-terminated wide string.
            let kind = unsafe { GetDriveTypeW(root.as_ptr()) };
            !matches!(kind, DRIVE_REMOVABLE | DRIVE_CDROM | DRIVE_REMOTE)
        }
        // A network path.
        Some('\\') if chars.next() == Some('\\') => false,
        _ => true,
    }
}
