//! Tests of the encodings. Expectations that come from Sakura cite the file they come from; the
//! files are in `src/test/cpp/tests1/` of Sakura Editor.

use super::*;

fn decode(encoding: Encoding, bytes: &[u8]) -> String {
    let mut out = String::new();
    encoding.codec().decode(bytes, &mut out);
    out
}

fn encode(encoding: Encoding, text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let _ = encoding.codec().encode(text, &mut out);
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

fn unmappable(encoding: Encoding, text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    encoding
        .codec()
        .encode(text, &mut out)
        .expect_err("some chars should not be encodable")
        .offsets
}

/// The stand-in for a byte, as a string.
fn s(byte: u8) -> String {
    stand_in(byte).to_string()
}

/// ASCII 0x01..=0x7F, which Sakura's tests use for every encoding (`test-ccodebase.cpp`).
fn ascii_text() -> String {
    (1u8..=0x7F).map(char::from).collect()
}

const STATELESS: [Encoding; 7] = [
    Encoding::ShiftJis,
    Encoding::EucJp,
    Encoding::Utf16Le,
    Encoding::Utf16Be,
    Encoding::Utf8,
    Encoding::Cesu8,
    Encoding::Latin1,
];

// ---- Encoding ----

#[test]
fn sakura_codes_match_ecodetype() {
    // sakura_core/charset/charset.h
    let expected = [
        (Encoding::ShiftJis, 0),
        (Encoding::Jis, 1),
        (Encoding::EucJp, 2),
        (Encoding::Utf16Le, 3),
        (Encoding::Utf8, 4),
        (Encoding::Utf7, 5),
        (Encoding::Utf16Be, 6),
        (Encoding::Cesu8, 7),
        (Encoding::Latin1, 8),
    ];
    for (encoding, code) in expected {
        assert_eq!(encoding.sakura_code(), code);
        assert_eq!(Encoding::from_sakura_code(code), Some(encoding));
    }
    assert_eq!(Encoding::from_sakura_code(AUTODETECT_CODE), None);
    assert_eq!(Encoding::from_sakura_code(12000), None);
    assert_eq!(Encoding::from_sakura_code(-1), None);
    assert_eq!(Encoding::ALL.len(), 9);
}

#[test]
fn names_match_sakura_resources() {
    // sakura_core/charset/charset.cpp, ASCodeSet
    assert_eq!(Encoding::ShiftJis.short_name(), "SJIS");
    assert_eq!(Encoding::Utf16Le.short_name(), "UTF-16");
    assert_eq!(Encoding::Utf16Be.short_name(), "U16BE");
    assert_eq!(Encoding::Utf16Be.combo_name(), "UTF-16BE");
    assert_eq!(Encoding::EucJp.short_name(), "EUC");
    assert_eq!(Encoding::EucJp.combo_name(), "EUC-JP");
    assert_eq!(Encoding::Latin1.combo_name(), "Latin1");
    // test-charset.cpp: Bracket() is Short() in "  [" and "]"
    assert_eq!(Encoding::EucJp.bracket_name(), "  [EUC]");
    assert!(Encoding::Utf8.can_have_bom() && !Encoding::Utf8.bom_default_on());
    assert!(Encoding::Utf16Le.can_have_bom() && Encoding::Utf16Le.bom_default_on());
    assert!(!Encoding::ShiftJis.can_have_bom());
    assert!(!Encoding::Jis.can_be_default() && !Encoding::Utf7.can_be_default());
    assert!(Encoding::Latin1.can_be_default());
}

#[test]
fn bom_bytes_match_sakura() {
    // test-ccodebase.cpp, GetBomCases
    assert_eq!(Encoding::ShiftJis.bom(), b"");
    assert_eq!(Encoding::Jis.bom(), b"");
    assert_eq!(Encoding::EucJp.bom(), b"");
    assert_eq!(Encoding::Latin1.bom(), b"");
    assert_eq!(Encoding::Utf7.bom(), b"+/v8-");
    assert_eq!(Encoding::Utf16Le.bom(), b"\xFF\xFE");
    assert_eq!(Encoding::Utf16Be.bom(), b"\xFE\xFF");
    assert_eq!(Encoding::Utf8.bom(), b"\xEF\xBB\xBF");
    assert_eq!(Encoding::Cesu8.bom(), b"\xEF\xBB\xBF");
}

#[test]
fn bom_is_found_and_the_bom_text_is_u_feff() {
    assert_eq!(Encoding::Utf8.bom_len(b"\xEF\xBB\xBFabc"), 3);
    assert_eq!(Encoding::Utf8.bom_len(b"abc"), 0);
    assert_eq!(Encoding::Utf8.bom_len(b"\xEF\xBB"), 0);
    assert_eq!(Encoding::Utf16Le.bom_len(b"\xFF\xFEa\0"), 2);
    assert_eq!(Encoding::Utf16Be.bom_len(b"\xFF\xFEa\0"), 0);
    assert_eq!(Encoding::ShiftJis.bom_len(b"\xEF\xBB\xBF"), 0);
    // Every BOM decodes to U+FEFF in its own encoding, and U+FEFF encodes to the BOM.
    for encoding in Encoding::ALL {
        if encoding.bom().is_empty() {
            continue;
        }
        assert_eq!(decode(encoding, encoding.bom()), "\u{FEFF}", "{encoding:?}");
        assert_eq!(encode(encoding, "\u{FEFF}"), encoding.bom(), "{encoding:?}");
    }
}

// ---- Shift_JIS ----

#[test]
fn sjis_ascii_and_kana_kanji_from_sakura() {
    // test-ccodebase.cpp, codeSJis
    let ascii = ascii_text();
    assert_eq!(decode(Encoding::ShiftJis, ascii.as_bytes()), ascii);
    assert_eq!(encode_ok(Encoding::ShiftJis, &ascii), ascii.as_bytes());

    let text = "ｶﾅかなカナ漢字";
    let bytes = b"\xB6\xC5\x82\xA9\x82\xC8\x83\x4A\x83\x69\x8A\xBF\x8E\x9A";
    assert_eq!(decode(Encoding::ShiftJis, bytes), text);
    assert_eq!(encode_ok(Encoding::ShiftJis, text), bytes);
}

#[test]
fn sjis_characters_that_do_not_come_back_are_invalid_bytes() {
    // test-ccodebase.cpp, codeSJis: NEC extension, NEC-selected IBM extension, IBM extension
    assert_eq!(
        decode(Encoding::ShiftJis, b"\x87\x40\xED\x40\xFA\x40"),
        format!("\u{2460}{}{}\u{2170}", s(0xED), s(0x40))
    );
    // invalid first byte
    assert_eq!(
        decode(Encoding::ShiftJis, b"\x80\x40\xFD\x40\xFE\x40\xFF\x40"),
        format!("{}@{}@{}@{}@", s(0x80), s(0xFD), s(0xFE), s(0xFF))
    );
    // invalid second byte
    assert_eq!(
        decode(
            Encoding::ShiftJis,
            b"\x81\x0A\x81\x7F\x81\xFD\x81\xFE\x81\xFF"
        ),
        format!(
            "{}\n{}\x7F{}{}{}{}{}{}",
            s(0x81),
            s(0x81),
            s(0x81),
            s(0xFD),
            s(0x81),
            s(0xFE),
            s(0x81),
            s(0xFF)
        )
    );
}

#[test]
fn sjis_unmappable_is_a_question_mark() {
    // test-ccodebase.cpp, codeSJis: 森鷗外 gives 森?外 (鷗 is not in CP932)
    let mut out = Vec::new();
    let error = Encoding::ShiftJis
        .codec()
        .encode("森鷗外", &mut out)
        .unwrap_err();
    assert_eq!(out, b"\x90\x58\x3F\x8A\x4F");
    assert_eq!(error.offsets, vec!["森".len()]);
    // A char outside the BMP is one `?`.
    assert_eq!(encode(Encoding::ShiftJis, "a😀b"), b"a?b");
    assert_eq!(unmappable(Encoding::ShiftJis, "a😀b"), vec![1]);
}

#[test]
fn sjis_cp932_special_cases() {
    // The Windows CP932 table, which Sakura uses (see the module documentation of cp932.rs).
    let cases: [(&[u8], char); 9] = [
        (b"\x81\x5F", '\u{FF3C}'), // FULLWIDTH REVERSE SOLIDUS
        (b"\x81\x60", '\u{FF5E}'), // FULLWIDTH TILDE, not U+301C
        (b"\x81\x61", '\u{2225}'), // PARALLEL TO, not U+2016
        (b"\x81\x7C", '\u{FF0D}'), // FULLWIDTH HYPHEN-MINUS, not U+2212
        (b"\x81\x91", '\u{FFE0}'), // FULLWIDTH CENT SIGN
        (b"\x81\x92", '\u{FFE1}'), // FULLWIDTH POUND SIGN
        (b"\x81\xCA", '\u{FFE2}'), // FULLWIDTH NOT SIGN
        (b"\x81\x5C", '\u{2015}'), // HORIZONTAL BAR
        (b"\x5C", '\\'),           // a backslash, not a yen sign
    ];
    for (bytes, c) in cases {
        assert_eq!(
            decode(Encoding::ShiftJis, bytes),
            c.to_string(),
            "{bytes:02X?}"
        );
        assert_eq!(encode_ok(Encoding::ShiftJis, &c.to_string()), bytes);
    }
    assert_eq!(decode(Encoding::ShiftJis, b"\x7E"), "~");
}

#[test]
fn sjis_characters_windows_would_approximate_are_not_encoded() {
    // Difference from Sakura, item 2 of the module documentation of `encoding`:
    // Windows converts some of these without a report; we write `?`.
    for c in [
        '\u{301C}', // WAVE DASH: JIS X 0208 has it, CP932 does not
        '\u{2016}', // DOUBLE VERTICAL LINE
        '\u{2212}', // MINUS SIGN
        '\u{00A2}', '\u{00A3}',
        '\u{00AC}', // cent, pound, not signs (Sakura rejects them too)
        '\u{00A5}', // yen sign (Sakura rejects it too: TABLE_WctombSpec)
        '\u{203E}', // overline
        '\u{3094}', // HIRAGANA LETTER VU (Sakura rejects it: IsWctombcNonroundtrip)
    ] {
        assert_eq!(
            unmappable(Encoding::ShiftJis, &c.to_string()),
            vec![0],
            "{c:?}"
        );
    }
}

#[test]
fn sjis_duplicate_codes_resolve_like_sakura() {
    // codeutil.cpp, TABLE_SjisPoorcodeDef: a char with two codes is written with the code of
    // row 2 of JIS X 0208 first, then NEC row 13, then the IBM extension.
    let cases: [(char, &[u8]); 8] = [
        ('\u{2252}', b"\x81\xE0"), // ≒: 81E0 (not 8790)
        ('\u{2235}', b"\x81\xE6"), // ∵: 81E6 (not 879A, FA5B)
        ('\u{FFE2}', b"\x81\xCA"), // ￢: 81CA (not EEF9, FA54)
        ('\u{2160}', b"\x87\x54"), // Ⅰ: 8754 (not FA4A)
        ('\u{3231}', b"\x87\x8A"), // ㈱: 878A (not FA58)
        ('\u{2116}', b"\x87\x82"), // №: 8782 (not FA59)
        ('\u{2170}', b"\xFA\x40"), // ⅰ: FA40 (not EEEF)
        ('\u{7E8A}', b"\xFA\x5C"), // 纊: FA5C (not ED40)
    ];
    for (c, bytes) in cases {
        assert_eq!(
            encode_ok(Encoding::ShiftJis, &c.to_string()),
            bytes,
            "{c:?}"
        );
        assert_eq!(
            decode(Encoding::ShiftJis, bytes),
            c.to_string(),
            "{bytes:02X?}"
        );
    }
    // The other codes of the same chars are invalid, because they would not come back.
    for bytes in [
        &b"\x87\x90"[..], // ≒ in NEC row 13
        b"\x87\x9A",      // ∵
        b"\xEE\xF9",      // ￢
        b"\xFA\x54",      // ￢ in the IBM extension
        b"\xFA\x4A",      // Ⅰ
        b"\xEE\xEF",      // ⅰ in the NEC-selected IBM extension
        b"\xED\x40",      // 纊
    ] {
        assert_eq!(
            decode(Encoding::ShiftJis, bytes),
            format!("{}{}", s(bytes[0]), s(bytes[1])),
            "{bytes:02X?}"
        );
    }
}

#[test]
fn sjis_user_defined_area_round_trips() {
    // 0xF040..=0xF9FC are U+E000..=U+E757 in CP932.
    assert_eq!(decode(Encoding::ShiftJis, b"\xF0\x40"), "\u{E000}");
    assert_eq!(encode_ok(Encoding::ShiftJis, "\u{E000}"), b"\xF0\x40");
    assert_eq!(decode(Encoding::ShiftJis, b"\xF9\xFC"), "\u{E757}");
}

#[test]
fn sjis_char_len_is_the_unit_that_decode_consumes() {
    let codec = Encoding::ShiftJis.codec();
    let bytes = b"a\xB6\x82\xA0\xED\x40\x81\x0A\x80\x81";
    let lens: Vec<usize> = [0, 1, 2, 4, 6, 7, 8, 9]
        .map(|i| codec.char_len(bytes, i))
        .to_vec();
    // a, ｶ, あ, ED40 (two stand-ins together), 81 (alone), LF, 80, 81 (at the end)
    assert_eq!(lens, vec![1, 1, 2, 2, 1, 1, 1, 1]);
}

// ---- EUC-JP ----

#[test]
fn euc_kana_kanji_from_sakura() {
    // test-ccodebase.cpp, codeEucJp
    let ascii = ascii_text();
    assert_eq!(decode(Encoding::EucJp, ascii.as_bytes()), ascii);
    assert_eq!(encode_ok(Encoding::EucJp, &ascii), ascii.as_bytes());

    let text = "ｶﾅかなカナ漢字";
    let bytes = b"\x8E\xB6\x8E\xC5\xA4\xAB\xA4\xCA\xA5\xAB\xA5\xCA\xB4\xC1\xBB\xFA";
    assert_eq!(decode(Encoding::EucJp, bytes), text);
    assert_eq!(encode_ok(Encoding::EucJp, text), bytes);
}

#[test]
fn euc_has_no_jis_x_0212() {
    // test-ccodebase.cpp, codeEucJp: 森鷗外 gives 森?外
    let mut out = Vec::new();
    let error = Encoding::EucJp
        .codec()
        .encode("森鷗外", &mut out)
        .unwrap_err();
    assert_eq!(out, b"\xBF\xB9\x3F\xB3\xB0");
    assert_eq!(error.offsets, vec![3]);
    // The 0x8F is one invalid byte; the next two bytes are a char of their own.
    assert_eq!(
        decode(Encoding::EucJp, b"\x8F\xB0\xA1"),
        format!("{}\u{4E9C}", s(0x8F))
    );
}

#[test]
fn euc_goes_through_cp932() {
    // IBM extension 纊 is JIS row 89 (NEC-selected IBM extension 0xED40) in EUC.
    assert_eq!(encode_ok(Encoding::EucJp, "\u{7E8A}"), b"\xF9\xA1");
    assert_eq!(decode(Encoding::EucJp, b"\xF9\xA1"), "\u{7E8A}");
    // NEC row 13 is JIS row 13: ① is 0x2D21 (SJIS 0x8740).
    assert_eq!(encode_ok(Encoding::EucJp, "\u{2460}"), b"\xAD\xA1");
    assert_eq!(decode(Encoding::EucJp, b"\xAD\xA1"), "\u{2460}");
    // ≒ in row 13 (JIS 0x2D70, SJIS 0x8790) does not come back; row 2 does.
    assert_eq!(
        decode(Encoding::EucJp, b"\xAD\xF0"),
        format!("{}{}", s(0xAD), s(0xF0))
    );
    assert_eq!(encode_ok(Encoding::EucJp, "\u{2252}"), b"\xA2\xE2");
    // Rows beyond what CP932 has.
    assert_eq!(
        decode(Encoding::EucJp, b"\xFE\xFE"),
        format!("{}{}", s(0xFE), s(0xFE))
    );
    // The user-defined area has no JIS X 0208 code.
    assert_eq!(unmappable(Encoding::EucJp, "\u{E000}"), vec![0]);
    // Half-width katakana is 0x8E and the Shift_JIS byte.
    assert_eq!(encode_ok(Encoding::EucJp, "ｱ"), b"\x8E\xB1");
}

// ---- Latin1 ----

#[test]
fn latin1_from_sakura() {
    // test-ccodebase.cpp, codeLatin1: every byte has a char; 0x81 0x8D 0x8F 0x90 0x9D stay as they are
    let bytes: Vec<u8> = (0x80..=0xFF).collect();
    let text = decode(Encoding::Latin1, &bytes);
    let expected = "€\u{81}‚ƒ„…†‡ˆ‰Š‹Œ\u{8D}Ž\u{8F}\u{90}‘’“”•–—˜™š›œ\u{9D}žŸ\u{A0}¡¢£¤¥¦§¨©ª«¬\u{AD}®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ";
    assert_eq!(text, expected);
    assert_eq!(encode_ok(Encoding::Latin1, &text), bytes);

    // Japanese is `?` for each char
    let mut out = Vec::new();
    let error = Encoding::Latin1
        .codec()
        .encode("ｶﾅかなカナ漢字", &mut out)
        .unwrap_err();
    assert_eq!(out, b"????????");
    assert_eq!(error.offsets.len(), 8);
}

// ---- UTF-8 and CESU-8 ----

#[test]
fn utf8_from_sakura() {
    // test-ccodebase.cpp, codeUtf8
    let ascii = ascii_text();
    assert_eq!(decode(Encoding::Utf8, ascii.as_bytes()), ascii);
    let text = "ｶﾅかなカナ漢字";
    assert_eq!(decode(Encoding::Utf8, text.as_bytes()), text);
    assert_eq!(encode_ok(Encoding::Utf8, text), text.as_bytes());
    assert_eq!(decode(Encoding::Utf8, "😀".as_bytes()), "😀");
}

#[test]
fn utf8_invalid_sequences_are_one_stand_in_per_byte() {
    // CheckUtf8Char: the lead byte is invalid by itself and reading goes on with the next byte.
    assert_eq!(
        decode(Encoding::Utf8, b"A\xE3\x81B"),
        format!("A{}{}B", s(0xE3), s(0x81))
    );
    assert_eq!(
        decode(Encoding::Utf8, b"\xC0\x80"),
        format!("{}{}", s(0xC0), s(0x80))
    );
    assert_eq!(
        decode(Encoding::Utf8, b"\xED\xA0\x80"),
        format!("{}{}{}", s(0xED), s(0xA0), s(0x80))
    );
    assert_eq!(
        decode(Encoding::Utf8, b"\xF4\x90\x80\x80"),
        format!("{}{}{}{}", s(0xF4), s(0x90), s(0x80), s(0x80))
    );
    // A truncated char at the end.
    assert_eq!(
        decode(Encoding::Utf8, b"a\xE3\x81"),
        format!("a{}{}", s(0xE3), s(0x81))
    );
    assert_eq!(decode(Encoding::Utf8, b"\xFF"), s(0xFF));
    // A stand-in char itself is written as one byte (the documented collision).
    assert_eq!(encode(Encoding::Utf8, &s(0xE3)), b"\xE3");
}

#[test]
fn utf8_matches_std_for_valid_text_and_the_sakura_checker() {
    // CheckUtf8Char accepts exactly what std accepts: compare on every 1-4 byte prefix pattern.
    let mut sample = 0x1234_5678u32;
    for _ in 0..200_000 {
        sample = sample.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let len = 1 + (sample >> 28) as usize % 4;
        let mut bytes = [0u8; 4];
        for (k, byte) in bytes.iter_mut().enumerate() {
            sample = sample.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *byte = if k == 0 {
                (sample >> 24) as u8
            } else {
                0x80 | (sample >> 24) as u8 & 0x3F
            };
        }
        let bytes = &bytes[..len];
        let (checked_len, kind) = utf8::check_utf8_char(bytes, true);
        match std::str::from_utf8(bytes) {
            Ok(text) if text.chars().next().is_some_and(|c| c.len_utf8() == len) => {
                assert_eq!(
                    (checked_len, kind != utf8::CharKind::Binary),
                    (len, true),
                    "{bytes:02X?}"
                );
            }
            _ => {
                let first_len = std::str::from_utf8(&bytes[..checked_len]);
                assert!(
                    kind == utf8::CharKind::Binary || first_len.is_ok(),
                    "{bytes:02X?}"
                );
            }
        }
    }
}

#[test]
fn cesu8_from_sakura_and_surrogates() {
    // test-ccodebase.cpp, codeUtf8_OracleImplementation
    let text = "ｶﾅかなカナ漢字";
    assert_eq!(decode(Encoding::Cesu8, text.as_bytes()), text);
    assert_eq!(encode_ok(Encoding::Cesu8, text), text.as_bytes());
    // U+1F600 is two three-byte surrogates.
    let emoji = b"\xED\xA0\xBD\xED\xB8\x80";
    assert_eq!(decode(Encoding::Cesu8, emoji), "😀");
    assert_eq!(encode_ok(Encoding::Cesu8, "😀"), emoji);
    // A four-byte sequence is invalid in CESU-8.
    assert_eq!(
        decode(Encoding::Cesu8, "😀".as_bytes()),
        "😀".bytes().map(s).collect::<String>()
    );
    // A half of a pair is invalid.
    assert_eq!(
        decode(Encoding::Cesu8, b"\xED\xA0\xBDa"),
        format!("{}{}{}a", s(0xED), s(0xA0), s(0xBD))
    );
    assert_eq!(
        decode(Encoding::Cesu8, b"\xED\xB8\x80\xED\xA0\xBD"),
        [0xED, 0xB8, 0x80, 0xED, 0xA0, 0xBD].map(s).concat()
    );
}

// ---- UTF-16 ----

#[test]
fn utf16_from_sakura() {
    // test-ccodebase.cpp, codeUtf16Le and codeUtf16Be
    let text = "ｶﾅかなカナ漢字";
    let le: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let be: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
    assert_eq!(decode(Encoding::Utf16Le, &le), text);
    assert_eq!(decode(Encoding::Utf16Be, &be), text);
    assert_eq!(encode_ok(Encoding::Utf16Le, text), le);
    assert_eq!(encode_ok(Encoding::Utf16Be, text), be);
    let ascii = ascii_text();
    let le: Vec<u8> = ascii.encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert_eq!(decode(Encoding::Utf16Le, &le), ascii);
}

#[test]
fn utf16_surrogates_and_truncation() {
    assert_eq!(decode(Encoding::Utf16Le, b"\x3D\xD8\x00\xDE"), "😀");
    assert_eq!(decode(Encoding::Utf16Be, b"\xD8\x3D\xDE\x00"), "😀");
    // A lone high surrogate, a lone low surrogate: two stand-ins each.
    assert_eq!(
        decode(Encoding::Utf16Le, b"\x3D\xD8a\0"),
        format!("{}{}a", s(0x3D), s(0xD8))
    );
    assert_eq!(
        decode(Encoding::Utf16Le, b"\x00\xDEa\0"),
        format!("{}{}a", s(0x00), s(0xDE))
    );
    // A high surrogate at the end of the bytes.
    assert_eq!(
        decode(Encoding::Utf16Le, b"a\0\x3D\xD8"),
        format!("a{}{}", s(0x3D), s(0xD8))
    );
    // An odd last byte.
    assert_eq!(decode(Encoding::Utf16Le, b"a\0b"), format!("a{}", s(b'b')));
    let codec = Encoding::Utf16Le.codec();
    assert_eq!(codec.char_len(b"\x3D\xD8\x00\xDE", 0), 4);
    assert_eq!(codec.char_len(b"\x3D\xD8a\0", 0), 2);
    assert_eq!(codec.char_len(b"a", 0), 1);
}

// ---- UTF-7 ----

#[test]
fn utf7_from_sakura() {
    // test-ccodebase.cpp, codeUtf7: the ASCII block
    let mbs = "+AAEAAgADAAQABQAGAAcACA-\t\n+AAsADA-\r+AA4ADwAQABEAEgATABQAFQAWABcAGAAZABoAGwAcAB0AHgAf- +ACEAIgAjACQAJQAm-'()+ACoAKw-,-./0123456789:+ADsAPAA9AD4-?+AEA-ABCDEFGHIJKLMNOPQRSTUVWXYZ+AFsAXABdAF4AXwBg-abcdefghijklmnopqrstuvwxyz+AHsAfAB9AH4Afw-";
    let ascii = ascii_text();
    assert_eq!(decode(Encoding::Utf7, mbs.as_bytes()), ascii);
    assert_eq!(encode_ok(Encoding::Utf7, &ascii), mbs.as_bytes());

    // the kana and kanji
    let text = "ｶﾅかなカナ漢字";
    assert_eq!(decode(Encoding::Utf7, b"+/3b/hTBLMGowqzDKbyJbVw-"), text);
    assert_eq!(encode_ok(Encoding::Utf7, text), b"+/3b/hTBLMGowqzDKbyJbVw-");

    // C++
    assert_eq!(decode(Encoding::Utf7, b"C+-+-"), "C++");
    assert_eq!(encode_ok(Encoding::Utf7, "C++"), b"C+-+-");
}

#[test]
fn utf7_rfc_2152_examples_and_invalid_input() {
    // RFC 2152 section 5
    assert_eq!(
        decode(Encoding::Utf7, b"Hi Mom -+Jjo--!"),
        "Hi Mom -\u{263A}-!"
    );
    // `!` is in set O, which Sakura writes in Base64 (UniToUtf7 only writes set D directly).
    assert_eq!(
        encode_ok(Encoding::Utf7, "Hi Mom -\u{263A}-!"),
        b"Hi Mom -+Jjo--+ACE-"
    );
    assert_eq!(decode(Encoding::Utf7, b"+ZeVnLIqe-"), "日本語");
    assert_eq!(encode_ok(Encoding::Utf7, "日本語"), b"+ZeVnLIqe-");
    // A surrogate pair in Base64.
    assert_eq!(decode(Encoding::Utf7, b"+2D3eAA-"), "😀");
    assert_eq!(encode_ok(Encoding::Utf7, "😀"), b"+2D3eAA-");
    // The run ends at a byte that is not Base64, and `-` after it belongs to the run.
    assert_eq!(decode(Encoding::Utf7, b"a+Jjo.b"), "a\u{263A}.b");
    // Not direct: a byte above 0x7F, `\` and `~`; a `+` that starts nothing.
    assert_eq!(
        decode(Encoding::Utf7, b"\x80\\~"),
        format!("{}\\~", s(0x80))
            .replace('\\', &s(b'\\'))
            .replace('~', &s(b'~'))
    );
    assert_eq!(decode(Encoding::Utf7, b"+."), format!("{}.", s(b'+')));
    // Stand-ins are written as their byte.
    assert_eq!(encode(Encoding::Utf7, &format!("a{}b", s(0x80))), b"a\x80b");
    assert!(!Encoding::Utf7.codec().is_stateless());
}

// ---- ISO-2022-JP ----

#[test]
fn jis_kana_kanji_from_sakura() {
    // test-ccodebase.cpp, codeJis
    let text = "ｶﾅかなカナ漢字";
    let bytes = b"\x1B(I6E\x1B$B$+$J%+%J4A;z\x1B(B";
    assert_eq!(decode(Encoding::Jis, bytes), text);
    assert_eq!(encode_ok(Encoding::Jis, text), bytes);
    assert!(!Encoding::Jis.codec().is_stateless());
    // 森鷗外: 鷗 cannot be written; in JIS X 0208 mode Sakura writes the middle dot 0x2126.
    let mut out = Vec::new();
    let error = Encoding::Jis
        .codec()
        .encode("\u{9DD7}", &mut out)
        .unwrap_err();
    assert_eq!(out, b"?");
    assert_eq!(error.offsets, vec![0]);
    // In JIS X 0208 mode the middle dot 0x2126 stands for it (森 is 0x3F39).
    assert_eq!(encode(Encoding::Jis, "森\u{9DD7}"), b"\x1B$B?9!&\x1B(B");
}

#[test]
fn jis_mixed_text_and_escape_variants() {
    // 日 0x467C, 本 0x4B5C, 語 0x386C
    let bytes = b"abc\x1B$BF|K\\8l\x1B(Bdef";
    assert_eq!(decode(Encoding::Jis, bytes), "abc日本語def");
    assert_eq!(encode_ok(Encoding::Jis, "abc日本語def"), bytes);
    // JIS X 0201 Roman and the old JIS X 0208-1978 are read.
    assert_eq!(decode(Encoding::Jis, b"\x1B(Ja\\\x1B(Hb"), "a\\b");
    assert_eq!(decode(Encoding::Jis, b"\x1B$@F|\x1B(B"), "日");
    // Row 13 duplicates and the NEC-selected IBM extensions are read (no round-trip check).
    assert_eq!(decode(Encoding::Jis, b"\x1B$B-p\x1B(B"), "\u{2252}"); // 0x2D70 = SJIS 0x8790
    assert_eq!(decode(Encoding::Jis, b"\x1B$By!\x1B(B"), "\u{7E8A}"); // 0x7921 = SJIS 0xED40
    // ... and written as row 2 / the IBM extension's JIS code.
    assert_eq!(encode_ok(Encoding::Jis, "\u{2252}"), b"\x1B$B\"b\x1B(B");
    assert_eq!(encode_ok(Encoding::Jis, "\u{7E8A}"), b"\x1B$By!\x1B(B");
    // The newline is written in ASCII mode.
    assert_eq!(
        encode_ok(Encoding::Jis, "日\n日"),
        b"\x1B$BF|\x1B(B\n\x1B$BF|\x1B(B"
    );
}

#[test]
fn jis_invalid_bytes_are_stand_ins() {
    // An unknown escape sequence is three invalid bytes, and then only 0x21..=0x7E is read.
    assert_eq!(
        decode(Encoding::Jis, b"\x1B&@ab c\x7F"),
        format!("{}{}{}ab{}c{}", s(0x1B), s(b'&'), s(b'@'), s(b' '), s(0x7F))
    );
    // test-ccodebase.cpp, codeJis: ESC 0x1C 0x1D is such a sequence
    assert_eq!(
        decode(Encoding::Jis, b"\x1B\x1C\x1D!"),
        format!("{}{}{}!", s(0x1B), s(0x1C), s(0x1D))
    );
    // ESC at the end: one invalid byte. A byte above 0x7F in ASCII mode.
    assert_eq!(decode(Encoding::Jis, b"a\x1B"), format!("a{}", s(0x1B)));
    assert_eq!(decode(Encoding::Jis, b"a\x80"), format!("a{}", s(0x80)));
    // An odd last byte of JIS X 0208 and a pair that is not JIS (a line feed in the pair).
    assert_eq!(
        decode(Encoding::Jis, b"\x1B$BF|K"),
        format!("日{}", s(b'K'))
    );
    assert_eq!(
        decode(Encoding::Jis, b"\x1B$B\nF"),
        format!("{}{}", s(b'\n'), s(b'F'))
    );
    // test-ccodebase.cpp, codeJis: bytes above 0x7F in JIS X 0208 mode
    assert_eq!(
        decode(Encoding::Jis, b"\x1B$B\xFF\xFF\x1B(B"),
        format!("{}{}", s(0xFF), s(0xFF))
    );
    // JIS X 0212 (ESC ( D) is not supported.
    assert_eq!(
        decode(Encoding::Jis, b"\x1B(D33\x1B(B"),
        format!("{}{}{}33", s(0x1B), s(b'('), s(b'D'))
    );
    // Invalid bytes are written back as they are, in ASCII mode.
    let text = decode(Encoding::Jis, b"a\x80\x1B$BF|\x1B(B");
    assert_eq!(encode_ok(Encoding::Jis, &text), b"a\x80\x1B$BF|\x1B(B");
}

#[test]
fn jis_char_len_follows_the_state() {
    let codec = Encoding::Jis.codec();
    let bytes = b"a\x1B$BF|\x1B(Bb";
    // a, ESC $ B, 日, ESC ( B, b
    let starts = [0, 1, 4, 6, 9];
    let lens: Vec<usize> = starts.map(|i| codec.char_len(bytes, i)).to_vec();
    assert_eq!(lens, vec![1, 3, 2, 3, 1]);
}

// ---- Round trips of any bytes ----

fn pseudo_random_bytes(seed: u32, len: usize, high: bool) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let byte = (state >> 24) as u8;
            // Many bytes of the range where the multi-byte encodings keep their lead bytes.
            if high && byte < 0x60 {
                byte | 0x80
            } else {
                byte
            }
        })
        .collect()
}

