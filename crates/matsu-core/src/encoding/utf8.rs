// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CUtf8.cpp
//   - sakura_core/charset/CUtf8.h
//   - sakura_core/charset/CCesu8.h
//   - sakura_core/charset/codechecker.cpp (CheckUtf8Char, CheckCesu8Char)
//
// Modified: rewritten in Rust (not a line-by-line port). The Windows API calls the original
// uses (MultiByteToWideChar and the like) are replaced by tables built from `encoding_rs`,
// and the invalid-byte stand-ins are U+10FF00..=U+10FFFF instead of U+DC00..=U+DCFF.
// Differences from the original behavior are listed in the module documentation of
// `encoding/mod.rs` and in the comments next to the code.
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! UTF-8 and CESU-8.
//!
//! UTF-8 is read as the Unicode standard says (overlong forms, surrogates and values above
//! U+10FFFF are invalid); Sakura's `CheckUtf8Char` accepts exactly the same sequences. Each byte
//! of an invalid sequence is its own stand-in and reading goes on with the next byte, as in
//! `CUtf8::Utf8ToUni`.
//!
//! CESU-8 writes a char outside the BMP as two three-byte surrogates (`CheckCesu8Char`). A
//! four-byte sequence and a surrogate that is not half of a pair are invalid.

use super::{Codec, Eol, Unmappable, ascii_eol_bytes, stand_in, stand_in_byte};

/// The result of looking at one char.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CharKind {
    Normal,
    /// A char outside the BMP: four bytes in UTF-8, six in CESU-8.
    Supplementary,
    /// Invalid; the length returned with it is 1.
    Binary,
}

fn is_continuation(b: u8) -> bool {
    b & 0xC0 == 0x80
}

/// `CheckUtf8Char`: the length and kind of the char at the start of `s` (not empty).
/// `allow_4byte` is false for CESU-8, where a four-byte sequence is invalid.
pub(crate) fn check_utf8_char(s: &[u8], allow_4byte: bool) -> (usize, CharKind) {
    let c0 = s[0];
    if c0 < 0x80 {
        return (1, CharKind::Normal);
    }
    if s.len() > 1 && c0 & 0xE0 == 0xC0 {
        if is_continuation(s[1]) {
            // C0 and C1 would give a value below U+0080.
            return if c0 & 0x1E == 0 {
                (1, CharKind::Binary)
            } else {
                (2, CharKind::Normal)
            };
        }
    } else if s.len() > 2 && c0 & 0xF0 == 0xE0 {
        if is_continuation(s[1]) && is_continuation(s[2]) {
            // Below U+0800 (overlong), or a surrogate (U+D800..=U+DFFF).
            let overlong = c0 & 0x0F == 0 && s[1] & 0x20 == 0;
            let surrogate = allow_4byte && c0 & 0x0F == 0x0D && s[1] & 0x20 != 0;
            return if overlong || surrogate {
                (1, CharKind::Binary)
            } else {
                (3, CharKind::Normal)
            };
        }
    } else if s.len() > 3
        && c0 & 0xF8 == 0xF0
        && is_continuation(s[1])
        && is_continuation(s[2])
        && is_continuation(s[3])
    {
        // Below U+10000 (overlong), or above U+10FFFF.
        let overlong = c0 & 0x07 == 0 && s[1] & 0x30 == 0;
        let too_large = c0 & 0x04 != 0 && (c0 & 0x03 != 0 || s[1] & 0x30 != 0);
        return if overlong || too_large || !allow_4byte {
            (1, CharKind::Binary)
        } else {
            (4, CharKind::Supplementary)
        };
    }
    (1, CharKind::Binary)
}

fn is_utf8_surrogate_high(s: &[u8]) -> bool {
    s[0] == 0xED && s[1] & 0xF0 == 0xA0
}

fn is_utf8_surrogate_low(s: &[u8]) -> bool {
    s[0] == 0xED && s[1] & 0xF0 == 0xB0
}

/// `CheckCesu8Char`: the length and kind of the char at the start of `s` (not empty).
pub(crate) fn check_cesu8_char(s: &[u8]) -> (usize, CharKind) {
    let (len, kind) = check_utf8_char(s, false);
    if len != 3 {
        return (len, kind);
    }
    let second = if s.len() > 3 {
        Some(check_utf8_char(&s[3..], false))
    } else {
        None
    };
    match second {
        Some((3, CharKind::Normal))
            if is_utf8_surrogate_high(s) && is_utf8_surrogate_low(&s[3..]) =>
        {
            (6, CharKind::Supplementary)
        }
        _ if is_utf8_surrogate_high(s) || is_utf8_surrogate_low(s) => (1, CharKind::Binary),
        _ => (3, CharKind::Normal),
    }
}

