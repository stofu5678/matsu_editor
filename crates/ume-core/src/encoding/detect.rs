// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CESI.cpp
//   - sakura_core/charset/CESI.h
//   - sakura_core/charset/CCodeMediator.cpp
//   - sakura_core/charset/codechecker.cpp
//   - sakura_core/io/CFileLoad.cpp (the length of the sample)
//
// Modified: rewritten in Rust (not a line-by-line port). The Windows API calls the original
// uses (MultiByteToWideChar and the like) are replaced by tables built from `encoding_rs`,
// and the invalid-byte stand-ins are U+10FF00..=U+10FFFF instead of U+DC00..=U+DCFF.
// Differences from the original behavior are listed in the module documentation of
// `encoding/mod.rs` and in the comments next to the code.
//
// Copyright (C) 2006
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

//! Automatic detection of the encoding of a file: Sakura's `CCodeMediator::CheckKanjiCode`,
//! which runs `CESI::CheckKanjiCode`.
//!
//! The order is the same as in Sakura:
//!
//! 1. A BOM (UTF-8, UTF-16 BE, UTF-16 LE; a UTF-7 BOM is not used).
//! 2. A declaration in the text: `<?xml ... encoding="..."?>`, then `coding: ...` or
//!    `coding= ...` in a comment of the first two lines, then `<meta charset=...>`.
//! 3. UTF-16 without a BOM: if only one byte order has line breaks and no invalid unit, and the
//!    lines are 200 bytes long on average or less.
//! 4. Scoring of Shift_JIS, ISO-2022-JP, EUC-JP, UTF-8, UTF-7, CESU-8 and Latin1: the encoding
//!    with the most points (the number of bytes that only that encoding uses, minus the invalid
//!    bytes) wins when the invalid bytes are less than 0.05% of the sample and it has points.
//! 5. Otherwise the default encoding.
//!
//! # Differences from Sakura
//!
//! - Sakura first asks `uchardet` when its DLL exists (`CharsetDetector`) and only falls back to
//!   the above when it does not know. That DLL is optional and is not ported.
//! - A declaration can name a Windows code page (`windows-1251` and the like). Sakura then reads
//!   the file in that code page. The code pages of `codepage.rs` are read as
//!   [`Encoding::CodePage`]. For a code page without a table (437, 850 and so on) the result is
//!   the default encoding and [`Detection::unsupported_code_page`] tells the caller what was
//!   declared.
//! - Sakura reads at most 32768 bytes for opening a file (`CFileLoad::m_nAutoDetectReadLen`;
//!   the file properties dialog uses 16384). [`detect`] looks at the first
//!   [`DETECT_SAMPLE_LEN`] bytes of what it is given.

use super::Encoding;
use super::cp932;
use super::euc::zen_to_char;
use super::jis::{ESC, Mode, detect_escape};
use super::utf7::{base64_value, decode_base64, is_direct_byte, is_set_d};
use super::utf8::{CharKind, check_cesu8_char, check_utf8_char};

/// How many bytes from the start of the file [`detect`] looks at (`CFileLoad::m_nAutoDetectReadLen`).
pub const DETECT_SAMPLE_LEN: usize = 32768;

/// The settings that detection reads (`SEncodingConfig` in `sakura_core/types/CType.h`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectOptions {
    /// The encoding when nothing decides (`m_eDefaultCodetype`). Sakura's default is UTF-8.
    pub default_encoding: Encoding,
    /// Prefer CESU-8 over UTF-8 when both score the same ("CESU-8 を判別する", `m_bPriorCesu8`).
    /// Sakura's default is off.
    pub prefer_cesu8: bool,
}

impl Default for DetectOptions {
    fn default() -> Self {
        DetectOptions {
            default_encoding: Encoding::Utf8,
            prefer_cesu8: false,
        }
    }
}

/// How sure the detection is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Certainty {
    /// A BOM at the start of the file.
    Bom,
    /// The text says it (an XML declaration, `coding:`, `<meta charset>`).
    Declared,
    /// Scoring picked it (UTF-16 line breaks, or the points of the other encodings).
    Scored,
    /// Nothing decided (an empty file, ASCII only, too many invalid bytes): the default encoding.
    Default,
}

/// The result of [`detect`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detection {
    pub encoding: Encoding,
    /// Whether the file starts with the BOM of `encoding`.
    pub bom: bool,
    pub certainty: Certainty,
    /// The Windows code page number that the text declares when it is not one of
    /// [`Encoding`] (a code page without a table). `encoding` is then the default encoding.
    pub unsupported_code_page: Option<u32>,
}

/// What a declaration names.
#[derive(Clone, Copy)]
enum Declared {
    Known(Encoding),
    CodePage(u32),
}

