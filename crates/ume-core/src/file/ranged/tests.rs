use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::*;
use crate::buffer::{Document, EncodingCodec, Pos};
use crate::encoding::Encoding;

fn opts(block_size: usize, cache_blocks: usize) -> RangedOptions {
    RangedOptions {
        block_size,
        cache_blocks,
    }
}

/// 0, 1, 2, ... 250, 0, 1, ... so that a wrong offset shows up.
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

fn file_with(dir: &tempfile::TempDir, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.path().join("data.bin");
    fs::write(&path, bytes).unwrap();
    path
}

/// Moves the modification time so that a rewrite is seen even on a file system with coarse
/// timestamps.
fn touch_later(path: &std::path::Path) {
    let file = OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() + Duration::from_secs(10))
        .unwrap();
}

#[test]
fn reads_inside_one_block() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(100);
    let f = RangedFile::open_with(file_with(&dir, &bytes), opts(16, 4)).unwrap();
    assert_eq!(f.len(), 100);
    assert_eq!(f.read(17, 25).unwrap().as_ref(), &bytes[17..25]);
    assert_eq!(f.read(16, 32).unwrap().as_ref(), &bytes[16..32]);
    assert_eq!(f.cache_stats(), CacheStats { hits: 1, misses: 1 });
}

#[test]
fn reads_across_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(1000);
    let f = RangedFile::open_with(file_with(&dir, &bytes), opts(16, 100)).unwrap();
    assert_eq!(f.read(10, 22).unwrap().as_ref(), &bytes[10..22]);
    assert_eq!(f.read(15, 17).unwrap().as_ref(), &bytes[15..17]);
    assert_eq!(f.read(3, 997).unwrap().as_ref(), &bytes[3..997]);
    assert_eq!(f.read(0, 1000).unwrap().as_ref(), &bytes[..]);
}

#[test]
fn reads_at_the_end_and_the_short_last_block() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(50); // blocks of 16: 16, 16, 16, 2
    let f = RangedFile::open_with(file_with(&dir, &bytes), opts(16, 4)).unwrap();
    assert_eq!(f.read(48, 50).unwrap().as_ref(), &bytes[48..]);
    assert_eq!(f.read(40, 50).unwrap().as_ref(), &bytes[40..]);
    assert!(f.read(40, 51).is_err());
    assert!(f.read(51, 51).is_err());
    assert!(f.read(5, 4).is_err());
}

#[test]
fn a_file_of_exactly_one_block_multiple() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(32);
    let f = RangedFile::open_with(file_with(&dir, &bytes), opts(16, 4)).unwrap();
    assert_eq!(f.read(0, 32).unwrap().as_ref(), &bytes[..]);
    assert_eq!(f.read(32, 32).unwrap().as_ref(), b"");
}

#[test]
fn empty_ranges_and_an_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = RangedFile::open_with(file_with(&dir, b""), opts(16, 4)).unwrap();
    assert_eq!(f.len(), 0);
    assert!(f.is_empty());
    assert_eq!(f.read(0, 0).unwrap().as_ref(), b"");
    assert!(f.read(0, 1).is_err());

    let bytes = pattern(40);
    let f = RangedFile::open_with(file_with(&dir, &bytes), opts(16, 4)).unwrap();
    assert_eq!(f.read(20, 20).unwrap().as_ref(), b"");
    // An empty range reads no block.
    assert_eq!(f.cache_stats(), CacheStats::default());
}

#[test]
fn the_cache_evicts_the_least_recently_used_block() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(16 * 10);
    let f = RangedFile::open_with(file_with(&dir, &bytes), opts(16, 3)).unwrap();
    for block in 0..3 {
        f.read(block * 16, block * 16 + 1).unwrap(); // blocks 0, 1, 2
    }
    assert_eq!(f.cache_stats(), CacheStats { hits: 0, misses: 3 });
    f.read(0, 1).unwrap(); // block 0 is the newest now
    f.read(48, 49).unwrap(); // block 3 evicts block 1
    assert_eq!(f.cached_bytes(), 3 * 16);
    let before = f.cache_stats();
    f.read(0, 1).unwrap(); // still cached
    f.read(32, 33).unwrap(); // still cached
    assert_eq!(f.cache_stats().hits, before.hits + 2);
    f.read(16, 17).unwrap(); // block 1 was evicted
    assert_eq!(f.cache_stats().misses, before.misses + 1);
    assert_eq!(f.cached_bytes(), 3 * 16);
}

