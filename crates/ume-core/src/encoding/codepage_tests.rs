//! Tests of the Windows code pages. The expected bytes of the sample texts were made with
//! Python's codecs (independent of `encoding_rs`).

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

/// (code page, text, bytes as Python's codec makes them)
const SAMPLES: &[(u16, &str, &str)] = &[
    (936, "简体中文测试", "bcf2cce5d6d0cec4b2e2cad4"),
    (54936, "简体中文𠀀", "bcf2cce5d6d0cec495328236"),
    (950, "繁體中文測試", "c163c5e9a4a4a4e5b4fab8d5"),
    (949, "한국어 시험", "c7d1b1b9beee20bdc3c7e8"),
    (51949, "한국어 시험", "c7d1b1b9beee20bdc3c7e8"),
    (1251, "Привет, мир", "cff0e8e2e5f22c20ece8f0"),
    (
        1250,
        "Zażółć gęślą jaźń",
        "5a61bff3b3e62067ea9c6cb9206a619ff1",
    ),
    (1253, "Γειά σου Κόσμε", "c3e5e9dc20f3eff520cafcf3ece5"),
    (1255, "שלום עולם", "f9ece5ed20f2e5eced"),
    (1256, "مرحبا", "e3d1cdc8c7"),
    (1254, "Şişli ğüzel", "de69fe6c6920f0fc7a656c"),
    (1257, "Ąžuolas", "c0fe756f6c6173"),
    (1258, "Viêt ngôn", "5669ea74206e67f46e"),
    (874, "สวัสดี", "cac7d1cab4d5"),
    (866, "Привет", "8fe0a8a2a5e2"),
    (20866, "Привет", "f0d2c9d7c5d4"),
    (21866, "Привіт ґ", "f0d2c9d7a6d420ad"),
    (28592, "Zażółć", "5a61bff3b3e6"),
    (28595, "Привет", "bfe0d8d2d5e2"),
    (28597, "Γειά", "c3e5e9dc"),
    (28605, "€ œ", "a420bd"),
    (10000, "café ∑", "6361668e20b7"),
];

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn text_of_each_script_round_trips() {
    for &(number, text, bytes) in SAMPLES {
        let encoding = cp(number);
        let bytes = hex(bytes);
        assert_eq!(decode(encoding, &bytes), text, "CP{number} decode");
        assert_eq!(encode_ok(encoding, text), bytes, "CP{number} encode");
    }
}

#[test]
fn single_byte_code_pages_keep_every_byte() {
    // Every byte becomes a character or a stand-in, and encoding gives the same byte back.
    for encoding in Encoding::choices() {
        let Encoding::CodePage(code_page) = encoding else {
            continue;
        };
        if [936, 949, 950, 51949, 54936].contains(&code_page.number()) {
            continue;
        }
        let bytes: Vec<u8> = (0..=255).collect();
        let text = decode(encoding, &bytes);
        assert_eq!(text.chars().count(), 256, "{encoding:?}");
        assert_eq!(encode_ok(encoding, &text), bytes, "{encoding:?}");
    }
}

#[test]
fn undefined_bytes_become_stand_ins() {
    // windows-874: 0xDB..=0xDE have no character.
    assert_eq!(decode(cp(874), b"a\xDBb"), format!("a{}b", s(0xDB)));
    // ISO-8859-3: 0xA5 is not assigned.
    assert_eq!(decode(cp(28593), b"\xA5"), s(0xA5));
}

#[test]
fn invalid_double_byte_sequences_become_stand_ins() {
    // GBK: a lead byte alone at the end, a lead byte before a byte that is not a trail byte, and 0xFF.
    assert_eq!(decode(cp(936), b"\xB0"), s(0xB0));
    assert_eq!(decode(cp(936), b"\xB0 "), format!("{} ", s(0xB0)));
    assert_eq!(decode(cp(936), b"\xFF"), s(0xFF));
    // The first byte is invalid and the second one starts a character: 0xFF 0xB0 0xA1 is
    // stand-in + 0xB0A1 (U+554A).
    assert_eq!(
        decode(cp(936), b"\xFF\xB0\xA1"),
        format!("{}\u{554A}", s(0xFF))
    );
    // Big5 and EUC-KR: 0x80 and 0xFF are not characters.
    assert_eq!(
        decode(cp(950), b"\x80\xFFa"),
        format!("{}{}a", s(0x80), s(0xFF))
    );
    assert_eq!(decode(cp(949), b"\xC7"), s(0xC7));
    // GB18030: a truncated four-byte sequence.
    assert_eq!(
        decode(cp(54936), b"\x95\x32\x82"),
        format!("{}2{}", s(0x95), s(0x82))
    );
    // Stand-ins are written as their byte.
    let text = decode(cp(936), b"\xB0 \xFF\xBC\xF2");
    assert_eq!(encode_ok(cp(936), &text), b"\xB0 \xFF\xBC\xF2");
}