/// Detects the encoding from the start of a file.
pub fn detect(bytes: &[u8], options: &DetectOptions) -> Detection {
    let sample = &bytes[..bytes.len().min(DETECT_SAMPLE_LEN)];
    let known = |encoding, certainty| Detection {
        encoding,
        bom: encoding.bom_len(sample) > 0,
        certainty,
        unsupported_code_page: None,
    };
    let default = || known(options.default_encoding, Certainty::Default);
    if sample.is_empty() {
        return default();
    }
    if let Some(encoding) = detect_bom(sample) {
        return known(encoding, Certainty::Bom);
    }

    let info = Info::collect(sample, options);
    match info.declared {
        Some(Declared::Known(encoding)) => return known(encoding, Certainty::Declared),
        Some(Declared::CodePage(code_page)) => {
            return Detection {
                unsupported_code_page: Some(code_page),
                ..known(options.default_encoding, Certainty::Declared)
            };
        }
        None => {}
    }
    if let Some(encoding) = info.detect_unicode() {
        return known(encoding, Certainty::Scored);
    }
    if let Some(encoding) = info.detect_multibyte() {
        return known(encoding, Certainty::Scored);
    }
    default()
}

/// `CESI::DetectUnicodeBom`
fn detect_bom(s: &[u8]) -> Option<Encoding> {
    if s.len() < 2 {
        return None;
    }
    // UTF-32 first: the BOM of UTF-32 LE starts with the BOM of UTF-16 LE. Sakura's own function
    // knows only UTF-8 and UTF-16 (UTF-32 only comes from uchardet), but a file that UME wrote
    // itself with a UTF-32 BOM must not open as UTF-16 full of NULs (the user, 2026-10-21: do not
    // keep bug-like behavior of Sakura).
    if s.starts_with(b"\xFF\xFE\x00\x00") {
        Some(Encoding::Utf32Le)
    } else if s.starts_with(b"\x00\x00\xFE\xFF") {
        Some(Encoding::Utf32Be)
    } else if s.starts_with(b"\xEF\xBB\xBF") {
        Some(Encoding::Utf8)
    } else if s.starts_with(b"\xFE\xFF") {
        Some(Encoding::Utf16Be)
    } else if s.starts_with(b"\xFF\xFE") {
        Some(Encoding::Utf16Le)
    } else {
        None
    }
}

/// "Invalid" for an encoding that cannot read the sample (`INT_MIN` in Sakura).
const REJECTED: i64 = i32::MIN as i64;

/// The score of one encoding (`MBCODE_INFO`, `WCCODE_INFO`).
#[derive(Clone, Copy, Debug)]
struct Score {
    encoding: Encoding,
    /// For the multibyte encodings the bytes only this encoding uses; for UTF-16 the line breaks.
    specific: i64,
    /// For the multibyte encodings `specific` minus the invalid bytes; for UTF-16 the same as
    /// `specific`, or `REJECTED`.
    points: i64,
}

impl Score {
    fn new(encoding: Encoding, specific: i64, points: i64) -> Score {
        Score {
            encoding,
            specific,
            points,
        }
    }
}

/// Everything `CESI::ScanCode` collects.
struct Info {
    len: usize,
    /// In the order of `gm_aMbcPriority` and then sorted by points.
    multibyte: Vec<Score>,
    utf16_le: Score,
    utf16_be: Score,
    declared: Option<Declared>,
}

impl Info {
    fn collect(s: &[u8], options: &DetectOptions) -> Info {
        let (sjis, _hankata) = score_sjis(s);
        let jis = score_jis(s);
        let (euc, euc_zen, euc_hirakata) = score_euc(s);
        let utf8 = score_utf8(s);
        let utf7 = score_utf7(s);
        let cesu8 = score_cesu8(s);
        let latin1 = Score::new(Encoding::Latin1, 0, -(s.len() as i64));
        let (utf16_le, utf16_be) = score_utf16(s);

        // The order of `gm_aMbcPriority`: UTF-8 and CESU-8 come first so that they win a tie with
        // Shift_JIS.
        let mut multibyte = vec![utf8, cesu8, sjis, euc, jis, utf7, latin1];
        sort_by_points(&mut multibyte);
        guess_euc_or_sjis(&mut multibyte, euc_zen, euc_hirakata);
        guess_utf8_or_cesu8(&mut multibyte, options.prefer_cesu8);

        Info {
            len: s.len(),
            multibyte,
            utf16_le,
            utf16_be,
            declared: detect_declared(s),
        }
    }

    /// `DetectUnicode` (with `GuessUtf16Bom`).
    fn detect_unicode(&self) -> Option<Encoding> {
        let (le, be) = (self.utf16_le, self.utf16_be);
        let byte_order = if le.specific > be.specific && be.specific < 1 {
            le
        } else if le.specific < be.specific && le.specific < 1 {
            be
        } else {
            return None;
        };
        if byte_order.specific - byte_order.points > 0 {
            return None;
        }
        // Lines that are 200 bytes or longer on average: not UTF-16.
        if self.len as f64 / byte_order.specific as f64 > 200.0 {
            return None;
        }
        Some(byte_order.encoding)
    }

