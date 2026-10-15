//! The only module with `unsafe` code: a small wrapper around the C shim
//! (`c/shim.c`) that calls the vendored Onigmo.
//!
//! The workspace denies `unsafe_code`; this module opts out because calling C
//! needs it. Everything that leaves the module is safe: raw pointers stay in
//! [`Compiled`] and [`Region`], which free what Onigmo allocates on drop, also
//! when a search fails. The rest of the crate never sees Onigmo's types.
#![allow(unsafe_code)]

use std::ffi::{c_int, c_uint, c_void};
use std::ptr::{self, NonNull};
use std::sync::Once;

/// Size of the message buffer (`ONIG_MAX_ERROR_MESSAGE_LEN` is 90).
const MESSAGE_LEN: usize = 128;

/// `ONIGERR_INVALID_ARGUMENT` in `onigmo.h`, used for bounds we check ourselves.
const ERR_INVALID_ARGUMENT: i32 = -30;
const MISMATCH: isize = -1;

unsafe extern "C" {
    fn ume_onig_init() -> c_int;
    fn ume_opt_ignorecase() -> c_uint;
    fn ume_opt_extend() -> c_uint;
    fn ume_opt_negate_singleline() -> c_uint;
    fn ume_opt_ascii_range() -> c_uint;
    fn ume_opt_newline_crlf() -> c_uint;
    fn ume_regex_new(
        out: *mut *mut c_void,
        pattern: *const u8,
        len: usize,
        options: c_uint,
        syntax_kind: c_int,
        msg: *mut u8,
    ) -> c_int;
    fn ume_regex_free(reg: *mut c_void);
    fn ume_regex_captures(reg: *mut c_void) -> c_int;
    fn ume_region_new() -> *mut c_void;
    fn ume_region_free(region: *mut c_void);
    fn ume_region_len(region: *const c_void) -> c_int;
    fn ume_region_get(region: *const c_void, i: c_int, beg: *mut isize, end: *mut isize) -> c_int;
    fn ume_regex_search(
        reg: *mut c_void,
        text: *const u8,
        len: usize,
        gpos: isize,
        start: usize,
        range: usize,
        options: c_uint,
        region: *mut c_void,
        msg: *mut u8,
    ) -> isize;
    #[cfg_attr(not(test), allow(dead_code))]
    fn ume_regex_match(
        reg: *mut c_void,
        text: *const u8,
        len: usize,
        at: usize,
        options: c_uint,
        region: *mut c_void,
        msg: *mut u8,
    ) -> isize;
    #[cfg_attr(not(test), allow(dead_code))]
    fn ume_set_match_stack_limit(size: c_uint) -> c_int;
    #[cfg_attr(not(test), allow(dead_code))]
    fn ume_set_parse_depth_limit(depth: c_uint) -> c_int;
}

/// Option bits of Onigmo, as `u32`. Obtained from the C side so that they
/// always match the vendored `onigmo.h`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OptionBits;

impl OptionBits {
    pub(crate) fn ignore_case() -> u32 {
        // SAFETY: pure functions without arguments.
        unsafe { ume_opt_ignorecase() }
    }
    pub(crate) fn extend() -> u32 {
        // SAFETY: as above.
        unsafe { ume_opt_extend() }
    }
    pub(crate) fn negate_singleline() -> u32 {
        // SAFETY: as above.
        unsafe { ume_opt_negate_singleline() }
    }
    pub(crate) fn ascii_range() -> u32 {
        // SAFETY: as above.
        unsafe { ume_opt_ascii_range() }
    }
    pub(crate) fn newline_crlf() -> u32 {
        // SAFETY: as above.
        unsafe { ume_opt_newline_crlf() }
    }
}

/// Which Onigmo syntax to compile with. Mirrors `UME_SYNTAX_*` in `shim.c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Syntax {
    /// What bregonig.dll uses: Perl syntax plus a few extensions.
    Bregonig = 0,
    /// Ruby syntax without `ONIG_OPTION_ASCII_RANGE`, the setting of `testpy.py`.
    #[cfg_attr(not(test), allow(dead_code))] // used by the Onigmo test suite only
    OnigmoDefault = 1,
    #[cfg_attr(not(test), allow(dead_code))]
    Perl = 2,
    #[cfg_attr(not(test), allow(dead_code))]
    Grep = 3,
}

/// A failure reported by Onigmo: its error code and message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OnigError {
    pub(crate) code: i32,
    pub(crate) message: String,
}

fn message_to_string(buf: &[u8; MESSAGE_LEN]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

fn ensure_initialized() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // SAFETY: called once, before any other Onigmo call (guarded by `Once`).
        let code = unsafe { ume_onig_init() };
        assert_eq!(code, 0, "onig_init failed with {code}");
    });
}

/// Sets the match stack limit (0 = Onigmo's default). Process-wide.
#[cfg_attr(not(test), allow(dead_code))] // used by the Onigmo test suite only
pub(crate) fn set_match_stack_limit(size: u32) {
    ensure_initialized();
    // SAFETY: plain setter of a global integer.
    unsafe { ume_set_match_stack_limit(size) };
}

/// Sets the parse depth limit (0 = Onigmo's default). Process-wide.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn set_parse_depth_limit(depth: u32) {
    ensure_initialized();
    // SAFETY: plain setter of a global integer.
    unsafe { ume_set_parse_depth_limit(depth) };
}

/// A compiled `regex_t`. Owns it and frees it on drop.
pub(crate) struct Compiled {
    raw: NonNull<c_void>,
}

