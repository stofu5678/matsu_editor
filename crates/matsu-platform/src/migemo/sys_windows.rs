//! `LoadLibraryExW`, `GetProcAddress` and `FreeLibrary`.

// The Win32 calls are `unsafe`. See the list of the modules that need it in `lib.rs`.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_void};
use std::os::windows::ffi::OsStrExt as _;
use std::path::Path;

use windows_sys::Win32::Foundation::{FreeLibrary, GetLastError, HMODULE};
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
