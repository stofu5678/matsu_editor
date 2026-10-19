// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/charset.h
//   - sakura_core/charset/charset.cpp
//   - sakura_core/charset/CCodeBase.cpp
//   - sakura_core/basis/CEol.h
//   - sakura_core/basis/CEol.cpp
//   - sakura_core/io/CFileLoad.cpp (EOL scanning)
//
// Modified: rewritten in Rust (not a line-by-line port). The Windows API calls the original
// uses (MultiByteToWideChar and the like) are replaced by tables built from `encoding_rs`,
// and the invalid-byte stand-ins are U+10FF00..=U+10FFFF instead of U+DC00..=U+DCFF.
// Differences from the original behavior are listed in the module documentation of
// `encoding/mod.rs` and in the comments next to the code.
//
// Copyright (C) 2008, kobake
// Copyright (C) 2009, rastiv
// Copyright (C) 2010, Uchi
// Copyright (C) 2012, novice
// Copyright (C) 2013, Moca
// Copyright (C) 2000-2001, genta
// Copyright (C) 2000, Frozen, Moca
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

//! Character encodings of Sakura Editor: conversion, line endings, BOM and detection.
//!
//! The conversion layer behaves like Sakura on every OS and does not use any Windows API.
//! The buffer keeps the file's original bytes and asks a [`Codec`] to decode only the parts it
//! shows or searches (ADR-0005).
//!
//! # Scope
//!
//! The nine encodings of Sakura's `ECodeType` (`sakura_core/charset/charset.h`) are supported
//! as they are in Sakura, and so are the Windows code pages that `encoding_rs` has (GBK, Big5,
//! EUC-KR, the single-byte code pages and so on): [`Encoding::CodePage`]. The code pages and their
//! differences from Windows are listed in `codepage.rs`. `CODE_CPACP`, `CODE_CPOEM` (the system
//! code page) and UTF-32 (`CODE_UTF32LE`, `CODE_UTF32BE`) are out of scope.
//!
//! # Invalid bytes
//!
//! Sakura maps every byte that is invalid in its encoding to U+DC00..=U+DCFF
//! (`CCodeBase::BinToText`) and writes it back as the same byte. A Rust `String` cannot hold
//! those, so [`INVALID_BYTE_BASE`] moves the 256 stand-ins to the last 256 code points of
//! Supplementary Private Use Area-B. A real character in that range in a file (very rare) is
//! written back as a single byte; this is the one case where decode-then-encode is not exact.
//!
//! # Differences from Sakura
//!
//! Differences that remain, with the reason. Each is also commented next to the code.
//!
//! 1. **Stand-ins** are U+10FF00.. instead of U+DC00.. (see above).
//! 2. **Windows best-fit conversions.** Sakura calls `WideCharToMultiByte(932, ...)` and only
//!    rejects the characters it knows are silently approximated (U+3094 and part of
//!    U+00A0..=U+00FF). Characters that Windows approximates without reporting (for example
//!    U+2212 MINUS SIGN to 0x817C, U+203E OVERLINE to 0x7E) are encoded by Sakura on Windows.
//!    We only encode characters that have an exact CP932 mapping and write `?` for the rest, so
//!    that the result does not depend on the OS. The best-fit table is not available outside
//!    Windows.
//! 3. **UTF-16** (also UTF-7 inside its Base64 runs): a lone surrogate cannot be held by a Rust
//!    `String`. In UTF-16 it becomes two stand-ins (the contract). A byte count that is odd
//!    becomes one stand-in for the last byte (Sakura pads a 0x00 and reports a loss). In UTF-7 a
//!    lone surrogate decoded from Base64 becomes U+FFFD.
//! 4. **ISO-2022-JP** (`jis.rs`) writes bytes that Sakura replaces with `?` (invalid bytes, the
//!    swallowed bytes of an unknown escape sequence) as stand-ins, and an odd trailing byte of a
//!    JIS X 0208 block is a stand-in (Sakura drops it). Encoding writes a stand-in as its raw
//!    byte after returning to ASCII (Sakura writes `?`). Decode-then-encode of invalid ISO-2022-JP
//!    keeps the invalid bytes but may add escape sequences.
//! 5. **UTF-7**: a `+` that is followed by neither a Base64 letter nor `-` becomes a stand-in
//!    (Sakura drops it), and a NUL byte does not end decoding (Sakura stops at the first NUL).
//! 6. **Detection** does not call uchardet (an optional DLL of Sakura) and only supports the
//!    Windows code pages of `codepage.rs` when an XML/HTML/`coding:` declaration names them; see
//!    [`detect`].
//! 7. The unverified CRT details: `_mbcjmstojis` is assumed to accept lead bytes 0x81..=0x9F and
//!    0xE0..=0xEF only (the `?` Sakura writes for PUA characters in EUC/JIS follows from this).
//!
//! `char_len` returns 2 for a pair that Sakura consumes as a unit even when both bytes become
//! stand-ins (for example the NEC-selected IBM extension 0xED40 in Shift_JIS, a lone surrogate
//! in UTF-16), so that decoding a range that starts at a boundary gives the same result as
//! decoding the whole.

