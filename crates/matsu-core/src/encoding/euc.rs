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
//!
//! # Code pages 51932 and 20932
//!
//! `FileSaveAs('...', 51932, 0)` and the status bar's `CP51932` are not `CODE_EUC`: a number from
//! `CODE_CODEMAX` up that `::IsValidCodePage` accepts is a Windows code page, and Sakura converts
//! it with `WideCharToMultiByte` and `MultiByteToWideChar` (`CCodeFactory::CreateCodeBase`,
//! `IsValidCodePageEx` in `charset.h`; `CCodePage::UniToCP`, `CCodePage.cpp`), not with `CEuc`.
//! Windows is not here, so MATSU Editor builds both from the tables it already has:
//!
//! - 51932 ([`EucJp`], the same codec as `CODE_EUC`): JIS X 0208 through the CP932 table of
//!   `encoding_rs` (its Shift_JIS decoder and encoder) and the algorithmic Shift_JIS to JIS step.
//!   That covers the half-width katakana (`0x8E`, U+FF61..=U+FF9F), JIS X 0208 (rows 1 to 84),
//!   NEC row 13 (EUC `0xADA1..`; the codes that duplicate JIS X 0208 row 2 are not read, as in Shift_JIS), and the IBM extensions
//!   (CP932 `0xFA40..=0xFC4B`), which are written as the NEC-selected IBM extensions
//!   (`SjisFilter_ibm2nec`, as `CEuc` does) and read from either form. JIS X 0212 (`0x8F`) is
//!   not in 51932.
//! - 20932 ([`EucJp0212`]): everything of 51932 and JIS X 0212 (`0x8F` and two bytes in
//!   `0xA1..=0xFE`). The JIS X 0212 table is the one `encoding_rs` has in its EUC-JP decoder; the
//!   reverse table is made from it once. A character that has both a JIS X 0208 and a JIS X 0212
//!   code is written with the JIS X 0208 one. A JIS X 0212 code is read as its character even when
//!   the character writes back as a JIS X 0208 code.
//!
//! What this cannot say, because Windows was not run: the bytes of the NEC row 13 and the IBM
//! extensions in 20932 (Windows may not have them), and which of two codes Windows writes for a
//! character that has both. Characters that only Windows' best-fit conversion maps are `?` here
//! (the best-fit table is the user's decision, see `known_differences.txt`).

use super::cp932::{
    self, Mapped, hankata_char, ibm_to_nec, is_hankata, jis_to_sjis, nec_to_ibm, sjis_to_jis,
};
use super::{Codec, Eol, Unmappable, ascii_eol_bytes, encode_chars, find_cr_lf, stand_in};
use std::collections::HashMap;
use std::sync::OnceLock;

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
    /// `0x8F` and two bytes in 0xA1..=0xFE (JIS X 0212; only when `with_0212`).
    Zen0212,
    /// Anything else: one invalid byte.
    Invalid,
}

fn unit(bytes: &[u8], i: usize, with_0212: bool) -> Unit {
    match bytes.get(i..) {
        Some([b, ..]) if *b < 0x80 => Unit::Ascii,
        Some([0x8F, a, b, ..]) if with_0212 && is_zen_byte(*a) && is_zen_byte(*b) => Unit::Zen0212,
        Some([lead, trail, ..]) if is_zen_byte(*lead) && is_zen_byte(*trail) => Unit::Zen,
        Some([0x8E, trail, ..]) if is_hankata(*trail) => Unit::Hankata,
        _ => Unit::Invalid,
    }
}

/// EUC-JP: `CODE_EUC`, and the code page 51932.
#[derive(Debug)]
pub(crate) struct EucJp;

/// The code page 20932: [`EucJp`] and JIS X 0212 (see the module documentation).
#[derive(Debug)]
pub(crate) struct EucJp0212;

fn decode_with(bytes: &[u8], out: &mut String, with_0212: bool) {
    let mut i = 0;
    while i < bytes.len() {
        match unit(bytes, i, with_0212) {
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
            Unit::Zen0212 => {
                match jisx0212_to_char(bytes[i + 1], bytes[i + 2]) {
                    Some(c) => out.push(c),
                    None => out.extend(bytes[i..i + 3].iter().map(|&b| stand_in(b))),
                }
                i += 3;
            }
            Unit::Invalid => {
                out.push(stand_in(bytes[i]));
                i += 1;
            }
        }
    }
}

fn char_len_with(bytes: &[u8], i: usize, with_0212: bool) -> usize {
    match unit(bytes, i, with_0212) {
        Unit::Ascii | Unit::Invalid => 1,
        Unit::Hankata | Unit::Zen => 2,
        Unit::Zen0212 => 3,
    }
}

/// Writes `c` as the half-width katakana or a JIS X 0208 (with NEC row 13 and IBM extensions)
/// code; false if it has none.
fn write_0208(c: char, out: &mut Vec<u8>) -> bool {
    match cp932::encode_char(c) {
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
    }
}

impl Codec for EucJp {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        decode_with(bytes, out, false);
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        // Every byte of a multi-byte char is 0x8E or more.
        find_cr_lf(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        ascii_eol_bytes(eol)
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        char_len_with(bytes, i, false)
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        encode_chars(text, out, write_0208)
    }
}

impl Codec for EucJp0212 {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        decode_with(bytes, out, true);
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        find_cr_lf(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        ascii_eol_bytes(eol)
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        char_len_with(bytes, i, true)
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        encode_chars(text, out, |c, out| {
            if write_0208(c, out) {
                return true;
            }
            match jisx0212_table().encode.get(&c) {
                Some(&(a, b)) => {
                    out.extend_from_slice(&[0x8F, a, b]);
                    true
                }
                None => false,
            }
        })
    }
}

/// The JIS X 0212 table of `encoding_rs` (its EUC-JP decoder), both ways.
struct Jisx0212 {
    encode: HashMap<char, (u8, u8)>,
}

fn jisx0212_table() -> &'static Jisx0212 {
    static TABLE: OnceLock<Jisx0212> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut encode = HashMap::new();
        for a in 0xA1..=0xFEu8 {
            for b in 0xA1..=0xFEu8 {
                if let Some(c) = jisx0212_to_char(a, b) {
                    // The first code wins for a character that two codes have.
                    encode.entry(c).or_insert((a, b));
                }
            }
        }
        Jisx0212 { encode }
    })
}

/// The char of a JIS X 0212 code (the two bytes after `0x8F`), if `encoding_rs` has one: exactly
/// one char, no replacement.
fn jisx0212_to_char(a: u8, b: u8) -> Option<char> {
    let bytes = [0x8F, a, b];
    let text = encoding_rs::EUC_JP.decode_without_bom_handling_and_without_replacement(&bytes)?;
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

/// The char of an EUC-JP double-byte code, with Sakura's round-trip check.
pub(crate) fn zen_to_char(lead: u8, trail: u8) -> Option<char> {
    let (s1, s2) = jis_to_sjis(lead & 0x7F, trail & 0x7F)?;
    let [s1, s2] = nec_to_ibm(u16::from_be_bytes([s1, s2])).to_be_bytes();
    cp932::roundtrip_decode(s1, s2)
}
