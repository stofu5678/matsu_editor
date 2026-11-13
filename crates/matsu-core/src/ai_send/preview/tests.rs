use super::*;

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

fn all(doc: &Document, limits: &PreviewLimits) -> TextPreview {
    let end = doc.end_pos().unwrap();
    preview_range(doc, Pos::new(0, 0), end, limits).unwrap()
}

fn numbered(n: usize) -> String {
    (1..=n).map(|i| format!("行{i}\n")).collect()
}

#[test]
fn a_short_text_is_all_head() {
    let d = doc("一行目\r\n二行目\r\n三行目");
    let p = all(&d, &PreviewLimits::default());
    assert_eq!(p.lines, 3);
    assert!(p.tail.is_empty());
    assert_eq!(p.omitted_lines, 0);
    let heads: Vec<_> = p.head.iter().map(|l| (l.number, l.text.as_str())).collect();
    assert_eq!(heads, [(1, "一行目"), (2, "二行目"), (3, "三行目")]);
    // 9 characters and two CR LF.
    assert_eq!(p.chars, 9 + 4);
    assert!(p.chars_exact);
    assert!(!p.large);
}

#[test]
fn a_long_text_shows_the_first_10_and_the_last_5_lines() {
    let d = doc(&numbered(100));
    let p = all(&d, &PreviewLimits::default());
    // The range ends at the start of the empty line 101: it ends with the break of line 100.
    assert_eq!(p.lines, 100);
    assert_eq!(p.head.len(), 10);
    assert_eq!(p.head[0].text, "行1");
    assert_eq!(p.head[9].text, "行10");
    assert_eq!(p.tail.len(), 5);
    assert_eq!(p.tail[4].number, 100);
    assert_eq!(p.tail[4].text, "行100");
    assert_eq!(p.omitted_lines, 100 - 15);
}

#[test]
fn a_range_that_ends_at_the_start_of_a_line_ends_with_the_line_break() {
    let d = doc("a\nb\nc\n");
    let p = preview_range(
        &d,
        Pos::new(0, 0),
        Pos::new(2, 0),
        &PreviewLimits::default(),
    )
    .unwrap();
    assert_eq!(p.lines, 2);
    assert_eq!(p.chars, 4);
    assert_eq!(p.head.len(), 2);
}

#[test]
fn a_range_inside_one_line_counts_only_the_selected_part() {
    let d = doc("あいうえお\nかきくけこ");
    // columns are bytes: 3 per character
    let p = preview_range(
        &d,
        Pos::new(0, 3),
        Pos::new(0, 9),
        &PreviewLimits::default(),
    )
    .unwrap();
    assert_eq!(p.lines, 1);
    assert_eq!(p.chars, 2);
    assert_eq!(p.head[0].text, "いう");
}

#[test]
fn the_ends_may_be_given_in_either_order() {
    let d = doc("abc\ndef");
    let a = preview_range(
        &d,
        Pos::new(0, 1),
        Pos::new(1, 2),
        &PreviewLimits::default(),
    )
    .unwrap();
    let b = preview_range(
        &d,
        Pos::new(1, 2),
        Pos::new(0, 1),
        &PreviewLimits::default(),
    )
    .unwrap();
    assert_eq!(a, b);
    assert_eq!(a.chars, 2 + 1 + 2);
}

#[test]
fn an_empty_range_has_no_lines() {
    let d = doc("abc");
    let p = preview_range(
        &d,
        Pos::new(0, 1),
        Pos::new(0, 1),
        &PreviewLimits::default(),
    )
    .unwrap();
    assert_eq!(p.lines, 0);
    assert!(p.head.is_empty() && p.tail.is_empty());
    assert_eq!(p.chars, 0);
}

#[test]
fn text_over_100_kb_is_large() {
    let line = "x".repeat(1000) + "\n";
    let d = doc(&line.repeat(103));
    let p = all(&d, &PreviewLimits::default());
    assert!(p.large);
    assert!(p.bytes > LARGE_TEXT_BYTES);
    let d = doc(&line.repeat(102));
    assert!(!all(&d, &PreviewLimits::default()).large, "102102 bytes");
}

#[test]
fn a_long_line_is_cut_for_display() {
    let d = doc(&"あ".repeat(500));
    let p = all(&d, &PreviewLimits::default());
    assert_eq!(p.head[0].text.chars().count(), 201);
    assert!(p.head[0].text.ends_with('…'));
    assert_eq!(p.chars, 500);
}

