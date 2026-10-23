// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CUtf7.cpp
//   - sakura_core/charset/codechecker.cpp (TABLE_IsUtf7Direct, CheckUtf7DPart, CheckUtf7BPart)
//   - sakura_core/convert/convert_util2.h (_DecodeBase64, _EncodeBase64)
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

//! UTF-7, which depends on the bytes before it: the whole text is converted at once
//! (`is_stateless` is false).
//!
//! Reading (`CUtf7::Utf7ToUni`): the characters of UTF-7 sets D and O are read as they are, `+-`
//! is `+`, and a `+` followed by Base64 letters is a run of UTF-16 big endian that ends at the
//! first other byte; a `-` right after the run belongs to it. Any other byte is invalid.
//!
//! Writing (`CUtf7::UniToUtf7`): the characters of set D are written as they are, a `+` at the
//! start of a stretch is `+-`, and every other stretch becomes `+`, Base64 without padding, `-`.
//! Set O characters (`!"#$%&*;<=>@[]^_` and others) are Base64 encoded like any other character;
//! Sakura does not write them directly.

use super::{Codec, Eol, Unmappable, ascii_eol_bytes, find_cr_lf, stand_in, stand_in_byte};

/// `TABLE_IsUtf7Direct`: 1 for set D, 2 for set O, 0 for the characters that need Base64.
const DIRECT: [u8; 128] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, // 00-0f
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 10-1f
    1, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 0, 1, 1, 1, 1, // 20-2f
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 1, // 30-3f
    2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 40-4f
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 0, 2, 2, 2, // 50-5f
    2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 60-6f
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 0, 0, // 70-7f
];

/// `IsUtf7SetD`
pub(crate) fn is_set_d(c: char) -> bool {
    (c as usize) < 128 && DIRECT[c as usize] == 1
}

/// `IsUtf7SetD || IsUtf7SetO`: read as they are.
pub(crate) fn is_direct_byte(b: u8) -> bool {
    b < 0x80 && DIRECT[usize::from(b)] != 0
}

/// `IsBase64`
pub(crate) fn base64_value(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

const BASE64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `_DecodeBase64`: whole groups of 4 letters give 3 bytes, a last group of 2 or 3 gives 1 or 2.
/// The remaining bits are dropped.
pub(crate) fn decode_base64(letters: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(letters.len() * 3 / 4);
    let mut bits = 0u32;
    let mut count = 0u32;
    for &letter in letters {
        let value = base64_value(letter).expect("Base64 letter");
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            bytes.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    bytes
}

/// `_EncodeBase64`: no padding; the last letter is padded with zero bits.
fn encode_base64(bytes: &[u8], out: &mut Vec<u8>) {
    for group in bytes.chunks(3) {
        let mut value = 0u32;
        for (k, &byte) in group.iter().enumerate() {
            value |= u32::from(byte) << (16 - 8 * k);
        }
        let letters = (group.len() * 8).div_ceil(6);
        for k in 0..letters {
            out.push(BASE64_CHARS[((value >> (18 - 6 * k)) & 0x3F) as usize]);
        }
    }
}

/// What the text at `bytes[i]` is.
enum Token {
    /// A byte read as it is, or `+-`.
    Plain(char, usize),
    /// A byte that is not valid; a `+` that starts nothing.
    Invalid,
    /// `+`, the Base64 letters (`len` is the number of letters) and an optional `-`.
    Run { letters: usize, total: usize },
}

fn token(bytes: &[u8], i: usize) -> Token {
    let b = bytes[i];
    if b != b'+' {
        return if is_direct_byte(b) {
            Token::Plain(char::from(b), 1)
        } else {
            Token::Invalid
        };
    }
    let letters = bytes[i + 1..]
        .iter()
        .take_while(|&&c| base64_value(c).is_some())
        .count();
    let has_minus = bytes.get(i + 1 + letters) == Some(&b'-');
    if letters == 0 {
        // `+-` is `+`. A `+` followed by anything else starts nothing.
        return if has_minus {
            Token::Plain('+', 2)
        } else {
            Token::Invalid
        };
    }
    Token::Run {
        letters,
        total: 1 + letters + usize::from(has_minus),
    }
}

/// UTF-7: `CODE_UTF7`.
#[derive(Debug)]
pub(crate) struct Utf7;

impl Codec for Utf7 {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        let mut i = 0;
        while i < bytes.len() {
            match token(bytes, i) {
                Token::Plain(c, len) => {
                    out.push(c);
                    i += len;
                }
                Token::Invalid => {
                    out.push(stand_in(bytes[i]));
                    i += 1;
                }
                Token::Run { letters, total } => {
                    let data = decode_base64(&bytes[i + 1..i + 1 + letters]);
                    let (pairs, remainder) = data.as_chunks::<2>();
                    let units = pairs.iter().map(|pair| u16::from_be_bytes(*pair));
                    for decoded in char::decode_utf16(units) {
                        // A lone surrogate cannot be held by a String.
                        out.push(decoded.unwrap_or('\u{FFFD}'));
                    }
                    if let [odd] = remainder {
                        out.push(stand_in(*odd));
                    }
                    i += total;
                }
            }
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        // CR and LF are set D, so they are always written as they are. NEL, LS and PS are inside
        // Base64 runs and are not found here.
        find_cr_lf(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        // Sakura: `CCodeBase::GetEol` converts the char, which gives these runs.
        match eol {
            Eol::Nel => Some(b"+AIU-"),
            Eol::Ls => Some(b"+ICg-"),
            Eol::Ps => Some(b"+ICk-"),
            other => ascii_eol_bytes(other),
        }
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        // A Base64 run is one unit: its chars cannot be separated.
        let mut position = 0;
        while position < bytes.len() {
            let len = match token(bytes, position) {
                Token::Plain(_, len) => len,
                Token::Invalid => 1,
                Token::Run { total, .. } => total,
            };
            if position >= i {
                return len;
            }
            position += len;
        }
        1
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        let mut chars = text.chars().peekable();
        while let Some(&c) = chars.peek() {
            if let Some(byte) = stand_in_byte(c) {
                out.push(byte);
                chars.next();
            } else if is_set_d(c) {
                out.push(c as u8);
                chars.next();
            } else if c == '+' {
                out.extend_from_slice(b"+-");
                chars.next();
            } else {
                let mut units = Vec::new();
                while let Some(&c) = chars.peek() {
                    if stand_in_byte(c).is_some() || is_set_d(c) {
                        break;
                    }
                    let mut buffer = [0u16; 2];
                    for &unit in c.encode_utf16(&mut buffer).iter() {
                        units.extend_from_slice(&unit.to_be_bytes());
                    }
                    chars.next();
                }
                out.push(b'+');
                encode_base64(&units, out);
                out.push(b'-');
            }
        }
        Ok(())
    }

    fn is_stateless(&self) -> bool {
        false
    }
}
