// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/convert/CConvert_CodeToSjis.cpp
//   sakura_core/convert/CConvert_CodeFromSjis.cpp
//   sakura_core/convert/CConvert_CodeAutoToSjis.cpp
//   sakura_core/convert/CDecode_Base64Decode.cpp
//   sakura_core/convert/CDecode_UuDecode.cpp
//   sakura_core/convert/convert_util2.h (MIME header, Base64 and uu helpers)
//   sakura_core/charset/CCodeBase.cpp (MIMEHeaderDecode)
//
// Copyright (C) 2007, kobake
// Copyright (C) 2008, kobake, ryoji
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (UTF-16 buffers and
// the code bases of `CCodeFactory`) was rewritten in Rust on UTF-8 strings and the codecs of
// `ume_core::encoding`, the encoding detection is the one of `ume_core::encoding::detect`, and
// the tests are new.
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

//! Character code conversions of the selected text, and the Base64 and uu decoders.
//!
//! # What the text means
//!
//! Sakura's conversions work on text that was read with the wrong encoding. The Unicode text in
//! the buffer is first written back as Shift_JIS bytes (`UnicodeToCode` of `CODE_SJIS`), which
//! gives back the bytes of the file when it was read as Shift_JIS, and then
//!
//! * [`to_sjis`] (`CConvert_CodeToSjis`, the "xxx to SJIS" functions) reads those bytes as the
//!   encoding `xxx` and replaces the text by the result;
//! * [`from_sjis`] (`CConvert_CodeFromSjis`, "SJIS to xxx") writes the text as `xxx` and
//!   replaces it by those bytes read as Shift_JIS;
//! * [`auto_to_sjis`] (`CConvert_CodeAutoToSjis`) detects `xxx` from the bytes first.
//!
//! Bytes that are not valid in an encoding are kept as stand-in chars
//! (`ume_core::encoding::INVALID_BYTE_BASE`), so a round trip gives the same bytes back, where
//! Sakura keeps them in U+DC00..U+DCFF.
//!
//! The functions return the new text; the commands replace the selection with it (see
//! [`CommandContext::convert_selection`](crate::context::CommandContext::convert_selection)).

use ume_core::encoding::{
    DetectOptions, Encoding, base64_value, decode_base64_digits, detect, mime_header_decode_jis,
};

/// Shift_JIS bytes of `text` (the stand-ins become their bytes again; characters Shift_JIS has
/// no code for become `?`, as in Sakura's `UnicodeToCode` with the loss ignored).
fn to_sjis_bytes(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len());
    // The error only lists the characters that were replaced; Sakura ignores it here.
    let _ = Encoding::ShiftJis.codec().encode(text, &mut bytes);
    bytes
}

fn decode(encoding: Encoding, bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    encoding.codec().decode(bytes, &mut out);
    out
}

/// Reads the Shift_JIS bytes of `text` as `from` (`CConvert_CodeToSjis::DoConvert`).
///
/// For ISO-2022-JP the MIME headers (`=?ISO-2022-JP?B?...?=`) are decoded first, as in Sakura
/// (`CCodeFactory::CreateCodeBase(CODE_JIS, true)`, which is what `F_CODECNV_EMAIL` is).
pub fn to_sjis(from: Encoding, text: &str) -> String {
    let bytes = to_sjis_bytes(text);
    if from == Encoding::Jis {
        decode(from, &mime_header_decode_jis(&bytes))
    } else {
        decode(from, &bytes)
    }
}

/// Writes `text` as `to` and reads the bytes as Shift_JIS (`CConvert_CodeFromSjis::DoConvert`).
pub fn from_sjis(to: Encoding, text: &str) -> String {
    let mut bytes = Vec::with_capacity(text.len());
    // Sakura ignores the loss here too (the characters become what the encoding writes for them).
    let _ = to.codec().encode(text, &mut bytes);
    decode(Encoding::ShiftJis, &bytes)
}

