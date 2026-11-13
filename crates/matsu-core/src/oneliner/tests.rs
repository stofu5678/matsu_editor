use super::*;

fn collector(cap: usize, overflow: Overflow) -> Collector {
    Collector::new(Box::new(Utf8Decoder::new()), cap, overflow)
}

fn collect(chunks: &[&[u8]]) -> String {
    let mut c = collector(usize::MAX, Overflow::Fail);
    for chunk in chunks {
        c.push(chunk).unwrap();
    }
    c.finish().unwrap().text()
}

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

fn fed(doc: &Document, range: Option<(Pos, Pos)>) -> String {
    let mut all = String::new();
    feed_lf(doc, range, |chunk| {
        all.push_str(chunk);
        true
    })
    .unwrap();
    all
}

#[test]
fn the_input_has_lf_breaks_only() {
    assert_eq!(fed(&doc("a\r\nb\rc\nd"), None), "a\nb\nc\nd");
    assert_eq!(fed(&doc("a\r\n"), None), "a\n");
    assert_eq!(fed(&doc(""), None), "");
}

#[test]
fn a_selection_is_the_input_and_its_end_is_not_a_break() {
    let d = doc("abc\r\ndef\r\nghi");
    let range = Some((Pos::new(0, 1), Pos::new(2, 1)));
    assert_eq!(fed(&d, range), "bc\ndef\ng");
    // up to the start of the next line: that break is the last one of the selection
    let range = Some((Pos::new(0, 0), Pos::new(1, 0)));
    assert_eq!(fed(&d, range), "abc\n");
}

#[test]
fn a_big_input_comes_in_chunks_and_stops_when_asked() {
    let d = doc(&format!("{}\r\n", "x".repeat(99)).repeat(5000));
    let mut chunks = 0;
    let mut total = 0;
    feed_lf(&d, None, |chunk| {
        chunks += 1;
        total += chunk.len();
        true
    })
    .unwrap();
    assert!(chunks > 5);
    assert_eq!(total, 5000 * 100);
    let mut chunks = 0;
    feed_lf(&d, None, |_| {
        chunks += 1;
        false
    })
    .unwrap();
    assert_eq!(chunks, 1);
}

#[test]
fn line_breaks_become_lf() {
    assert_eq!(collect(&[b"a\r\nb\rc\nd\r\n"]), "a\nb\nc\nd\n");
    assert_eq!(collect(&[b"a\r\r\nb"]), "a\n\nb");
    assert_eq!(collect(&[b"a\r\n\r\n"]), "a\n\n");
}

#[test]
fn a_crlf_cut_between_two_pieces_is_one_break() {
    assert_eq!(collect(&[b"a\r", b"\nb"]), "a\nb");
    assert_eq!(collect(&[b"a\r", b"b"]), "a\nb");
    // a CR at the very end is a break
    assert_eq!(collect(&[b"a\r"]), "a\n");
}

#[test]
fn a_character_cut_between_two_pieces_is_put_together() {
    let text = "日本語😀".as_bytes();
    for cut in 1..text.len() {
        assert_eq!(
            collect(&[&text[..cut], &text[cut..]]),
            "日本語😀",
            "cut {cut}"
        );
    }
    let one_by_one: Vec<&[u8]> = text.chunks(1).collect();
    assert_eq!(collect(&one_by_one), "日本語😀");
}

#[test]
fn bytes_that_are_not_utf8_become_the_replacement_character() {
    assert_eq!(collect(&[b"a\xFFb"]), "a\u{FFFD}b");
    assert_eq!(collect(&[b"a\xE3\x81"]), "a\u{FFFD}");
    assert_eq!(collect(&[b"\x82\xA0"]), "\u{FFFD}\u{FFFD}");
    // a lead byte that the next piece does not finish
    assert_eq!(collect(&[b"\xE3", b"x"]), "\u{FFFD}x");
}

