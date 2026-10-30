//! The hook of `matsu_platform::dialog_title` on Windows: a window of the dialog class (`#32770`, the class
//! of a TaskDialog) that is shown with one of the words of GPUI as its title gets the title of the
//! application; one with another title is left alone. No TaskDialog is made: GPUI's box is only a
//! window of that class with that title, and the hook looks at nothing else.
#![cfg(windows)]
#![allow(unsafe_code)]

use std::ptr::null;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, GetWindowTextW, MSG, PM_REMOVE, PeekMessageW,
    SW_SHOWNOACTIVATE, ShowWindow, TranslateMessage, WS_OVERLAPPED,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn dialog_window(title: &str) -> HWND {
    let class = wide("#32770");
    let title = wide(title);
    // SAFETY: NUL-terminated UTF-16 strings that live until the call returns; no parent, no menu.
    unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            200,
            100,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            null(),
        )
    }
}

fn title_of(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    // SAFETY: the buffer is valid for 256 units.
    let len = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), 256) };
    String::from_utf16_lossy(&buffer[..usize::try_from(len).unwrap_or(0)])
}

/// Runs the message loop of this thread for a while: the hook is called from it.
fn pump(until: impl Fn() -> bool) {
    let begun = Instant::now();
    while begun.elapsed() < Duration::from_secs(5) && !until() {
        // SAFETY: `msg` is a plain struct that `PeekMessageW` fills.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_box_of_gpui_gets_the_title_of_the_application() {
    matsu_platform::dialog_title::install();
    // a second call changes nothing
    matsu_platform::dialog_title::install();
    let new = matsu_platform::dialog_title::app_title();

    let boxes: Vec<HWND> = ["Info", "Warning", "Critical", "ファイルを開く"]
        .into_iter()
        .map(dialog_window)
        .collect();
    assert!(boxes.iter().all(|h| !h.is_null()));
    for &hwnd in &boxes {
        // SAFETY: a window of this thread.
        unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    }
    pump(|| boxes[..3].iter().all(|&h| title_of(h) == new));

    for &hwnd in &boxes[..3] {
        assert_eq!(title_of(hwnd), new);
    }
    // another title is left alone
    assert_eq!(title_of(boxes[3]), "ファイルを開く");
    for hwnd in boxes {
        // SAFETY: a window of this thread.
        unsafe { DestroyWindow(hwnd) };
    }
}
