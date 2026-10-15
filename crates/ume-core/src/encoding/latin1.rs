// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CLatin1.cpp
//   - sakura_core/charset/CLatin1.h
//
// Modified: rewritten in Rust (not a line-by-line port). The Windows API calls the original
// uses (MultiByteToWideChar and the like) are replaced by tables built from `encoding_rs`,
// and the invalid-byte stand-ins are U+10FF00..=U+10FFFF instead of U+DC00..=U+DCFF.
// Differences from the original behavior are listed in the module documentation of
// `encoding/mod.rs` and in the comments next to the code.
//
// Copyright (C) 2010, Uchi
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

//! Latin1 as Sakura reads and writes it: ISO 8859-1 plus the Windows-1252 characters in
//! 0x80..=0x9F.
//!
//! Sakura keeps 0xA0..=0xFF and below 0x80 as they are and asks Windows code page 1252 for
//! 0x80..=0x9F (`CLatin1::Latin1ToUni`, `_UniToLatin1_char`). The five bytes that CP1252 leaves
//! undefined (0x81, 0x8D, 0x8F, 0x90, 0x9D) stay as the C1 controls with the same number
//! (`test-ccodebase.cpp`, `codeLatin1`). There are no invalid bytes.

use super::{Codec, Eol, Unmappable, ascii_eol_bytes, encode_chars, find_cr_lf};

/// Windows-1252 for the bytes 0x80..=0x9F.
const WINDOWS_1252: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// Latin1: `CODE_LATIN1`.
#[derive(Debug)]
pub(crate) struct Latin1;

impl Codec for Latin1 {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        out.extend(bytes.iter().map(|&b| match b {
            0x80..=0x9F => WINDOWS_1252[usize::from(b - 0x80)],
            _ => char::from(b),
        }));
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        find_cr_lf(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        ascii_eol_bytes(eol)
    }

    fn char_len(&self, _bytes: &[u8], _i: usize) -> usize {
        1
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        encode_chars(text, out, |c, out| {
            let value = c as u32;
            if value < 0x80 || (0xA0..=0xFF).contains(&value) {
                out.push(value as u8);
                return true;
            }
            match WINDOWS_1252.iter().position(|&w| w == c) {
                Some(index) => {
                    out.push(0x80 + index as u8);
                    true
                }
                None => false,
            }
        })
    }
}