mod char_code;
mod codepage;
mod cp932;
mod detect;
mod euc;
mod jis;
mod latin1;
mod mime;
mod utf16;
mod utf7;
mod utf8;

use std::fmt;

pub use char_code::{CharCodeOptions, char_code, sakura_utf16_len, try_char_code, unicode_code};
pub use detect::{Certainty, DETECT_SAMPLE_LEN, DetectOptions, Detection, detect};
pub use mime::{base64_value, decode_base64_digits, mime_header_decode_jis};

/// A line ending. Sakura: `EEolType` in sakura_core/basis/CEol.h.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Eol {
    CrLf,
    Lf,
    Cr,
    /// U+0085
    Nel,
    /// U+2028
    Ls,
    /// U+2029
    Ps,
}

impl Eol {
    /// The line ending of new documents when no setting says otherwise: CR LF on Windows, LF on
    /// macOS and Linux (the user's decision of 2026-10-15, ADR-0010). Sakura's own default is
    /// CR LF (`m_eDefaultEoltype = EEolType::cr_and_lf`, `sakura_core/types/CType.cpp`), which is a
    /// Windows habit.
    pub fn platform_default() -> Eol {
        if cfg!(windows) { Eol::CrLf } else { Eol::Lf }
    }

    /// The line ending as text.
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::CrLf => "\r\n",
            Eol::Lf => "\n",
            Eol::Cr => "\r",
            Eol::Nel => "\u{85}",
            Eol::Ls => "\u{2028}",
            Eol::Ps => "\u{2029}",
        }
    }
}

/// The first char of the 256 stand-ins for bytes that are invalid in their encoding.
/// Byte `b` decodes to `char::from_u32(INVALID_BYTE_BASE + b as u32)`, and encoding that char gives
/// `b` back. Sakura maps them to U+DC00..=U+DCFF (`CCodeBase::BinToText`), which a Rust `String`
/// cannot hold, so we use the last 256 code points of Supplementary Private Use Area-B.
pub const INVALID_BYTE_BASE: u32 = 0x10FF00;

/// Reads and writes the bytes of one encoding. The buffer keeps the file's original bytes and asks
/// the codec to decode only the parts it shows or searches (ADR-0005).
pub trait Codec: Send + Sync + std::fmt::Debug {
    /// Decodes `bytes`, which start at a character boundary, and appends the text to `out`.
    /// Each invalid byte becomes one stand-in char (see INVALID_BYTE_BASE). A truncated character
    /// at the end of `bytes` is treated as invalid bytes.
    fn decode(&self, bytes: &[u8], out: &mut String);

    /// Finds the first line ending that starts at or after `from` (a character boundary). Returns
    /// the offset of its first byte and its kind. CR followed by LF is one `CrLf`. A CR as the last
    /// unit of `bytes` is reported as `Cr`; the caller joins it with an LF at the start of the next
    /// chunk. Which kinds an encoding recognizes follows Sakura (`sakura_core/basis/CEol.cpp`).
    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)>;

    /// The bytes of a line ending in this encoding, or None if the encoding cannot write it.
    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]>;

    /// The length in bytes of the character that starts at `bytes[i]` (1 for an invalid byte).
    fn char_len(&self, bytes: &[u8], i: usize) -> usize;

    /// Encodes `text` and appends the bytes to `out`. Stand-ins become their byte again.
    /// Characters this encoding cannot represent are written as Sakura writes them and reported in
    /// the error (their byte offsets in `text`), so the caller can warn and still save.
    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable>;

    /// False for encodings whose meaning depends on earlier bytes (ISO-2022-JP, UTF-7). The buffer
    /// cannot decode a range of those from the middle, so it converts the whole file at load time.
    fn is_stateless(&self) -> bool {
        true
    }
}

