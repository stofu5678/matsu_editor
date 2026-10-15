// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CUnicode.cpp
//   - sakura_core/charset/CUnicodeBe.cpp
//   - sakura_core/charset/codechecker.cpp (_CheckUtf16Char)
//   - sakura_core/io/CFileLoad.cpp (GetNextLineCharCode)
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

//! UTF-16 little endian ("Unicode" in Sakura) and big endian.
//!
//! Everything works in 2-byte units. A surrogate pair is one 4-byte char. A surrogate that is not
//! half of a pair cannot be held by a `String`, so its two bytes become two stand-ins, and a last
//! odd byte becomes one stand-in (Sakura pads a 0x00 and reports a loss instead).

use super::{Codec, Eol, Unmappable, stand_in, stand_in_byte};

/// UTF-16: `CODE_UNICODE` (little endian) and `CODE_UNICODEBE`.
#[derive(Debug)]
pub(crate) struct Utf16 {
    pub(crate) big_endian: bool,
}

fn is_high(unit: u16) -> bool {
    unit & 0xFC00 == 0xD800
}

fn is_low(unit: u16) -> bool {
    unit & 0xFC00 == 0xDC00
}

impl Utf16 {
    /// The unit that starts at `bytes[i]`, if both its bytes are there.
    fn unit(&self, bytes: &[u8], i: usize) -> Option<u16> {
        let pair = [*bytes.get(i)?, *bytes.get(i + 1)?];
        Some(if self.big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        })
    }

    /// The bytes of a unit.
    fn unit_bytes(&self, unit: u16) -> [u8; 2] {
        if self.big_endian {
            unit.to_be_bytes()
        } else {
            unit.to_le_bytes()
        }
    }
}

impl Codec for Utf16 {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        let mut i = 0;
        while let Some(unit) = self.unit(bytes, i) {
            if is_high(unit) {
                match self.unit(bytes, i + 2) {
                    Some(low) if is_low(low) => {
                        let value =
                            0x10000 + (u32::from(unit - 0xD800) << 10) + u32::from(low - 0xDC00);
                        out.push(char::from_u32(value).expect("valid surrogate pair"));
                        i += 4;
                    }
                    _ => {
                        out.push(stand_in(bytes[i]));
                        out.push(stand_in(bytes[i + 1]));
                        i += 2;
                    }
                }
            } else if is_low(unit) {
                out.push(stand_in(bytes[i]));
                out.push(stand_in(bytes[i + 1]));
                i += 2;
            } else {
                out.push(char::from_u32(u32::from(unit)).expect("not a surrogate"));
                i += 2;
            }
        }
        if let Some(&last) = bytes.get(i) {
            out.push(stand_in(last));
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        let mut i = from;
        while let Some(unit) = self.unit(bytes, i) {
            let eol = match unit {
                0x000A => Some(Eol::Lf),
                0x000D if self.unit(bytes, i + 2) == Some(0x000A) => Some(Eol::CrLf),
                0x000D => Some(Eol::Cr),
                0x0085 => Some(Eol::Nel),
                0x2028 => Some(Eol::Ls),
                0x2029 => Some(Eol::Ps),
                _ => None,
            };
            if let Some(eol) = eol {
                return Some((i, eol));
            }
            i += 2;
        }
        None
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        Some(match (eol, self.big_endian) {
            (Eol::CrLf, false) => b"\r\0\n\0",
            (Eol::CrLf, true) => b"\0\r\0\n",
            (Eol::Lf, false) => b"\n\0",
            (Eol::Lf, true) => b"\0\n",
            (Eol::Cr, false) => b"\r\0",
            (Eol::Cr, true) => b"\0\r",
            (Eol::Nel, false) => b"\x85\0",
            (Eol::Nel, true) => b"\0\x85",
            (Eol::Ls, false) => b"\x28\x20",
            (Eol::Ls, true) => b"\x20\x28",
            (Eol::Ps, false) => b"\x29\x20",
            (Eol::Ps, true) => b"\x20\x29",
        })
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        match self.unit(bytes, i) {
            None => 1,
            Some(unit) if is_high(unit) => match self.unit(bytes, i + 2) {
                Some(low) if is_low(low) => 4,
                _ => 2,
            },
            Some(_) => 2,
        }
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        let mut units = [0u16; 2];
        for c in text.chars() {
            if let Some(byte) = stand_in_byte(c) {
                out.push(byte);
                continue;
            }
            for &unit in c.encode_utf16(&mut units).iter() {
                out.extend_from_slice(&self.unit_bytes(unit));
            }
        }
        Ok(())
    }
}