/// Detects the encoding of the Shift_JIS bytes of `text` and reads them as that encoding
/// (`CConvert_CodeAutoToSjis::DoConvert`). Returns `None` when the detected encoding is not one
/// of ISO-2022-JP, EUC-JP, UTF-16 (both byte orders), UTF-8 and UTF-7; Sakura fails the
/// conversion then and shows "変換でエラーが発生しました" (`STR_CONVERT_ERR`).
///
/// Differences from Sakura: the detection is the one of `ume_core::encoding::detect` (no uchardet),
/// and `options.default_encoding` should be the default code of the document type
/// (`m_sEncodingConfig`); ISO-2022-JP has its MIME headers decoded as in [`to_sjis`].
pub fn auto_to_sjis(text: &str, options: &DetectOptions) -> Option<String> {
    let bytes = to_sjis_bytes(text);
    let detection = detect(&bytes, options);
    match detection.encoding {
        Encoding::Jis => Some(decode(Encoding::Jis, &mime_header_decode_jis(&bytes))),
        Encoding::EucJp
        | Encoding::Utf16Le
        | Encoding::Utf16Be
        | Encoding::Utf8
        | Encoding::Utf7 => Some(decode(detection.encoding, &bytes)),
        _ => None,
    }
}

// ---- Base64 and uudecode of the selection ----

fn is_line_delimiter(c: char) -> bool {
    c == '\r' || c == '\n'
}

fn is_white(c: char) -> bool {
    is_line_delimiter(c) || c == ' ' || c == '\t'
}

/// Decodes the text as Base64 (`CDecode_Base64Decode::DoDecode`). Line endings, spaces and tabs
/// are skipped, the first `=` ends the data, and any other char that is not a Base64 digit fails
/// (`None`; Sakura does nothing then, without a message).
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut digits = Vec::with_capacity(text.len());
    for c in text.chars() {
        if is_white(c) {
            continue;
        }
        if c == '=' {
            break;
        }
        if !c.is_ascii() || base64_value(c as u8) == 255 {
            return None;
        }
        digits.push(c as u8);
    }
    Some(decode_base64_digits(&digits))
}

/// The result of [`uudecode`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UuDecoded {
    /// The file name of the `begin` line, the name the save dialog starts with.
    pub file_name: String,
    pub data: Vec<u8>,
}

/// The value of a uu digit (`_UUDECODE_CHAR`): `` ` `` and `~` are spaces.
fn uu_value(c: char) -> u8 {
    let c = match c {
        '`' | '~' => b' ',
        c => c as u32 as u8,
    };
    c.wrapping_sub(0x20) & 0x3F
}

/// The next line of `chars` from `*pos` without its line ending, and that line ending
/// (`GetNextLineW` without the extended line endings). `None` at the end.
fn next_line<'a>(chars: &'a [char], pos: &mut usize) -> Option<(&'a [char], &'static str)> {
    if *pos >= chars.len() {
        return None;
    }
    let start = *pos;
    let mut i = start;
    while i < chars.len() && !is_line_delimiter(chars[i]) {
        i += 1;
    }
    let eol = match (chars.get(i), chars.get(i + 1)) {
        (Some('\r'), Some('\n')) => "\r\n",
        (Some('\r'), _) => "\r",
        (Some('\n'), _) => "\n",
        _ => "",
    };
    *pos = i + eol.len();
    Some((&chars[start..i], eol))
}

/// `CheckUUHeader`: `begin <3 octal digits> <file name>`, split by spaces and tabs. The file
/// name is the third word only (a name with spaces is cut at the first space).
fn uu_header_file_name(line: &[char]) -> Option<String> {
    let mut words = line
        .split(|&c| c == ' ' || c == '\t')
        .filter(|w| !w.is_empty());
    if words.next()?.iter().collect::<String>() != "begin" {
        return None;
    }
    let mode = words.next()?;
    if mode.len() != 3 || !mode.iter().all(|c| ('0'..='7').contains(c)) {
        return None;
    }
    let name: String = words.next()?.iter().collect();
    // `_MAX_PATH` is 260 and the name must leave room for the NUL.
    (!name.is_empty() && name.chars().count() < 260).then_some(name)
}

/// `CheckUUFooter`: only white space, then `end`, then only white space.
fn uu_footer_ok(rest: &[char]) -> bool {
    let rest: Vec<char> = rest.iter().copied().skip_while(|c| is_white(*c)).collect();
    rest.len() >= 3 && rest[..3] == ['e', 'n', 'd'] && rest[3..].iter().all(|c| is_white(*c))
}