    /// `DetectMBCode`
    fn detect_multibyte(&self) -> Option<Encoding> {
        let top = self.multibyte[0];
        // The invalid bytes have to be less than 0.05% of the sample.
        if (self.len as i64) < (top.specific - top.points) * 2000 {
            return None;
        }
        if top.points <= 0 {
            return None;
        }
        Some(top.encoding)
    }
}

/// `SortMBCInfo`: a bubble sort by points, descending. It keeps the order of equal scores.
fn sort_by_points(scores: &mut [Score]) {
    for i in 1..scores.len() {
        for j in 0..scores.len() - i {
            if scores[j].points < scores[j + 1].points {
                scores.swap(j, j + 1);
            }
        }
    }
}

/// `IsAmbiguousEucAndSjis` and `GuessEucOrSjis`.
fn guess_euc_or_sjis(scores: &mut [Score], euc_zen: i64, euc_hirakata: i64) {
    let (a, b) = (scores[0], scores[1]);
    let pair = matches!(
        (a.encoding, b.encoding),
        (Encoding::ShiftJis, Encoding::EucJp) | (Encoding::EucJp, Encoding::ShiftJis)
    );
    // 0.25 is Sakura's own "arbitrary" value.
    if pair
        && a.points == b.points
        && euc_hirakata as f64 / euc_zen as f64 >= 0.25
        && b.encoding == Encoding::EucJp
    {
        scores.swap(0, 1);
    }
}

/// `IsAmbiguousUtf8AndCesu8` and `GuessUtf8OrCesu8`.
fn guess_utf8_or_cesu8(scores: &mut [Score], prefer_cesu8: bool) {
    let (a, b) = (scores[0], scores[1]);
    let pair = matches!(
        (a.encoding, b.encoding),
        (Encoding::Utf8, Encoding::Cesu8) | (Encoding::Cesu8, Encoding::Utf8)
    );
    if pair && a.points == b.points && prefer_cesu8 && b.encoding == Encoding::Cesu8 {
        scores.swap(0, 1);
    }
}

/// `CESI::GetEncodingInfo_sjis`: the score and the number of half-width katakana bytes.
fn score_sjis(s: &[u8]) -> (Score, i64) {
    let (mut encoded, mut invalid, mut hankata) = (0, 0, 0);
    let mut i = 0;
    while i < s.len() {
        let b = s[i];
        if b < 0x80 {
            i += 1;
        } else if cp932::is_hankata(b) {
            encoded += 1;
            hankata += 1;
            i += 1;
        } else if i + 1 < s.len() && cp932::is_lead(b) && cp932::is_trail(s[i + 1]) {
            encoded += 2;
            if cp932::roundtrip_decode(b, s[i + 1]).is_none() {
                invalid += 2;
            }
            i += 2;
        } else {
            // A lead byte at the very end may continue in the next part of the file.
            if s.len() - i < if cp932::is_lead(b) { 2 } else { 1 } {
                break;
            }
            invalid += 1;
            i += 1;
        }
    }
    (
        Score::new(Encoding::ShiftJis, encoded, encoded - invalid),
        hankata,
    )
}

/// `_CheckJisAnyPart`: the length of the block that starts at `s[0]` (up to the next escape
/// sequence), where the next block starts, the mode of the next block (None if there is no escape
/// sequence) and the number of invalid bytes.
fn check_jis_part(s: &[u8], mode: Mode) -> (usize, usize, Option<Mode>, i64) {
    let is_jis = |b: u8| (0x21..=0x7E).contains(&b);
    let mut errors = 0;
    let mut pr = 0;
    while pr < s.len() {
        if s[pr] == ESC {
            let (len, next) = detect_escape(s, pr);
            return (pr, pr + len, Some(next.unwrap_or(Mode::Unknown)), errors);
        }
        let b = s[pr];
        match mode {
            Mode::Ascii => errors += i64::from(b >= 0x80),
            Mode::Katakana => errors += i64::from(!is_jis(b)),
            Mode::Kanji => {
                if pr % 2 == 1 && !(is_jis(s[pr - 1]) && is_jis(b)) {
                    errors += 2;
                }
            }
            Mode::Unknown => errors += i64::from(!is_jis(b)),
        }
        pr += 1;
    }
    (pr, pr, None, errors)
}

/// `CESI::GetEncodingInfo_jis`
fn score_jis(s: &[u8]) -> Score {
    let (mut escape_bytes, mut invalid) = (0, 0);
    let mut mode = Mode::Ascii;
    let mut pos = 0;
    loop {
        let (len, next, next_mode, errors) = check_jis_part(&s[pos..], mode);
        escape_bytes += (next - len) as i64;
        invalid += errors;
        pos += next;
        if let Some(next_mode) = next_mode {
            mode = next_mode;
        }
        if pos >= s.len() {
            break;
        }
    }
    if invalid > 0 {
        Score::new(Encoding::Jis, 0, REJECTED)
    } else {
        Score::new(Encoding::Jis, escape_bytes, escape_bytes)
    }
}

