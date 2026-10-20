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
    let bytes = cf_html(fragment);
    let data = String::from_utf8(bytes.clone()).unwrap();
    // Sakura's offsets for the header and the start of the fragment
    // (`CClipboard::SetHtmlText`: "StartHTML:00000097", "StartFragment:00000134").
    assert_eq!(field(&data, "StartHTML"), 97);
    assert_eq!(field(&data, "StartFragment"), 134);
    assert!(data.starts_with("Version:0.9\r\nStartHTML:00000097\r\nEndHTML:"));
    assert!(data.contains("\r\nStartFragment:00000134\r\nEndFragment:"));
    assert!(bytes[97..].starts_with(b"<html>"));
    let start = field(&data, "StartFragment");
    let end = field(&data, "EndFragment");
    assert_eq!(&bytes[start..end], fragment.as_bytes());
    assert_eq!(field(&data, "EndHTML"), bytes.len());
    assert!(data.ends_with("<!--EndFragment-->\r\n</body></html>\r\n"));
}

#[test]
fn the_offsets_count_bytes_not_characters() {
    let fragment = "<pre>日本語 &amp; é</pre>";
    let bytes = cf_html(fragment);
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
    let bytes = cf_html(&fragment);
    let data = String::from_utf8(bytes.clone()).unwrap();
    assert_eq!(field(&data, "StartHTML"), 97);
    assert_eq!(field(&data, "StartFragment"), 134);
    assert_eq!(field(&data, "EndFragment"), 134 + 123_456);
    assert_eq!(field(&data, "EndHTML"), bytes.len());
}
