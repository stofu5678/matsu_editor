// Ported from Sakura Editor (https://github.com/sakura-editor/sakura), modified.
//
// Original source files:
//   - sakura_core/charset/CCodeBase.cpp (MIMEHeaderDecode)
//   - sakura_core/convert/convert_util2.h (_DecodeMimeHeader, _DecodeQP, _DecodeBase64)
//
// Modified: rewritten in Rust on byte slices (not a line-by-line port); `CMemory` and the
// ACHAR/WCHAR templates are gone, and only the ISO-2022-JP side of `MIMEHeaderDecode` is kept.
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

//! MIME encoded words in ISO-2022-JP mail headers (`=?ISO-2022-JP?B?...?=`), decoded before the
//! bytes are read as ISO-2022-JP. Sakura does this when the option 「MIMEエンコードされたヘッダを
//! デコードする」 (`bAutoMIMEdecode`) is on (`CJis::JISToUnicode` with `base64decode`), and for
//! 「E-Mail(JIS→SJIS)コード変換」.

fn starts_with_ignore_case(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && bytes[..prefix.len()].eq_ignore_ascii_case(prefix)
}

/// The value of a Base64 digit; 255 for anything else (`Base64ToVal`).
pub fn base64_value(c: u8) -> u8 {
    match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => 255,
    }
}

/// Decodes Base64 digits without checking the padding bits (`_DecodeBase64`): the `=` at the end
/// are dropped, and a last group of 2 or 3 digits gives 1 or 2 bytes.
pub fn decode_base64_digits(src: &[u8]) -> Vec<u8> {
    let mut len = src.len();
    while len > 0 && src[len - 1] == b'=' {
        len -= 1;
    }
    let src = &src[..len];
    let mut out = Vec::with_capacity(len / 4 * 3 + 2);
    for group in src.chunks(4) {
        let mut data: u32 = 0;
        for (j, &c) in group.iter().enumerate() {
            data |= u32::from(base64_value(c)) << ((3 - j) * 6);
        }
        for j in 0..group.len() * 6 / 8 {
            out.push(((data >> (8 * (2 - j))) & 0xFF) as u8);
        }
    }
    out
}

fn hex_value(c: u8) -> Option<u8> {
    char::from(c).to_digit(16).map(|v| v as u8)
}

