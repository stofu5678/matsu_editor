//! Tests of the code pages 51932 and 20932 (`euc.rs`). They run the same on every OS: nothing here
//! reads a path or a Windows API.
//!
//! The bytes of JIS X 0208 and JIS X 0212 were made with Python's `euc_jp` codec (independent of
//! `encoding_rs`). NEC row 13 and the IBM extensions are not in that codec: their bytes follow from
//! the CP932 code and the Shift_JIS to JIS arithmetic (CP932 `0x8740` is JIS `0x2D21`, EUC
//! `0xADA1`; the NEC-selected form `0xEEE0` is JIS `0x7C62`, EUC `0xFCE2`).

use super::*;

fn cp(number: u16) -> Encoding {
    Encoding::CodePage(CodePage::new(number).expect("supported code page"))
}

fn decode(encoding: Encoding, bytes: &[u8]) -> String {
    let mut out = String::new();
    encoding.codec().decode(bytes, &mut out);
    out
}

fn encode_ok(encoding: Encoding, text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    encoding
        .codec()
        .encode(text, &mut out)
        .unwrap_or_else(|e| panic!("{text:?} in {encoding:?}: {e:?}"));
    out
}

fn s(byte: u8) -> String {
    stand_in(byte).to_string()
}

/// A range of characters and the bytes of each code page (None: the code page cannot hold it).
struct Range {
    name: &'static str,
    text: &'static str,
    cp51932: Option<&'static [u8]>,
    cp20932: Option<&'static [u8]>,
}

const RANGES: &[Range] = &[
    Range {
        name: "ASCII",
        text: "Abc 123",
        cp51932: Some(b"Abc 123"),
        cp20932: Some(b"Abc 123"),
    },
    Range {
        name: "half-width katakana (0x8E)",
        text: "\u{FF71}\u{FF72}\u{FF73}\u{FF9E}",
        cp51932: Some(b"\x8E\xB1\x8E\xB2\x8E\xB3\x8E\xDE"),
        cp20932: Some(b"\x8E\xB1\x8E\xB2\x8E\xB3\x8E\xDE"),
    },
    Range {
        name: "JIS X 0208",
        text: "日本語ＡＢＣ\u{3000}１２３",
        cp51932: Some(
            b"\xC6\xFC\xCB\xDC\xB8\xEC\xA3\xC1\xA3\xC2\xA3\xC3\xA1\xA1\xA3\xB1\xA3\xB2\xA3\xB3",
        ),
        cp20932: Some(
            b"\xC6\xFC\xCB\xDC\xB8\xEC\xA3\xC1\xA3\xC2\xA3\xC3\xA1\xA1\xA3\xB1\xA3\xB2\xA3\xB3",
        ),
    },
    Range {
        name: "NEC row 13",
        text: "\u{2460}",
        cp51932: Some(b"\xAD\xA1"),
        cp20932: Some(b"\xAD\xA1"),
    },
    Range {
        name: "IBM extension (written as the NEC-selected form)",
        text: "\u{9AD9}",
        cp51932: Some(b"\xFC\xE2"),
        cp20932: Some(b"\xFC\xE2"),
    },
    Range {
        name: "JIS X 0212 (0x8F)",
        text: "\u{2D8}\u{4E02}",
        cp51932: None,
        cp20932: Some(b"\x8F\xA2\xAF\x8F\xB0\xA1"),
    },
];

fn bytes_of(range: &Range, number: u16) -> Option<&'static [u8]> {
    if number == 51932 {
        range.cp51932
    } else {
        range.cp20932
    }
}

#[test]
fn each_range_is_written_and_read_back() {
    for range in RANGES {
        for number in [51932u16, 20932] {
            let name = format!("{} in CP{number}", range.name);
            let encoding = cp(number);
            let mut out = Vec::new();
            let result = encoding.codec().encode(range.text, &mut out);
            match bytes_of(range, number) {
                Some(bytes) => {
                    assert!(result.is_ok(), "{name}");
                    assert_eq!(out, bytes, "{name}");
                    assert_eq!(decode(encoding, bytes), range.text, "{name}");
                    // Reading char by char gives the same text as reading the whole.
                    let codec = encoding.codec();
                    let (mut pieces, mut i) = (String::new(), 0);
                    while i < bytes.len() {
                        let len = codec.char_len(bytes, i);
                        codec.decode(&bytes[i..i + len], &mut pieces);
                        i += len;
                    }
                    assert_eq!(pieces, range.text, "{name} by pieces");
                }
                None => {
                    // `?` for each character, and the offsets are reported (the save warning).
                    assert!(result.is_err(), "{name}");
                    assert_eq!(out, b"?".repeat(range.text.chars().count()), "{name}");
                }
            }
        }
    }
}

