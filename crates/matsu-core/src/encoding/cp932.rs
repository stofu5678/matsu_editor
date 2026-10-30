// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CShiftJis.cpp
//   - sakura_core/charset/CShiftJis.h
//   - sakura_core/charset/codeutil.cpp
//   - sakura_core/charset/codeutil.h
//   - sakura_core/charset/codechecker.h
//
// Modified: rewritten in Rust (not a line-by-line port). The Windows API calls the original
// uses (MultiByteToWideChar and the like) are replaced by tables built from `encoding_rs`,
// and the invalid-byte stand-ins are U+10FF00..=U+10FFFF instead of U+DC00..=U+DCFF.
// Differences from the original behavior are listed in the module documentation of
// `encoding/mod.rs` and in the comments next to the code.
//
// Copyright (C) 2006, D. S. Koba, genta
// Copyright (C) 2007
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

//! Shift_JIS (CP932) as Sakura converts it.
//!
//! Sakura converts with the Windows code page 932 and then throws away every double-byte
//! character that does not survive a round trip (`MyMultiByteToWideChar_JP` with `bKeepRt` in
//! `codeutil.h`). The tables here come from the WHATWG `Shift_JIS` of `encoding_rs`, which is
//! built from the same CP932 mapping, so the result is the same as Sakura's on Windows for:
//!
//! - NEC row 13 duplicates of JIS X 0208 row 2 (0x8790 and the like): not decodable, because the
//!   character writes back as the row 2 code (`TABLE_SjisPoorcodeDef` in `codeutil.cpp`).
//! - NEC-selected IBM extensions (0xED40..=0xEEFC): not decodable, because they write back as the
//!   IBM extensions (0xFA40..=0xFC4B). Sakura's own test says so (`test-ccodebase.cpp`,
//!   `codeSJis`: "NEC選定IBM拡張文字 と呼ばれる約400字").
//! - IBM extensions (0xFA40..=0xFC4B), NEC row 13 and the user-defined area (0xF040..=0xF9FC,
//!   U+E000..=U+E757): decodable and written back as they are.
//! - 0x80, 0xA0, 0xFD..=0xFF and a lead byte without a valid trail byte: invalid bytes.
//!
//! Which character a CP932 code means follows Windows (0x8160 is U+FF5E, 0x8161 is U+2225,
//! 0x815F is U+FF3C, 0x817C is U+FF0D, 0x5C is a backslash).

use std::collections::HashMap;
use std::sync::OnceLock;

use super::{Codec, Eol, Unmappable, ascii_eol_bytes, encode_chars, find_cr_lf, stand_in};

/// A CP932 code for one char.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mapped {
    /// ASCII or a half-width katakana byte.
    Single(u8),
    Double(u8, u8),
}

const LEAD_COUNT: usize = 0xFC - 0x81 + 1;
const TRAIL_COUNT: usize = 0xFC - 0x40 + 1;

struct Tables {
    /// `decode[(lead - 0x81) * TRAIL_COUNT + (trail - 0x40)]`
    decode: Vec<Option<char>>,
    encode: HashMap<char, (u8, u8)>,
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut decode = vec![None; LEAD_COUNT * TRAIL_COUNT];
        let mut encode = HashMap::new();
        for lead in 0x81..=0xFCu8 {
            if !is_lead(lead) {
                continue;
            }
            for trail in 0x40..=0xFCu8 {
                if !is_trail(trail) {
                    continue;
                }
                let pair = [lead, trail];
                let Some(text) = encoding_rs::SHIFT_JIS
                    .decode_without_bom_handling_and_without_replacement(&pair)
                else {
                    continue;
                };
                let mut chars = text.chars();
                let (Some(c), None) = (chars.next(), chars.next()) else {
                    continue;
                };
                decode[table_index(lead, trail)] = Some(c);
                // WideCharToMultiByte takes the first code in the table for a char that has
                // several; the NEC-selected IBM extensions (0xED, 0xEE) are never the answer.
                if lead != 0xED && lead != 0xEE {
                    encode.entry(c).or_insert((lead, trail));
                }
            }
        }
        Tables { decode, encode }
    })
}

fn table_index(lead: u8, trail: u8) -> usize {
    usize::from(lead - 0x81) * TRAIL_COUNT + usize::from(trail - 0x40)
}