/// Decodes `=XX` (`_DecodeQP`). Like Sakura, an `=` that has fewer than two bytes after it, not
/// counting the last byte, is dropped; `_` is not turned into a space.
fn decode_quoted_printable(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        if src[i] != b'=' {
            out.push(src[i]);
            i += 1;
            continue;
        }
        // `pr + 2 < pS + nLen`
        if i + 2 < src.len() {
            match (hex_value(src[i + 1]), hex_value(src[i + 2])) {
                (Some(h), Some(l)) => out.push(h << 4 | l),
                _ => out.extend_from_slice(&src[i..i + 3]),
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    out
}

/// One encoded word at the start of `src` (`_DecodeMimeHeader`): its charset is ISO-2022-JP or
/// UTF-8. Returns whether the charset is ISO-2022-JP, the decoded bytes, and the length of the
/// whole word.
fn decode_mime_word(src: &[u8]) -> Option<(bool, Vec<u8>, usize)> {
    // Sakura needs more than the prefix: `pSrc + 14 < pSrc + nSrcLen`.
    let (is_jis, part1) = if src.len() > 14 && starts_with_ignore_case(src, b"=?ISO-2022-JP?") {
        (true, 14)
    } else if src.len() > 8 && starts_with_ignore_case(src, b"=?UTF-8?") {
        (false, 8)
    } else {
        return None;
    };
    if part1 + 2 >= src.len() {
        return None;
    }
    let base64 = if starts_with_ignore_case(&src[part1..], b"B?") {
        true
    } else if starts_with_ignore_case(&src[part1..], b"Q?") {
        false
    } else {
        return None;
    };
    let body_start = part1 + 2;
    let end = (body_start..src.len() - 1).find(|&i| &src[i..i + 2] == b"?=")?;
    let body = &src[body_start..end];
    let decoded = if base64 {
        decode_base64_digits(body)
    } else {
        decode_quoted_printable(body)
    };
    Some((is_jis, decoded, end + 2))
}

/// Replaces the ISO-2022-JP encoded words of mail headers by their bytes
/// (`CCodeBase::MIMEHeaderDecode` for `CODE_JIS`). Encoded words of UTF-8 stay as they are.
pub fn mime_header_decode_jis(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        if src[i] == b'='
            && let Some((is_jis, decoded, skip)) = decode_mime_word(&src[i..])
        {
            if is_jis {
                out.extend_from_slice(&decoded);
            } else {
                out.extend_from_slice(&src[i..i + skip]);
            }
            i += skip;
        } else {
            out.push(src[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // "日本語" in ISO-2022-JP is 1B 24 42 46 7C 4B 5C 38 6C 1B 28 42.
    const JIS: &[u8] = b"\x1B$BF|K\\8l\x1B(B";

    #[test]
    fn a_b_word_is_decoded() {
        let word = b"=?ISO-2022-JP?B?GyRCRnxLXDhsGyhC?= x";
        let mut want = JIS.to_vec();
        want.extend_from_slice(b" x");
        assert_eq!(mime_header_decode_jis(word), want);
    }

    #[test]
    fn a_q_word_is_decoded_and_the_case_does_not_matter() {
        let word = b"=?iso-2022-jp?q?=1B$BF|K\\8l=1B(B?=";
        assert_eq!(mime_header_decode_jis(word), JIS);
    }

    #[test]
    fn q_keeps_what_is_not_a_hex_pair_and_underscore_is_not_a_space() {
        let word = b"=?ISO-2022-JP?Q?a=ZZb_c?=";
        assert_eq!(mime_header_decode_jis(word), b"a=ZZb_c");
    }

    #[test]
    fn q_drops_an_equal_sign_near_the_end() {
        // `_DecodeQP` needs two more bytes after `=` (`pr + 2 < pS + nLen`).
        assert_eq!(mime_header_decode_jis(b"=?ISO-2022-JP?Q?ab=?="), b"ab");
        assert_eq!(mime_header_decode_jis(b"=?ISO-2022-JP?Q?ab=4?="), b"ab4");
        assert_eq!(mime_header_decode_jis(b"=?ISO-2022-JP?Q?ab=41?="), b"abA");
    }

    #[test]
    fn base64_takes_the_padding_off_and_reads_a_short_last_group() {
        assert_eq!(decode_base64_digits(b"QUJD"), b"ABC");
        assert_eq!(decode_base64_digits(b"QUI="), b"AB");
        assert_eq!(decode_base64_digits(b"QQ=="), b"A");
        // padding bits are not checked
        assert_eq!(decode_base64_digits(b"QR"), b"A");
        assert_eq!(decode_base64_digits(b""), b"");
    }

    #[test]
    fn a_word_of_another_charset_stays() {
        let word = b"=?UTF-8?B?44GC?= =?ISO-8859-1?Q?a?=";
        assert_eq!(mime_header_decode_jis(word), word);
    }

    #[test]
    fn text_that_is_not_a_word_stays() {
        for text in [
            &b"a = b"[..],
            b"=?ISO-2022-JP?B?",
            b"=?ISO-2022-JP?B?GyRC",
            b"=?ISO-2022-JP?X?abc?=",
            b"=?ISO-2022-JP?",
            b"=?UTF-8?B",
            b"",
        ] {
            assert_eq!(mime_header_decode_jis(text), text, "{text:?}");
        }
    }

    #[test]
    fn the_prefix_alone_is_not_enough() {
        // Sakura needs more bytes than the prefix (`pSrc + 14 < pSrc + nSrcLen`).
        assert_eq!(mime_header_decode_jis(b"=?ISO-2022-JP?"), b"=?ISO-2022-JP?");
    }

    #[test]
    fn several_words_and_the_text_between_them() {
        let word = b"A =?ISO-2022-JP?B?QUJD?= B =?ISO-2022-JP?Q?=41?= C";
        assert_eq!(mime_header_decode_jis(word), b"A ABC B A C");
    }
}