/// `CESI::GetEncodingInfo_eucjp`: the score, the bytes of double-byte chars and the bytes of
/// hiragana and katakana among them.
fn score_euc(s: &[u8]) -> (Score, i64, i64) {
    let is_zen = |b: u8| (0xA1..=0xFE).contains(&b);
    let (mut encoded, mut invalid, mut zen, mut hirakata) = (0, 0, 0, 0);
    let mut i = 0;
    while i < s.len() {
        let b = s[i];
        if b < 0x80 {
            i += 1;
            continue;
        }
        let rest = s.len() - i;
        // (length, whether Sakura cannot convert it, whether it is hiragana or katakana)
        let unit: Option<(i64, bool, bool)> = if rest > 1 && is_zen(b) && is_zen(s[i + 1]) {
            let hiragana_katakana = (b == 0xA4 && (0xA1..=0xF3).contains(&s[i + 1]))
                || (b == 0xA5 && (0xA1..=0xF6).contains(&s[i + 1]));
            Some((
                2,
                !hiragana_katakana && zen_to_char(b, s[i + 1]).is_none(),
                hiragana_katakana,
            ))
        } else if rest > 1 && b == 0x8E && cp932::is_hankata(s[i + 1]) {
            Some((2, false, false))
        } else if rest > 2 && b == 0x8F && is_zen(s[i + 1]) && is_zen(s[i + 2]) {
            // JIS X 0212: Sakura cannot convert it.
            Some((3, true, false))
        } else {
            None
        };
        match unit {
            Some((len, unconvertible, hiragana_katakana)) => {
                encoded += len;
                zen += len;
                if hiragana_katakana {
                    hirakata += 2;
                } else if unconvertible {
                    invalid += len;
                }
                i += len as usize;
            }
            None => {
                let wanted = match b {
                    0x8F => 3,
                    0x8E | 0xA1..=0xFE => 2,
                    _ => 1,
                };
                if rest < wanted {
                    break;
                }
                invalid += 1;
                i += 1;
            }
        }
    }
    (
        Score::new(Encoding::EucJp, encoded, encoded - invalid),
        zen,
        hirakata,
    )
}

/// `CESI::GetEncodingInfo_utf8`
fn score_utf8(s: &[u8]) -> Score {
    let (mut encoded, mut invalid) = (0, 0);
    let mut i = 0;
    while i < s.len() {
        let (len, kind) = check_utf8_char(&s[i..], true);
        if kind == CharKind::Binary {
            let wanted = match s[i] {
                b if b & 0xE0 == 0xC0 => 2,
                b if b & 0xF0 == 0xE0 => 3,
                b if b & 0xF8 == 0xF0 => 4,
                _ => 1,
            };
            if s.len() - i < wanted {
                break;
            }
            invalid += 1;
        } else if len > 1 {
            encoded += len as i64;
        }
        i += len;
    }
    Score::new(Encoding::Utf8, encoded, encoded - invalid)
}

/// `CESI::GetEncodingInfo_cesu8`
fn score_cesu8(s: &[u8]) -> Score {
    let (mut encoded, mut invalid) = (0, 0);
    let mut i = 0;
    while i < s.len() {
        let (len, kind) = check_cesu8_char(&s[i..]);
        if kind == CharKind::Binary {
            let wanted = match s[i] {
                b if b & 0xE0 == 0xC0 => 2,
                b if b & 0xF0 == 0xE0 => 6,
                _ => 1,
            };
            if s.len() - i < wanted {
                break;
            }
            invalid += 1;
        } else if len > 1 {
            encoded += len as i64;
        }
        i += len;
    }
    Score::new(Encoding::Cesu8, encoded, encoded - invalid)
}

/// `CheckUtf7DPart`: the bytes before the next `+`, whether one of them is not UTF-7 direct, and
/// where the Base64 part starts.
fn check_utf7_d_part(s: &[u8]) -> (bool, usize) {
    let mut error = false;
    for (i, &b) in s.iter().enumerate() {
        if b == b'+' {
            return (error, i + 1);
        }
        if !is_direct_byte(b) {
            error = true;
        }
    }
    (error, s.len())
}