/// First byte of a double-byte char (`IsSjisZen1`).
pub(crate) fn is_lead(b: u8) -> bool {
    (0x81..=0x9F).contains(&b) || (0xE0..=0xFC).contains(&b)
}

/// Second byte of a double-byte char (`IsSjisZen2`).
pub(crate) fn is_trail(b: u8) -> bool {
    (0x40..=0xFC).contains(&b) && b != 0x7F
}

/// Half-width katakana byte (`IsSjisHankata`).
pub(crate) fn is_hankata(b: u8) -> bool {
    (0xA1..=0xDF).contains(&b)
}

/// The char of a half-width katakana byte 0xA1..=0xDF (U+FF61..=U+FF9F).
pub(crate) fn hankata_char(b: u8) -> char {
    char::from_u32(0xFF61 + u32::from(b - 0xA1)).expect("half-width katakana")
}

/// The CP932 char of a double-byte code, whether or not it writes back to the same code.
/// This is `MultiByteToWideChar(932, ...)` without Sakura's round-trip check.
pub(crate) fn raw_decode(lead: u8, trail: u8) -> Option<char> {
    if !is_lead(lead) || !is_trail(trail) {
        return None;
    }
    tables().decode[table_index(lead, trail)]
}

/// Converts text with `MultiByteToWideChar(932, ...)`, without the round-trip check of the
/// document codec: the duplicate codes (NEC row 13, NEC-selected IBM extensions) convert too.
/// None if a byte cannot be converted. For text that Sakura reads with `CP_SJIS` outside of
/// documents (a key macro file, `CTextInputStream::ReadLineW`).
pub fn decode_cp932_strict(bytes: &[u8]) -> Option<String> {
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            out.push(char::from(b));
            i += 1;
        } else if is_hankata(b) {
            out.push(hankata_char(b));
            i += 1;
        } else {
            out.push(raw_decode(b, *bytes.get(i + 1)?)?);
            i += 2;
        }
    }
    Some(out)
}

/// `MyMultiByteToWideChar_JP` with the round-trip check: None if the code does not write back to
/// itself.
pub(crate) fn roundtrip_decode(lead: u8, trail: u8) -> Option<char> {
    let c = raw_decode(lead, trail)?;
    match encode_char(c) {
        Some(Mapped::Double(l, t)) if l == lead && t == trail => Some(c),
        _ => None,
    }
}

/// `MyWideCharToMultiByte_JP`: the CP932 code of a char, if it has an exact one.
///
/// Difference from Sakura: Windows also approximates some other characters (see the
/// module documentation of `encoding`); we do not.
pub(crate) fn encode_char(c: char) -> Option<Mapped> {
    let value = c as u32;
    if value < 0x80 {
        return Some(Mapped::Single(value as u8));
    }
    if (0xFF61..=0xFF9F).contains(&value) {
        return Some(Mapped::Single((value - 0xFF61) as u8 + 0xA1));
    }
    let &(lead, trail) = tables().encode.get(&c)?;
    Some(Mapped::Double(lead, trail))
}

/// SJIS to JIS X 0208 code (`_mbcjmstojis`). None if the code is outside the JIS X 0208 plane.
pub(crate) fn sjis_to_jis(lead: u8, trail: u8) -> Option<(u8, u8)> {
    let base = match lead {
        0x81..=0x9F => 0x81,
        0xE0..=0xEF => 0xC1,
        _ => return None,
    };
    if !is_trail(trail) {
        return None;
    }
    let j1 = ((lead - base) << 1) + 0x21;
    let (j1, j2) = if trail >= 0x9F {
        (j1 + 1, trail - 0x7E)
    } else {
        (j1, trail - 0x1F - u8::from(trail >= 0x80))
    };
    Some((j1, j2))
}

/// JIS X 0208 code to SJIS (`_mbcjistojms`). None if either byte is outside 0x21..=0x7E.
pub(crate) fn jis_to_sjis(j1: u8, j2: u8) -> Option<(u8, u8)> {
    if !(0x21..=0x7E).contains(&j1) || !(0x21..=0x7E).contains(&j2) {
        return None;
    }
    let mut lead = ((j1 - 0x21) >> 1) + 0x81;
    if lead > 0x9F {
        lead += 0x40;
    }
    let trail = if j1 & 1 == 1 {
        j2 + 0x1F + u8::from(j2 + 0x1F >= 0x7F)
    } else {
        j2 + 0x7E
    };
    Some((lead, trail))
}