#[test]
fn the_scan_stops_at_its_budget() {
    let limits = PreviewLimits {
        scan_bytes: 1000,
        ..PreviewLimits::default()
    };
    let line = "0123456789".repeat(10) + "\n";
    let d = doc(&line.repeat(200));
    let p = all(&d, &limits);
    assert!(!p.chars_exact);
    assert!(!p.secrets_complete);
    assert!(p.chars >= 1000 && p.chars < 2000, "{}", p.chars);
    // The line count and the head and tail do not depend on the budget.
    assert_eq!(p.lines, 200);
    assert_eq!(p.tail.len(), 5);
    // The bytes still say how big the range is.
    assert_eq!(p.bytes, 200 * 101);
}

#[test]
fn one_huge_line_is_read_only_up_to_the_budget() {
    let limits = PreviewLimits {
        scan_bytes: 5000,
        ..PreviewLimits::default()
    };
    let d = doc(&"a".repeat(1_000_000));
    let p = all(&d, &limits);
    assert!(!p.chars_exact);
    assert_eq!(p.chars, 5000);
    assert_eq!(p.lines, 1);
    assert!(p.large, "the size comes from the document, not the scan");
}

#[test]
fn secrets_are_listed_with_line_numbers_and_masked() {
    let d = doc(
        "普通の行\nKEY=sk-proj-abc123DEF456ghi789JKL012mno345\n普通の行\n-----BEGIN PRIVATE KEY-----\n",
    );
    let p = all(&d, &PreviewLimits::default());
    assert_eq!(p.secret_lines_total, 2);
    assert!(p.secrets_complete);
    assert_eq!(p.secrets[0].number, 2);
    assert_eq!(p.secrets[0].kinds, [SecretKind::ApiKey]);
    assert!(p.secrets[0].text.starts_with("KEY=sk-p"));
    assert!(!p.secrets[0].text.contains("abc123"));
    assert_eq!(p.secrets[1].number, 4);
    // The head shows the same line, masked too.
    assert!(!p.head[1].text.contains("abc123"));
    assert!(!p.head[3].text.contains("PRIVATE"));
}

#[test]
fn secret_lines_are_capped_but_counted() {
    let limits = PreviewLimits {
        max_secret_lines: 2,
        ..PreviewLimits::default()
    };
    let line = "AKIAIOSFODNN7EXAMPLE\n";
    let d = doc(&line.repeat(5));
    let p = all(&d, &limits);
    assert_eq!(p.secrets.len(), 2);
    assert_eq!(p.secret_lines_total, 5);
}

#[test]
fn a_secret_far_in_a_long_line_is_shown_in_its_window() {
    let text = format!(
        "{}AKIAIOSFODNN7EXAMPLE{}",
        "あ".repeat(300),
        "い".repeat(300)
    );
    let d = doc(&text);
    let p = all(&d, &PreviewLimits::default());
    assert_eq!(p.secrets.len(), 1);
    let shown = &p.secrets[0].text;
    assert!(shown.starts_with('…') && shown.ends_with('…'));
    assert!(shown.contains("AKIA"));
    assert!(!shown.contains("IOSFODNN7"));
}

#[test]
fn ordinary_japanese_and_code_have_no_secret_lines() {
    let d = doc(
        "吾輩は猫である。名前はまだ無い。\nfn main() {\n    let task_runner = risk_level;\n}\n",
    );
    let p = all(&d, &PreviewLimits::default());
    assert_eq!(p.secret_lines_total, 0);
    assert!(p.secrets.is_empty());
}

#[test]
fn a_secret_whose_end_lies_past_the_display_cut_is_still_masked() {
    let text = format!("https://user:{}@host/", "p".repeat(400));
    let d = doc(&text);
    let p = all(&d, &PreviewLimits::default());
    assert_eq!(p.secret_lines_total, 1);
    assert!(!p.head[0].text.contains("ppppp"));
}

#[test]
fn the_lines_of_a_private_key_are_masked_whole() {
    let key = "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC7VJTUt9Us8cKj";
    let d = doc(&format!(
        "前\n-----BEGIN PRIVATE KEY-----\n{key}\nshort+/=\n-----END PRIVATE KEY-----\n後\n"
    ));
    let p = all(&d, &PreviewLimits::default());
    let shown: String = p.head.iter().map(|l| l.text.clone() + "\n").collect();
    assert!(!shown.contains("MIIEv"), "{shown}");
    assert!(!shown.contains("short"), "{shown}");
    assert!(shown.contains("前") && shown.contains("後"));
    assert_eq!(p.secret_lines_total, 1, "only the marker line is listed");
}

#[test]
fn a_key_payload_in_the_tail_is_masked_even_when_the_block_start_is_far() {
    let key = "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC7VJTUt9Us8cKj";
    let limits = PreviewLimits {
        scan_bytes: 100,
        ..PreviewLimits::default()
    };
    let d = doc(&format!("{}{key}\n{key}\n", "行\n".repeat(200)));
    let p = all(&d, &limits);
    assert!(
        p.tail.iter().all(|l| !l.text.contains("MIIEv")),
        "{:?}",
        p.tail
    );
}