/// Decodes the text as uuencoded data (`CDecode_UuDecode::DoDecode`): a `begin` header, body
/// lines that all end with CR LF, a last line of one space, `` ` `` or `~`, and `end`. Returns
/// `None` when the text is not like that (Sakura does nothing then, without a message).
///
/// Difference: a body line that is shorter than its length char says (some gateways cut the
/// trailing spaces) is padded with spaces; Sakura reads the chars after the line.
pub fn uudecode(text: &str) -> Option<UuDecoded> {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = chars.iter().take_while(|c| is_white(**c)).count();
    let (header, _) = next_line(&chars, &mut pos)?;
    let file_name = uu_header_file_name(header)?;

    let mut data = Vec::new();
    let mut finished = false;
    while let Some((line, eol)) = next_line(&chars, &mut pos) {
        if eol != "\r\n" || line.is_empty() {
            return None;
        }
        if line.len() == 1 && matches!(line[0], ' ' | '`' | '~') {
            finished = true;
            break;
        }
        let count = usize::from(uu_value(line[0]));
        let digits = &line[1..];
        let mut produced = Vec::with_capacity(digits.len() / 4 * 3 + 3);
        for group in 0..digits.len().div_ceil(4) {
            let mut value: u32 = 0;
            for j in 0..4 {
                let c = digits.get(group * 4 + j).copied().unwrap_or(' ');
                value |= u32::from(uu_value(c)) << ((3 - j) * 6);
            }
            for j in 0..3 {
                produced.push(((value >> ((2 - j) * 8)) & 0xFF) as u8);
            }
        }
        produced.resize(count, 0);
        data.extend_from_slice(&produced[..count]);
    }
    if !finished {
        return None;
    }
    uu_footer_ok(&chars[pos..]).then_some(UuDecoded { file_name, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SJIS_NIHONGO: &[u8] = b"\x93\xFA\x96\x7B\x8C\xEA";

    /// The text a Shift_JIS reading of `bytes` shows.
    fn shown(bytes: &[u8]) -> String {
        decode(Encoding::ShiftJis, bytes)
    }

    #[test]
    fn utf8_read_as_sjis_is_fixed() {
        let garbled = shown("日本語".as_bytes());
        assert_ne!(garbled, "日本語");
        assert_eq!(to_sjis(Encoding::Utf8, &garbled), "日本語");
        assert_eq!(to_sjis(Encoding::Utf8, "abc\r\n"), "abc\r\n");
    }

    #[test]
    fn euc_utf16_utf7_read_as_sjis_are_fixed() {
        let euc = b"\xC6\xFC\xCB\xDC\xB8\xEC";
        assert_eq!(to_sjis(Encoding::EucJp, &shown(euc)), "日本語");
        let mut le = Vec::new();
        Encoding::Utf16Le.codec().encode("日本a", &mut le).unwrap();
        assert_eq!(to_sjis(Encoding::Utf16Le, &shown(&le)), "日本a");
        let mut be = Vec::new();
        Encoding::Utf16Be.codec().encode("日本a", &mut be).unwrap();
        assert_eq!(to_sjis(Encoding::Utf16Be, &shown(&be)), "日本a");
        assert_eq!(to_sjis(Encoding::Utf7, "+ZeVnLIqe-"), "日本語");
    }

    #[test]
    fn jis_reads_the_escape_sequences_and_the_mime_headers() {
        let jis = b"\x1B$BF|K\\8l\x1B(B";
        assert_eq!(to_sjis(Encoding::Jis, &shown(jis)), "日本語");
        // "=?ISO-2022-JP?B?" + base64 of the JIS bytes + "?=".
        let word = "=?ISO-2022-JP?B?GyRCRnxLXDhsGyhC?= x";
        assert_eq!(to_sjis(Encoding::Jis, word), "日本語 x");
        // Case of the charset and method does not matter; Q needs =XX.
        let word = "=?iso-2022-jp?q?=1B$BF|K\\8l=1B(B?=";
        assert_eq!(to_sjis(Encoding::Jis, word), "日本語");
        // UTF-8 words and other text stay.
        let word = "=?UTF-8?B?44GC?= =";
        assert_eq!(to_sjis(Encoding::Jis, word), word);
    }

    #[test]
    fn unconvertible_text_does_not_fail() {
        // Characters that Shift_JIS cannot write become `?` before the conversion.
        assert_eq!(to_sjis(Encoding::Utf8, "€"), "?");
        // Invalid UTF-8 keeps its bytes as stand-ins, which Shift_JIS writes back.
        let kept = to_sjis(Encoding::Utf8, &shown(b"\xFF\xFEabc"));
        assert_eq!(to_sjis_bytes(&kept), b"\xFF\xFEabc");
    }

    #[test]
    fn sjis_to_other_codes_shows_the_bytes_as_sjis() {
        let utf8 = from_sjis(Encoding::Utf8, "日本語");
        assert_eq!(to_sjis_bytes(&utf8), "日本語".as_bytes());
        assert_eq!(from_sjis(Encoding::Utf8, "abc\r\n"), "abc\r\n");
        let euc = from_sjis(Encoding::EucJp, "日本語");
        assert_eq!(to_sjis_bytes(&euc), b"\xC6\xFC\xCB\xDC\xB8\xEC");
        let jis = from_sjis(Encoding::Jis, "日本語");
        assert_eq!(to_sjis_bytes(&jis), b"\x1B$BF|K\\8l\x1B(B");
        let utf7 = from_sjis(Encoding::Utf7, "日本語");
        assert_eq!(utf7, "+ZeVnLIqe-");
    }

    #[test]
    fn the_two_directions_undo_each_other() {
        for encoding in [
            Encoding::EucJp,
            Encoding::Utf8,
            Encoding::Utf7,
            Encoding::Utf16Le,
            Encoding::Utf16Be,
            Encoding::Jis,
        ] {
            let there = from_sjis(encoding, "日本語 abc\r\nカナ");
            assert_eq!(
                to_sjis(encoding, &there),
                "日本語 abc\r\nカナ",
                "{encoding:?}"
            );
        }
    }

    #[test]
    fn auto_detects_the_code() {
        let options = DetectOptions::default();
        let utf8 = shown("日本語のテキストです。\r\n".as_bytes());
        assert_eq!(
            auto_to_sjis(&utf8, &options).as_deref(),
            Some("日本語のテキストです。\r\n")
        );
        let jis = shown(b"\x1B$BF|K\\8l\x1B(B");
        assert_eq!(auto_to_sjis(&jis, &options).as_deref(), Some("日本語"));
        // Shift_JIS itself is not a conversion Sakura does.
        let sjis = shown(SJIS_NIHONGO);
        assert_eq!(auto_to_sjis(&sjis, &options), None);
    }

    #[test]
    fn base64_skips_white_space_and_stops_at_the_pad() {
        assert_eq!(base64_decode("aGVs\r\nbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("aGVsbG8").unwrap(), b"hello");
        assert_eq!(base64_decode(" a G V s \t bG8= xyz!").unwrap(), b"hello");
        assert_eq!(base64_decode("YQ==").unwrap(), b"a");
        assert_eq!(base64_decode("").unwrap(), b"");
        assert_eq!(base64_decode("aGV!"), None);
        assert_eq!(base64_decode("aGVs日"), None);
        // A 1 digit rest gives no byte.
        assert_eq!(base64_decode("YWJjZ").unwrap(), b"abc");
    }

    const UU: &str = "begin 644 cat.txt\r\n#0V%T\r\n`\r\nend\r\n";

    #[test]
    fn uudecode_reads_the_header_the_body_and_the_footer() {
        let decoded = uudecode(UU).unwrap();
        assert_eq!(decoded.file_name, "cat.txt");
        assert_eq!(decoded.data, b"Cat");
        // Leading blank lines and the other ends of the last line.
        assert!(uudecode(&format!("\r\n  {UU}")).is_some());
        assert!(uudecode("begin 644 a\r\n \r\nend").is_some());
        assert!(uudecode("begin 644 a\r\n~\r\n  end  ").is_some());
    }

    #[test]
    fn uudecode_rejects_what_is_not_uu() {
        assert_eq!(uudecode(""), None);
        assert_eq!(uudecode("hello"), None);
        assert_eq!(uudecode("begin 64 a\r\n`\r\nend"), None);
        assert_eq!(uudecode("begin 688 a\r\n`\r\nend"), None);
        assert_eq!(uudecode("begin 644\r\n`\r\nend"), None);
        // Body lines end with CR LF only.
        assert_eq!(uudecode("begin 644 a\n#0V%T\n`\nend"), None);
        // An empty line, no end mark, no footer, or something after the footer.
        assert_eq!(uudecode("begin 644 a\r\n\r\n`\r\nend"), None);
        assert_eq!(uudecode("begin 644 a\r\n#0V%T\r\n"), None);
        assert_eq!(uudecode("begin 644 a\r\n`\r\n"), None);
        assert_eq!(uudecode("begin 644 a\r\n`\r\nend x"), None);
    }

    #[test]
    fn uudecode_pads_a_short_line() {
        // The count says 3 bytes, the line has one group of 4 digits missing its last one.
        let decoded = uudecode("begin 644 a\r\n#0V%\r\n`\r\nend").unwrap();
        assert_eq!(decoded.data.len(), 3);
        assert_eq!(&decoded.data[..2], b"Ca");
    }
}
