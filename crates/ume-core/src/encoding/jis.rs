// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CJis.cpp
//   - sakura_core/charset/CJis.h
//   - sakura_core/charset/codechecker.cpp (DetectJisEscseq, _CheckJisAnyPart)
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

//! ISO-2022-JP (`CODE_JIS`), which depends on the bytes before it: the whole text is converted at
//! once (`is_stateless` is false).
//!
//! Reading (`CJis::JisToUni`): the text starts in ASCII. `ESC ( B`, `ESC ( J` and `ESC ( H` switch
//! to ASCII (JIS X 0201 Roman is read as ASCII), `ESC ( I` to JIS X 0201 katakana, `ESC $ B` and
//! `ESC $ @` to JIS X 0208. Any other `ESC` and the two bytes after it are not a known escape
//! sequence: they are invalid, and the mode becomes "unknown", where only 0x21..=0x7E is read.
//! (`ESC & @ ESC $ B` is not recognized as one sequence either; `DetectJisEscseq` has dead code
//! for it.) JIS X 0208 is read through Shift_JIS (CP932) without the round-trip check, so the NEC
//! row 13 duplicates and the NEC-selected IBM extensions (rows 89 to 92) are readable.
//!
//! Writing (`CJis::UniToJis`): ASCII, JIS X 0201 katakana (`ESC ( I`) or JIS X 0208 (`ESC $ B`),
//! switching only when the next char needs another set, and `ESC ( B` at the end of the text.
//! A char that cannot be written gives `?` in ASCII, 0x25 (the katakana middle dot) in katakana
//! and 0x2126 (the middle dot) in JIS X 0208, without a switch.

use super::cp932::{self, Mapped, hankata_char, ibm_to_nec, jis_to_sjis, sjis_to_jis};
use super::{Codec, Eol, Unmappable, ascii_eol_bytes, find_cr_lf, stand_in, stand_in_byte};

pub(crate) const ESC: u8 = 0x1B;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Ascii,
    Katakana,
    Kanji,
    /// After an escape sequence that is not known.
    Unknown,
}

/// A piece of the input.
enum Token {
    /// An escape sequence (`len` bytes) that switches the mode and has no text.
    Escape(usize),
    /// `len` bytes that give one char.
    Char(char, usize),
    /// `len` bytes, each an invalid byte.
    Invalid(usize),
}

/// `DetectJisEscseq`: the length of the escape sequence at `bytes[i]` (which is ESC) and the mode
/// it selects, or None for an unknown one.
pub(crate) fn detect_escape(bytes: &[u8], i: usize) -> (usize, Option<Mode>) {
    match bytes.get(i + 1..i + 3) {
        Some(b"(B" | b"(J" | b"(H") => (3, Some(Mode::Ascii)),
        Some(b"(I") => (3, Some(Mode::Katakana)),
        Some(b"$B" | b"$@") => (3, Some(Mode::Kanji)),
        // Two bytes after ESC: an unknown sequence. Fewer: only the ESC is taken.
        Some(_) => (3, None),
        None => (1, None),
    }
}

/// Splits `bytes` into tokens. Calls `f(start, token)`.
fn scan(bytes: &[u8], mut f: impl FnMut(usize, Token)) {
    let mut mode = Mode::Ascii;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let (token, len) = if b == ESC {
            let (len, next) = detect_escape(bytes, i);
            match next {
                Some(next) => {
                    mode = next;
                    (Token::Escape(len), len)
                }
                None => {
                    mode = Mode::Unknown;
                    (Token::Invalid(len), len)
                }
            }
        } else {
            match mode {
                Mode::Ascii if b < 0x80 => (Token::Char(char::from(b), 1), 1),
                Mode::Unknown if (0x21..=0x7E).contains(&b) => (Token::Char(char::from(b), 1), 1),
                // JIS X 0201 katakana is 0x21..=0x5F (0xA1..=0xDF in Shift_JIS).
                Mode::Katakana if (0x21..=0x5F).contains(&b) => {
                    (Token::Char(hankata_char(b | 0x80), 1), 1)
                }
                Mode::Kanji => match bytes.get(i + 1) {
                    // An ESC at either place ends the block (`_CheckJisAnyPart`).
                    Some(&second) if second != ESC => match kanji_char(b, second) {
                        Some(c) => (Token::Char(c, 2), 2),
                        None => (Token::Invalid(2), 2),
                    },
                    _ => (Token::Invalid(1), 1),
                },
                _ => (Token::Invalid(1), 1),
            }
        };
        f(i, token);
        i += len;
    }
}

