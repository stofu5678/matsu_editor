use std::path::PathBuf;

use super::*;

fn filter(patterns: &[&str]) -> FileFilter {
    FileFilter {
        name: "x".to_owned(),
        patterns: patterns.iter().map(|p| (*p).to_owned()).collect(),
    }
}

#[test]
fn the_extensions_of_a_filter_are_the_plain_ones() {
    assert_eq!(
        filter(&["*.cpp", "*.h", "*.*", "Make*", "*.t?t", "*.tar.gz", "*."]).extensions(),
        ["cpp", "h"]
    );
    assert!(filter(&[]).extensions().is_empty());
}

#[test]
fn a_filter_that_shows_everything() {
    assert!(filter(&["*.txt", "*.*"]).matches_all());
    assert!(filter(&["*"]).matches_all());
    assert!(!filter(&["*.txt"]).matches_all());
    assert!(!filter(&[]).matches_all());
}

#[test]
fn the_default_extension_is_added_to_a_name_without_one() {
    assert_eq!(
        with_default_extension(PathBuf::from("/a/無題_20261019_120000"), "txt"),
        PathBuf::from("/a/無題_20261019_120000.txt")
    );
    // a name that has an extension is the user's
    assert_eq!(
        with_default_extension(PathBuf::from("/a/b.md"), "txt"),
        PathBuf::from("/a/b.md")
    );
    assert_eq!(
        with_default_extension(PathBuf::from("/a/b"), ""),
        PathBuf::from("/a/b")
    );
    assert_eq!(
        with_default_extension(PathBuf::from("/"), "txt"),
        PathBuf::from("/")
    );
}

#[test]
fn a_file_that_is_there_under_the_longer_name_is_not_taken() {
    let dir = std::env::temp_dir().join(format!("matsu-dialog-ext-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("memo.txt"), "x").unwrap();
    // the dialog never asked about overwriting memo.txt
    assert_eq!(
        with_default_extension(dir.join("memo"), "txt"),
        dir.join("memo")
    );
    assert_eq!(
        with_default_extension(dir.join("other"), "txt"),
        dir.join("other.txt")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_label_shows_the_name_and_the_patterns() {
    assert_eq!(filter(&["*.c", "*.h"]).label(), "x (*.c;*.h)");
    assert_eq!(filter(&["*.*"]).label(), "x (*.*)");
    assert_eq!(filter(&[]).label(), "x");
}