#[test]
fn a_cache_of_one_block_still_gives_correct_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(500);
    let f = RangedFile::open_with(file_with(&dir, &bytes), opts(7, 1)).unwrap();
    assert_eq!(f.read(0, 500).unwrap().as_ref(), &bytes[..]);
    assert_eq!(f.read(100, 300).unwrap().as_ref(), &bytes[100..300]);
    // Long ranges skip the cache.
    assert_eq!(f.cached_bytes(), 0);
    for start in (0..480).step_by(13) {
        assert_eq!(
            f.read(start, start + 13).unwrap().as_ref(),
            &bytes[start as usize..start as usize + 13]
        );
    }
    assert_eq!(f.cached_bytes(), 7);
}

#[test]
fn truncation_makes_reads_fail_with_changed_outside() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(100);
    let path = file_with(&dir, &bytes);
    let f = RangedFile::open_with(&path, opts(16, 2)).unwrap();
    assert_eq!(f.read(0, 4).unwrap().as_ref(), &bytes[..4]);
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(40)
        .unwrap();
    // Not cached, past the new end.
    let err = f.read(80, 90).unwrap_err();
    assert!(is_changed_outside(&err));
    // Once it failed, everything fails, even a range in the cache.
    assert!(is_changed_outside(&f.read(0, 4).unwrap_err()));
    assert!(is_changed_outside(&f.check().unwrap_err()));
    // The length of the source does not change.
    assert_eq!(f.len(), 100);
}

#[test]
fn truncation_inside_the_range_that_is_still_there_is_found_too() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(100);
    let path = file_with(&dir, &bytes);
    let f = RangedFile::open_with(&path, opts(16, 2)).unwrap();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(40)
        .unwrap();
    // The first block is still on the disk, but the file is not the one we opened.
    assert!(is_changed_outside(&f.read(0, 4).unwrap_err()));
}

#[test]
fn appending_keeps_the_old_length_readable() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(100);
    let path = file_with(&dir, &bytes);
    let f = RangedFile::open_with(&path, opts(16, 2)).unwrap();
    assert_eq!(f.read(0, 4).unwrap().as_ref(), &bytes[..4]);
    OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(&[9; 70])
        .unwrap();
    touch_later(&path);
    assert_eq!(f.len(), 100);
    assert_eq!(f.read(90, 100).unwrap().as_ref(), &bytes[90..]);
    assert_eq!(f.read(0, 100).unwrap().as_ref(), &bytes[..]);
    f.check().unwrap();
    // Reading past the old length is still out of range, not an "outside" error.
    let err = f.read(95, 120).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    assert!(!is_changed_outside(&err));
}

#[test]
fn a_rewrite_with_another_time_and_the_same_length_is_found() {
    let dir = tempfile::tempdir().unwrap();
    let path = file_with(&dir, &pattern(100));
    let f = RangedFile::open_with(&path, opts(16, 2)).unwrap();
    fs::write(&path, vec![1u8; 100]).unwrap();
    touch_later(&path);
    assert!(is_changed_outside(&f.check().unwrap_err()));
    assert!(is_changed_outside(&f.read(60, 70).unwrap_err()));
}

#[test]
fn replacing_the_file_does_not_change_what_an_open_source_reads() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(100);
    let path = file_with(&dir, &bytes);
    let f = RangedFile::open_with(&path, opts(16, 2)).unwrap();
    let new = dir.path().join("new.bin");
    fs::write(&new, vec![7u8; 30]).unwrap();
    // A rename over an open file works on Unix; on Windows it may be refused. The open handle
    // keeps the old file either way (ADR-0020 asks the PoC to find out which).
    let _ = fs::rename(&new, &path);
    assert_eq!(f.read(0, 100).unwrap().as_ref(), &bytes[..]);
}