/// IBM extension to NEC-selected IBM extension (`SjisFilter_ibm2nec`).
pub(crate) fn ibm_to_nec(code: u16) -> u16 {
    let (c1, c2) = ((code >> 8) as u8, code as u8);
    if !(c1 == 0xFA || c1 == 0xFB || (c1 == 0xFC && c2 <= 0x4B)) {
        return code;
    }
    // (last code of the range, what to subtract); the codes between FA4A and FA54 and between
    // FA58 and FA5B (the ones NEC row 13 and row 2 also have) stay.
    const RANGES: [(u16, u16); 12] = [
        (0xFA49, 0x0B51),
        (0xFA54, 0),
        (0xFA57, 0x0B5B),
        (0xFA5B, 0),
        (0xFA7E, 0x0D1C),
        (0xFA9B, 0x0D1D),
        (0xFAFC, 0x0D1C),
        (0xFB5B, 0x0D5F),
        (0xFB7E, 0x0D1C),
        (0xFB9B, 0x0D1D),
        (0xFBFC, 0x0D1C),
        (0xFFFF, 0x0D5F),
    ];
    let &(_, delta) = RANGES
        .iter()
        .find(|(last, _)| code <= *last)
        .expect("covers u16");
    code - delta
}

/// NEC-selected IBM extension to IBM extension (`SjisFilter_nec2ibm`).
pub(crate) fn nec_to_ibm(code: u16) -> u16 {
    let c1 = (code >> 8) as u8;
    if c1 != 0xED && c1 != 0xEE {
        return code;
    }
    // (last code of the range, what to add)
    const RANGES: [(u16, u16); 12] = [
        (0xED62, 0x0D1C),
        (0xED7E, 0x0D1D),
        (0xEDE0, 0x0D1C),
        (0xEDFC, 0x0D5F),
        (0xEE62, 0x0D1C),
        (0xEE7E, 0x0D1D),
        (0xEEE0, 0x0D1C),
        (0xEEEC, 0x0D5F),
        (0xEEEE, 0),
        (0xEEF8, 0x0B51),
        (0xEEF9, 0),
        (0xFFFF, 0x0B5B),
    ];
    let &(_, delta) = RANGES
        .iter()
        .find(|(last, _)| code <= *last)
        .expect("covers u16");
    code + delta
}

/// Shift_JIS: `CODE_SJIS`.
#[derive(Debug)]
pub(crate) struct ShiftJis;

impl ShiftJis {
    /// The length of the unit that starts at `bytes[i]`: 2 for a double-byte shape, else 1.
    fn unit_len(bytes: &[u8], i: usize) -> usize {
        match bytes.get(i..) {
            Some([lead, trail, ..]) if is_lead(*lead) && is_trail(*trail) => 2,
            _ => 1,
        }
    }
}

impl Codec for ShiftJis {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b < 0x80 {
                out.push(char::from(b));
                i += 1;
            } else if is_hankata(b) {
                out.push(hankata_char(b));
                i += 1;
            } else if Self::unit_len(bytes, i) == 2 {
                match roundtrip_decode(b, bytes[i + 1]) {
                    Some(c) => out.push(c),
                    None => {
                        out.push(stand_in(b));
                        out.push(stand_in(bytes[i + 1]));
                    }
                }
                i += 2;
            } else {
                out.push(stand_in(b));
                i += 1;
            }
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        // Trail bytes are 0x40 or more, so 0x0D and 0x0A are always line endings.
        find_cr_lf(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        // Sakura: NEL, LS and PS become `?` (a lossy conversion), so it does not use them.
        ascii_eol_bytes(eol)
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        match bytes.get(i) {
            None => 1,
            Some(&b) if b < 0x80 || is_hankata(b) => 1,
            Some(_) => Self::unit_len(bytes, i),
        }
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        encode_chars(text, out, |c, out| match encode_char(c) {
            Some(Mapped::Single(b)) => {
                out.push(b);
                true
            }
            Some(Mapped::Double(lead, trail)) => {
                out.extend_from_slice(&[lead, trail]);
                true
            }
            None => false,
        })
    }
}