#[test]
fn a_bom_at_the_start_is_not_text() {
    assert_eq!(collect(&[b"\xEF\xBB\xBFabc"]), "abc");
    assert_eq!(collect(&[b"\xEF", b"\xBB\xBFabc"]), "abc");
    assert_eq!(collect(&[b"abc\xEF\xBB\xBF"]), "abc\u{FEFF}");
}

#[test]
fn the_cap_fails_the_output() {
    let mut c = collector(10, Overflow::Fail);
    assert!(c.push(b"0123456789").is_ok());
    assert_eq!(c.push(b"x"), Err(TooLarge));
    let mut c = collector(10, Overflow::Fail);
    c.push(b"0123456789").unwrap();
    assert_eq!(c.finish().unwrap().text(), "0123456789");
}

#[test]
fn the_cap_counts_text_after_the_line_breaks_are_made_lf() {
    let mut c = collector(4, Overflow::Fail);
    c.push(b"a\r\nb\r\n").unwrap();
    assert_eq!(c.finish().unwrap().text(), "a\nb\n");
}

#[test]
fn a_break_that_waits_at_the_end_counts_too() {
    let mut c = collector(1, Overflow::Fail);
    c.push(b"a\r").unwrap();
    assert!(c.finish().is_err());
}

#[test]
fn the_error_keeps_its_first_part_at_a_character() {
    let mut c = collector(7, Overflow::Truncate);
    c.push("日本語".as_bytes()).unwrap();
    c.push(b"more").unwrap();
    let kept = c.finish().unwrap();
    // 7 bytes hold two characters of 3 bytes
    assert_eq!(kept.text(), "日本");
    assert!(kept.truncated);
    let mut c = collector(100, Overflow::Truncate);
    c.push(b"ok").unwrap();
    let kept = c.finish().unwrap();
    assert_eq!(kept.text(), "ok");
    assert!(!kept.truncated);
}

fn pieces(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|p| (*p).to_owned()).collect()
}

#[test]
fn replace_drops_one_break_the_input_did_not_have() {
    let mut p = pieces(&["a\nb\n"]);
    for_replace(&mut p, false);
    assert_eq!(p.concat(), "a\nb");
    // exactly one
    let mut p = pieces(&["a\n\n"]);
    for_replace(&mut p, false);
    assert_eq!(p.concat(), "a\n");
    // the input had its own: the output is as it is
    let mut p = pieces(&["a\nb\n"]);
    for_replace(&mut p, true);
    assert_eq!(p.concat(), "a\nb\n");
    // no break at the end: nothing to drop
    let mut p = pieces(&["a\nb"]);
    for_replace(&mut p, false);
    assert_eq!(p.concat(), "a\nb");
}

#[test]
fn replace_finds_the_break_across_pieces() {
    let mut p = pieces(&["a", "\n", "", "\n"]);
    for_replace(&mut p, false);
    assert_eq!(p.concat(), "a\n");
    let mut p = pieces(&["a", "\n", ""]);
    for_replace(&mut p, false);
    assert_eq!(p.concat(), "a");
    let mut p = pieces(&["\n"]);
    for_replace(&mut p, false);
    assert!(p.concat().is_empty());
    let mut p: Vec<String> = Vec::new();
    for_replace(&mut p, false);
    assert!(p.is_empty());
}

#[test]
fn next_line_ends_with_a_break() {
    let mut p = pieces(&["a\nb"]);
    for_next_line(&mut p);
    assert_eq!(p.concat(), "a\nb\n");
    let mut p = pieces(&["a\n"]);
    for_next_line(&mut p);
    assert_eq!(p.concat(), "a\n");
    let mut p = pieces(&["a", ""]);
    for_next_line(&mut p);
    assert_eq!(p.concat(), "a\n");
    // no output is no line
    let mut p: Vec<String> = Vec::new();
    for_next_line(&mut p);
    assert!(p.is_empty());
}
