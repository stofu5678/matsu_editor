// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files (the names, numbers and stored form of the code pages only):
//   - sakura_core/charset/CCodePage.cpp
//   - sakura_core/charset/CCodePage.h
//   - sakura_core/charset/charset.h
//
// Modified: rewritten in Rust (not a line-by-line port). The Windows API calls the original
// uses (MultiByteToWideChar and WideCharToMultiByte) are replaced by `encoding_rs`, the invalid-byte
// stand-ins are U+10FF00..=U+10FFFF instead of U+DC00..=U+DCFF, and only the code pages that
// `encoding_rs` has are supported.
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

//! Windows code pages through `encoding_rs`.
//!
//! Sakura reads and writes any installed Windows code page (`CCodePage`) and stores it by its
//! number (`ECodeType` values from `CODE_CODEMAX` up are code page numbers). We do not call
//! Windows, so the same files open the same way on every OS. `encoding_rs` implements the
//! WHATWG Encoding Standard, which is close to the Windows code pages but not the same.
//!
//! # Mapping and known differences
//!
//! - 874, 1250, 1251, 1253 to 1258 are `windows-NNN`. 1252 is not here: it is Sakura's Latin1
//!   ([`super::Encoding::Latin1`]), which `Encoding::from_sakura_code` returns for the number 1252.
//! - 866 is IBM866, 20866 KOI8-R, 21866 KOI8-U (the WHATWG KOI8-U table).
//! - 28592 to 28598 are ISO-8859-2 to -8 (28598 and 38598 decode the same; the standard only
//!   differs in the direction of display). WHATWG ISO-8859-7 is the 2003 version, Windows 28597
//!   is older (it differs at a few bytes such as 0xA4, 0xA5 and 0xAA).
//! - 28599 (ISO-8859-9) is read as windows-1254, as the WHATWG standard does. The two only differ
//!   in 0x80..=0x9F, which ISO-8859-9 keeps as C1 controls. 28591 (ISO-8859-1) is not here for
//!   the same reason; use Latin1.
//! - 28603 and 28605 (ISO-8859-13, -15) are Windows code pages. 28600, 28604 and 28606
//!   (ISO-8859-10, -14, -16) follow the numbering "28590 + the part number"; Windows has no such
//!   code pages, so these numbers are only used by UME Editor.
//! - 936 is GBK in `encoding_rs`, which decodes like GB18030: it also accepts the four-byte
//!   sequences and 0x80 as the euro sign. Windows 936 has no four-byte sequences.
//! - 950 (Big5) in `encoding_rs` includes the Hong Kong extension and the ETEN extension; Windows 950
//!   does not. Writing follows Windows: the characters that only those extensions have become `?`
//!   (`is_beyond_windows_950`). Reading is the WHATWG table. Some
//!   byte pairs decode to two characters (Big5 U+00CA U+0304 and the like).
//! - 949 is the UHC table (the same as Windows 949). 51949 (EUC-KR) has no strict form in
//!   `encoding_rs`; it is read and written as 949. Windows 51949 has no UHC extension.
//! - 54936 is GB18030.
//! - 10000 is `macintosh`. 10007 and 10017 are both `x-mac-cyrillic`, which is the Ukrainian
//!   variant (it has U+0490 and U+0491). Windows 10007 differs from it at a few bytes.
//! - No code page is added that `encoding_rs` lacks (437, 850, 1026, 20932, ISO-2022-KR, and so
//!   on). `CODE_CPACP` and `CODE_CPOEM` (the system code page) are not supported; on macOS and
//!   Linux the system code page is UTF-8 anyway.
//!
//! Where the standard and Windows differ, a file written here can differ from what Sakura writes
//! for the same text. Bytes that the table does not map are kept as they are (stand-ins).
//!
//! # Contract
//!
//! Every code page here keeps ASCII as it is (0x00..=0x7F), so CR and LF are found by scanning
//! for the bytes. The decoder works one character at a time when the range is not valid as a
//! whole: a lead byte with a trail byte that the table does not map is one stand-in for the lead
//! byte and decoding goes on with the next byte (so the trail byte can start a character).
//! `char_len` follows the same rule, so decoding a range gives the same result as decoding the
//! whole.

use std::borrow::Cow;

use super::{Codec, Eol, Unmappable, ascii_eol_bytes, find_cr_lf, stand_in, stand_in_byte};

/// How the characters of a code page are made of bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    /// One byte per character.
    Single,
    /// One byte, or a lead byte (0x81..=0xFE) and a trail byte.
    Double,
    /// As `Double`, and four bytes (lead, 0x30..=0x39, 0x81..=0xFE, 0x30..=0x39).
    Gb,
}

