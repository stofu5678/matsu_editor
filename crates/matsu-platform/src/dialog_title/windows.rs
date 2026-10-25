//! The hook that renames the boxes of GPUI (see the parent module).
//!
//! `SetWinEventHook` with `EVENT_OBJECT_SHOW` and `WINEVENT_OUTOFCONTEXT` calls [`on_event`] on the thread
//! that installed the hook (it needs the message loop of that thread, which the application's main
//! thread has) for each window of this process that is shown. GPUI shows its box on a thread of its own
//! (`dialog.rs` of `gpui-pre-windows`); the title is changed with `WM_SETTEXT` sent with a time limit,
//! so that a box whose thread does not answer cannot stop the main thread.

// The Win32 calls are `unsafe`. See the list of the modules that need it in `lib.rs`.
#![allow(unsafe_code)]

use std::sync::OnceLock;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EVENT_OBJECT_SHOW, GetClassNameW, GetWindowTextW, OBJID_WINDOW, SMTO_ABORTIFHUNG,
    SendMessageTimeoutW, WINEVENT_OUTOFCONTEXT, WM_SETTEXT,
};

use super::replacement;

/// The title, in UTF-16 with the end NUL.
static TITLE: OnceLock<(String, Vec<u16>)> = OnceLock::new();

/// How long the box has to take the new title (ms).
const TIMEOUT_MS: u32 = 1000;

pub(super) fn install(title: String) {
    let wide: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    if TITLE.set((title, wide)).is_err() {
        return;
    }
    // SAFETY: `on_event` is a function with the signature `WINEVENTPROC` that lives for the whole
    // process. The hook is for the events of this process only (`idProcess`), from any thread of it. It
    // is never removed: it ends with the process.
    unsafe {
        SetWinEventHook(
            EVENT_OBJECT_SHOW,
            EVENT_OBJECT_SHOW,
            std::ptr::null_mut(),
            Some(on_event),
            GetCurrentProcessId(),
            0,
            WINEVENT_OUTOFCONTEXT,
        );
    }
}

/// The text of a window as the system gives it (`GetClassNameW`, `GetWindowTextW`).
fn read(hwnd: HWND, f: unsafe extern "system" fn(HWND, *mut u16, i32) -> i32) -> String {
    let mut buffer = [0u16; 256];
    // SAFETY: the buffer is valid for 256 units and the call writes at most that many, with the NUL.
    let len = unsafe { f(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..usize::try_from(len).unwrap_or(0)])
}

unsafe extern "system" fn on_event(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if id_object != OBJID_WINDOW || hwnd.is_null() {
        return;
    }
    let Some((title, wide)) = TITLE.get() else {
        return;
    };
    let class = read(hwnd, GetClassNameW);
    let current = read(hwnd, GetWindowTextW);
    if replacement(&class, &current, title).is_none() {
        return;
    }
    let mut result = 0usize;
    // SAFETY: `wide` is a NUL-terminated UTF-16 string that lives for the whole process (a static); the
    // call copies it before it returns.
    unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_SETTEXT,
            0,
            wide.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            TIMEOUT_MS,
            &mut result,
        );
    }
}
