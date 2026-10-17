use super::*;
use crate::buffer::Document;

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

fn fed(doc: &Document, range: Option<(Pos, Pos)>, encoding: Encoding, bom: bool) -> Vec<u8> {
    let mut all = Vec::new();
    feed_text(doc, range, encoding, bom, |piece| {
        all.extend(piece);
        true
    })
    .unwrap();
    all
}

#[test]
fn the_default_is_only_getting_the_output() {
    let f = ExecFlags::DEFAULT;
    assert!(f.get_stdout());
    assert!(!f.send_stdin());
    assert!(!f.uses_cur_dir());
    assert_eq!(f.output_encoding(), Encoding::ShiftJis);
    assert_eq!(f.input_encoding(), Encoding::ShiftJis);
}

#[test]
fn the_encodings_are_chosen_by_their_bits_and_unicode_wins() {
    assert_eq!(ExecFlags(0x08).output_encoding(), Encoding::Utf16Le);
    assert_eq!(ExecFlags(0x80).output_encoding(), Encoding::Utf8);
    assert_eq!(ExecFlags(0x88).output_encoding(), Encoding::Utf16Le);
    assert_eq!(ExecFlags(0x10).input_encoding(), Encoding::Utf16Le);
    assert_eq!(ExecFlags(0x100).input_encoding(), Encoding::Utf8);
    assert_eq!(ExecFlags(0x110).input_encoding(), Encoding::Utf16Le);
}

#[test]
fn the_window_is_only_for_a_document_that_can_be_edited() {
    let f = ExecFlags(0x03);
    assert!(f.to_window(true));
    assert!(!f.to_window(false));
    assert_eq!(f.for_dialog(false), ExecFlags(0x01));
    assert_eq!(f.for_dialog(true), f);
}

#[test]
fn the_extra_information_follows_the_destination_and_the_two_bits() {
    // by default only in the output window
    assert!(ExecFlags(0x01).extra_info(false));
    assert!(!ExecFlags(0x03).extra_info(true));
    // 0x20 asks for it in the window, 0x40 refuses it in the output window
    assert!(ExecFlags(0x23).extra_info(true));
    assert!(!ExecFlags(0x41).extra_info(false));
    // both: 0x40 is the later one (2010/04/12 Moca)
    assert!(!ExecFlags(0x61).extra_info(false));
}

#[test]
fn the_combo_boxes_show_the_choice_of_the_flags() {
    assert_eq!(output_choice(ExecFlags(0x01)), 0);
    assert_eq!(output_choice(ExecFlags(0x09)), 1);
    assert_eq!(output_choice(ExecFlags(0x81)), 2);
    assert_eq!(input_choice(ExecFlags(0x04)), 0);
    assert_eq!(input_choice(ExecFlags(0x14)), 1);
    assert_eq!(input_choice(ExecFlags(0x104)), 2);
    for (i, encoding) in DIALOG_ENCODINGS.iter().enumerate() {
        assert_eq!(ExecFlags(OUTPUT_BITS[i]).output_encoding(), *encoding);
        assert_eq!(ExecFlags(INPUT_BITS[i]).input_encoding(), *encoding);
    }
}

#[test]
fn the_words_around_the_output_are_sakuras() {
    assert_eq!(stopped_text("\r\n"), "\r\n中断しました。\r\n");
    assert_eq!(exit_code_text(3, "\r\n"), "\r\n終了コード: 3\r\n");
    assert_eq!(
        header_text("2026/10/17", "12:00:00", "dir", "\n"),
        "\n#============================================================\n\
         #DateTime : 2026/10/17 12:00:00\n#CmdLine  : dir\n\
         #============================================================\n"
    );
}

#[test]
fn a_long_command_is_cut_at_1024_units() {
    let long = "a".repeat(2000);
    assert_eq!(limit_command(&long).len(), 1024);
    assert_eq!(limit_command("short"), "short");
    // a character of two units is not cut in half
    let text = format!("{}😀", "a".repeat(1023));
    assert_eq!(limit_command(&text).len(), 1023);
}

#[test]
fn shift_jis_waits_for_the_second_byte() {
    let mut d = OutputDecoder::new(Encoding::ShiftJis);
    // "あい" is 82 A0 82 A2; cut after the first byte of the second character
    assert_eq!(d.push(&[0x82, 0xA0, 0x82]), "あ");
    assert_eq!(d.push(&[0xA2]), "い");
    assert_eq!(d.finish(), "");
}

#[test]
fn shift_jis_half_width_katakana_does_not_wait() {
    let mut d = OutputDecoder::new(Encoding::ShiftJis);
    assert_eq!(d.push(&[0xB1, 0xB2]), "ｱｲ");
}

#[test]
fn a_cr_at_the_end_waits_for_its_lf() {
    for encoding in [Encoding::ShiftJis, Encoding::Utf8] {
        let mut d = OutputDecoder::new(encoding);
        assert_eq!(d.push(b"a\r"), "a");
        assert_eq!(d.push(b"\nb"), "\r\nb");
        // a CR that ends the output is written at the end
        assert_eq!(d.push(b"\r"), "");
        assert_eq!(d.finish(), "\r");
    }
}

#[test]
fn utf8_waits_for_the_rest_of_a_character() {
    let bytes = "aあ😀".as_bytes();
    let mut d = OutputDecoder::new(Encoding::Utf8);
    let mut text = String::new();
    // one byte at a time
    for b in bytes {
        text.push_str(&d.push(&[*b]));
    }
    text.push_str(&d.finish());
    assert_eq!(text, "aあ😀");
    // cut inside the emoji
    let mut d = OutputDecoder::new(Encoding::Utf8);
    assert_eq!(d.push(&bytes[..6]), "aあ");
    assert_eq!(d.push(&bytes[6..]), "😀");
}