#[test]
fn any_bytes_survive_decode_then_encode() {
    for encoding in STATELESS {
        let codec = encoding.codec();
        let mut inputs: Vec<Vec<u8>> = Vec::new();
        // every byte, and every pair of bytes
        inputs.push((0..=255u8).collect());
        for first in 0..=255u8 {
            inputs.push((0..=255u8).flat_map(|second| [first, second]).collect());
        }
        for seed in 1..40 {
            inputs.push(pseudo_random_bytes(seed, 4001, seed % 2 == 0));
        }
        for input in inputs {
            let mut text = String::new();
            codec.decode(&input, &mut text);
            let mut again = Vec::new();
            codec
                .encode(&text, &mut again)
                .unwrap_or_else(|e| panic!("{encoding:?}: {e:?}"));
            // Pairs like [first, second] are concatenated without a reset, so a comparison of the
            // whole input also checks that decoding goes on correctly after an invalid pair.
            assert_eq!(again, input, "{encoding:?}");
        }
    }
}

#[test]
fn decoding_in_pieces_at_char_boundaries_gives_the_same_text() {
    for encoding in STATELESS {
        let codec = encoding.codec();
        for seed in 1..20 {
            let bytes = pseudo_random_bytes(seed * 7, 3001, seed % 2 == 0);
            let mut whole = String::new();
            codec.decode(&bytes, &mut whole);

            let mut pieces = String::new();
            let mut start = 0;
            let mut i = 0;
            let mut counter = seed as usize;
            while i < bytes.len() {
                let len = codec.char_len(&bytes, i).max(1);
                i = (i + len).min(bytes.len());
                counter += 1;
                if counter.is_multiple_of(5) || i == bytes.len() {
                    codec.decode(&bytes[start..i], &mut pieces);
                    start = i;
                }
            }
            assert_eq!(pieces, whole, "{encoding:?} seed {seed}");
        }
    }
}