#[test]
fn char_len_follows_the_lead_and_trail_bytes() {
    let gbk = cp(936).codec();
    let bytes = hex("bcf2cce541ff");
    assert_eq!(gbk.char_len(&bytes, 0), 2);
    assert_eq!(gbk.char_len(&bytes, 2), 2);
    assert_eq!(gbk.char_len(&bytes, 4), 1);
    assert_eq!(gbk.char_len(&bytes, 5), 1);
    let gb18030 = cp(54936).codec();
    let bytes = hex("bcf295328236");
    assert_eq!(gb18030.char_len(&bytes, 0), 2);
    assert_eq!(gb18030.char_len(&bytes, 2), 4);
    // A lead byte at the end is one invalid byte.
    assert_eq!(gbk.char_len(b"\xB0", 0), 1);
    assert_eq!(cp(1251).codec().char_len(b"\xCF", 0), 1);
}

/// A deterministic byte generator, so that the test needs no random crate.
fn pseudo_random_bytes(seed: u32, len: usize) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            // Favor the bytes that matter: lead, trail, digits, ASCII and LF.
            match (state >> 24) % 8 {
                0 => b'\n',
                1 => b'3',
                2 => (state >> 8) as u8 | 0x80,
                _ => (state >> 16) as u8,
            }
        })
        .collect()
}

#[test]
fn decoding_pieces_gives_the_same_text_as_decoding_the_whole() {
    for (n, encoding) in Encoding::choices().into_iter().enumerate() {
        if encoding.sakura_code() < 9 {
            continue;
        }
        let codec = encoding.codec();
        let bytes = pseudo_random_bytes(n as u32 + 1, 2000);
        let whole = decode(encoding, &bytes);
        let mut pieces = String::new();
        let mut i = 0;
        while i < bytes.len() {
            let len = codec.char_len(&bytes, i);
            assert!(len >= 1 && i + len <= bytes.len(), "{encoding:?} at {i}");
            codec.decode(&bytes[i..i + len], &mut pieces);
            i += len;
        }
        assert_eq!(pieces, whole, "{encoding:?}");
        // Decoding from a boundary in the middle gives the tail of the whole.
        let mut at = 0;
        while at < bytes.len() / 2 {
            at += codec.char_len(&bytes, at);
        }
        assert!(
            whole.ends_with(&decode(encoding, &bytes[at..])),
            "{encoding:?}"
        );
    }
}

#[test]
fn line_endings_are_found_next_to_double_byte_characters() {
    // 0x0A is never a trail byte in these code pages (trail bytes start at 0x30).
    for &(number, text, bytes) in SAMPLES {
        let mut data = hex(bytes);
        let n = data.len();
        data.extend_from_slice(b"\r\n");
        data.extend_from_slice(&hex(bytes));
        let codec = cp(number).codec();
        assert_eq!(codec.find_eol(&data, 0), Some((n, Eol::CrLf)), "{text}");
        assert_eq!(codec.eol_bytes(Eol::CrLf), Some(&b"\r\n"[..]));
        assert_eq!(codec.eol_bytes(Eol::Nel), None);
    }
}

#[test]
fn unmappable_characters_become_question_marks() {
    let mut out = Vec::new();
    let error = cp(874)
        .codec()
        .encode("a\u{44F}b\u{1F600}", &mut out)
        .expect_err("not in windows-874");
    assert_eq!(out, b"a?b?");
    assert_eq!(error.offsets, vec![1, 4]);
    // A stand-in between two unmappable characters keeps the offsets right.
    let text = format!("\u{44F}{}\u{44F}", s(0xE0));
    let mut out = Vec::new();
    let error = cp(874).codec().encode(&text, &mut out).unwrap_err();
    assert_eq!(out, b"?\xE0?");
    assert_eq!(error.offsets, vec![0, 6]);
}