/// A JIS X 0208 char through CP932, without the round-trip check (`MyMultiByteToWideChar_JP` with
/// `bKeepRt` false).
fn kanji_char(j1: u8, j2: u8) -> Option<char> {
    let (s1, s2) = jis_to_sjis(j1, j2)?;
    cp932::raw_decode(s1, s2)
}

/// ISO-2022-JP: `CODE_JIS`.
#[derive(Debug)]
pub(crate) struct Jis;

impl Codec for Jis {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        scan(bytes, |start, token| match token {
            Token::Escape(_) => {}
            Token::Char(c, _) => out.push(c),
            Token::Invalid(len) => {
                out.extend(bytes[start..start + len].iter().map(|&b| stand_in(b)));
            }
        });
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        // CR and LF are control codes in every mode.
        find_cr_lf(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        ascii_eol_bytes(eol)
    }

    /// The mode depends on the bytes before `i`, so this reads from the start. A valid escape
    /// sequence counts as one char.
    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        let mut found = 1;
        scan(bytes, |start, token| {
            if start == i {
                found = match token {
                    Token::Escape(len) | Token::Char(_, len) | Token::Invalid(len) => len,
                };
            }
        });
        found
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        let mut offsets = Vec::new();
        let mut mode = Mode::Ascii;
        for (offset, c) in text.char_indices() {
            if let Some(byte) = stand_in_byte(c) {
                // The original byte, in ASCII so that it is not taken for the middle of a pair.
                switch_mode(out, &mut mode, Mode::Ascii);
                out.push(byte);
                continue;
            }
            match cp932::encode_char(c) {
                None => {
                    offsets.push(offset);
                    match mode {
                        Mode::Katakana => out.push(0x25),
                        Mode::Kanji => out.extend_from_slice(&[0x21, 0x26]),
                        _ => out.push(b'?'),
                    }
                }
                Some(Mapped::Single(b)) if b < 0x80 => {
                    switch_mode(out, &mut mode, Mode::Ascii);
                    out.push(b);
                }
                Some(Mapped::Single(b)) => {
                    switch_mode(out, &mut mode, Mode::Katakana);
                    out.push(b & 0x7F);
                }
                Some(Mapped::Double(lead, trail)) => {
                    switch_mode(out, &mut mode, Mode::Kanji);
                    let code = ibm_to_nec(u16::from_be_bytes([lead, trail]));
                    let [lead, trail] = code.to_be_bytes();
                    match sjis_to_jis(lead, trail) {
                        Some((j1, j2)) => out.extend_from_slice(&[j1, j2]),
                        None => {
                            offsets.push(offset);
                            out.extend_from_slice(&[0x21, 0x26]);
                        }
                    }
                }
            }
        }
        switch_mode(out, &mut mode, Mode::Ascii);
        if offsets.is_empty() {
            Ok(())
        } else {
            Err(Unmappable { offsets })
        }
    }

    fn is_stateless(&self) -> bool {
        false
    }
}

/// Writes the escape sequence for `to` if the mode changes.
fn switch_mode(out: &mut Vec<u8>, mode: &mut Mode, to: Mode) {
    if *mode == to {
        return;
    }
    out.extend_from_slice(match to {
        Mode::Katakana => b"\x1B(I",
        Mode::Kanji => b"\x1B$B",
        _ => b"\x1B(B",
    });
    *mode = to;
}
