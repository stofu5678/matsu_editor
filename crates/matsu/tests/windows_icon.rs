//! The built `matsu.exe` has the icon as resource 1 (`crates/matsu/build.rs`, `matsu.rc`). GPUI's Windows
//! backend loads the icon with ID 1 from the executable for its windows (gpui-pre-windows,
//! `platform.rs` `load_icon`), so without it the windows, Explorer and the taskbar show the default
//! icon. The test opens the file as data (no window, no program started).
#![cfg(windows)]
// Win32 calls (the resource functions of the loader).
#![allow(unsafe_code)]

use std::os::windows::ffi::OsStrExt;

use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{
    FindResourceW, LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW, SizeofResource,
};
use windows::Win32::UI::WindowsAndMessaging::RT_GROUP_ICON;
use windows::core::PCWSTR;

/// The ICON resource ID of `MAKEINTRESOURCE(id)`.
fn int_resource(id: u16) -> PCWSTR {
    PCWSTR(usize::from(id) as *const u16)
}

#[test]
fn matsu_exe_has_the_icon_resource_with_id_1() {
    let exe = env!("CARGO_BIN_EXE_matsu");
    let wide: Vec<u16> = std::ffi::OsStr::new(exe)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `wide` is a NUL-terminated path; the module is freed before the test ends.
    unsafe {
        let module: HMODULE = LoadLibraryExW(PCWSTR(wide.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE)
            .expect("matsu.exe opens as a data file");
        let group = FindResourceW(Some(module), int_resource(1), RT_GROUP_ICON);
        let group_size = if group.is_invalid() {
            0
        } else {
            SizeofResource(Some(module), group)
        };
        FreeLibrary(module).expect("the module is freed");
        // The group header is 6 bytes and every picture has a 14 byte entry: six pictures
        // (16, 32, 48, 64, 128 and 256 px) in assets/icons/matsu_editor.ico.
        assert_eq!(group_size, 6 + 6 * 14, "the icon group with ID 1");
    }
}
