//! Tests of the code pages 51932 (`euc.rs`) and 20932 (`cp20932.rs`). They run the same on every
//! OS: nothing here reads a path or calls a Windows API.
//!
//! The bytes of JIS X 0208 were made with Python's `euc_jp` codec (independent of `encoding_rs`).
//! The facts about 20932 are what Windows' `WideCharToMultiByte(20932, ...)` and
//! `MultiByteToWideChar(20932, ...)` gave on 2026-11-27 (measured by the coordinator); the ranges
//! outside JIS X 0208 of 51932 are the bytes of Sakura's `CEuc` (the CP932 code and the
//! Shift_JIS to JIS arithmetic: CP932 `0x8740` is JIS `0x2D21`, EUC `0xADA1`; the NEC-selected
//! form of `髙`, `0xEEE0`, is JIS `0x7C62`, EUC `0xFCE2`).

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
        // 51932: the NEC-selected form (`SjisFilter_ibm2nec`). 20932: JIS X 0212 plane, row 0xF4.
        name: "IBM extension",
        text: "\u{9AD9}",
        cp51932: Some(b"\xFC\xE2"),
        cp20932: Some(b"\xF4\x29"),
    },
    Range {
        // 20932 has no 0x8F prefix: the trail byte has no top bit.
        name: "JIS X 0212",
        text: "\u{2D8}\u{4E02}",
        cp51932: None,
        cp20932: Some(b"\xA2\x2F\xB0\x21"),
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
    // 20932: 鷗 is `0xEC 0x3F` (JIS X 0212 plane), and a character outside the BMP is two `?`
    // (Windows writes `?` for each UTF-16 unit).
    let mut out = Vec::new();
    let error = cp(20932).codec().encode(text, &mut out).unwrap_err();
    assert_eq!(out, b"\xBF\xB9\xEC\x3F\xB3\xB0??");
    assert_eq!(error.offsets, vec![9]);
    // The user-defined area of CP932 (U+E000) has no JIS X 0208 code in 51932.
    let mut out = Vec::new();
    assert!(cp(51932).codec().encode("a\u{E000}b", &mut out).is_err());
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
fn code_page_51932_has_no_jis_x_0212() {
    // The 0x8F is an invalid byte; 0xA2AF after it is no JIS X 0208 code either.
    assert_eq!(
        decode(cp(51932), b"\x8F\xA2\xAF"),
        format!("{}{}{}", s(0x8F), s(0xA2), s(0xAF))
    );
    assert_eq!(cp(51932).codec().char_len(b"\x8F\xA2\xAF", 0), 1);
}

/// Windows' `WideCharToMultiByte(20932, ...)` and `MultiByteToWideChar(20932, ...)`, a few facts
/// of each range.
#[test]
fn code_page_20932_follows_windows() {
    let both = |text: &str, bytes: &[u8]| {
        assert_eq!(encode_ok(cp(20932), text), bytes, "write {text:?}");
        assert_eq!(decode(cp(20932), bytes), text, "read {bytes:02X?}");
    };
    // JIS X 0212 plane: the trail byte has no top bit.
    both("\u{4E02}", b"\xB0\x21"); // 丂
    both("\u{C0}", b"\xAA\x22"); // À
    both("\u{A6}", b"\xA2\x43"); // ¦
    both("\u{2122}", b"\xA2\x6F"); // ™
    // The IBM extensions that JIS X 0212 has stay where it has them (纊), the others are in
    // rows 0xF3 and 0xF4 of that plane in the order of CP932.
    both("\u{7E8A}", b"\xD4\x63");
    both("\u{2170}", b"\xF3\x21"); // ⅰ
    both("\u{FF07}", b"\xF3\x35"); // ＇
    both("\u{FF02}", b"\xF3\x36"); // ＂
    both("\u{FA11}", b"\xF3\x49"); // 﨑
    both("\u{9AD9}", b"\xF4\x29"); // 髙
    both("\u{974D}", b"\xF4\x63");
    // NEC row 13 and the half-width katakana.
    both("\u{2460}", b"\xAD\xA1");
    both("\u{FF71}", b"\x8E\xB1");
    // JIS-style where CP932 is MS-style: the MS-style characters are `?`.
    both("\u{301C}", b"\xA1\xC1");
    both("\u{2016}", b"\xA1\xC2");
    both("\u{2212}", b"\xA1\xDD");
    both("\u{A2}", b"\xA1\xF1");
    both("\u{A3}", b"\xA1\xF2");
    both("\u{AC}", b"\xA2\xCC");
    both("\u{2015}", b"\xA1\xBD");
    for c in [
        "\u{FF5E}", "\u{FF0D}", "\u{2225}", "\u{FFE0}", "\u{FFE1}", "\u{FFE2}", "\u{FFE4}",
        "\u{2014}", "\u{A5}", "\u{203E}", "\u{663B}",
    ] {
        let mut out = Vec::new();
        assert!(cp(20932).codec().encode(c, &mut out).is_err(), "{c:?}");
        assert_eq!(out, b"?", "{c:?}");
    }
    // The user-defined area: JIS X 0208 plane first, then the JIS X 0212 plane.
    both("\u{E000}", b"\xF5\xA1");
    both("\u{E178}", b"\xF9\xA1");
    both("\u{E3AB}", b"\xFE\xFE");
    both("\u{E3AC}", b"\xF5\x21");
    both("\u{E757}", b"\xFE\x7E");
    // No best fit; U+0080..=U+009F are read (as C1 controls) but not written; outside the BMP
    // is one `?` per UTF-16 unit.
    assert_eq!(decode(cp(20932), b"\x80\x8F\x9F"), "\u{80}\u{8F}\u{9F}");
    let mut out = Vec::new();
    assert!(
        cp(20932)
            .codec()
            .encode("\u{80}\u{1F600}", &mut out)
            .is_err()
    );
    assert_eq!(out, b"???");
}

#[test]
fn code_page_20932_reads_invalid_bytes_as_stand_ins() {
    let codec = cp(20932).codec();
    // 0xA0 and 0xFF are not in the code page; a lone 0x8E, a lead byte at the end, and a lead
    // byte before a byte that is no trail byte are one stand-in, and the next byte goes on.
    assert_eq!(
        decode(cp(20932), b"\xA0\xFF"),
        format!("{}{}", s(0xA0), s(0xFF))
    );
    assert_eq!(decode(cp(20932), b"\x8Ea"), format!("{}a", s(0x8E)));
    assert_eq!(decode(cp(20932), b"\xB0"), s(0xB0));
    assert_eq!(decode(cp(20932), b"\xB0 "), format!("{} ", s(0xB0)));
    assert_eq!(codec.char_len(b"\xB0 ", 0), 1);
    assert_eq!(codec.char_len(b"\xB0\x21", 0), 2);
    // A pair with no character (JIS X 0212 plane, row 2, column 0x01): the lead is a stand-in,
    // and the trail is read again (here as ASCII).
    let text = decode(cp(20932), b"\xA2\x21");
    assert_eq!(text, format!("{}!", s(0xA2)));
    assert_eq!(encode_ok(cp(20932), &text), b"\xA2!");
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