/// `CheckUtf7BPart` with no option: the number of Base64 letters, whether the part is wrong, where
/// the next part starts, and whether the letters do not count as points.
fn check_utf7_b_part(s: &[u8]) -> (usize, bool, usize, bool) {
    let len = s.iter().take_while(|&&b| base64_value(b).is_some()).count();
    let minus = s.get(len) == Some(&b'-');
    let at_end = len >= s.len();
    let mut error = false;

    // The letters must fill whole bytes with zero padding bits: 3, 6 or a multiple of 8 more.
    match len % 8 {
        0 => {}
        3 => error = base64_value(s[len - 1]).is_some_and(|v| v & 0x03 != 0),
        6 => error = base64_value(s[len - 1]).is_some_and(|v| v & 0x0F != 0),
        _ => error = true,
    }
    // `+` followed by neither a letter nor `-`.
    if !at_end && len < 1 && !minus {
        error = true;
    }
    if !error && len >= 1 {
        let data = decode_base64(&s[..len]);
        let units: Vec<u16> = data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_be_bytes(*pair))
            .collect();
        let mut i = 0;
        while i < units.len() {
            let (width, binary) = check_utf16_units(&units[i..]);
            // A lone surrogate, or a char that UTF-7 would have written directly.
            if binary || (width == 1 && char::from_u32(u32::from(units[i])).is_some_and(is_set_d)) {
                error = true;
                break;
            }
            i += width;
        }
    }
    let next = if !error && !at_end && minus {
        len + 1
    } else {
        len
    };
    // A run that ends without `-` before the end of the sample (Moca, 2015-03-05).
    let no_points = !minus && !at_end;
    (len, error, next, no_points)
}

/// `CESI::GetEncodingInfo_utf7`
fn score_utf7(s: &[u8]) -> Score {
    let mut points = 0i64;
    let mut error;
    let mut pos = 0;
    loop {
        let (d_error, next) = check_utf7_d_part(&s[pos..]);
        error = d_error;
        if error || pos + next == s.len() {
            break;
        }
        pos += next;
        let (len, b_error, next, no_points) = check_utf7_b_part(&s[pos..]);
        error = b_error;
        if len == next && pos + next == s.len() {
            // No `-` at the end of the sample: the sample may have been cut here.
            error = false;
            break;
        }
        if error {
            break;
        }
        pos += next;
        if !no_points {
            points += len as i64;
        }
        if pos >= s.len() {
            break;
        }
    }
    if error {
        points = REJECTED;
    }
    if points < 0 {
        Score::new(Encoding::Utf7, 0, points)
    } else {
        Score::new(Encoding::Utf7, points, points)
    }
}

fn is_high(unit: u16) -> bool {
    unit & 0xFC00 == 0xD800
}

fn is_low(unit: u16) -> bool {
    unit & 0xFC00 == 0xDC00
}

/// `_CheckUtf16Char` without options: the number of units and whether the char is invalid.
fn check_utf16_units(units: &[u16]) -> (usize, bool) {
    let Some(&first) = units.first() else {
        return (0, false);
    };
    if units
        .get(1)
        .is_some_and(|&second| is_high(first) && is_low(second))
    {
        return (2, false);
    }
    (1, is_high(first) || is_low(first))
}

/// `CESI::GetEncodingInfo_uni`: the scores of UTF-16 LE and BE.
fn score_utf16(s: &[u8]) -> (Score, Score) {
    if s.len() % 2 == 1 {
        return (
            Score::new(Encoding::Utf16Le, 0, REJECTED),
            Score::new(Encoding::Utf16Be, 0, REJECTED),
        );
    }
    let unit = |pos: usize, big: bool| {
        let pair = [s[pos], s[pos + 1]];
        if big {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        }
    };
    // Looks at the char at byte offset `pos`: (units, invalid).
    let check = |pos: usize, big: bool| {
        let remaining = (s.len() - pos) / 2;
        let mut units = [0u16; 2];
        for (k, slot) in units.iter_mut().enumerate().take(remaining.min(2)) {
            *slot = unit(pos + 2 * k, big);
        }
        if remaining == 0 {
            (0, false)
        } else {
            check_utf16_units(&units[..remaining.min(2)])
        }
    };

    let mut pos = [0usize; 2];
    let mut invalid = [0usize; 2];
    let mut line_breaks = [0i64; 2];
    'scan: loop {
        let found = [check(pos[0], false), check(pos[1], true)];
        if found[0].0 == 0 && found[1].0 == 0 {
            break;
        }
        for k in 0..2 {
            let (width, binary) = found[k];
            if width == 0 {
                continue;
            }
            let big = k == 1;
            if !binary {
                if width == 1 && matches!(unit(pos[k], big), 0x000D | 0x000A) {
                    line_breaks[k] += 1;
                }
            } else {
                // A high surrogate at the very end may continue in the next part of the file.
                let wanted = if is_high(unit(pos[k], big)) { 2 } else { 1 };
                if (s.len() - pos[k]) / 2 < wanted {
                    break 'scan;
                }
                invalid[k] += width * 2;
            }
            pos[k] += width * 2;
        }
    }
    let score = |k: usize, encoding| {
        if invalid[k] < 1 {
            Score::new(encoding, line_breaks[k], line_breaks[k])
        } else {
            Score::new(encoding, 0, REJECTED)
        }
    };
    (score(0, Encoding::Utf16Le), score(1, Encoding::Utf16Be))
}

