//! The OEM code page of Windows (`GetOEMCP`: CP932 on Japanese Windows, 437 on US English), which
//! `cmd.exe` and the programs of its console (`sort`, `more`, `findstr`) read and write.
//!
//! One-liners run through `cmd` (ADR-0029, decision 5) feed and read `cmd` in this code page.
//! `encoding_rs` has no CP437 and the like, so the system's own conversion is used. Code page
//! 65001 (the "Beta: Use Unicode UTF-8" setting of Windows makes it the OEM one) is plain UTF-8.

// The Win32 calls are `unsafe` (ADR-0013: only the OS modules of this crate have unsafe code).
#![allow(unsafe_code)]

use std::ptr::{null, null_mut};

use windows_sys::Win32::Globalization::{
    GetOEMCP, IsDBCSLeadByteEx, MB_ERR_INVALID_CHARS, MultiByteToWideChar, WC_NO_BEST_FIT_CHARS,
    WideCharToMultiByte,
};

/// `CP_UTF8`.
const UTF8: u32 = 65001;

/// The number of the OEM code page.
pub fn code_page() -> u32 {
    // SAFETY: `GetOEMCP` takes nothing and returns a number.
    unsafe { GetOEMCP() }
}

/// `text` in the code page; `None` when a character cannot be written in it (no best-fit
/// replacement: an emoji is not turned into `?`).
pub fn encode(text: &str) -> Option<Vec<u8>> {
    encode_in(code_page(), text)
}

fn encode_in(page: u32, text: &str) -> Option<Vec<u8>> {
    if page == UTF8 || text.is_empty() {
        return Some(text.as_bytes().to_vec());
    }
    let wide: Vec<u16> = text.encode_utf16().collect();
    let len = i32::try_from(wide.len()).ok()?;
    let mut used_default = 0;
    // SAFETY: the pointers are those of `wide` (with its length) and of `used_default`; the first
    // call only measures (no output buffer), the second writes at most `size` bytes into `out`.
    unsafe {
        let size = WideCharToMultiByte(
            page,
            WC_NO_BEST_FIT_CHARS,
            wide.as_ptr(),
            len,
            null_mut(),
            0,
            null(),
            &mut used_default,
        );
        if size <= 0 || used_default != 0 {
            return None;
        }
        let mut out = vec![0u8; usize::try_from(size).ok()?];
        let written = WideCharToMultiByte(
            page,
            WC_NO_BEST_FIT_CHARS,
            wide.as_ptr(),
            len,
            out.as_mut_ptr(),
            size,
            null(),
            &mut used_default,
        );
        (written == size && used_default == 0).then_some(out)
    }
}

/// The text of `bytes`; bytes that are not valid in the code page become U+FFFD.
pub fn decode_lossy(bytes: &[u8]) -> String {
    decode_in(code_page(), bytes)
}

fn decode_in(page: u32, bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if page == UTF8 {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    if let Some(text) = convert(page, bytes) {
        return text;
    }
    // Some part is not valid: character by character, so that only that part is lost.
    let mut text = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let width = if is_lead_byte_in(page, bytes[i]) && i + 1 < bytes.len() {
            2
        } else {
            1
        };
        match convert(page, &bytes[i..i + width]) {
            Some(part) => {
                text.push_str(&part);
                i += width;
            }
            None => {
                text.push('\u{FFFD}');
                i += 1;
            }
        }
    }
    text
}

/// `None` when `bytes` is not valid in the code page.
fn convert(page: u32, bytes: &[u8]) -> Option<String> {
    let len = i32::try_from(bytes.len()).ok()?;
    // SAFETY: the pointers are those of `bytes` (with its length); the first call only measures,
    // the second writes at most `size` units into `out`.
    unsafe {
        let size = MultiByteToWideChar(
            page,
            MB_ERR_INVALID_CHARS,
            bytes.as_ptr(),
            len,
            null_mut(),
            0,
        );
        if size <= 0 {
            // 0: not valid in the code page (`ERROR_NO_UNICODE_TRANSLATION`) or no other reason
            // that a retry would cure
            return None;
        }
        let mut out = vec![0u16; usize::try_from(size).ok()?];
        let written = MultiByteToWideChar(
            page,
            MB_ERR_INVALID_CHARS,
            bytes.as_ptr(),
            len,
            out.as_mut_ptr(),
            size,
        );
        (written == size).then(|| String::from_utf16_lossy(&out))
    }
}

fn is_lead_byte_in(page: u32, byte: u8) -> bool {
    // SAFETY: takes a code page and a byte.
    page != UTF8 && unsafe { IsDBCSLeadByteEx(page, byte) != 0 }
}

/// How many bytes at the end of `bytes` are the start of a character that the next piece
/// finishes (0, 1, or up to 3 in UTF-8). `bytes` must start at the start of a character.
pub fn held_len(bytes: &[u8]) -> usize {
    held_len_in(code_page(), bytes)
}

fn held_len_in(page: u32, bytes: &[u8]) -> usize {
    if page == UTF8 {
        // the last 1 to 3 bytes of an unfinished sequence
        for back in 1..=bytes.len().min(3) {
            let b = bytes[bytes.len() - back];
            if b & 0xC0 == 0x80 {
                continue;
            }
            let need = match b {
                0xC0..=0xDF => 2,
                0xE0..=0xEF => 3,
                0xF0..=0xF7 => 4,
                _ => 1,
            };
            return if need > back { back } else { 0 };
        }
        return 0;
    }
    let mut i = 0;
    while i < bytes.len() {
        if is_lead_byte_in(page, bytes[i]) {
            if i + 1 == bytes.len() {
                return 1;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    0
}