/// Characters that could not be encoded: byte offsets into the encoded `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unmappable {
    pub offsets: Vec<usize>,
}

impl fmt::Display for Unmappable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} character(s) cannot be encoded", self.offsets.len())
    }
}

impl std::error::Error for Unmappable {}

/// The encodings of Sakura Editor. Sakura: `ECodeType` in `sakura_core/charset/charset.h`.
///
/// [`Encoding::sakura_code`] is the number that `sakura.ini` stores (`eDefaultCodetype` and
/// others), so settings can be read and written as they are (ADR-0010). A Windows code page is
/// stored by its number, as Sakura does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// Shift_JIS (MS-CP932, Windows-31J): `CODE_SJIS`.
    ShiftJis,
    /// ISO-2022-JP: `CODE_JIS`.
    Jis,
    /// EUC-JP (MS-CP51932 style): `CODE_EUC`.
    EucJp,
    /// UTF-16 little endian ("Unicode"): `CODE_UNICODE`.
    Utf16Le,
    /// UTF-8: `CODE_UTF8`.
    Utf8,
    /// UTF-7: `CODE_UTF7`.
    Utf7,
    /// UTF-16 big endian: `CODE_UNICODEBE`.
    Utf16Be,
    /// CESU-8: `CODE_CESU8`.
    Cesu8,
    /// Latin1 (Windows-1252): `CODE_LATIN1`.
    Latin1,
    /// A Windows code page that `encoding_rs` has, such as 936 (GBK) or 950 (Big5).
    CodePage(CodePage),
}

/// The value Sakura stores for "detect automatically" (`CODE_AUTODETECT`).
pub const AUTODETECT_CODE: i32 = 99;

impl Encoding {
    /// The nine fixed encodings in the order of Sakura's encoding combo box (after "Auto").
    /// Use [`Encoding::choices`] for a list the user can pick from; it also has the code pages.
    pub const ALL: [Encoding; 9] = [
        Encoding::ShiftJis,
        Encoding::Jis,
        Encoding::EucJp,
        Encoding::Latin1,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Utf8,
        Encoding::Cesu8,
        Encoding::Utf7,
    ];

    /// Every encoding a user can choose: Sakura's nine in the order of its combo box, then the
    /// Windows code pages by number. Dialogs and lists take their items from here.
    pub fn choices() -> Vec<Encoding> {
        let code_pages = codepage::CODE_PAGES
            .iter()
            .filter_map(|info| CodePage::new(info.number()))
            .map(Encoding::CodePage);
        Encoding::ALL.into_iter().chain(code_pages).collect()
    }

    /// The number Sakura stores in `sakura.ini` for this encoding (`ECodeType`, or the code page
    /// number).
    pub fn sakura_code(self) -> i32 {
        match self {
            Encoding::ShiftJis => 0,
            Encoding::Jis => 1,
            Encoding::EucJp => 2,
            Encoding::Utf16Le => 3,
            Encoding::Utf8 => 4,
            Encoding::Utf7 => 5,
            Encoding::Utf16Be => 6,
            Encoding::Cesu8 => 7,
            Encoding::Latin1 => 8,
            Encoding::CodePage(code_page) => i32::from(code_page.number()),
        }
    }