// ---- Declarations in the text ----

fn is_xml_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n')
}

/// The names that a declaration can use and what Sakura reads them as (`encodingNameToCode`).
/// Numbers 0 to 8 are `ECodeType`; the others are Windows code pages.
const ENCODING_NAMES: &[(&str, i32)] = &[
    ("shift_jis", 0),
    ("windows-31j", 0),
    ("x-sjis", 0),
    ("cp932", 0),
    ("ms932", 0),
    ("shift-jis", 0),
    ("csWindows31J", 0),
    ("MS_Kanji", 0),
    ("csShiftJIS", 0),
    ("sjis", 0),
    ("iso-2022-jp", 1),
    ("iso2022jp", 1),
    ("csISO2022jp", 1),
    ("euc-jp", 2),
    ("euc_jp", 2),
    ("eucjp", 2),
    ("cseucpkdfmtjapanese", 2),
    ("extended_unix_code_packed_format_for_japanese", 2),
    ("x-euc-jp", 2),
    ("utf-8", 4),
    ("utf_8", 4),
    ("utf8", 4),
    ("csutf8", 4),
    ("unicode-1-1-utf-8", 4),
    ("unicode11utf8", 4),
    ("unicode20utf8", 4),
    ("x-unicode20utf8", 4),
    ("cesu-8", 7),
    ("cscesu-8", 7),
    ("cscesu8", 7),
    ("iso-8859-1", 8),
    ("latin1", 8),
    ("latin-1", 8),
    ("iso8859_1", 8),
    ("iso_8859_1", 8),
    ("iso88591", 8),
    ("cp819", 8),
    ("csisolatin1", 8),
    ("ibm819", 8),
    ("iso-ir-100", 8),
    ("iso8859-1", 8),
    ("iso_8859-1", 8),
    ("l1", 8),
    ("windows-1252", 8),
    ("cp1252", 8),
    ("cswindows1252", 8),
    ("x-cp1252", 8),
    ("ibm437", 437),
    ("asmo-708", 708),
    ("dos-720", 720),
    ("ibm737", 737),
    ("ibm775", 775),
    ("ibm850", 850),
    ("ibm852", 852),
    ("ibm855", 855),
    ("ibm857", 857),
    ("ibm00858", 858),
    ("ibm860", 860),
    ("ibm861", 861),
    ("dos-862", 862),
    ("ibm863", 863),
    ("ibm864", 864),
    ("ibm865", 865),
    ("cp866", 866),
    ("ibm869", 869),
    ("windows-874", 874),
    ("gb2312", 936),
    ("ks_c_5601-1987", 949),
    ("big5", 950),
    ("ibm1026", 1026),
    ("windows-1250", 1250),
    ("windows-1251", 1251),
    ("windows-1253", 1253),
    ("windows-1254", 1254),
    ("windows-1255", 1255),
    ("windows-1256", 1256),
    ("windows-1257", 1257),
    ("windows-1258", 1258),
    ("johab", 1361),
    ("macintosh", 10000),
    ("x-mac-japanese", 10001),
    ("x-mac-chinesetrad", 10002),
    ("x-mac-korean", 10003),
    ("x-mac-arabic", 10004),
    ("x-mac-hebrew", 10005),
    ("x-mac-greek", 10006),
    ("x-mac-cyrillic", 10007),
    ("x-mac-chinesesimp", 10008),
    ("x-mac-romanian", 10010),
    ("x-mac-ukrainian", 10017),
    ("x-mac-thai", 10021),
    ("x-mac-ce", 10029),
    ("x-mac-icelandic", 10079),
    ("x-mac-turkish", 10081),
    ("x-mac-croatian", 10082),
    ("x-chinese-cns", 20000),
    ("x-cp20001", 20001),
    ("x-chinese-eten", 20002),
    ("x-cp20003", 20003),
    ("x-cp20004", 20004),
    ("x-cp20005", 20005),
    ("x-ia5", 20105),
    ("x-ia5-german", 20106),
    ("x-ia5-swedish", 20107),
    ("x-ia5-norwegian", 20108),
    ("x-cp20261", 20261),
    ("x-cp20269", 20269),
    ("koi8-r", 20866),
    ("x-cp20936", 20936),
    ("x-cp20949", 20949),
    ("koi8-u", 21866),
    ("iso-8859-2", 28592),
    ("latin2", 28592),
    ("iso-8859-3", 28593),
    ("latin3", 28593),
    ("iso-8859-4", 28594),
    ("latin4", 28594),
    ("iso-8859-5", 28595),
    ("iso-8859-6", 28596),
    ("iso-8859-7", 28597),
    ("iso-8859-8", 28598),
    ("iso-8859-9", 28599),
    ("latin5", 28599),
    ("iso-8859-13", 28603),
    ("iso-8859-15", 28605),
    ("latin-9", 28605),
    ("x-europa", 29001),
    ("iso-8859-8-i", 38598),
    ("iso-2022-kr", 50225),
    ("x-cp50227", 50227),
    ("euc-cn", 51936),
    ("euc-kr", 51949),
    ("hz-gb-2312", 52936),
    ("gb18030", 54936),
    ("x-iscii-de", 57002),
    ("x-iscii-be", 57003),
    ("x-iscii-ta", 57004),
    ("x-iscii-te", 57005),
    ("x-iscii-as", 57006),
    ("x-iscii-or", 57007),
    ("x-iscii-ka", 57008),
    ("x-iscii-ma", 57009),
    ("x-iscii-gu", 57010),
    ("x-iscii-pa", 57011),
];

