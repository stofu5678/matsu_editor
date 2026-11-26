//! The calls into the library of C/Migemo. The owner of the library handle and of the `migemo`
//! object is [`Migemo`]: its `Drop` closes the object and then unloads the library.

// The calls into a loaded library are `unsafe`. See the list of the modules that need it in
// `lib.rs`.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_uchar, c_uint, c_void};
use std::path::Path;
use std::ptr::NonNull;

#[cfg(unix)]
use super::sys_unix as sys;
#[cfg(windows)]
use super::sys_windows as sys;
use super::{decode_char, encode_char};

/// Why the library could not be opened.
#[derive(Debug, PartialEq, Eq)]
pub enum OpenError {
    /// The path is not absolute. The OS is not asked: it would look in the current folder and in
    /// `PATH`.
    NotAbsolute,
    /// The OS could not load the file (the text is its message).
    Load(String),
    /// The library does not have this function.
    MissingFunction(&'static str),
    /// `migemo_open` gave nothing.
    Init,
}

type OpenFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type CloseFn = unsafe extern "C" fn(*mut c_void);
type QueryFn = unsafe extern "C" fn(*mut c_void, *const c_uchar) -> *mut c_uchar;
type ReleaseFn = unsafe extern "C" fn(*mut c_void, *mut c_uchar);
type Char2Int = unsafe extern "C" fn(*const c_uchar, *mut c_uint) -> c_int;
type Int2Char = unsafe extern "C" fn(c_uint, *mut c_uchar) -> c_int;
type SetChar2IntFn = unsafe extern "C" fn(*mut c_void, Char2Int) -> c_int;
type SetInt2CharFn = unsafe extern "C" fn(*mut c_void, Int2Char) -> c_int;
type LoadFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char) -> c_int;
type IsEnableFn = unsafe extern "C" fn(*mut c_void) -> c_int;

/// `MIGEMO_DICTID_*` of `migemo.h`.
const DICT_MIGEMO: c_int = 1;
const DICT_ROMA2HIRA: c_int = 2;
const DICT_HIRA2KATA: c_int = 3;
const DICT_HAN2ZEN: c_int = 4;
const DICT_ZEN2HAN: c_int = 5;

/// The functions Sakura binds (`CMigemo::InitDllImp`).
struct Api {
    open: OpenFn,
    close: CloseFn,
    query: QueryFn,
    release: ReleaseFn,
    setproc_char2int: SetChar2IntFn,
    setproc_int2char: SetInt2CharFn,
    load: LoadFn,
    is_enable: IsEnableFn,
}

/// A loaded C/Migemo with its `migemo` object.
///
/// It is not `Send`: the object belongs to the thread that made it.
pub struct Migemo {
    handle: NonNull<c_void>,
    api: Api,
    // Dropped after `Migemo::drop` has closed the object: the functions above live in it.
    _library: sys::Library,
}

/// Binds `$name` of the library as the function type `$ty`.
macro_rules! bind {
    ($library:expr, $name:literal, $ty:ty) => {{
        let address = $library
            .symbol(
                CStr::from_bytes_with_nul(concat!($name, "\0").as_bytes())
                    .map_err(|_| OpenError::MissingFunction($name))?,
            )
            .ok_or(OpenError::MissingFunction($name))?;
        // SAFETY: `$name` has the signature `$ty` in `migemo.h` of C/Migemo.
        unsafe { std::mem::transmute::<*const c_void, $ty>(address) }
    }};
}

impl Migemo {
    /// Loads the library at `library` (an absolute path: the library is never searched for),
    /// binds its functions and makes the `migemo` object without a dictionary
    /// (`CMigemo::InitDllImp`).
    pub fn open(library: &Path) -> Result<Self, OpenError> {
        if !library.is_absolute() {
            return Err(OpenError::NotAbsolute);
        }
        let library = sys::Library::open(library).map_err(OpenError::Load)?;
        let api = Api {
            open: bind!(library, "migemo_open", OpenFn),
            close: bind!(library, "migemo_close", CloseFn),
            query: bind!(library, "migemo_query", QueryFn),
            release: bind!(library, "migemo_release", ReleaseFn),
            setproc_char2int: bind!(library, "migemo_setproc_char2int", SetChar2IntFn),
            setproc_int2char: bind!(library, "migemo_setproc_int2char", SetInt2CharFn),
            load: bind!(library, "migemo_load", LoadFn),
            is_enable: bind!(library, "migemo_is_enable", IsEnableFn),
        };
        // SAFETY: `migemo_open(NULL)` makes an object without a dictionary.
        let raw = unsafe { (api.open)(std::ptr::null()) };
        let handle = NonNull::new(raw).ok_or(OpenError::Init)?;
        Ok(Self {
            handle,
            api,
            _library: library,
        })
    }