    /// The encoding for a number stored by Sakura, or None for the other values
    /// (`CODE_AUTODETECT`, `CODE_CPACP`, `CODE_CPOEM`, UTF-32, code pages that are not supported,
    /// garbage).
    ///
    /// Numbers of `ECodeType` (0 to 8) are the fixed encodings. From `CODE_CODEMAX` up Sakura
    /// stores a Windows code page number: the supported ones are [`Encoding::CodePage`], and the
    /// numbers of code pages that are one of the fixed encodings here are read as those
    /// (932 Shift_JIS, 1200 UTF-16, 1201 UTF-16BE, 1252 Latin1, 65000 UTF-7, 65001 UTF-8).
    pub fn from_sakura_code(code: i32) -> Option<Encoding> {
        if let Some(fixed) = Encoding::ALL
            .into_iter()
            .find(|encoding| encoding.sakura_code() == code)
        {
            return Some(fixed);
        }
        match code {
            932 => Some(Encoding::ShiftJis),
            1200 => Some(Encoding::Utf16Le),
            1201 => Some(Encoding::Utf16Be),
            1252 => Some(Encoding::Latin1),
            65000 => Some(Encoding::Utf7),
            65001 => Some(Encoding::Utf8),
            _ => u16::try_from(code)
                .ok()
                .and_then(CodePage::new)
                .map(Encoding::CodePage),
        }
    }

    /// The short name Sakura shows in the status bar and in lists (`SCodeSet::m_sShort`). For a
    /// code page it is `cp936` (`CCodePage::GetNameShort`, used in the menu bar).
    pub fn short_name(self) -> &'static str {
        match self {
            Encoding::CodePage(code_page) => code_page.info().short,
            Encoding::ShiftJis => "SJIS",
            Encoding::Jis => "JIS",
            Encoding::EucJp => "EUC",
            Encoding::Latin1 => "Latin1",
            Encoding::Utf16Le => "UTF-16",
            Encoding::Utf16Be => "U16BE",
            Encoding::Utf8 => "UTF-8",
            Encoding::Cesu8 => "CESU-8",
            Encoding::Utf7 => "UTF-7",
        }
    }

    /// The name in Sakura's encoding combo box (`SCodeSet::m_sLong`). For a code page it is the
    /// name Sakura shows in the status bar (`CP936`, `CCodePage::GetNameNormal`).
    pub fn combo_name(self) -> &'static str {
        match self {
            Encoding::CodePage(code_page) => code_page.info().normal,
            Encoding::EucJp => "EUC-JP",
            Encoding::Utf16Be => "UTF-16BE",
            other => other.short_name(),
        }
    }

    /// The name for a list the user picks from: [`Encoding::combo_name`], and for a code page
    /// also what it is, such as `CP936 - Simplified Chinese (GBK)`. (Sakura lists the names that
    /// Windows gives to the code pages, which depend on the language of the OS.)
    pub fn list_name(self) -> &'static str {
        match self {
            Encoding::CodePage(code_page) => code_page.info().list,
            other => other.combo_name(),
        }
    }

    /// The status bar text, such as `  [SJIS]` or `  [CP936]` (`CCodeTypeName::Bracket`,
    /// `CCodePage::GetNameBracket`).
    pub fn bracket_name(self) -> String {
        match self {
            Encoding::CodePage(_) => format!("  [{}]", self.combo_name()),
            _ => format!("  [{}]", self.short_name()),
        }
    }

    /// Whether Sakura offers a BOM option for this encoding (`SCodeSet::m_bUseBom`).
    pub fn can_have_bom(self) -> bool {
        matches!(
            self,
            Encoding::Utf16Le
                | Encoding::Utf16Be
                | Encoding::Utf8
                | Encoding::Cesu8
                | Encoding::Utf7
        )
    }

    /// Whether the BOM option is on by default for this encoding (`SCodeSet::m_bIsBomDefOn`).
    pub fn bom_default_on(self) -> bool {
        matches!(self, Encoding::Utf16Le | Encoding::Utf16Be)
    }

    /// Whether this encoding can be chosen as the default encoding (`SCodeSet::m_bCanDefault`).
    pub fn can_be_default(self) -> bool {
        !matches!(self, Encoding::Jis | Encoding::Utf7)
    }

    /// The BOM bytes written for "with BOM" (`CCodeBase::GetBom`): what U+FEFF becomes in this
    /// encoding, or empty if the encoding cannot hold it.
    pub fn bom(self) -> &'static [u8] {
        match self {
            Encoding::Utf8 | Encoding::Cesu8 => b"\xEF\xBB\xBF",
            Encoding::Utf16Le => b"\xFF\xFE",
            Encoding::Utf16Be => b"\xFE\xFF",
            Encoding::Utf7 => b"+/v8-",
            _ => b"",
        }
    }

    /// The codec of this encoding.
    pub fn codec(self) -> &'static dyn Codec {
        match self {
            Encoding::ShiftJis => &cp932::ShiftJis,
            Encoding::Jis => &jis::Jis,
            Encoding::EucJp => &euc::EucJp,
            Encoding::Utf16Le => &utf16::Utf16 { big_endian: false },
            Encoding::Utf16Be => &utf16::Utf16 { big_endian: true },
            Encoding::Utf8 => &utf8::Utf8,
            Encoding::Utf7 => &utf7::Utf7,
            Encoding::Cesu8 => &utf8::Cesu8,
            Encoding::Latin1 => &latin1::Latin1,
            Encoding::CodePage(code_page) => code_page.info(),
        }
    }

    /// The length of the BOM at the start of `bytes` when the file is read as this encoding
    /// (0 if there is none). Sakura decodes the first bytes and checks for U+FEFF
    /// (`CFileLoad::FileOpen`); UTF-7 is limited to the exact `+/v8-`.
    pub fn bom_len(self, bytes: &[u8]) -> usize {
        let bom = self.bom();
        if !bom.is_empty() && bytes.starts_with(bom) {
            bom.len()
        } else {
            0
        }
    }
}