#[test]
fn japanese_text_round_trips_in_every_encoding() {
    let text = "吾輩は猫である。名前はまだ無い。\n① Ⅳ ㈱ № ≒ ∵ ￢ ｱｲｳ 髙 﨑 纊\n";
    for encoding in [
        Encoding::ShiftJis,
        Encoding::Jis,
        Encoding::EucJp,
        Encoding::Utf8,
        Encoding::Cesu8,
        Encoding::Utf7,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
    ] {
        let bytes = encode_ok(encoding, text);
        assert_eq!(decode(encoding, &bytes), text, "{encoding:?}");
    }
    // Shift_JIS is the code that Sakura users have most: pin the bytes of a few chars.
    assert_eq!(encode_ok(Encoding::ShiftJis, "髙﨑"), b"\xFB\xFC\xFA\xB1");
}

// ---- Line endings ----

#[test]
fn find_eol_in_every_encoding() {
    for encoding in Encoding::ALL {
        let codec = encoding.codec();
        let line = |text: &str| encode_ok(encoding, text);
        let text = line("ab\r\ncd\ref\ngh");
        // CR LF, CR, LF in this order
        let (first, kind) = codec.find_eol(&text, 0).unwrap();
        assert_eq!(kind, Eol::CrLf, "{encoding:?}");
        let unit = codec.eol_bytes(Eol::CrLf).unwrap();
        assert_eq!(&text[first..first + unit.len()], unit, "{encoding:?}");
        let (second, kind) = codec.find_eol(&text, first + unit.len()).unwrap();
        assert_eq!(kind, Eol::Cr, "{encoding:?}");
        let cr = codec.eol_bytes(Eol::Cr).unwrap();
        let (third, kind) = codec.find_eol(&text, second + cr.len()).unwrap();
        assert_eq!(kind, Eol::Lf, "{encoding:?}");
        let lf = codec.eol_bytes(Eol::Lf).unwrap();
        assert_eq!(
            codec.find_eol(&text, third + lf.len()),
            None,
            "{encoding:?}"
        );

        // CR as the last unit of the chunk is reported as CR; the caller joins it with the LF
        // at the start of the next chunk.
        let chunk = line("ab\r");
        let (at, kind) = codec.find_eol(&chunk, 0).unwrap();
        assert_eq!(
            (kind, at + cr.len()),
            (Eol::Cr, chunk.len()),
            "{encoding:?}"
        );
        // `from` after the line ending.
        assert_eq!(codec.find_eol(&chunk, chunk.len()), None);
    }
}