    /// Whether a dictionary is loaded (`migemo_is_enable`).
    pub fn is_enabled(&self) -> bool {
        // SAFETY: the object is live until `drop`.
        unsafe { (self.api.is_enable)(self.handle.as_ptr()) != 0 }
    }

    /// Loads the dictionary and the four tables from `dir` (the folder with `migemo-dict`), as
    /// `CMigemo::migemo_load_all` does when no dictionary is loaded yet, and tells whether the
    /// library has a dictionary now. The functions that turn a char into a number and back are
    /// set after the load, because the load puts the library's own back.
    pub fn load_dictionary(&mut self, dir: &Path) -> bool {
        if self.is_enabled() {
            return true;
        }
        for (id, name) in [
            (DICT_MIGEMO, "migemo-dict"),
            (DICT_HAN2ZEN, "han2zen.dat"),
            (DICT_HIRA2KATA, "hira2kata.dat"),
            (DICT_ROMA2HIRA, "roma2hira.dat"),
            (DICT_ZEN2HAN, "zen2han.dat"),
        ] {
            // The library opens the file with the narrow functions of the C runtime: a path
            // that holds characters outside the system code page cannot be given on Windows.
            let Ok(file) = CString::new(dir.join(name).to_string_lossy().into_owned()) else {
                continue;
            };
            // SAFETY: a live object, and a NUL-terminated path that lives until the call returns.
            unsafe { (self.api.load)(self.handle.as_ptr(), id, file.as_ptr()) };
        }
        // SAFETY: a live object, and functions with the signatures the library calls them with.
        unsafe {
            (self.api.setproc_char2int)(self.handle.as_ptr(), char2int_utf8);
            (self.api.setproc_int2char)(self.handle.as_ptr(), int2char_utf8);
        }
        self.is_enabled()
    }

    /// The regular expression for `text` (`CMigemo::migemo_query_w`). `None` when the library
    /// gives nothing, or `text` has a NUL.
    pub fn query(&self, text: &str) -> Option<String> {
        let query = CString::new(text).ok()?;
        // SAFETY: a live object and a NUL-terminated query.
        let found = unsafe { (self.api.query)(self.handle.as_ptr(), query.as_ptr().cast()) };
        if found.is_null() {
            return None;
        }
        // SAFETY: the library gives a NUL-terminated string that it owns until `migemo_release`.
        let pattern = unsafe { CStr::from_ptr(found.cast()) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: `found` came from `migemo_query` of this object and is released once.
        unsafe { (self.api.release)(self.handle.as_ptr(), found) };
        Some(pattern)
    }
}

impl Drop for Migemo {
    fn drop(&mut self) {
        // SAFETY: the object is closed once; the library is unloaded after this (field order).
        unsafe { (self.api.close)(self.handle.as_ptr()) };
    }
}

/// `pcre_char2int_utf8`: the library calls it on a NUL-terminated string.
unsafe extern "C" fn char2int_utf8(input: *const c_uchar, out: *mut c_uint) -> c_int {
    // The bytes are read one at a time and only as far as each lead byte says, so that the NUL
    // that ends the string is never passed.
    let mut bytes = [0u8; 4];
    // SAFETY: `input` points into a NUL-terminated string.
    bytes[0] = unsafe { *input };
    let wanted = match bytes[0] {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    };
    for (i, slot) in bytes.iter_mut().enumerate().take(wanted).skip(1) {
        // SAFETY: the byte before was not the NUL (a tail byte is not 0), so the next is there.
        let next = unsafe { *input.add(i) };
        *slot = next;
        if next & 0xC0 != 0x80 {
            break;
        }
    }
    let (code, len) = decode_char(&bytes).unwrap_or((0, 1));
    if !out.is_null() {
        // SAFETY: the library gives a place for the number, or NULL.
        unsafe { *out = code };
    }
    c_int::try_from(len).unwrap_or(1)
}

/// `pcre_int2char_utf8`: the library gives a buffer of at least 16 bytes.
unsafe extern "C" fn int2char_utf8(code: c_uint, out: *mut c_uchar) -> c_int {
    let mut bytes = [0u8; 8];
    let len = encode_char(code, &mut bytes);
    if !out.is_null() {
        // SAFETY: `out` has room for 16 bytes (`rxgen.c` says so) and `len` is at most 4.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, len) };
    }
    c_int::try_from(len).unwrap_or(0)
}
