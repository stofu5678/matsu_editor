//! Starting a shell on Windows with `ShellExecuteW`, as Sakura does
//! (`sakura_core/cmd/CViewCommander_File.cpp`).

// The Win32 call is `unsafe`. This module, `print::windows` and `print::mac` are the only places
// of the program that need it (ADR-0013).
#![allow(unsafe_code)]

use std::ptr::null;

use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::{ShellError, WindowsLaunch};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Calls `ShellExecuteW`. It returns a value above 32 when it worked.
pub(super) fn run(launch: &WindowsLaunch) -> Result<(), ShellError> {
    let verb = wide(launch.verb);
    let file = wide(&launch.file);
    let parameters = launch.parameters.as_deref().map(wide);
    let directory = wide(&launch.directory);
    // SAFETY: every pointer is to a NUL-terminated UTF-16 string that lives until the call returns
    // (or null for "none"), and the call keeps none of them.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            parameters.as_ref().map_or(null(), |p| p.as_ptr()),
            directory.as_ptr(),
            SW_SHOWNORMAL,
        )
    };
    if result as usize > 32 {
        Ok(())
    } else {
        Err(ShellError::Failed(format!(
            "ShellExecute failed ({})",
            result as usize
        )))
    }
}