#[test]
fn utf8_that_is_bad_is_not_held_forever() {
    let mut d = OutputDecoder::new(Encoding::Utf8);
    // 0xFF can never start a character: it is decoded at once
    let text = d.push(&[b'a', 0xFF, b'b']);
    assert!(text.starts_with('a') && text.ends_with('b'));
    assert_eq!(text.chars().count(), 3);
}

#[test]
fn unicode_waits_for_an_odd_byte_and_for_a_cr() {
    let units: Vec<u8> = "a\r\nあ😀"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut d = OutputDecoder::new(Encoding::Utf16Le);
    let mut text = String::new();
    // every cut, including the middle of a unit, of a pair and between CR and LF
    for piece in units.chunks(3) {
        text.push_str(&d.push(piece));
    }
    text.push_str(&d.finish());
    assert_eq!(text, "a\r\nあ😀");
    let mut d = OutputDecoder::new(Encoding::Utf16Le);
    assert_eq!(d.push(&[b'a', 0, b'\r', 0]), "a");
    assert_eq!(d.push(&[b'\n', 0]), "\r\n");
}

#[test]
fn a_bom_at_the_start_of_the_output_is_not_text() {
    // UTF-8, with the BOM cut in the middle by the reads
    let mut d = OutputDecoder::new(Encoding::Utf8);
    assert_eq!(d.push(b"\xEF\xBB"), "");
    assert_eq!(d.push(b"\xBFab"), "ab");
    // only the first one: a later U+FEFF is a character
    assert_eq!(d.push("\u{FEFF}c".as_bytes()), "\u{FEFF}c");
    // UTF-16
    let mut d = OutputDecoder::new(Encoding::Utf16Le);
    assert_eq!(d.push(&[0xFF, 0xFE, b'a', 0]), "a");
    // an output that is only the BOM
    let mut d = OutputDecoder::new(Encoding::Utf8);
    assert_eq!(d.push(b"\xEF\xBB\xBF"), "");
    assert_eq!(d.finish(), "");
    // an encoding without a BOM keeps it as it decodes (Shift_JIS has no such character)
    let mut d = OutputDecoder::new(Encoding::ShiftJis);
    assert_eq!(d.push(b"ab"), "ab");
}

#[test]
fn a_last_odd_byte_of_unicode_gets_a_zero_high_byte() {
    let mut d = OutputDecoder::new(Encoding::Utf16Le);
    assert_eq!(d.push(&[b'a', 0, b'b']), "a");
    assert_eq!(d.finish(), "b");
}

#[test]
fn the_whole_document_is_fed_with_its_line_endings() {
    let d = doc("ab\r\ncd\nef");
    assert_eq!(fed(&d, None, Encoding::Utf8, false), b"ab\r\ncd\nef");
}

#[test]
fn a_selection_is_fed_from_its_start_to_its_end() {
    let d = doc("abc\ndef\nghi\n");
    let range = Some((Pos::new(0, 1), Pos::new(2, 2)));
    assert_eq!(fed(&d, range, Encoding::Utf8, false), b"bc\ndef\ngh");
    // in one line, and the ends given the other way round
    let range = Some((Pos::new(1, 2), Pos::new(1, 1)));
    assert_eq!(fed(&d, range, Encoding::Utf8, false), b"e");
}

#[test]
fn the_input_is_encoded_with_the_bom_of_the_encoding() {
    let d = doc("あ\n");
    assert_eq!(
        fed(&d, None, Encoding::Utf8, true),
        b"\xEF\xBB\xBF\xE3\x81\x82\n"
    );
    assert_eq!(fed(&d, None, Encoding::ShiftJis, false), b"\x82\xA0\n");
    assert_eq!(
        fed(&d, None, Encoding::Utf16Le, true),
        [0xFF, 0xFE, 0x42, 0x30, 0x0A, 0x00]
    );
    // Shift_JIS has no BOM, whatever the document has
    assert_eq!(fed(&d, None, Encoding::ShiftJis, true), b"\x82\xA0\n");
}

#[test]
fn a_big_document_is_fed_in_pieces() {
    let line = format!("{}\n", "x".repeat(99));
    let text = line.repeat(5000);
    let d = doc(&text);
    let mut pieces = 0;
    let mut total = 0;
    feed_text(&d, None, Encoding::Utf8, false, |piece| {
        assert!(piece.len() <= FEED_CHUNK_BYTES + 100);
        pieces += 1;
        total += piece.len();
        true
    })
    .unwrap();
    assert!(pieces > 5, "{pieces} pieces");
    assert_eq!(total, text.len());
}

#[test]
fn feeding_stops_when_the_command_stops_reading() {
    let line = format!("{}\n", "x".repeat(99));
    let d = doc(&line.repeat(5000));
    let mut pieces = 0;
    feed_text(&d, None, Encoding::Utf8, false, |_| {
        pieces += 1;
        false
    })
    .unwrap();
    assert_eq!(pieces, 1);
}

#[test]
fn an_empty_selection_or_document_feeds_nothing() {
    assert!(fed(&doc(""), None, Encoding::Utf8, false).is_empty());
    let d = doc("abc");
    let at = Pos::new(0, 1);
    assert!(fed(&d, Some((at, at)), Encoding::Utf8, false).is_empty());
}