#[test]
fn code_pages_are_stored_by_number() {
    for encoding in Encoding::choices() {
        let code = encoding.sakura_code();
        assert_eq!(Encoding::from_sakura_code(code), Some(encoding), "{code}");
    }
    assert_eq!(cp(936).sakura_code(), 936);
    assert_eq!(Encoding::Utf8.sakura_code(), 4);
    // Code pages that are one of the fixed encodings here.
    assert_eq!(Encoding::from_sakura_code(932), Some(Encoding::ShiftJis));
    assert_eq!(Encoding::from_sakura_code(1252), Some(Encoding::Latin1));
    assert_eq!(Encoding::from_sakura_code(65001), Some(Encoding::Utf8));
    assert_eq!(Encoding::from_sakura_code(1200), Some(Encoding::Utf16Le));
    assert_eq!(Encoding::from_sakura_code(1201), Some(Encoding::Utf16Be));
    // Not supported: CODE_CODEMAX, CP_ACP/CP_OEM, auto, UTF-32, code pages without a table.
    for code in [9, 10, 90, 91, 99, 437, 850, 12000, 12001, 50220, 70000, -1] {
        assert_eq!(Encoding::from_sakura_code(code), None, "{code}");
    }
}

#[test]
fn choices_have_sakuras_nine_and_then_the_code_pages() {
    let choices = Encoding::choices();
    assert_eq!(choices[..9], Encoding::ALL);
    let numbers: Vec<i32> = choices[9..].iter().map(|e| e.sakura_code()).collect();
    assert!(numbers.len() > 20);
    assert!(numbers.windows(2).all(|w| w[0] < w[1]), "{numbers:?}");
    for number in [936, 950, 949, 54936, 1251, 20866] {
        assert!(numbers.contains(&number), "{number}");
    }
    assert!(!numbers.contains(&1252));
}

#[test]
fn code_pages_are_named_as_sakura_names_them() {
    // CCodePage::GetNameNormal / GetNameShort / GetNameBracket
    let gbk = cp(936);
    assert_eq!(gbk.combo_name(), "CP936");
    assert_eq!(gbk.short_name(), "cp936");
    assert_eq!(gbk.bracket_name(), "  [CP936]");
    assert_eq!(gbk.list_name(), "CP936 - Simplified Chinese (GBK)");
    assert_eq!(Encoding::Utf8.list_name(), "UTF-8");
    assert_eq!(Encoding::Utf16Be.list_name(), "UTF-16BE");
    // No BOM, can be the default.
    assert!(!gbk.can_have_bom());
    assert!(!gbk.bom_default_on());
    assert!(gbk.can_be_default());
    assert!(gbk.bom().is_empty());
}

#[test]
fn unsupported_code_page_numbers_are_not_made() {
    assert!(CodePage::new(437).is_none());
    assert!(CodePage::new(1252).is_none());
    assert_eq!(CodePage::new(936).unwrap().number(), 936);
}

#[test]
fn declared_charsets_open_in_the_code_page() {
    let detect = |bytes: &[u8]| detect(bytes, &DetectOptions::default());
    let found = detect(b"<?xml version=\"1.0\" encoding=\"windows-1251\"?>\n<a/>\n");
    assert_eq!(found.encoding, cp(1251));
    assert_eq!(found.certainty, Certainty::Declared);
    assert_eq!(found.unsupported_code_page, None);
    assert!(!found.bom);
    let found = detect(b"<html><head><meta charset=\"gb2312\"></head><body></body></html>");
    assert_eq!(found.encoding, cp(936));
    let found = detect(b"# coding: koi8-r\nprint('hello')\n");
    assert_eq!(found.encoding, cp(20866));
    let found = detect(b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=big5\">");
    assert_eq!(found.encoding, cp(950));
    // A code page without a table is still reported.
    let found = detect(b"<?xml version=\"1.0\" encoding=\"ibm437\"?>\n<a/>\n");
    assert_eq!(found.encoding, Encoding::Utf8);
    assert_eq!(found.unsupported_code_page, Some(437));
}

#[test]
fn guessing_does_not_pick_a_code_page() {
    // Automatic guessing stays Sakura's: GBK text without a declaration is not recognized.
    let bytes = hex("bcf2cce5d6d0cec4b2e2cad4");
    let found = detect(&bytes, &DetectOptions::default());
    assert!(!matches!(found.encoding, Encoding::CodePage(_)));
}
