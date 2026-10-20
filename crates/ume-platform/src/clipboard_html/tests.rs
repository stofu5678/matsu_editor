use super::*;

/// The number after `name:` in the header.
fn field(data: &str, name: &str) -> usize {
    let line = data
        .lines()
        .find_map(|l| l.strip_prefix(name))
        .unwrap_or_else(|| panic!("{name} is missing"));
    line.strip_prefix(':').unwrap().parse().unwrap()
}

#[test]
fn the_header_points_at_the_page_and_the_fragment() {
    let fragment = "<pre>abc</pre>";
    let bytes = cf_html(fragment).unwrap();
    let data = String::from_utf8(bytes.clone()).unwrap();
    // The header has 10 digits for each number (Sakura: 8, so 97 and 134 there).
    assert_eq!(field(&data, "StartHTML"), 105);
    assert_eq!(field(&data, "StartFragment"), 142);
    assert!(data.starts_with("Version:0.9\r\nStartHTML:0000000105\r\nEndHTML:"));
    assert!(bytes[105..].starts_with(b"<html>"));
    let start = field(&data, "StartFragment");
    let end = field(&data, "EndFragment");
    assert_eq!(&bytes[start..end], fragment.as_bytes());
    assert_eq!(field(&data, "EndHTML"), bytes.len());
    assert!(data.ends_with("<!--EndFragment-->\r\n</body></html>\r\n"));
}

#[test]
fn the_offsets_count_bytes_not_characters() {
    let fragment = "<pre>日本語 &amp; é</pre>";
    let bytes = cf_html(fragment).unwrap();
    let data = String::from_utf8(bytes.clone()).unwrap();
    let start = field(&data, "StartFragment");
    let end = field(&data, "EndFragment");
    assert_eq!(end - start, fragment.len());
    assert!(fragment.len() > fragment.chars().count());
    assert_eq!(&bytes[start..end], fragment.as_bytes());
    assert_eq!(field(&data, "EndHTML"), bytes.len());
}

#[test]
fn a_long_fragment_keeps_the_header_the_same_length() {
    let fragment = "x".repeat(123_456);
    let bytes = cf_html(&fragment).unwrap();
    let data = String::from_utf8(bytes.clone()).unwrap();
    assert_eq!(field(&data, "StartHTML"), 105);
    assert_eq!(field(&data, "StartFragment"), 142);
    assert_eq!(field(&data, "EndFragment"), 142 + 123_456);
    assert_eq!(field(&data, "EndHTML"), bytes.len());
}

#[test]
fn a_fragment_of_100_megabytes_keeps_the_header_the_same_length() {
    // Only the header is made: no 100 MB string is needed.
    let small = header(10).unwrap();
    for len in [99_999_999, 100_000_000, 1_500_000_000] {
        let big = header(len).unwrap();
        assert_eq!(big.len(), small.len(), "{len}");
        assert_eq!(field(&big, "StartHTML"), big.len());
        assert_eq!(field(&big, "StartFragment"), big.len() + PREFIX.len());
        assert_eq!(field(&big, "EndFragment"), big.len() + PREFIX.len() + len);
        assert_eq!(
            field(&big, "EndHTML"),
            big.len() + PREFIX.len() + len + SUFFIX.len()
        );
    }
}

#[test]
fn a_fragment_too_long_for_the_header_is_left_out() {
    assert!(header(9_999_999_000).is_some());
    assert!(header(10_000_000_000).is_none());
}