/// A code page that UME Editor can read and write: a number, the table, and its names.
pub(crate) struct CodePageInfo {
    number: u16,
    encoding: &'static encoding_rs::Encoding,
    shape: Shape,
    /// `CP936`: `CCodePage::GetNameNormal`.
    pub(crate) normal: &'static str,
    /// `cp936`: `CCodePage::GetNameShort`.
    pub(crate) short: &'static str,
    /// For lists: the Normal name and what it is.
    pub(crate) list: &'static str,
}

macro_rules! code_pages {
    ($($number:literal $encoding:ident $shape:ident $description:literal;)*) => {
        /// All supported code pages, sorted by number.
        pub(crate) static CODE_PAGES: &[CodePageInfo] = &[$(
            CodePageInfo {
                number: $number,
                encoding: encoding_rs::$encoding,
                shape: Shape::$shape,
                normal: concat!("CP", stringify!($number)),
                short: concat!("cp", stringify!($number)),
                list: concat!("CP", stringify!($number), " - ", $description),
            },
        )*];
    };
}

code_pages! {
    866 IBM866 Single "Cyrillic (DOS)";
    874 WINDOWS_874 Single "Thai (Windows)";
    936 GBK Gb "Simplified Chinese (GBK)";
    949 EUC_KR Double "Korean (UHC)";
    950 BIG5 Double "Traditional Chinese (Big5)";
    1250 WINDOWS_1250 Single "Central European (Windows)";
    1251 WINDOWS_1251 Single "Cyrillic (Windows)";
    1253 WINDOWS_1253 Single "Greek (Windows)";
    1254 WINDOWS_1254 Single "Turkish (Windows)";
    1255 WINDOWS_1255 Single "Hebrew (Windows)";
    1256 WINDOWS_1256 Single "Arabic (Windows)";
    1257 WINDOWS_1257 Single "Baltic (Windows)";
    1258 WINDOWS_1258 Single "Vietnamese (Windows)";
    10000 MACINTOSH Single "Western European (Mac)";
    10007 X_MAC_CYRILLIC Single "Cyrillic (Mac)";
    10017 X_MAC_CYRILLIC Single "Ukrainian (Mac)";
    20866 KOI8_R Single "Cyrillic (KOI8-R)";
    21866 KOI8_U Single "Ukrainian (KOI8-U)";
    28592 ISO_8859_2 Single "Central European (ISO-8859-2)";
    28593 ISO_8859_3 Single "Latin 3 (ISO-8859-3)";
    28594 ISO_8859_4 Single "Baltic (ISO-8859-4)";
    28595 ISO_8859_5 Single "Cyrillic (ISO-8859-5)";
    28596 ISO_8859_6 Single "Arabic (ISO-8859-6)";
    28597 ISO_8859_7 Single "Greek (ISO-8859-7)";
    28598 ISO_8859_8 Single "Hebrew (ISO-8859-8)";
    28599 WINDOWS_1254 Single "Turkish (ISO-8859-9)";
    28600 ISO_8859_10 Single "Nordic (ISO-8859-10)";
    28603 ISO_8859_13 Single "Baltic (ISO-8859-13)";
    28604 ISO_8859_14 Single "Celtic (ISO-8859-14)";
    28605 ISO_8859_15 Single "Latin 9 (ISO-8859-15)";
    28606 ISO_8859_16 Single "South-Eastern European (ISO-8859-16)";
    38598 ISO_8859_8 Single "Hebrew (ISO-8859-8-I)";
    51949 EUC_KR Double "Korean (EUC-KR)";
    54936 GB18030 Gb "Chinese (GB18030)";
}

/// How many bytes of text one call of the encoder gets.
const ENCODE_PIECE: usize = 1024;

