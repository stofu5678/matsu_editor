//! `HtmlHelp` with `HH_KEYWORD_LOOKUP` on Windows, as Sakura calls it (`OpenHtmlHelp` in
//! `sakura_core/util/shell.cpp`, `sakura_core/cmd/CViewCommander_Support.cpp`).
//!
//! The function is in `hhctrl.ocx`, which has no import library in `windows-sys`, so the library
//! is loaded and the function looked up by name, as Sakura's `CHtmlHelp` does.

// The Win32 calls are `unsafe`. See the list of the modules that need it in `lib.rs`.
#![allow(unsafe_code)]

use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::UI::WindowsAndMessaging::{BringWindowToTop, GetDesktopWindow};

use super::HelpError;

/// `HH_KEYWORD_LOOKUP`
const HH_KEYWORD_LOOKUP: u32 = 0x000D;

/// `HH_AKLINK`
#[repr(C)]
struct AkLink {
    cb_struct: i32,
    f_reserved: i32,
    psz_keywords: *const u16,
    psz_url: *const u16,
    psz_msg_text: *const u16,
    psz_msg_title: *const u16,
    psz_window: *const u16,
    f_index_on_fail: i32,
}

/// `HtmlHelpW( hwndCaller, pszFile, uCommand, dwData )`
type HtmlHelpW = unsafe extern "system" fn(HWND, *const u16, u32, usize) -> HWND;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub(super) fn keyword_lookup(file: &Path, keyword: &str, single: bool) -> Result<(), HelpError> {
    let library = wide("hhctrl.ocx");
    // SAFETY: a NUL-terminated UTF-16 string that lives until the call returns. The library stays
    // loaded: the viewer window it makes needs it.
    let module = unsafe { LoadLibraryW(library.as_ptr()) };
    if module.is_null() {
        return Err(HelpError::Failed("hhctrl.ocx cannot be loaded".to_owned()));
    }
    // SAFETY: a NUL-terminated name.
    let Some(function) = (unsafe { GetProcAddress(module, c"HtmlHelpW".as_ptr().cast()) }) else {
        return Err(HelpError::Failed("HtmlHelpW is not there".to_owned()));
    };
    // SAFETY: `HtmlHelpW` has this signature (`htmlhelp.h`).
    let html_help: HtmlHelpW = unsafe { std::mem::transmute(function) };
    let keywords = wide(keyword);
    let path = wide(&file.to_string_lossy());
    let link = AkLink {
        cb_struct: i32::try_from(size_of::<AkLink>()).unwrap_or(0),
        f_reserved: 0,
        psz_keywords: keywords.as_ptr(),
        psz_url: null(),
        psz_msg_text: null(),
        psz_msg_title: null(),
        psz_window: null(),
        // The index is shown when the keyword is not there.
        f_index_on_fail: 1,
    };
    // One viewer for all: the desktop is the owner every call has; else each call is its own.
    let caller = if single {
        // SAFETY: no arguments.
        unsafe { GetDesktopWindow() }
    } else {
        null_mut()
    };
    // SAFETY: the strings and the structure live until the call returns, and the viewer copies
    // what it keeps.
    let window = unsafe {
        html_help(
            caller,
            path.as_ptr(),
            HH_KEYWORD_LOOKUP,
            std::ptr::from_ref(&link) as usize,
        )
    };
    if window.is_null() {
        return Err(HelpError::Failed("HtmlHelp returned no window".to_owned()));
    }
    // SAFETY: a window handle that `HtmlHelp` returned.
    unsafe { BringWindowToTop(window) };
    Ok(())
}
