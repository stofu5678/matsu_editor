//! `SystemParametersInfo(SPI_GETCARETWIDTH)`, as `CCaret::ShowEditCaret` calls it.

// The Win32 call is `unsafe`. See the list of the modules that need it in `lib.rs`.
#![allow(unsafe_code)]

use std::ffi::c_void;

use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETCARETWIDTH, SystemParametersInfoW};

/// The system caret width in pixels, or `None` when the call fails.
pub(super) fn caret_width() -> Option<u32> {
    let mut width: u32 = 0;
    // SAFETY: `SPI_GETCARETWIDTH` writes one `DWORD` through `pvParam`, which points to `width`.
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCARETWIDTH,
            0,
            std::ptr::from_mut(&mut width).cast::<c_void>(),
            0,
        )
    };
    (ok != 0).then_some(width)
}