/// `MatchEncoding`
fn match_encoding(name: &[u8]) -> Option<Declared> {
    let &(_, code) = ENCODING_NAMES
        .iter()
        .find(|(candidate, _)| candidate.as_bytes().eq_ignore_ascii_case(name))?;
    Some(match Encoding::from_sakura_code(code) {
        Some(encoding) => Declared::Known(encoding),
        None => Declared::CodePage(code as u32),
    })
}

/// `CESI::GetEncodingInfo_meta`
fn detect_declared(s: &[u8]) -> Option<Declared> {
    detect_by_xml(s)
        .or_else(|| detect_by_coding(s))
        .or_else(|| detect_by_html(s))
}

/// `CESI::AutoDetectByXML`
fn detect_by_xml(s: &[u8]) -> Option<Declared> {
    let n = s.len();
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    if n <= 20 {
        return None;
    }
    if s.starts_with(b"<?xml") {
        if !is_xml_space(at(5)) {
            return None;
        }
        let mut i = 5;
        // `encoding` has to come after a space; `<`, `>` or the end of the declaration stop it.
        while i < n - 12 {
            if is_xml_space(at(i)) && s.get(i + 1..i + 9) == Some(b"encoding") {
                i += 9;
                while i < n - 2 && is_xml_space(at(i)) {
                    i += 1;
                }
                if at(i) != b'=' {
                    break;
                }
                i += 1;
                while i < n - 1 && is_xml_space(at(i)) {
                    i += 1;
                }
                if at(i) != b'\'' && at(i) != b'"' {
                    break;
                }
                let quote = at(i);
                i += 1;
                let end = s[i..].iter().position(|&c| c == quote)? + i;
                return match_encoding(&s[i..end]);
            }
            if at(i) == b'<' || at(i) == b'>' {
                break;
            }
            // The declaration ends without an `encoding`: go on with the other checks.
            if at(i) == b'?' && at(i + 1) == b'>' {
                return None;
            }
            i += 1;
        }
        while i < n {
            if at(i) == b'<' || at(i) == b'>' {
                break;
            }
            if at(i) == b'?' && at(i + 1) == b'>' {
                return None;
            }
            i += 1;
        }
    } else if s.starts_with(b"<\0?\0x\0m\0l\0") {
        if is_xml_space(at(10)) && at(11) == 0 {
            return Some(Declared::Known(Encoding::Utf16Le));
        }
    } else if s.starts_with(b"\0<\0?\0x\0m\0l") && at(10) == 0 && is_xml_space(at(11)) {
        return Some(Declared::Known(Encoding::Utf16Be));
    }
    None
}

fn is_encoding_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}

/// `CESI::AutoDetectByCoding`: `# coding: utf-8` and the like in the first two lines.
fn detect_by_coding(s: &[u8]) -> Option<Declared> {
    let n = s.len();
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut in_comment = false;
    let mut line = 1;
    let mut i = 0;
    while i + 8 < n {
        if in_comment && s[i..].starts_with(b"coding") && (at(i + 6) == b'=' || at(i + 6) == b':') {
            i += 7;
            while i < n && (at(i) == b' ' || at(i) == b'\t') {
                i += 1;
            }
            if n <= i {
                return None;
            }
            let begin = i;
            while i < n && is_encoding_name_char(at(i)) {
                i += 1;
            }
            if begin == i {
                return None;
            }
            return match_encoding(&s[begin..i]);
        } else if at(i) == b'\r' || at(i) == b'\n' {
            if at(i) == b'\r' && at(i + 1) == b'\n' {
                i += 1;
            }
            line += 1;
            in_comment = false;
            if line >= 3 {
                break;
            }
        } else if at(i) == b'#' {
            in_comment = true;
        }
        i += 1;
    }
    None
}

