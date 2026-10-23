//! The clipboard format `HTML Format` on Windows (`CClipboard::SetHtmlText`).

// The Win32 calls are `unsafe`. See the list of the modules that need it in `lib.rs`.
#![allow(unsafe_code)]

use std::ptr::{copy_nonoverlapping, null_mut};

use windows_sys::Win32::Foundation::GlobalFree;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};

/// Puts `data` (the bytes of the format, see [`super::cf_html`]) on the clipboard in `HTML
/// Format`, keeping what is there. The clipboard is opened for the task alone (no owner window),
/// as GPUI opens it to write the text.
pub(super) fn add_html(data: &[u8]) -> bool {
    let name: Vec<u16> = "HTML Format".encode_utf16().chain(Some(0)).collect();
    // SAFETY: a NUL-terminated UTF-16 string that lives until the call returns.
    let format = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
    if format == 0 {
        return false;
    }
    // SAFETY: no owner window; the clipboard is closed below on every path after it opened.
    if unsafe { OpenClipboard(null_mut()) } == 0 {
        return false;
    }
    let added = set_bytes(format, data);
    // SAFETY: the clipboard is open.
    unsafe { CloseClipboard() };
    added
}

/// Hands a moveable block with `data` and a NUL at the end to the open clipboard.
fn set_bytes(format: u32, data: &[u8]) -> bool {
    // SAFETY: a plain allocation; checked for null below.
    let block = unsafe { GlobalAlloc(GMEM_MOVEABLE, data.len() + 1) };
    if block.is_null() {
        return false;
    }
    // SAFETY: `block` is a moveable block of `data.len() + 1` bytes, locked while it is written.
    unsafe {
        let ptr = GlobalLock(block).cast::<u8>();
        if ptr.is_null() {
            GlobalFree(block);
            return false;
        }
        copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        ptr.add(data.len()).write(0);
        GlobalUnlock(block);
    }
    // SAFETY: the clipboard is open. On success it owns the block; otherwise it is ours to free.
    unsafe {
        if SetClipboardData(format, block).is_null() {
            GlobalFree(block);
            return false;
        }
    }
    true
}