#[test]
fn eol_bytes_match_sakura() {
    // test-ccodebase.cpp, GetEolCases (CCodeBase::GetEol)
    for encoding in [
        Encoding::ShiftJis,
        Encoding::Jis,
        Encoding::EucJp,
        Encoding::Latin1,
    ] {
        let codec = encoding.codec();
        assert_eq!(codec.eol_bytes(Eol::CrLf), Some(&b"\r\n"[..]));
        assert_eq!(codec.eol_bytes(Eol::Lf), Some(&b"\n"[..]));
        assert_eq!(codec.eol_bytes(Eol::Cr), Some(&b"\r"[..]));
        assert_eq!(codec.eol_bytes(Eol::Nel), None, "{encoding:?}");
        assert_eq!(codec.eol_bytes(Eol::Ls), None, "{encoding:?}");
        assert_eq!(codec.eol_bytes(Eol::Ps), None, "{encoding:?}");
    }
    let utf7 = Encoding::Utf7.codec();
    assert_eq!(utf7.eol_bytes(Eol::Nel), Some(&b"+AIU-"[..]));
    assert_eq!(utf7.eol_bytes(Eol::Ls), Some(&b"+ICg-"[..]));
    assert_eq!(utf7.eol_bytes(Eol::Ps), Some(&b"+ICk-"[..]));
    let le = Encoding::Utf16Le.codec();
    assert_eq!(le.eol_bytes(Eol::CrLf), Some(&b"\r\0\n\0"[..]));
    assert_eq!(le.eol_bytes(Eol::Nel), Some(&b"\x85\0"[..]));
    assert_eq!(le.eol_bytes(Eol::Ls), Some(&b"\x28\x20"[..]));
    assert_eq!(le.eol_bytes(Eol::Ps), Some(&b"\x29\x20"[..]));
    let be = Encoding::Utf16Be.codec();
    assert_eq!(be.eol_bytes(Eol::CrLf), Some(&b"\0\r\0\n"[..]));
    assert_eq!(be.eol_bytes(Eol::Nel), Some(&b"\0\x85"[..]));
    assert_eq!(be.eol_bytes(Eol::Ls), Some(&b"\x20\x28"[..]));
    assert_eq!(be.eol_bytes(Eol::Ps), Some(&b"\x20\x29"[..]));
    for encoding in [Encoding::Utf8, Encoding::Cesu8] {
        let codec = encoding.codec();
        assert_eq!(codec.eol_bytes(Eol::Nel), Some(&b"\xC2\x85"[..]));
        assert_eq!(codec.eol_bytes(Eol::Ls), Some(&b"\xE2\x80\xA8"[..]));
        assert_eq!(codec.eol_bytes(Eol::Ps), Some(&b"\xE2\x80\xA9"[..]));
    }
    // Whatever the codec writes, it finds again (except UTF-7, whose NEL, LS and PS are inside
    // Base64 runs).
    for encoding in Encoding::ALL {
        for eol in [Eol::CrLf, Eol::Lf, Eol::Cr, Eol::Nel, Eol::Ls, Eol::Ps] {
            let codec = encoding.codec();
            let Some(bytes) = codec.eol_bytes(eol) else {
                continue;
            };
            if encoding == Encoding::Utf7 && matches!(eol, Eol::Nel | Eol::Ls | Eol::Ps) {
                continue;
            }
            assert_eq!(
                codec.find_eol(bytes, 0),
                Some((0, eol)),
                "{encoding:?} {eol:?}"
            );
            assert_eq!(
                decode(encoding, bytes),
                eol.as_str(),
                "{encoding:?} {eol:?}"
            );
        }
    }
}