/// `DecodeUtf8` for a valid char of 1 to 3 bytes (also one half of a CESU-8 pair).
fn decode_unit(s: &[u8]) -> u32 {
    match s.len() {
        2 => (u32::from(s[0] & 0x1F) << 6) | u32::from(s[1] & 0x3F),
        3 => {
            (u32::from(s[0] & 0x0F) << 12) | (u32::from(s[1] & 0x3F) << 6) | u32::from(s[2] & 0x3F)
        }
        _ => u32::from(s[0]),
    }
}

/// Finds the line endings of UTF-8 and CESU-8: CR, LF, NEL (C2 85), LS (E2 80 A8) and PS (E2 80 A9).
fn find_eol_utf8(bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
    let mut i = from;
    while i < bytes.len() {
        let eol = match bytes[i] {
            b'\n' => Some(Eol::Lf),
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => Some(Eol::CrLf),
            b'\r' => Some(Eol::Cr),
            0xC2 if bytes.get(i + 1) == Some(&0x85) => Some(Eol::Nel),
            0xE2 if bytes.get(i + 1) == Some(&0x80) => match bytes.get(i + 2) {
                Some(0xA8) => Some(Eol::Ls),
                Some(0xA9) => Some(Eol::Ps),
                _ => None,
            },
            _ => None,
        };
        if let Some(eol) = eol {
            return Some((i, eol));
        }
        i += 1;
    }
    None
}

fn eol_bytes_utf8(eol: Eol) -> Option<&'static [u8]> {
    match eol {
        Eol::Nel => Some(b"\xC2\x85"),
        Eol::Ls => Some(b"\xE2\x80\xA8"),
        Eol::Ps => Some(b"\xE2\x80\xA9"),
        other => ascii_eol_bytes(other),
    }
}

/// UTF-8: `CODE_UTF8`.
#[derive(Debug)]
pub(crate) struct Utf8;

impl Codec for Utf8 {
    fn decode(&self, mut bytes: &[u8], out: &mut String) {
        loop {
            match std::str::from_utf8(bytes) {
                Ok(valid) => {
                    out.push_str(valid);
                    return;
                }
                Err(error) => {
                    let (valid, rest) = bytes.split_at(error.valid_up_to());
                    out.push_str(std::str::from_utf8(valid).expect("valid up to"));
                    out.push(stand_in(rest[0]));
                    bytes = &rest[1..];
                }
            }
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        find_eol_utf8(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        eol_bytes_utf8(eol)
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        match bytes.get(i..) {
            Some(rest) if !rest.is_empty() => check_utf8_char(rest, true).0,
            _ => 1,
        }
    }

    fn boundary_at_or_after(&self, bytes: &[u8], from: usize, target: usize) -> usize {
        // In UTF-8 a boundary is the next byte that is not a continuation byte (10xxxxxx).
        let mut at = from.max(target);
        let stop = bytes.len().min(at + 3);
        while at < stop && bytes[at] & 0xC0 == 0x80 {
            at += 1;
        }
        at
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        for c in text.chars() {
            match stand_in_byte(c) {
                Some(byte) => out.push(byte),
                None => {
                    let mut buffer = [0; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
                }
            }
        }
        Ok(())
    }
}

/// CESU-8: `CODE_CESU8`.
#[derive(Debug)]
pub(crate) struct Cesu8;

impl Codec for Cesu8 {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        let mut i = 0;
        while i < bytes.len() {
            let rest = &bytes[i..];
            let (len, kind) = check_cesu8_char(rest);
            match kind {
                CharKind::Normal => {
                    let value = decode_unit(&rest[..len]);
                    out.push(char::from_u32(value).expect("valid CESU-8 char"));
                }
                CharKind::Supplementary => {
                    let high = decode_unit(&rest[..3]);
                    let low = decode_unit(&rest[3..6]);
                    let value = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
                    out.push(char::from_u32(value).expect("valid CESU-8 pair"));
                }
                CharKind::Binary => out.push(stand_in(rest[0])),
            }
            i += len;
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        find_eol_utf8(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        eol_bytes_utf8(eol)
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        match bytes.get(i..) {
            Some(rest) if !rest.is_empty() => check_cesu8_char(rest).0,
            _ => 1,
        }
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        for c in text.chars() {
            if let Some(byte) = stand_in_byte(c) {
                out.push(byte);
            } else if (c as u32) < 0x10000 {
                let mut buffer = [0; 4];
                out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            } else {
                let mut units = [0u16; 2];
                for &unit in c.encode_utf16(&mut units).iter() {
                    out.extend_from_slice(&[
                        0xE0 | (unit >> 12) as u8,
                        0x80 | ((unit >> 6) & 0x3F) as u8,
                        0x80 | (unit & 0x3F) as u8,
                    ]);
                }
            }
        }
        Ok(())
    }
}
