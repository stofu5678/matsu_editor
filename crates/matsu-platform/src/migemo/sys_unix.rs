//! `dlopen`, `dlsym` and `dlclose`.

// The calls are `unsafe`. See the list of the modules that need it in `lib.rs`.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_void};
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;

/// A library loaded by `dlopen`, unloaded by `dlclose` when it is dropped.
pub(super) struct Library(*mut c_void);

impl Library {
    /// Loads the library at `path`, which is absolute: a name with a `/` is not searched for.
    pub(super) fn open(path: &Path) -> Result<Self, String> {
        debug_assert!(path.is_absolute());
        let name = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| "the path has a NUL".to_owned())?;
        // SAFETY: a NUL-terminated path that lives until the call returns.
        let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        if handle.is_null() {
            return Err(last_error());
        }
        Ok(Self(handle))
    }

    pub(super) fn symbol(&self, name: &CStr) -> Option<*const c_void> {
        // SAFETY: a live handle and a NUL-terminated name.
        let address = unsafe { libc::dlsym(self.0, name.as_ptr()) };
        (!address.is_null()).then_some(address.cast_const())
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: the handle is closed once.
        unsafe { libc::dlclose(self.0) };
    }
}

/// `dlerror`: the message of the last failure.
fn last_error() -> String {
    // SAFETY: `dlerror` gives NULL or a NUL-terminated string that is valid until the next call
    // of `dl*`; it is copied here.
    let message = unsafe { libc::dlerror() };
    if message.is_null() {
        return "the library cannot be loaded".to_owned();
    }
    // SAFETY: not NULL, see above.
    unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned()
}