#[test]
fn extended_line_endings_only_where_sakura_has_them() {
    // UTF-8 NEL, LS, PS (CFileLoad::GetNextLineCharCode with ext EOL on)
    let utf8 = Encoding::Utf8.codec();
    assert_eq!(utf8.find_eol("a\u{85}b".as_bytes(), 0), Some((1, Eol::Nel)));
    assert_eq!(
        utf8.find_eol("a\u{2028}b".as_bytes(), 0),
        Some((1, Eol::Ls))
    );
    assert_eq!(
        utf8.find_eol("a\u{2029}b".as_bytes(), 0),
        Some((1, Eol::Ps))
    );
    // U+00C2 followed by something else, and U+2027 are not line endings.
    assert_eq!(utf8.find_eol("\u{C2}\u{86}\u{2027}".as_bytes(), 0), None);
    // The same bytes in Shift_JIS or EUC-JP are ordinary bytes.
    for encoding in [Encoding::ShiftJis, Encoding::EucJp, Encoding::Latin1] {
        assert_eq!(encoding.codec().find_eol(b"\xC2\x85\xE2\x80\xA8", 0), None);
    }
    // UTF-16 works in 2-byte units: a 0x0A byte that is half of another unit is not a line feed.
    let le = Encoding::Utf16Le.codec();
    assert_eq!(le.find_eol(b"\x0A\x01\x0D\0", 0), Some((2, Eol::Cr)));
    assert_eq!(le.find_eol(b"a\0\x85\0", 0), Some((2, Eol::Nel)));
    // A Shift_JIS trail byte is never 0x0D or 0x0A, so these are line endings.
    assert_eq!(
        Encoding::ShiftJis.codec().find_eol(b"\x82\xA0\r\n", 0),
        Some((2, Eol::CrLf))
    );
    // `from` is respected.
    assert_eq!(
        Encoding::Utf8.codec().find_eol(b"\n\n", 1),
        Some((1, Eol::Lf))
    );
    assert_eq!(Encoding::Utf8.codec().find_eol(b"\n", 5), None);
}