/// `CESI::AutoDetectByHTML`: `<meta charset=...>` and `<meta http-equiv content>`.
fn detect_by_html(s: &[u8]) -> Option<Declared> {
    let n = s.len() as i64;
    let at = |i: i64| {
        usize::try_from(i)
            .ok()
            .and_then(|i| s.get(i))
            .copied()
            .unwrap_or(0)
    };
    // `_memicmp` of the bytes from `i` with `word`.
    let starts_with_ignore_case = |i: i64, word: &str| {
        (0..word.len() as i64).all(|k| at(i + k).eq_ignore_ascii_case(&word.as_bytes()[k as usize]))
            && i + word.len() as i64 <= n
    };
    let slice = |from: i64, to: i64| &s[from as usize..to as usize];

    let mut i = 0i64;
    while i + 14 < n {
        if at(i) == b'<' && starts_with_ignore_case(i + 1, "meta") && is_xml_space(at(i + 5)) {
            i += 5;
            let mut declared: Option<Declared> = None;
            let mut content_type = false;
            while i < n {
                if is_xml_space(at(i)) && i + 1 < n && !is_xml_space(at(i + 1)) {
                    // 1: http-equiv, 2: content, 3: charset
                    let mut attribute = 0;
                    i += 1;
                    if i + 12 < n
                        && starts_with_ignore_case(i, "http-equiv")
                        && (is_xml_space(at(i + 10)) || at(i + 10) == b'=')
                    {
                        i += 10;
                        attribute = 1;
                    } else if i + 9 < n
                        && starts_with_ignore_case(i, "content")
                        && (is_xml_space(at(i + 7)) || at(i + 7) == b'=')
                    {
                        i += 7;
                        attribute = 2;
                    } else if i + 9 < n
                        && starts_with_ignore_case(i, "charset")
                        && (is_xml_space(at(i + 7)) || at(i + 7) == b'=')
                    {
                        i += 7;
                        attribute = 3;
                    } else {
                        // Another attribute name. Like Sakura, this skips to the next space even
                        // across `>` (so `<meta x><!-- charset=... -->` is read as a declaration).
                        // Kept on purpose: the results for real files must be the same as Sakura's.
                        while i < n && !is_xml_space(at(i)) {
                            i += 1;
                        }
                    }
                    if n <= i {
                        return None;
                    }
                    while i < n && is_xml_space(at(i)) {
                        i += 1;
                    }
                    if n <= i {
                        return None;
                    }
                    if at(i) == b'=' {
                        i += 1;
                        if n <= i {
                            return None;
                        }
                    } else {
                        // `<meta attribute ...`: no value.
                        i -= 1;
                        continue;
                    }
                    while i < n && is_xml_space(at(i)) {
                        i += 1;
                    }
                    if n <= i {
                        return None;
                    }
                    let (begin, end, next);
                    if at(i) == b'\'' || at(i) == b'"' {
                        let quote = at(i);
                        i += 1;
                        begin = i;
                        if n <= i {
                            return None;
                        }
                        while i < n && quote != at(i) && at(i) != b'<' && at(i) != b'>' {
                            i += 1;
                        }
                        end = i;
                        i += 1;
                        next = i;
                    } else {
                        begin = i;
                        while i < n && !is_xml_space(at(i)) && at(i) != b'<' && at(i) != b'>' {
                            i += 1;
                        }
                        end = i;
                        next = i;
                    }
                    match attribute {
                        1 => {
                            if end - begin == 12 && starts_with_ignore_case(begin, "content-type") {
                                content_type = true;
                                if declared.is_some() {
                                    return declared;
                                }
                            }
                        }
                        2 => {
                            i = begin;
                            while i < end && at(i) != b';' {
                                i += 1;
                            }
                            if end <= i {
                                i = next;
                                continue;
                            }
                            i += 1;
                            while i < end && is_xml_space(at(i)) {
                                i += 1;
                            }
                            if end <= i {
                                i = next;
                                continue;
                            }
                            if i + 7 < end && starts_with_ignore_case(i, "charset") {
                                i += 7;
                                while i < end && is_xml_space(at(i)) {
                                    i += 1;
                                }
                                if end <= i || at(i) != b'=' {
                                    i = next;
                                    continue;
                                }
                                i += 1;
                                while i < end && is_xml_space(at(i)) {
                                    i += 1;
                                }
                                if end <= i {
                                    i = next;
                                    continue;
                                }
                                let name_begin = i;
                                while i < end && !is_xml_space(at(i)) {
                                    i += 1;
                                }
                                if let Some(found) = match_encoding(slice(name_begin, i)) {
                                    if content_type {
                                        return Some(found);
                                    }
                                    declared = Some(found);
                                }
                            }
                            i = next;
                        }
                        3 => {
                            if let Some(found) = match_encoding(slice(begin, end)) {
                                return Some(found);
                            }
                        }
                        _ => {}
                    }
                } else if at(i) == b'<' {
                    i -= 1;
                    break;
                } else if at(i) == b'>' {
                    break;
                } else {
                    // Consecutive spaces.
                    i += 1;
                }
            }
        }
        i += 1;
    }
    None
}