#[test]
fn deleting_the_file_keeps_an_open_source_readable_where_the_os_allows_it() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(100);
    let path = file_with(&dir, &bytes);
    let f = RangedFile::open_with(&path, opts(16, 2)).unwrap();
    if fs::remove_file(&path).is_ok() {
        assert_eq!(f.read(0, 100).unwrap().as_ref(), &bytes[..]);
    }
}

#[test]
fn a_missing_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let err = RangedFile::open(dir.path().join("nope")).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(!is_changed_outside(&err));
}

#[test]
fn the_error_says_what_happened() {
    let err = changed_outside();
    assert!(is_changed_outside(&err));
    assert_eq!(err.to_string(), "the file was changed outside UME");
    assert!(!is_changed_outside(&io::Error::other("something else")));
    assert!(!is_changed_outside(&io::Error::from(
        io::ErrorKind::UnexpectedEof
    )));
}

#[test]
fn a_long_range_does_not_push_out_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = pattern(1000);
    let f = RangedFile::open_with(file_with(&dir, &bytes), opts(16, 2)).unwrap();
    f.read(0, 4).unwrap();
    assert_eq!(f.read(100, 900).unwrap().as_ref(), &bytes[100..900]);
    assert_eq!(f.cache_stats(), CacheStats { hits: 0, misses: 1 });
    f.read(1, 2).unwrap();
    assert_eq!(f.cache_stats(), CacheStats { hits: 1, misses: 1 });
}

#[test]
fn a_long_range_of_a_truncated_file_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = file_with(&dir, &pattern(1000));
    let f = RangedFile::open_with(&path, opts(16, 2)).unwrap();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(500)
        .unwrap();
    assert!(is_changed_outside(&f.read(0, 900).unwrap_err()));
}

#[test]
fn several_threads_read_one_file() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = Arc::new(pattern(10_000));
    let f = Arc::new(RangedFile::open_with(file_with(&dir, &bytes), opts(64, 8)).unwrap());
    let handles: Vec<_> = (0..4usize)
        .map(|t| {
            let (f, bytes) = (f.clone(), bytes.clone());
            std::thread::spawn(move || {
                for i in 0..500usize {
                    let start = (i * 37 + t * 1013) % 9_000;
                    let end = start + 1 + (i * 11) % 900;
                    assert_eq!(f.read(start as u64, end as u64).unwrap().as_ref(), {
                        &bytes[start..end]
                    });
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    assert!(f.cached_bytes() <= 8 * 64);
}

#[test]
fn a_document_reads_lines_through_a_ranged_file() {
    let dir = tempfile::tempdir().unwrap();
    let text = "first\r\nsecond line\n日本語の行\nlast";
    let path = file_with(&dir, text.as_bytes());
    // Blocks of 5 bytes cut the CRLF and the Japanese characters.
    let f = RangedFile::open_with(&path, opts(5, 3)).unwrap();
    let doc = Document::new(f, Arc::new(EncodingCodec(Encoding::Utf8))).unwrap();
    assert_eq!(doc.line_count(), 4);
    assert_eq!(doc.line(0).unwrap().unwrap().text, "first");
    assert_eq!(doc.line(2).unwrap().unwrap().text, "日本語の行");
    assert_eq!(
        doc.text_between(Pos::new(1, 0), Pos::new(2, 9)).unwrap(),
        "second line\n日本語"
    );
}

#[test]
fn positions_past_4_gib_are_read_with_64_bit_offsets() {
    // A sparse file: only the pages written exist on disk (where the file system supports
    // holes; others write zeros, so this stays small to be safe).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sparse.bin");
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    let at = (4u64 << 30) + 100;
    if file.set_len(at + 16).is_err() {
        return;
    }
    let mut file = file;
    {
        use std::io::Seek;
        file.seek(io::SeekFrom::Start(at)).unwrap();
    }
    file.write_all(b"beyond 4 GiB!!!").unwrap();
    drop(file);
    let f = RangedFile::open_with(&path, opts(4096, 2)).unwrap();
    assert_eq!(f.len(), at + 16);
    assert_eq!(f.read(at, at + 15).unwrap().as_ref(), b"beyond 4 GiB!!!");
    assert_eq!(f.read(at - 3, at).unwrap().as_ref(), &[0, 0, 0]);
}