// ---- Detection ----

fn detect_with_default(bytes: &[u8]) -> Detection {
    detect(bytes, &DetectOptions::default())
}

/// A paragraph of Japanese text with line breaks.
const NOVEL: &str = "吾輩は猫である。名前はまだ無い。\r\nどこで生れたかとんと見当がつかぬ。何でも薄暗いじめじめした所でニャーニャー泣いていた事だけは記憶している。\r\n吾輩はここで始めて人間というものを見た。\r\n";

#[test]
fn detect_bom() {
    let utf8 = detect_with_default(b"\xEF\xBB\xBFabc");
    assert_eq!(
        (utf8.encoding, utf8.bom, utf8.certainty),
        (Encoding::Utf8, true, Certainty::Bom)
    );
    let le = detect_with_default(b"\xFF\xFEa\0");
    assert_eq!((le.encoding, le.bom), (Encoding::Utf16Le, true));
    let be = detect_with_default(b"\xFE\xFF\0a");
    assert_eq!((be.encoding, be.bom), (Encoding::Utf16Be, true));
    // A UTF-7 BOM is not used (CESI.h, DetectUnicodeBom); it is only valid UTF-7 to the scoring.
    assert_ne!(detect_with_default(b"+/v8-abc").certainty, Certainty::Bom);
}

