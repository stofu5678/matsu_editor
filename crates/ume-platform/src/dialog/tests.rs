use super::*;

#[test]
fn a_file_uri_is_a_path() {
    assert_eq!(
        path_from_file_uri("file:///home/a/b.txt"),
        Some(PathBuf::from("/home/a/b.txt"))
    );
    assert_eq!(
        path_from_file_uri("file://localhost/home/a"),
        Some(PathBuf::from("/home/a"))
    );
}

#[test]
fn escapes_are_decoded_as_utf8() {
    assert_eq!(
        path_from_file_uri("file:///home/a/%E3%83%A1%E3%83%A2%20%231.txt"),
        Some(PathBuf::from("/home/a/メモ #1.txt"))
    );
}

#[test]
fn other_uris_are_not_paths() {
    assert_eq!(path_from_file_uri("https://example.com/a"), None);
    assert_eq!(path_from_file_uri("file://server/share/a"), None);
    assert_eq!(path_from_file_uri("file:///a/%zz"), None);
    assert_eq!(path_from_file_uri("file:///a/%E3%83"), None);
    assert_eq!(path_from_file_uri("file:///a/%4"), None);
}

#[test]
fn the_errors_say_what_happened() {
    assert!(
        Error::Unavailable("portal".into())
            .to_string()
            .contains("portal")
    );
    assert!(Error::Failed("x".into()).to_string().contains('x'));
}