// SAFETY: after `onig_new` returns, Onigmo never writes to a `regex_t` while
// searching (the search state lives on the caller's stack and in the
// per-call `OnigRegion`), so one `regex_t` can be searched from many threads
// at once. Freeing happens in `Drop`, which needs unique ownership.
unsafe impl Send for Compiled {}
unsafe impl Sync for Compiled {}

/// Where each capture group matched, as byte ranges. Index 0 is the whole match.
pub(crate) type Groups = Vec<Option<(usize, usize)>>;

impl Compiled {
    /// Compiles `pattern` (UTF-8) with Onigmo option bits `options`.
    pub(crate) fn new(pattern: &[u8], options: u32, syntax: Syntax) -> Result<Self, OnigError> {
        ensure_initialized();
        let mut raw: *mut c_void = ptr::null_mut();
        let mut msg = [0u8; MESSAGE_LEN];
        // SAFETY: `pattern` is valid for `pattern.len()` bytes, `raw` and `msg`
        // are valid for writes, and `msg` is larger than any Onigmo message.
        let code = unsafe {
            ume_regex_new(
                &mut raw,
                pattern.as_ptr(),
                pattern.len(),
                options,
                syntax as c_int,
                msg.as_mut_ptr(),
            )
        };
        if code != 0 {
            return Err(OnigError {
                code,
                message: message_to_string(&msg),
            });
        }
        let raw = NonNull::new(raw).expect("onig_new returned success with a null regex");
        Ok(Self { raw })
    }

    /// Number of capture groups, not counting group 0.
    pub(crate) fn capture_count(&self) -> usize {
        // SAFETY: `self.raw` is a live regex.
        let n = unsafe { ume_regex_captures(self.raw.as_ptr()) };
        usize::try_from(n).unwrap_or(0)
    }

    /// Searches `text` from byte `start` towards `range` (a `range` smaller
    /// than `start` searches backward). `gpos` is the position `\G` refers to;
    /// `None` makes it `start`. `text` is the whole subject, so look-behind and
    /// `^` see the text before `start`.
    pub(crate) fn search(
        &self,
        text: &[u8],
        gpos: Option<usize>,
        start: usize,
        range: usize,
        options: u32,
    ) -> Result<Option<Groups>, OnigError> {
        let len = text.len();
        if start > len || range > len || gpos.is_some_and(|g| g > len) {
            return Err(invalid_argument());
        }
        // `isize::MAX` bounds every real slice length, so this cannot fail.
        let gpos = gpos.map_or(-1, |g| isize::try_from(g).unwrap_or(-1));
        let region = Region::new();
        let mut msg = [0u8; MESSAGE_LEN];
        // SAFETY: `start`, `range` and `gpos` were checked to be within
        // `text`; the regex, region and buffers are live and valid.
        let r = unsafe {
            ume_regex_search(
                self.raw.as_ptr(),
                text.as_ptr(),
                len,
                gpos,
                start,
                range,
                options,
                region.raw.as_ptr(),
                msg.as_mut_ptr(),
            )
        };
        finish(r, &region, &msg)
    }

    /// Matches `text` anchored at byte `at` (no scanning forward).
    #[cfg_attr(not(test), allow(dead_code))] // used by the Onigmo test suite only
    pub(crate) fn match_at(
        &self,
        text: &[u8],
        at: usize,
        options: u32,
    ) -> Result<Option<Groups>, OnigError> {
        if at > text.len() {
            return Err(invalid_argument());
        }
        let region = Region::new();
        let mut msg = [0u8; MESSAGE_LEN];
        // SAFETY: `at` was checked to be within `text`; the rest is as in `search`.
        let r = unsafe {
            ume_regex_match(
                self.raw.as_ptr(),
                text.as_ptr(),
                text.len(),
                at,
                options,
                region.raw.as_ptr(),
                msg.as_mut_ptr(),
            )
        };
        finish(r, &region, &msg)
    }
}

impl Drop for Compiled {
    fn drop(&mut self) {
        // SAFETY: `self.raw` came from `onig_new` and is freed exactly once.
        unsafe { ume_regex_free(self.raw.as_ptr()) };
    }
}

fn invalid_argument() -> OnigError {
    OnigError {
        code: ERR_INVALID_ARGUMENT,
        message: "invalid argument".to_owned(),
    }
}

fn finish(r: isize, region: &Region, msg: &[u8; MESSAGE_LEN]) -> Result<Option<Groups>, OnigError> {
    if r == MISMATCH {
        Ok(None)
    } else if r < MISMATCH {
        Err(OnigError {
            code: i32::try_from(r).unwrap_or(i32::MIN),
            message: message_to_string(msg),
        })
    } else {
        Ok(Some(region.groups()))
    }
}

/// An `OnigRegion`, freed on drop (so also when the search fails).
struct Region {
    raw: NonNull<c_void>,
}

impl Region {
    fn new() -> Self {
        // SAFETY: allocates a fresh region.
        let raw = unsafe { ume_region_new() };
        Self {
            raw: NonNull::new(raw).expect("out of memory allocating an OnigRegion"),
        }
    }

    fn groups(&self) -> Groups {
        // SAFETY: `self.raw` is a live region filled by a successful search.
        let len = unsafe { ume_region_len(self.raw.as_ptr()) };
        (0..len)
            .map(|i| {
                let (mut beg, mut end) = (0isize, 0isize);
                // SAFETY: `i` is in range and `beg`/`end` are valid for writes.
                let found = unsafe { ume_region_get(self.raw.as_ptr(), i, &mut beg, &mut end) };
                if found == 0 {
                    Some((beg as usize, end as usize))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // SAFETY: `self.raw` came from `onig_region_new` and is freed once.
        unsafe { ume_region_free(self.raw.as_ptr()) };
    }
}