#[test]
fn detect_empty_and_ascii_give_the_default() {
    let options = DetectOptions {
        default_encoding: Encoding::ShiftJis,
        prefer_cesu8: false,
    };
    let empty = detect(b"", &options);
    assert_eq!(
        (empty.encoding, empty.certainty, empty.bom),
        (Encoding::ShiftJis, Certainty::Default, false)
    );
    let ascii = detect(b"hello, world\r\n", &options);
    assert_eq!(
        (ascii.encoding, ascii.certainty),
        (Encoding::ShiftJis, Certainty::Default)
    );
    // Sakura's default default is UTF-8.
    assert_eq!(detect_with_default(b"hello").encoding, Encoding::Utf8);
    assert_eq!(DetectOptions::default().default_encoding, Encoding::Utf8);
    assert!(!DetectOptions::default().prefer_cesu8);
}

#[test]
fn detect_japanese_encodings() {
    for encoding in [
        Encoding::ShiftJis,
        Encoding::Jis,
        Encoding::EucJp,
        Encoding::Utf8,
    ] {
        let bytes = encode_ok(encoding, NOVEL);
        let found = detect_with_default(&bytes);
        assert_eq!(found.encoding, encoding, "{encoding:?}");
        assert_eq!(found.certainty, Certainty::Scored, "{encoding:?}");
        assert!(!found.bom);
    }
}

