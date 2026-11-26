//! `LoadLibraryExW`, `GetProcAddress` and `FreeLibrary`.

// The Win32 calls are `unsafe`. See the list of the modules that need it in `lib.rs`.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_void};
use std::os::windows::ffi::OsStrExt as _;
use std::path::Path;

use windows_sys::Win32::Foundation::{FreeLibrary, GetLastError, HMODULE};
use windows_sys::Win32::Globalization::{
    CP_ACP, GetACP, WC_NO_BEST_FIT_CHARS, WideCharToMultiByte,
};
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_WITH_ALTERED_SEARCH_PATH, LoadLibraryExW,
};

/// A library loaded by `LoadLibraryExW`, unloaded by `FreeLibrary` when it is dropped.
pub(super) struct Library(HMODULE);

impl Library {
    /// Loads the library at `path`, which is absolute. `LOAD_WITH_ALTERED_SEARCH_PATH` looks for
    /// the DLLs the library needs in its own folder first, and not in the folder of the program.
    pub(super) fn open(path: &Path) -> Result<Self, String> {
        debug_assert!(path.is_absolute());
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: a NUL-terminated full path that lives until the call returns, and no file
        // handle (it must be null).
        let module = unsafe {
            LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        };
        if module.is_null() {
            // SAFETY: reads the error code of this thread.
            let code = unsafe { GetLastError() };
            return Err(format!("LoadLibraryExW failed with the error {code}"));
        }
        Ok(Self(module))
    }

    pub(super) fn symbol(&self, name: &CStr) -> Option<*const c_void> {
        // SAFETY: a live module and a NUL-terminated name.
        let address = unsafe { GetProcAddress(self.0, name.as_ptr().cast()) };
        address.map(|function| function as *const c_void)
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: the module is freed once.
        unsafe { FreeLibrary(self.0) };
    }
}

/// `CP_UTF8`: the system code page when the user turned on UTF-8 for the whole system.
const CP_UTF8: u32 = 65001;

/// The path in the system code page, which is what the narrow file functions of the C runtime
/// read. `None` when a character is not in the code page (the conversion would put a `?` or a
/// look-alike in its place and the library would open another file or none).
pub(super) fn narrow_path(path: &Path) -> Option<Vec<u8>> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() {
        return Some(Vec::new());
    }
    let length = i32::try_from(wide.len()).ok()?;
    // SAFETY: reads the code page of the system.
    if unsafe { GetACP() } == CP_UTF8 {
        // The conversion with a "used default" flag is refused for UTF-8, and nothing is lost.
        return String::from_utf16(&wide).ok().map(String::into_bytes);
    }
    let mut used_default: i32 = 0;
    // SAFETY: a buffer of `length` units that lives until the call returns; with no output
    // buffer, the call only measures.
    let size = unsafe {
        WideCharToMultiByte(
            CP_ACP,
            WC_NO_BEST_FIT_CHARS,
            wide.as_ptr(),
            length,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            &mut used_default,
        )
    };
    let size = usize::try_from(size).ok().filter(|size| *size > 0)?;
    let mut bytes = vec![0u8; size];
    used_default = 0;
    // SAFETY: the buffers live until the call returns, and `bytes` has the size measured above.
    let written = unsafe {
        WideCharToMultiByte(
            CP_ACP,
            WC_NO_BEST_FIT_CHARS,
            wide.as_ptr(),
            length,
            bytes.as_mut_ptr(),
            i32::try_from(size).ok()?,
            std::ptr::null(),
            &mut used_default,
        )
    };
    (written > 0 && used_default == 0).then_some(bytes)
}