/// A supported Windows code page (see `codepage.rs`). It can only be made from a number that is
/// supported, so [`Encoding::CodePage`] always has a codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CodePage(u16);

impl CodePage {
    /// The code page with this Windows code page number, or None if it is not supported.
    pub fn new(number: u16) -> Option<CodePage> {
        codepage::CodePageInfo::find(number).map(|info| CodePage(info.number()))
    }

    /// The Windows code page number, such as 936.
    pub fn number(self) -> u16 {
        self.0
    }

    fn info(self) -> &'static codepage::CodePageInfo {
        codepage::CodePageInfo::find(self.0).expect("a CodePage always has an info")
    }
}

/// A stand-in char for an invalid byte.
pub(crate) fn stand_in(byte: u8) -> char {
    char::from_u32(INVALID_BYTE_BASE + u32::from(byte)).expect("stand-ins are scalar values")
}

/// The byte a stand-in char stands for, or None for any other char.
pub(crate) fn stand_in_byte(c: char) -> Option<u8> {
    let value = c as u32;
    (INVALID_BYTE_BASE..INVALID_BYTE_BASE + 256)
        .contains(&value)
        .then(|| (value - INVALID_BYTE_BASE) as u8)
}

/// Finds CR, LF or CR LF in an encoding that keeps the bytes 0x0D and 0x0A free for line endings.
pub(crate) fn find_cr_lf(bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
    let rest = bytes.get(from..)?;
    let position = rest.iter().position(|&b| b == b'\r' || b == b'\n')?;
    let at = from + position;
    let eol = match bytes[at] {
        b'\n' => Eol::Lf,
        _ if bytes.get(at + 1) == Some(&b'\n') => Eol::CrLf,
        _ => Eol::Cr,
    };
    Some((at, eol))
}

/// The bytes of CR, LF and CR LF, which are the same in every ASCII-compatible encoding.
pub(crate) fn ascii_eol_bytes(eol: Eol) -> Option<&'static [u8]> {
    match eol {
        Eol::CrLf => Some(b"\r\n"),
        Eol::Lf => Some(b"\n"),
        Eol::Cr => Some(b"\r"),
        Eol::Nel | Eol::Ls | Eol::Ps => None,
    }
}

/// Encodes `text` char by char. `write` appends the bytes of one char and returns false if the
/// encoding cannot represent it, in which case `?` is written (as Sakura does) and the offset is
/// reported. Stand-ins are written as their byte.
pub(crate) fn encode_chars(
    text: &str,
    out: &mut Vec<u8>,
    mut write: impl FnMut(char, &mut Vec<u8>) -> bool,
) -> Result<(), Unmappable> {
    let mut offsets = Vec::new();
    for (offset, c) in text.char_indices() {
        if let Some(byte) = stand_in_byte(c) {
            out.push(byte);
        } else if !write(c, out) {
            out.push(b'?');
            offsets.push(offset);
        }
    }
    if offsets.is_empty() {
        Ok(())
    } else {
        Err(Unmappable { offsets })
    }
}

#[cfg(test)]
mod codepage_tests;
#[cfg(test)]
mod tests;
