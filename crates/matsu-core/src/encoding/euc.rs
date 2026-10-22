// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CEuc.cpp
//   - sakura_core/charset/CEuc.h
//   - sakura_core/charset/codechecker.cpp (CheckEucjpChar)
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

//! EUC-JP as Sakura converts it: through Shift_JIS (CP932).
//!
//! Sakura does not have an EUC-JP table of its own. A double-byte char is turned into JIS X 0208,
//! then into Shift_JIS, and the NEC-selected IBM extensions are moved to the IBM extensions
//! (`SjisFilter_nec2ibm`) before the CP932 table is used (`CEuc::_EucjpToUni_char`). The other
//! direction is the reverse (`CEuc::_UniToEucjp_char`). So EUC-JP has exactly the characters of
//! CP932 that have a JIS X 0208 code, and JIS X 0212 (the three-byte `0x8F` chars) is not
//! supported: the `0x8F` is an invalid byte and the two bytes after it are read as a new char.
//! Sakura's own test (`test-ccodebase.cpp`, `codeEucJp`) writes `森鷗外` as `森?外` for this
//! reason.

use super::cp932::{
    self, Mapped, hankata_char, ibm_to_nec, is_hankata, jis_to_sjis, nec_to_ibm, sjis_to_jis,
};
use super::{Codec, Eol, Unmappable, ascii_eol_bytes, encode_chars, find_cr_lf, stand_in};

/// A byte of a double-byte char (`CHARCODE__IS_EUCJP_ZEN1`, `CHARCODE__IS_EUCJP_ZEN2`).
fn is_zen_byte(b: u8) -> bool {
    (0xA1..=0xFE).contains(&b)
}

/// What starts at `bytes[i]`.
enum Unit {
    Ascii,
    /// `0x8E` and a half-width katakana byte.
    Hankata,
    /// Two bytes in 0xA1..=0xFE.
    Zen,
    /// Anything else: one invalid byte.
    Invalid,
}

fn unit(bytes: &[u8], i: usize) -> Unit {
    match bytes.get(i..) {
        Some([b, ..]) if *b < 0x80 => Unit::Ascii,
        Some([lead, trail, ..]) if is_zen_byte(*lead) && is_zen_byte(*trail) => Unit::Zen,
        Some([0x8E, trail, ..]) if is_hankata(*trail) => Unit::Hankata,
        _ => Unit::Invalid,
    }
}

/// EUC-JP: `CODE_EUC`.
#[derive(Debug)]
pub(crate) struct EucJp;

impl Codec for EucJp {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        let mut i = 0;
        while i < bytes.len() {
            match unit(bytes, i) {
                Unit::Ascii => {
                    out.push(char::from(bytes[i]));
                    i += 1;
                }
                Unit::Hankata => {
                    out.push(hankata_char(bytes[i + 1]));
                    i += 2;
                }
                Unit::Zen => {
                    let (lead, trail) = (bytes[i], bytes[i + 1]);
                    match zen_to_char(lead, trail) {
                        Some(c) => out.push(c),
                        None => {
                            out.push(stand_in(lead));
                            out.push(stand_in(trail));
                        }
                    }
                    i += 2;
                }
                Unit::Invalid => {
                    out.push(stand_in(bytes[i]));
                    i += 1;
                }
            }
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        // Every byte of a multi-byte char is 0x8E or more.
        find_cr_lf(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        ascii_eol_bytes(eol)
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        match unit(bytes, i) {
            Unit::Ascii | Unit::Invalid => 1,
            Unit::Hankata | Unit::Zen => 2,
        }
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        encode_chars(text, out, |c, out| match cp932::encode_char(c) {
            Some(Mapped::Single(b)) if b < 0x80 => {
                out.push(b);
                true
            }
            Some(Mapped::Single(b)) => {
                out.extend_from_slice(&[0x8E, b]);
                true
            }
            Some(Mapped::Double(lead, trail)) => {
                let code = ibm_to_nec(u16::from_be_bytes([lead, trail]));
                let [lead, trail] = code.to_be_bytes();
                match sjis_to_jis(lead, trail) {
                    Some((j1, j2)) => {
                        out.extend_from_slice(&[j1 | 0x80, j2 | 0x80]);
                        true
                    }
                    // For example the user-defined area U+E000..: no JIS X 0208 code.
                    None => false,
                }
            }
            None => false,
        })
    }
}

/// The char of an EUC-JP double-byte code, with Sakura's round-trip check.
pub(crate) fn zen_to_char(lead: u8, trail: u8) -> Option<char> {
    let (s1, s2) = jis_to_sjis(lead & 0x7F, trail & 0x7F)?;
    let [s1, s2] = nec_to_ibm(u16::from_be_bytes([s1, s2])).to_be_bytes();
    cp932::roundtrip_decode(s1, s2)
}
