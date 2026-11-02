//! The icon of the document on Windows, as Sakura does (`CEditWnd::GetRelatedIcon` and
//! `SetWindowIcon`). Not run by CI, which has no desktop; the user checks it.

// The Win32 calls are `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::os::windows::ffi::OsStrExt as _;
use std::path::Path;
use std::sync::Mutex;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows_sys::Win32::UI::Shell::{
    SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SMALLICON, SHGFI_USEFILEATTRIBUTES,
    SHGetFileInfoW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GCLP_HICON, GCLP_HICONSM, GetClassLongPtrW, ICON_BIG, ICON_SMALL, IsWindow,
    SendMessageW, WM_SETICON,
};

use super::{Error, RawWindowHandle};

/// The icons this module made for a window (its handle, the big and the small one as numbers):
/// they are destroyed when the window gets another (`SetWindowIcon` destroys the one it
/// replaces), and are not the class icons, which belong to the window class.
static OWNED: Mutex<Vec<(isize, isize, isize)>> = Mutex::new(Vec::new());

fn hwnd(window: RawWindowHandle) -> Result<HWND, Error> {
    match window {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as HWND),
        _ => Err(Error::Failed("Win32 のウィンドウではない")),
    }
}

pub(super) fn unsupported() -> Option<&'static str> {
    None
}

/// The icon of the shell for the file's extension in one size, or 0.
fn shell_icon(path: &[u16], size: u32) -> isize {
    // SAFETY: `path` ends with a 0; `info` is a structure to write into, and the size says so.
    unsafe {
        let mut info: SHFILEINFOW = std::mem::zeroed();
        let found = SHGetFileInfoW(
            path.as_ptr(),
            FILE_ATTRIBUTE_NORMAL,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_USEFILEATTRIBUTES | size,
        );
        if found == 0 { 0 } else { info.hIcon as isize }
    }
}

pub(super) fn set(window: RawWindowHandle, file: Option<&Path>) -> Result<(), Error> {
    let hwnd = hwnd(window)?;
    let (big, small) = match file {
        Some(path) => {
            let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
            wide.push(0);
            (
                shell_icon(&wide, SHGFI_LARGEICON),
                shell_icon(&wide, SHGFI_SMALLICON),
            )
        }
        None => (0, 0),
    };
    // SAFETY: `hwnd` is a window of this process. The icons are handles this function made or
    // the ones of the window class; a handle that is 0 gives the window the icon of its class.
    unsafe {
        let class_big = GetClassLongPtrW(hwnd, GCLP_HICON) as isize;
        let class_small = GetClassLongPtrW(hwnd, GCLP_HICONSM) as isize;
        let new_big = if big != 0 { big } else { class_big };
        let new_small = if small != 0 { small } else { class_small };
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, new_big);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, new_small);
        let mut owned = OWNED.lock().unwrap_or_else(|e| e.into_inner());
        // The icons of this window are replaced, and those of the windows that are closed since
        // are let go (Windows does not destroy an icon that a program made when the window goes).
        let mut at = 0;
        while at < owned.len() {
            let (other, old_big, old_small) = owned[at];
            if other == hwnd as isize || IsWindow(other as HWND) == 0 {
                owned.remove(at);
                for old in [old_big, old_small] {
                    if old != 0 {
                        DestroyIcon(old as _);
                    }
                }
            } else {
                at += 1;
            }
        }
        if big != 0 || small != 0 {
            owned.push((hwnd as isize, big, small));
        }
    }
    Ok(())
}
