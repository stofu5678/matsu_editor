// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CCodePage.cpp (S_UTF32LEToUnicode, S_UTF32BEToUnicode,
//     S_UnicodeToUTF32LE, S_UnicodeToUTF32BE, GetEncodingTrait)
//   - sakura_core/io/CFileLoad.cpp (GetNextLineCharCode, the UTF-32 cases)
//
// Modified: rewritten in Rust (not a line-by-line port). The Windows API calls the original
// uses (MultiByteToWideChar and the like) are replaced by tables built from `encoding_rs`,
// and the invalid-byte stand-ins are U+10FF00..=U+10FFFF instead of U+DC00..=U+DCFF.
// Differences from the original behavior are listed in the module documentation of
// `encoding/mod.rs` and in the comments next to the code.
//
// Copyright (C) 2010-2012 Moca
// Copyright (C) 2018-2026, Sakura Editor Organization
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

//! UTF-32 little endian (code page 12000) and big endian (12001).
//!
//! Sakura has no `ECodeType` for them; they are the two code pages that `CCodePage` converts by
//! itself because Windows does not (`MultiByteToWideChar2`). A 4-byte unit that is a Unicode scalar
//! value is one char. Anything else (a surrogate, a value above U+10FFFF) and a last group of fewer
//! than four bytes become one stand-in per byte, as Sakura's `BinToText` does.
//!
//! Differences from Sakura when writing: a run of stand-ins is written as the raw bytes. Sakura
//! pads a run of fewer than four to four bytes with zeros and drops the bytes of a last odd group
//! (`nDstUseLen += nBinaryLen` after `nBinaryLen = 0` in `S_UnicodeToUTF32LE`).

use super::{Codec, Eol, Unmappable, stand_in, stand_in_byte};

/// UTF-32 as a code page: 12000 (little endian) and 12001 (big endian).
#[derive(Debug)]
pub(crate) struct Utf32 {
    pub(crate) big_endian: bool,
}

impl Utf32 {
    /// The value of the 4 bytes at `bytes[i]`, if all are there.
    fn unit(&self, bytes: &[u8], i: usize) -> Option<u32> {
        let quad: [u8; 4] = bytes.get(i..i + 4)?.try_into().ok()?;
        Some(if self.big_endian {
            u32::from_be_bytes(quad)
        } else {
            u32::from_le_bytes(quad)
        })
    }

    fn unit_bytes(&self, value: u32) -> [u8; 4] {
        if self.big_endian {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        }
    }
}

impl Codec for Utf32 {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        let mut i = 0;
        while i < bytes.len() {
            match self.unit(bytes, i) {
                Some(value) => match char::from_u32(value) {
                    Some(c) => out.push(c),
                    // A surrogate or a value above U+10FFFF: one stand-in per byte.
                    None => bytes[i..i + 4].iter().for_each(|&b| out.push(stand_in(b))),
                },
                None => {
                    bytes[i..].iter().for_each(|&b| out.push(stand_in(b)));
                    return;
                }
            }
            i += 4;
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        let mut i = from;
        while let Some(unit) = self.unit(bytes, i) {
            let eol = match unit {
                0x000A => Some(Eol::Lf),
                0x000D if self.unit(bytes, i + 4) == Some(0x000A) => Some(Eol::CrLf),
                0x000D => Some(Eol::Cr),
                0x0085 => Some(Eol::Nel),
                0x2028 => Some(Eol::Ls),
                0x2029 => Some(Eol::Ps),
                _ => None,
            };
            if let Some(eol) = eol {
                return Some((i, eol));
            }
            i += 4;
        }
        None
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        Some(match (eol, self.big_endian) {
            (Eol::CrLf, false) => b"\r\0\0\0\n\0\0\0",
            (Eol::CrLf, true) => b"\0\0\0\r\0\0\0\n",
            (Eol::Lf, false) => b"\n\0\0\0",
            (Eol::Lf, true) => b"\0\0\0\n",
            (Eol::Cr, false) => b"\r\0\0\0",
            (Eol::Cr, true) => b"\0\0\0\r",
            (Eol::Nel, false) => b"\x85\0\0\0",
            (Eol::Nel, true) => b"\0\0\0\x85",
            (Eol::Ls, false) => b"\x28\x20\0\0",
            (Eol::Ls, true) => b"\0\0\x20\x28",
            (Eol::Ps, false) => b"\x29\x20\0\0",
            (Eol::Ps, true) => b"\0\0\x20\x29",
        })
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        // Four bytes, or one stand-in byte for the last group of fewer than four.
        if i + 4 <= bytes.len() { 4 } else { 1 }
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        for c in text.chars() {
            match stand_in_byte(c) {
                Some(byte) => out.push(byte),
                None => out.extend_from_slice(&self.unit_bytes(u32::from(c))),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(big_endian: bool, bytes: &[u8]) -> String {
        let mut out = String::new();
        Utf32 { big_endian }.decode(bytes, &mut out);
        out
    }

    fn encode(big_endian: bool, text: &str) -> Vec<u8> {
        let mut out = Vec::new();
        Utf32 { big_endian }.encode(text, &mut out).unwrap();
        out
    }

    #[test]
    fn round_trips_bmp_and_astral_chars() {
        let le = b"a\0\0\0\x42\x30\0\0\x00\xF6\x01\0";
        let be = b"\0\0\0a\0\0\x30\x42\0\x01\xF6\x00";
        assert_eq!(decode(false, le), "aあ😀");
        assert_eq!(decode(true, be), "aあ😀");
        assert_eq!(encode(false, "aあ😀"), le);
        assert_eq!(encode(true, "aあ😀"), be);
    }

    #[test]
    fn invalid_units_and_the_odd_tail_become_stand_ins() {
        let s = |bytes: &[u8]| bytes.iter().map(|&b| stand_in(b)).collect::<String>();
        // A lone surrogate and a value above U+10FFFF (CCodePage.cpp: "バイトごとに出力する").
        assert_eq!(decode(false, b"\x3D\xD8\0\0"), s(b"\x3D\xD8\0\0"));
        assert_eq!(decode(false, b"\0\0\x11\0"), s(b"\0\0\x11\0"));
        assert_eq!(decode(true, b"\0\0\xD8\x3D"), s(b"\0\0\xD8\x3D"));
        // The last group of fewer than four bytes.
        assert_eq!(decode(false, b"a\0\0\0bc"), format!("a{}", s(b"bc")));
        // Stand-ins are written as their bytes.
        assert_eq!(encode(false, &s(b"\x3D\xD8\0\0")), b"\x3D\xD8\0\0");
    }

    #[test]
    fn finds_line_endings_in_units() {
        let codec = Utf32 { big_endian: false };
        let bytes = b"a\0\0\0\r\0\0\0\n\0\0\0b\0\0\0";
        assert_eq!(codec.find_eol(bytes, 0), Some((4, Eol::CrLf)));
        // A byte pattern that is not aligned to a unit is not a line ending.
        assert_eq!(codec.find_eol(b"\0\n\0\0\0", 0), None);
        assert_eq!(codec.char_len(bytes, 0), 4);
        assert_eq!(codec.char_len(bytes, 14), 1);
        assert_eq!(codec.eol_bytes(Eol::Ls), Some(&b"\x28\x20\0\0"[..]));
    }
}