#[test]
fn what_a_code_page_cannot_hold_is_a_question_mark() {
    // U+9DD7 (鷗) is only in JIS X 0212: Sakura's own test writes `森?外` in EUC (CP932 has none).
    let text = "森鷗外\u{1F600}";
    let mut out = Vec::new();
    let error = cp(51932).codec().encode(text, &mut out).unwrap_err();
    assert_eq!(out, b"\xBF\xB9?\xB3\xB0?");
    assert_eq!(error.offsets, vec![3, 9]);
    let mut out = Vec::new();
    let error = cp(20932).codec().encode(text, &mut out).unwrap_err();
    assert_eq!(out, b"\xBF\xB9\x8F\xEC\xBF\xB3\xB0?");
    assert_eq!(error.offsets, vec![9]);
    // The same for a character of the user-defined area (no JIS X 0208 code).
    let mut out = Vec::new();
    assert!(cp(20932).codec().encode("a\u{E000}b", &mut out).is_err());
    assert_eq!(out, b"a?b");
}

#[test]
fn line_endings_next_to_double_byte_characters() {
    for number in [51932u16, 20932] {
        let encoding = cp(number);
        // CR LF, LF and a lone CR (0x0A and 0x0D are never trail bytes).
        let bytes = b"\xA4\xA2\r\n\xA4\xA4\n\x8E\xB1\r";
        let text = decode(encoding, bytes);
        assert_eq!(text, "あ\r\nい\n\u{FF71}\r", "CP{number}");
        assert_eq!(encode_ok(encoding, &text), bytes, "CP{number}");
        let codec = encoding.codec();
        assert_eq!(codec.find_eol(bytes, 0), Some((2, Eol::CrLf)));
        assert_eq!(codec.find_eol(bytes, 4), Some((6, Eol::Lf)));
        assert_eq!(codec.eol_bytes(Eol::CrLf), Some(&b"\r\n"[..]));
        assert_eq!(codec.eol_bytes(Eol::Lf), Some(&b"\n"[..]));
    }
}

#[test]
fn jis_x_0212_is_only_in_20932() {
    // In 51932 the 0x8F is an invalid byte; 0xA2AF after it is no JIS X 0208 code either.
    assert_eq!(
        decode(cp(51932), b"\x8F\xA2\xAF"),
        format!("{}{}{}", s(0x8F), s(0xA2), s(0xAF))
    );
    assert_eq!(cp(51932).codec().char_len(b"\x8F\xA2\xAF", 0), 1);
    assert_eq!(cp(20932).codec().char_len(b"\x8F\xA2\xAF", 0), 3);
    // A JIS X 0212 code without a character is three stand-ins, and they are written back.
    let text = decode(cp(20932), b"\x8F\xFE\xFE");
    assert_eq!(text, format!("{}{}{}", s(0x8F), s(0xFE), s(0xFE)));
    assert_eq!(encode_ok(cp(20932), &text), b"\x8F\xFE\xFE");
    // A cut 0x8F at the end is one invalid byte.
    assert_eq!(decode(cp(20932), b"a\x8F"), format!("a{}", s(0x8F)));
}

#[test]
fn the_two_code_pages_are_stored_and_named_by_number() {
    for number in [51932u16, 20932] {
        let encoding = cp(number);
        assert_eq!(encoding.sakura_code(), i32::from(number));
        assert_eq!(
            Encoding::from_sakura_code(i32::from(number)),
            Some(encoding)
        );
        assert!(Encoding::choices().contains(&encoding));
        assert_eq!(encoding.combo_name(), format!("CP{number}"));
        assert_eq!(encoding.bracket_name(), format!("  [CP{number}]"));
    }
    // `CODE_EUC` stays a different encoding.
    assert_ne!(cp(51932), Encoding::EucJp);
}
