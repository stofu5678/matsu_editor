//! URL, HTML and Base64 encoding and decoding of text, for the MATSU-only commands `url_encode`,
//! `url_decode`, `html_escape`, `html_unescape`, `base64_encode` and `base64_decode_text`.
//!
//! Sakura has none of these. Text is the UTF-8 form of the selection (the buffer is Unicode),
//! and line breaks inside it are ordinary characters: they are encoded like any other and a
//! decoder gives them back as they were. The decoders return `None` for input they cannot read
//! (the command then changes nothing and beeps).

use matsu_core::encoding::base64_value;

/// RFC 3986's unreserved characters, which URL encoding leaves as they are (section 2.3).
fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

/// Percent-encodes the UTF-8 bytes of `text`, except RFC 3986's unreserved characters
/// (`A-Z a-z 0-9 - . _ ~`). The hex digits are upper case. A space becomes `%20` (never `+`).
pub fn url_encode(text: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(text.len());
    for &b in text.as_bytes() {
        if is_unreserved(b) {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(b >> 4)]));
            out.push(char::from(HEX[usize::from(b & 15)]));
        }
    }
    out
}

/// Decodes `%XX` sequences (either case of the hex digits) and reads the bytes as UTF-8. Every
/// other character stays as it is, a `+` too (that is a form's reading, not RFC 3986's, and
/// [`url_encode`] never writes one). `None` for a `%` that is not followed by two hex digits,
/// and for bytes that are not UTF-8.
pub fn url_decode(text: &str) -> Option<String> {
    let src = text.as_bytes();
    let mut bytes = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        if src[i] == b'%' {
            let hi = hex_digit(*src.get(i + 1)?)?;
            let lo = hex_digit(*src.get(i + 2)?)?;
            bytes.push(hi << 4 | lo);
            i += 3;
        } else {
            bytes.push(src[i]);
            i += 1;
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex_digit(b: u8) -> Option<u8> {
    char::from(b).to_digit(16).map(|d| d as u8)
}

/// Replaces `& < > " '` by `&amp; &lt; &gt; &quot; &#39;`. Everything else, line breaks and
/// non-ASCII text included, stays as it is.
pub fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// The reverse of [`html_escape`], in one pass (`&amp;lt;` gives `&lt;`, not `<`). It reads
/// `&amp; &lt; &gt; &quot; &apos;` and numeric references (`&#39;`, `&#x27;`, any digits up to a
/// valid scalar value except 0). Any other `&...` (an unknown name such as `&nbsp;`, a reference
/// without its `;`, a number that is no character) stays as it is: unescaping never fails.
pub fn html_unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        match reference(rest) {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The character of the reference at the start of `s` (which starts with `&`) and its length.
fn reference(s: &str) -> Option<(char, usize)> {
    // The longest reference we read is `&#x10FFFF;`; a `;` farther away than this is not ours.
    let end = s.bytes().take(12).position(|b| b == b';')?;
    let body = &s[1..end];
    let c = match body {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        _ => {
            let digits = body.strip_prefix('#')?;
            let (digits, radix) = match digits.strip_prefix(['x', 'X']) {
                Some(hex) => (hex, 16),
                None => (digits, 10),
            };
            if digits.is_empty() || !digits.bytes().all(|b| char::from(b).is_digit(radix)) {
                return None;
            }
            // NUL is no text (browsers turn it into U+FFFD); it stays as written.
            char::from_u32(u32::from_str_radix(digits, radix).ok()?).filter(|&c| c != '\0')?
        }
    };
    Some((c, end + 1))
}

const BASE64_DIGITS: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 of the UTF-8 bytes of `text` (RFC 4648's standard alphabet, with `=` padding, on one
/// line without wrapping).
pub fn base64_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len().div_ceil(3) * 4);
    for group in text.as_bytes().chunks(3) {
        let n = group
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= group.len() {
                out.push(char::from(BASE64_DIGITS[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Decodes standard Base64 and reads the bytes as UTF-8. Line breaks, spaces and tabs are
/// skipped (mail and PEM wrap lines). The padding may be missing; when there is any it must
/// complete the last group of four. `None` for any other character (the URL-safe `-` and `_`
/// too), a `=` before the end, a length that no data has, and bytes that are not UTF-8.
pub fn base64_decode_text(text: &str) -> Option<String> {
    let mut digits: Vec<u8> = Vec::with_capacity(text.len());
    let mut padding = 0;
    for c in text.chars() {
        if matches!(c, '\r' | '\n' | ' ' | '\t') {
            continue;
        }
        if c == '=' {
            padding += 1;
        } else if padding > 0 || !c.is_ascii() || base64_value(c as u8) == 255 {
            return None;
        } else {
            digits.push(c as u8);
        }
    }
    if padding > 2
        || digits.len() % 4 == 1
        || (padding > 0 && !(digits.len() + padding).is_multiple_of(4))
    {
        return None;
    }
    let mut bytes = Vec::with_capacity(digits.len() / 4 * 3 + 2);
    for group in digits.chunks(4) {
        let n = group.iter().enumerate().fold(0u32, |n, (i, &d)| {
            n | u32::from(base64_value(d)) << (18 - 6 * i)
        });
        for i in 0..group.len() * 6 / 8 {
            bytes.push((n >> (16 - 8 * i)) as u8);
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLES: [&str; 6] = [
        "",
        "abc XYZ-._~",
        "日本語 テキスト",
        "😀 a\u{301}",
        "line1\r\nline2\nline3\r",
        "a&b<c>\"d\"'e' %41 +",
    ];

    #[test]
    fn url_encode_keeps_the_unreserved_set_and_encodes_utf8() {
        assert_eq!(url_encode("abc XYZ-._~019"), "abc%20XYZ-._~019");
        assert_eq!(url_encode("a+b/c?d=e&f#g"), "a%2Bb%2Fc%3Fd%3De%26f%23g");
        assert_eq!(url_encode("日本"), "%E6%97%A5%E6%9C%AC");
        assert_eq!(url_encode("😀"), "%F0%9F%98%80");
        assert_eq!(url_encode("a\r\nb\nc"), "a%0D%0Ab%0Ac");
        assert_eq!(url_encode(""), "");
    }

    #[test]
    fn url_decode_reads_either_case_and_keeps_plus() {
        assert_eq!(url_decode("a%20b%2fc%2Fd").unwrap(), "a b/c/d");
        assert_eq!(url_decode("a+b").unwrap(), "a+b");
        assert_eq!(url_decode("%E6%97%A5%E6%9C%AC").unwrap(), "日本");
        assert_eq!(url_decode("%F0%9F%98%80").unwrap(), "😀");
        assert_eq!(url_decode("a%0D%0Ab").unwrap(), "a\r\nb");
        assert_eq!(url_decode("日本 abc").unwrap(), "日本 abc");
        assert_eq!(url_decode("").unwrap(), "");
    }

    #[test]
    fn url_decode_rejects_broken_input() {
        assert_eq!(url_decode("100%"), None);
        assert_eq!(url_decode("%4"), None);
        assert_eq!(url_decode("%zz"), None);
        assert_eq!(url_decode("%4g"), None);
        // Not UTF-8: a lone continuation byte, a truncated sequence.
        assert_eq!(url_decode("%80"), None);
        assert_eq!(url_decode("%E6%97"), None);
    }

    #[test]
    fn html_escape_changes_five_characters() {
        assert_eq!(
            html_escape("a&b<c>\"d\"'e'"),
            "a&amp;b&lt;c&gt;&quot;d&quot;&#39;e&#39;"
        );
        assert_eq!(html_escape("日本 😀\r\n"), "日本 😀\r\n");
        assert_eq!(html_escape("&amp;"), "&amp;amp;");
    }

    #[test]
    fn html_unescape_reads_names_and_numbers() {
        assert_eq!(
            html_unescape("&amp; &lt; &gt; &quot; &#39; &apos;"),
            "& < > \" ' '"
        );
        assert_eq!(
            html_unescape("&#x27;&#X27;&#65;&#x1F600;&#128512;"),
            "''A😀😀"
        );
        assert_eq!(html_unescape("&amp;lt;"), "&lt;");
        assert_eq!(html_unescape("日本&amp;\r\n&lt;"), "日本&\r\n<");
    }

    #[test]
    fn html_unescape_leaves_what_it_does_not_know() {
        for s in [
            "&nbsp;",
            "&amp",
            "& amp;",
            "&;",
            "&#;",
            "&#x;",
            "&#xZ1;",
            "&#0;&#xD800;&#x110000;&#99999999999;",
            "&&amp",
            "a & b",
            "&",
        ] {
            assert_eq!(html_unescape(s), s);
        }
    }

    #[test]
    fn base64_encode_pads() {
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("f"), "Zg==");
        assert_eq!(base64_encode("fo"), "Zm8=");
        assert_eq!(base64_encode("foo"), "Zm9v");
        assert_eq!(base64_encode("foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode("日本語"), "5pel5pys6Kqe");
        assert_eq!(base64_encode("😀"), "8J+YgA==");
        assert_eq!(base64_encode("a\r\nb"), "YQ0KYg==");
    }

    #[test]
    fn base64_decode_skips_white_space_and_accepts_missing_padding() {
        assert_eq!(base64_decode_text("Zm9vYmFy").unwrap(), "foobar");
        assert_eq!(base64_decode_text("Zg==").unwrap(), "f");
        assert_eq!(base64_decode_text("Zg").unwrap(), "f");
        assert_eq!(base64_decode_text("Zm8").unwrap(), "fo");
        assert_eq!(base64_decode_text("Zm9v\r\nYmFy\n").unwrap(), "foobar");
        assert_eq!(base64_decode_text(" Zm 9v\tYmFy ").unwrap(), "foobar");
        assert_eq!(base64_decode_text("5pel5pys6Kqe").unwrap(), "日本語");
        assert_eq!(base64_decode_text("8J+YgA==").unwrap(), "😀");
        assert_eq!(base64_decode_text("").unwrap(), "");
    }

    #[test]
    fn base64_decode_rejects_broken_input() {
        assert_eq!(base64_decode_text("Zm9v!"), None);
        assert_eq!(base64_decode_text("Zm9v日"), None);
        // URL-safe alphabet.
        assert_eq!(base64_decode_text("-_-_"), None);
        // A `=` in the middle, too much or too little padding, an impossible length.
        assert_eq!(base64_decode_text("Zg==Zg=="), None);
        assert_eq!(base64_decode_text("Zg==="), None);
        assert_eq!(base64_decode_text("Zm9v="), None);
        assert_eq!(base64_decode_text("Zg="), None);
        assert_eq!(base64_decode_text("Z"), None);
        assert_eq!(base64_decode_text("Zm9vY"), None);
        // Not UTF-8: the bytes FF FE.
        assert_eq!(base64_decode_text("//4="), None);
    }

    #[test]
    fn encode_then_decode_gives_the_original() {
        for s in SAMPLES {
            assert_eq!(url_decode(&url_encode(s)).as_deref(), Some(s), "{s:?}");
            assert_eq!(html_unescape(&html_escape(s)), s, "{s:?}");
            assert_eq!(
                base64_decode_text(&base64_encode(s)).as_deref(),
                Some(s),
                "{s:?}"
            );
        }
    }
}