impl CodePageInfo {
    /// The info for a code page number, or None if it is not supported.
    pub(crate) fn find(number: u16) -> Option<&'static CodePageInfo> {
        CODE_PAGES
            .binary_search_by_key(&number, |info| info.number)
            .ok()
            .map(|index| &CODE_PAGES[index])
    }

    pub(crate) fn number(&self) -> u16 {
        self.number
    }

    /// Decodes `bytes` as a whole, or None if any of it is not valid.
    fn decode_strict<'a>(&self, bytes: &'a [u8]) -> Option<Cow<'a, str>> {
        self.encoding
            .decode_without_bom_handling_and_without_replacement(bytes)
    }

    /// The length of the character at `bytes[i]` and its text, or None for an invalid byte.
    fn next_char<'a>(&self, bytes: &'a [u8], i: usize) -> (usize, Option<Cow<'a, str>>) {
        if bytes[i] < 0x80 {
            return (1, self.decode_strict(&bytes[i..=i]));
        }
        // 0x80 (the euro sign in GBK and GB18030) and 0xFF are single bytes, not lead bytes.
        let is_lead = (0x81..=0xFE).contains(&bytes[i]);
        let long = match self.shape {
            Shape::Single => None,
            _ if !is_lead => None,
            Shape::Double => Some(2),
            Shape::Gb if matches!(bytes.get(i + 1), Some(0x30..=0x39)) => Some(4),
            Shape::Gb => Some(2),
        };
        if let Some(len) = long
            && let Some(slice) = bytes.get(i..i + len)
            && let Some(text) = self.decode_strict(slice)
        {
            return (len, Some(text));
        }
        (1, self.decode_strict(&bytes[i..=i]))
    }

    /// Appends the bytes of `segment`, which holds no stand-ins. `base` is the offset of the
    /// segment in the whole text.
    fn encode_segment(
        &self,
        segment: &str,
        base: usize,
        out: &mut Vec<u8>,
        offsets: &mut Vec<usize>,
    ) {
        if self.number != 950 {
            return self.encode_run(segment, base, out, offsets);
        }
        // Code page 950: the characters that only the WHATWG table has are `?`, as in Windows.
        let mut run_start = 0;
        for (offset, c) in segment.char_indices() {
            if c.is_ascii() || !is_beyond_windows_950(c) {
                continue;
            }
            self.encode_run(&segment[run_start..offset], base + run_start, out, offsets);
            out.push(b'?');
            offsets.push(base + offset);
            run_start = offset + c.len_utf8();
        }
        self.encode_run(&segment[run_start..], base + run_start, out, offsets);
    }

    /// `encode_segment` for text the encoder takes as it is.
    fn encode_run(&self, segment: &str, base: usize, out: &mut Vec<u8>, offsets: &mut Vec<usize>) {
        let mut encoder = self.encoding.new_encoder();
        let mut pos = 0;
        while pos < segment.len() {
            // Encode a bounded piece: the encoder stops at every unmappable character, and the
            // buffer is sized for the whole input of a call.
            let mut end = (pos + ENCODE_PIECE).min(segment.len());
            while !segment.is_char_boundary(end) {
                end += 1;
            }
            let rest = &segment[pos..end];
            let capacity = encoder
                .max_buffer_length_from_utf8_without_replacement(rest.len())
                .unwrap_or(rest.len() * 4 + 16);
            let start = out.len();
            out.resize(start + capacity, 0);
            let (result, read, written) =
                encoder.encode_from_utf8_without_replacement(rest, &mut out[start..], true);
            out.truncate(start + written);
            pos += read;
            if let encoding_rs::EncoderResult::Unmappable(c) = result {
                // Sakura writes `?` for what the code page cannot hold (as for the other encodings).
                out.push(b'?');
                offsets.push(base + pos - c.len_utf8());
            }
        }
    }
}

/// Whether the Big5 table of `encoding_rs` writes `c` with bytes that Windows' code page 950 does
/// not have: the Hong Kong extension (lead bytes 0xFA..=0xFE, which Windows keeps for end-user
/// characters) and the ETEN extension (0xC6A1..=0xC8FE: kana, Cyrillic and so on). Windows writes
/// `?` for them (the oracle case `savezen-utf8-to-big5`: Sakura writes `?` for hiragana and
/// katakana). The rest of the table is the same as Windows'. An approximation without Windows'
/// table: it was not compared with the whole of it (the user, 2026-10-22: "follow Windows' table,
/// not urgent").
fn is_beyond_windows_950(c: char) -> bool {
    let mut buf = [0u8; 8];
    let mut text = [0u8; 4];
    let text = c.encode_utf8(&mut text);
    let (result, _, written) = encoding_rs::BIG5
        .new_encoder()
        .encode_from_utf8_without_replacement(text, &mut buf, true);
    if result != encoding_rs::EncoderResult::InputEmpty || written != 2 {
        return false;
    }
    let (lead, trail) = (buf[0], buf[1]);
    lead >= 0xFA || lead == 0xC7 || lead == 0xC8 || (lead == 0xC6 && trail >= 0xA1)
}

impl std::fmt::Debug for CodePageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.normal)
    }
}

impl Codec for CodePageInfo {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        if let Some(text) = self.decode_strict(bytes) {
            out.push_str(&text);
            return;
        }
        let mut i = 0;
        while i < bytes.len() {
            let (len, text) = self.next_char(bytes, i);
            match text {
                Some(text) => out.push_str(&text),
                None => out.push(stand_in(bytes[i])),
            }
            i += len;
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        find_cr_lf(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        ascii_eol_bytes(eol)
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        self.next_char(bytes, i).0
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        let mut offsets = Vec::new();
        let mut segment_start = 0;
        for (offset, c) in text.char_indices() {
            if let Some(byte) = stand_in_byte(c) {
                self.encode_segment(
                    &text[segment_start..offset],
                    segment_start,
                    out,
                    &mut offsets,
                );
                out.push(byte);
                segment_start = offset + c.len_utf8();
            }
        }
        self.encode_segment(&text[segment_start..], segment_start, out, &mut offsets);
        if offsets.is_empty() {
            Ok(())
        } else {
            Err(Unmappable { offsets })
        }
    }
}