#[test]
fn detect_utf16_without_bom_needs_line_breaks() {
    for (encoding, expected) in [
        (Encoding::Utf16Le, Encoding::Utf16Le),
        (Encoding::Utf16Be, Encoding::Utf16Be),
    ] {
        let bytes = encode_ok(encoding, NOVEL);
        assert_eq!(detect_with_default(&bytes).encoding, expected);
    }
    // One long line (more than 200 bytes per line break) is not taken for UTF-16.
    let long: String = "あ".repeat(400);
    let found = detect_with_default(&encode_ok(Encoding::Utf16Le, &format!("{long}\n")));
    assert_ne!(found.encoding, Encoding::Utf16Le);
    // A lone surrogate rejects UTF-16 of that byte order.
    let mut bytes = encode_ok(Encoding::Utf16Le, "a\nb\n");
    bytes.extend_from_slice(&[0x3D, 0xD8, b'x', 0]);
    assert_ne!(detect_with_default(&bytes).encoding, Encoding::Utf16Le);
}

#[test]
fn detect_utf7() {
    let found = detect_with_default(b"Hi Mom -+Jjo--!");
    assert_eq!(
        (found.encoding, found.certainty),
        (Encoding::Utf7, Certainty::Scored)
    );
    let japanese = detect_with_default(&encode_ok(Encoding::Utf7, "日本語のテキストです"));
    assert_eq!(japanese.encoding, Encoding::Utf7);
    // A sample cut inside a run is not an error, but its letters give no points
    // (CESI::GetEncodingInfo_utf7), so nothing is decided.
    assert_eq!(
        detect_with_default(b"abc +Jjo").certainty,
        Certainty::Default
    );
}

#[test]
fn detect_cesu8_and_the_tie_option() {
    // Only a pair of surrogates distinguishes CESU-8 from UTF-8.
    let bytes = b"abc \xED\xA0\xBD\xED\xB8\x80 def";
    assert_eq!(detect_with_default(bytes).encoding, Encoding::Cesu8);
    // Without such a pair they score the same: UTF-8 wins unless "CESU-8 を判別する" is on.
    let text = encode_ok(Encoding::Utf8, NOVEL);
    assert_eq!(detect_with_default(&text).encoding, Encoding::Utf8);
    let prefer = DetectOptions {
        default_encoding: Encoding::Utf8,
        prefer_cesu8: true,
    };
    assert_eq!(detect(&text, &prefer).encoding, Encoding::Cesu8);
}

#[test]
fn detect_ignores_a_few_invalid_bytes() {
    // Less than 0.05% invalid bytes are ignored (DetectMBCode).
    let mut bytes = encode_ok(Encoding::ShiftJis, &NOVEL.repeat(20));
    assert!(bytes.len() > 2000);
    bytes[1000] = 0xFD;
    assert_eq!(detect_with_default(&bytes).encoding, Encoding::ShiftJis);
    // A tenth of the bytes invalid is not Shift_JIS any more.
    let noisy: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| if i % 10 == 0 { 0xFD } else { b })
        .collect();
    assert_ne!(detect_with_default(&noisy).encoding, Encoding::ShiftJis);
}

#[test]
fn detect_euc_or_sjis_tie_uses_hiragana_ratio() {
    // GuessEucOrSjis: hiragana-heavy bytes read the same in both; the ratio of hiragana and
    // katakana among the double-byte chars decides for EUC-JP.
    let kana = encode_ok(Encoding::EucJp, "あいうえおかきくけこ\n");
    assert_eq!(detect_with_default(&kana).encoding, Encoding::EucJp);
}

#[test]
fn detect_declared_encodings() {
    let xml = b"<?xml version=\"1.0\" encoding=\"Shift_JIS\"?>\r\n<a/>\r\n";
    let found = detect_with_default(xml);
    assert_eq!(
        (found.encoding, found.certainty),
        (Encoding::ShiftJis, Certainty::Declared)
    );
    let single = b"<?xml version='1.0' encoding='EUC-JP'?>\n<a>x</a>\n";
    assert_eq!(detect_with_default(single).encoding, Encoding::EucJp);

    let html = b"<html><head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=Shift_JIS\"></head>";
    assert_eq!(detect_with_default(html).encoding, Encoding::ShiftJis);
    let html5 = b"<!DOCTYPE html><meta charset=\"euc-jp\"><title>x</title>";
    assert_eq!(detect_with_default(html5).encoding, Encoding::EucJp);

    let python = b"#!/usr/bin/python\n# -*- coding: euc-jp -*-\nprint('x')\n";
    assert_eq!(detect_with_default(python).encoding, Encoding::EucJp);
    // Only the first two lines.
    let late = b"print(1)\nprint(2)\n# coding: euc-jp\nprint('hello world')\n";
    assert_eq!(detect_with_default(late).certainty, Certainty::Default);

    // UTF-16 XML declarations
    let wide: Vec<u8> = "<?xml version=\"1.0\"?>\n<a/>\n"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(detect_with_default(&wide).encoding, Encoding::Utf16Le);
    let wide: Vec<u8> = "<?xml version=\"1.0\"?>\n<a/>\n"
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .collect();
    assert_eq!(detect_with_default(&wide).encoding, Encoding::Utf16Be);

    // A Windows code page is reported, not supported.
    let cyrillic = b"<?xml version=\"1.0\" encoding=\"windows-1251\"?>\n<a/>\n";
    let found = detect_with_default(cyrillic);
    assert_eq!(found.unsupported_code_page, Some(1251));
    assert_eq!(found.encoding, Encoding::Utf8);
    assert_eq!(found.certainty, Certainty::Declared);
    // latin1 names map to Latin1.
    let latin = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<a/>\n";
    assert_eq!(detect_with_default(latin).encoding, Encoding::Latin1);
    // The BOM wins over the declaration.
    let with_bom = b"\xEF\xBB\xBF<?xml version=\"1.0\" encoding=\"Shift_JIS\"?>\n<a/>\n";
    assert_eq!(detect_with_default(with_bom).encoding, Encoding::Utf8);
}

#[test]
fn detect_looks_at_the_first_part_only() {
    // 32768 bytes of ASCII and then Shift_JIS: the sample is all ASCII.
    let mut bytes = vec![b'a'; DETECT_SAMPLE_LEN];
    bytes.extend_from_slice(&encode_ok(Encoding::ShiftJis, NOVEL));
    assert_eq!(detect_with_default(&bytes).certainty, Certainty::Default);
}
