//! `GetFullPathNameW` and `GetLongPathNameW`, in the order of `GetLongFileName`.

// The two calls are `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};

use windows_sys::Win32::Storage::FileSystem::{GetFullPathNameW, GetLongPathNameW};

/// `MAX_PATH`: Sakura's buffers have this length, and a longer result is a failure there.
const MAX_PATH: usize = 260;

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

fn text(buffer: &[u16]) -> String {
    OsString::from_wide(buffer).to_string_lossy().into_owned()
}

/// `GetFullPathName` (without the NUL), or `None` when it fails or the result does not fit
/// `MAX_PATH`.
fn full(path: &[u16]) -> Option<Vec<u16>> {
    let mut buffer = vec![0u16; MAX_PATH + 1];
    // SAFETY: `path` is NUL-terminated, and `buffer` holds the length that is passed; the
    // output pointer for the file part is optional and left null.
    let len = unsafe {
        GetFullPathNameW(
            path.as_ptr(),
            MAX_PATH as u32,
            buffer.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    } as usize;
    if len == 0 || len >= MAX_PATH {
        return None;
    }
    buffer.truncate(len);
    Some(buffer)
}

/// `GetLongPathName` (without the NUL) of a NUL-terminated path, or `None` when it fails or the
/// result does not fit `MAX_PATH`.
fn long(path: &[u16]) -> Option<Vec<u16>> {
    let mut buffer = vec![0u16; MAX_PATH + 1];
    // SAFETY: `path` is NUL-terminated, and `buffer` holds the length that is passed.
    let len =
        unsafe { GetLongPathNameW(path.as_ptr(), buffer.as_mut_ptr(), MAX_PATH as u32) } as usize;
    if len == 0 || len > MAX_PATH {
        return None;
    }
    buffer.truncate(len);
    Some(buffer)
}

pub(super) fn long_path(path: &str) -> Option<String> {
    let source = wide(path);
    match full(&source) {
        // `GetFullPathName` failed: the long name of the path as given, or a failure.
        None => long(&source).map(|buffer| text(&buffer)),
        // The long name of the full path, or the full path when the file is not there.
        Some(mut full) => {
            // The NUL goes in place for the call; the text is read without it.
            full.push(0);
            Some(long(&full).map_or_else(|| text(&full[..full.len() - 1]), |buffer| text(&buffer)))
        }
    }
}
