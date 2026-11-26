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
use windows_sys::Win32::System::Diagnostics::Debug::{
    SEM_FAILCRITICALERRORS, SEM_NOOPENFILEERRORBOX, SetThreadErrorMode,
};
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LoadLibraryExW,
};

/// While it lives, the thread shows no box for a load that fails (the "Bad Image" box of the
/// loader for a file that is not a library for this machine waits for a click, and the load
/// call with it). The old mode comes back when it is dropped, on every path out of the load.
struct QuietLoadErrors(u32);

impl QuietLoadErrors {
    fn new() -> Self {
        let mut old = 0u32;
        // SAFETY: only this thread's error mode is read and set; `old` lives through the call.
        unsafe {
            SetThreadErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX, &mut old);
        }
        Self(old)
    }
}

impl Drop for QuietLoadErrors {
    fn drop(&mut self) {
        // SAFETY: puts back the mode this thread had; the old mode is not asked for again.
        unsafe { SetThreadErrorMode(self.0, std::ptr::null_mut()) };
    }
}

/// Whether the system code page is UTF-8 (65001).
pub(super) fn system_code_page_is_utf8() -> bool {
    // SAFETY: reads the code page of the system.
    unsafe { GetACP() == CP_UTF8 }
}

/// A library loaded by `LoadLibraryExW`, unloaded by `FreeLibrary` when it is dropped.
pub(super) struct Library(HMODULE);

impl Library {
    /// Loads the library at `path`, which is absolute. The DLLs that the library itself needs
    /// are looked for in its own folder (`LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR`), then in the folder
    /// of the program, the system folder and the folders added with `AddDllDirectory`
    /// (`LOAD_LIBRARY_SEARCH_DEFAULT_DIRS`). The current folder and `PATH` are not searched, so a
    /// DLL in the folder where MATSU Editor was started is not loaded in their place.
    /// (`LOAD_WITH_ALTERED_SEARCH_PATH` would search them.)
    pub(super) fn open(path: &Path) -> Result<Self, String> {
        debug_assert!(path.is_absolute());
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let quiet = QuietLoadErrors::new();
        // SAFETY: a NUL-terminated full path that lives until the call returns, and no file
        // handle (it must be null).
        let module = unsafe {
            LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            )
        };
        // The error code is read first: the call that puts the error mode back may change it.
        let code = if module.is_null() {
            // SAFETY: reads the error code of this thread.
            unsafe { GetLastError() }
        } else {
            0
        };
        drop(quiet);
        if module.is_null() {
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
    if system_code_page_is_utf8() {
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
