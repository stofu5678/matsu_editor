//! Tests of the Grep 置換 of huge files, read in order (ADR-0020, chapter 5;
//! `engine/replace/stream.rs`): the new file must be the bytes that the replacement of a document
//! writes, the result and the counts the same, and the memory the longest line, not the file.
//!
//! None of these tests uses a path of the operating system in its text (the paths in the result
//! are replaced by `<D>`) or a line break of it: the files are made with the breaks they test.

use std::sync::atomic::{AtomicBool, Ordering};

use matsu_core::encoding::Encoding;

use super::engine::stream_probe::{self, Seen};
use super::engine::{GrepInfo, Outcome, ReplaceInfo, Sink, run};
use super::format::{Layout, LineType};
use crate::search::state::SearchOptions;

/// What a Grep 置換 is asked to do.
#[derive(Clone)]
struct Job {
    key: &'static str,
    new: &'static str,
    regex: bool,
    charset: Option<Encoding>,
    backup: bool,
    file_only: bool,
}

impl Job {
    fn new(key: &'static str, new: &'static str) -> Self {
        Self {
            key,
            new,
            regex: false,
            charset: None,
            backup: true,
            file_only: false,
        }
    }

    fn regex(mut self) -> Self {
        self.regex = true;
        self
    }

    fn charset(mut self, charset: Encoding) -> Self {
        self.charset = Some(charset);
        self
    }

    fn file_only(mut self) -> Self {
        self.file_only = true;
        self
    }

    fn info(&self, dir: &tempfile::TempDir, huge: Option<u64>) -> GrepInfo {
        GrepInfo {
            key: self.key.to_owned(),
            file: "*.*".to_owned(),
            folder: super::long_temp_path(dir.path()),
            options: SearchOptions {
                regex: self.regex,
                ..SearchOptions::default()
            },
            sub_folder: false,
            charset: self.charset,
            layout: Layout {
                line_type: LineType::Line,
                ..Layout::default()
            },
            file_only: self.file_only,
            except_file_regex: false,
            mime_decode: false,
            huge_file_len: huge,
            window: None,
            window_gone: false,
            type_detect: None,
            replace: Some(ReplaceInfo {
                text: self.new.to_owned(),
                paste: false,
                backup: self.backup,
            }),
        }
    }
}

struct Collect<'a> {
    text: String,
    /// Sets the flag when a write has this text.
    stop_at: Option<(&'a str, &'a AtomicBool)>,
}

impl Sink for Collect<'_> {
    fn write(&mut self, text: &str) {
        self.text.push_str(text);
        if let Some((needle, flag)) = self.stop_at
            && text.contains(needle)
        {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

/// What a run left: the files of the folder, the result and what the readers saw.
struct Ran {
    file: Vec<u8>,
    backup: Option<Vec<u8>>,
    /// The names of the files in the folder.
    names: Vec<String>,
    text: String,
    outcome: Outcome,
    seen: Seen,
    whole_reads: usize,
}

/// Runs `job` on a file `data.txt` of `bytes`, with an earlier backup if `old_backup` is given.
/// `huge` is the threshold (`None`: the file is read whole) and `chunk` the size of the reads in
/// order. `stop_at` stops the search when the result has the text.
fn ran(
    bytes: &[u8],
    job: &Job,
    huge: Option<u64>,
    chunk: usize,
    old_backup: Option<&[u8]>,
    stop_at: Option<&str>,
) -> Ran {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("data.txt"), bytes).unwrap();
    if let Some(old) = old_backup {
        std::fs::write(dir.path().join("data.txt.skrold"), old).unwrap();
    }
    let info = job.info(&dir, huge);
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        stop_at: stop_at.map(|needle| (needle, &cancel)),
    };
    let (outcome, seen) = stream_probe::with_chunk(chunk, || {
        run(&info, &cancel, &mut sink).expect("the search starts")
    });
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ran {
        file: std::fs::read(dir.path().join("data.txt")).unwrap(),
        backup: std::fs::read(dir.path().join("data.txt.skrold")).ok(),
        names,
        text: sink.text.replace(&info.folder, "<D>"),
        outcome,
        seen,
        whole_reads: stream_probe::whole_reads_in(dir.path()).len(),
    }
}

/// The text after the heading.
fn body(text: &str) -> &str {
    text.split_once("\r\n\r\n\r\n").expect("a heading").1
}

/// Checks that the run in order does what the run of the whole file does, for every size of
/// read from 4 bytes up to past the end of the file and the default one. Returns the run of the
/// whole file.
fn same_as_the_whole_read(bytes: &[u8], job: &Job) -> Ran {
    let whole = ran(bytes, job, None, 4, Some(b"old backup"), None);
    assert_eq!(whole.seen.files, 0, "no threshold: the file is read whole");
    let mut chunks: Vec<usize> = (4..=bytes.len() + 8).step_by(4).collect();
    chunks.push(1024 * 1024);
    for chunk in chunks {
        let got = ran(bytes, job, Some(1), chunk, Some(b"old backup"), None);
        assert_eq!(got.seen.files, 1, "chunk {chunk}: read in order");
        assert_eq!(got.whole_reads, 0, "chunk {chunk}: never read whole");
        assert_eq!(got.file, whole.file, "chunk {chunk}: the new file");
        assert_eq!(got.backup, whole.backup, "chunk {chunk}: the backup");
        assert_eq!(got.names, whole.names, "chunk {chunk}: nothing is left");
        assert_eq!(got.outcome, whole.outcome, "chunk {chunk}");
        assert_eq!(body(&got.text), body(&whole.text), "chunk {chunk}");
    }
    whole
}

fn sjis(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    Encoding::ShiftJis.codec().encode(text, &mut out).unwrap();
    out
}

fn utf16le(text: &str, bom: bool) -> Vec<u8> {
    let mut bytes = if bom { vec![0xFF, 0xFE] } else { Vec::new() };
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    bytes
}

#[test]
fn utf8_with_a_crlf_that_falls_on_the_edge_of_a_chunk() {
    let bytes = "abc abc\r\nxyz\r\nabc\nlast abc\r\n\r\nend".as_bytes();
    let whole = same_as_the_whole_read(bytes, &Job::new("abc", "ふー"));
    assert_eq!(
        whole.file,
        "ふー ふー\r\nxyz\r\nふー\nlast ふー\r\n\r\nend".as_bytes()
    );
    assert_eq!(whole.outcome.hits, 4);
    assert_eq!(whole.backup.as_deref(), Some(bytes), "the old file is kept");
}

#[test]
fn shift_jis_with_double_byte_characters_on_the_edge_of_a_chunk() {
    let bytes = sjis("日本語 abc 表示ソ\r\nabc 能\r\n十 abc 十\r\n");
    let job = Job::new("abc", "文字");
    let forced = same_as_the_whole_read(&bytes, &job.clone().charset(Encoding::ShiftJis));
    assert_eq!(
        forced.file,
        sjis("日本語 文字 表示ソ\r\n文字 能\r\n十 文字 十\r\n")
    );
    // detected, as a Grep 置換 does without an encoding chosen
    let detected = same_as_the_whole_read(&bytes, &job);
    assert_eq!(detected.file, forced.file);
}

#[test]
fn shift_jis_bytes_that_decode_and_encode_differently_stay_in_the_unchanged_part_of_a_line() {
    // 0xEEF9 is the NEC selected IBM extension of "¬"; encoding "¬" gives 0x81CA. The start of
    // the line is not rewritten when only the end of it changes.
    let mut bytes = vec![0xEE, 0xF9];
    bytes.extend_from_slice(&sjis(" abc 能\r\n"));
    let whole = same_as_the_whole_read(&bytes, &Job::new("abc", "x").charset(Encoding::ShiftJis));
    assert_eq!(&whole.file[..2], &[0xEE, 0xF9]);
}

#[test]
fn utf16_with_a_bom_and_surrogate_pairs_on_the_edge_of_a_chunk() {
    let bytes = utf16le("𠮷abc𠮷\r\n😀abc\r\nabc", true);
    let whole = same_as_the_whole_read(&bytes, &Job::new("abc", "x😀y"));
    assert_eq!(whole.file, utf16le("𠮷x😀y𠮷\r\n😀x😀y\r\nx😀y", true));
    let no_bom = utf16le("a abc\r\n", false);
    same_as_the_whole_read(&no_bom, &Job::new("abc", "z").charset(Encoding::Utf16Le));
}

#[test]
fn a_utf8_bom_is_kept() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice("abc\r\nabc\r\n".as_bytes());
    let whole = same_as_the_whole_read(&bytes, &Job::new("abc", "d"));
    assert_eq!(whole.file, b"\xEF\xBB\xBFd\r\nd\r\n");
}

#[test]
fn a_regular_expression_that_covers_the_line_break_joins_the_lines() {
    let bytes = "one\r\ntwo\r\nthree\nfour\rfive abc\r\n".as_bytes();
    same_as_the_whole_read(bytes, &Job::new(r"\r\n", " | ").regex());
    same_as_the_whole_read(bytes, &Job::new(r"[\r\n]+", "/").regex());
    same_as_the_whole_read(bytes, &Job::new(r"e\r?$", "E").regex());
    same_as_the_whole_read(bytes, &Job::new(r"\s", "_").regex());
    same_as_the_whole_read(bytes, &Job::new(r"(t\w+)", "<$1>").regex());
}

#[test]
fn a_change_that_keeps_the_break_and_one_that_changes_it_in_the_same_file() {
    let bytes = "a abc\r\nabc\r\nplain\nabc\n\nabc\r\nz".as_bytes();
    same_as_the_whole_read(bytes, &Job::new("abc", "").regex());
    same_as_the_whole_read(bytes, &Job::new("abc\r?\n?", "").regex());
    same_as_the_whole_read(bytes, &Job::new("abc", "x\ry"));
}

#[test]
fn only_the_first_hit_is_shown_with_the_option_but_all_are_replaced() {
    let bytes = "abc\r\nabc abc\r\nabc\r\n".as_bytes();
    let whole = same_as_the_whole_read(bytes, &Job::new("abc", "x").file_only());
    assert_eq!(whole.outcome.hits, 4);
    assert_eq!(whole.file, b"x\r\nx x\r\nx\r\n");
}

#[test]
fn bytes_that_are_not_valid_in_the_encoding_are_copied_as_they_are() {
    let bytes = b"abc \xFF\xFE abc\r\n\xC2x\xA2 abc\r\n\xE3\x81 abc\r\n\xFF\r\n";
    let whole = same_as_the_whole_read(bytes, &Job::new("abc", "d"));
    assert!(
        whole.file.starts_with(b"d \xFF\xFE d\r\n"),
        "{:?}",
        whole.file
    );
}

#[test]
fn a_replacement_that_changes_nothing_makes_the_same_file_and_the_backup() {
    let bytes = "abc\r\nx\r\n".as_bytes();
    let whole = same_as_the_whole_read(bytes, &Job::new("abc", "abc"));
    assert_eq!(whole.file, bytes);
    assert_eq!(whole.outcome.hits, 1);
    assert_eq!(whole.backup.as_deref(), Some(bytes));
}

#[test]
fn a_file_without_a_hit_is_not_written_and_gets_no_backup() {
    let bytes = "nothing here\r\nat all\r\n".as_bytes();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.txt");
    std::fs::write(&path, bytes).unwrap();
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    let job = Job::new("abc", "d");
    let info = job.info(&dir, Some(1));
    let mut sink = Collect {
        text: String::new(),
        stop_at: None,
    };
    let (outcome, seen) = stream_probe::with_chunk(8, || {
        run(&info, &AtomicBool::new(false), &mut sink).unwrap()
    });
    assert_eq!(outcome.hits, 0);
    assert_eq!(seen.files, 1);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        before
    );
    let names = std::fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(names, 1, "no backup, no temporary file");
}

#[test]
fn a_file_below_the_threshold_is_read_whole_as_before() {
    let bytes = b"abc\r\n";
    let run = ran(bytes, &Job::new("abc", "d"), Some(1024), 4, None, None);
    assert_eq!(run.seen.files, 0);
    assert_eq!(run.whole_reads, 1);
    assert_eq!(run.file, b"d\r\n");
}

#[test]
fn a_character_the_encoding_cannot_write_leaves_the_file_as_it_was() {
    // the hit that cannot be written is in the third line: the lines before it were written to
    // the new file already, and it is dropped
    let bytes = sjis("日本\r\nabc 能\r\nabc\r\n");
    let job = Job::new("abc", "🙂").charset(Encoding::ShiftJis);
    let whole = ran(&bytes, &job, None, 4, Some(b"old"), None);
    assert_eq!(whole.file, bytes);
    for chunk in [4, 8, 1024 * 1024] {
        let got = ran(&bytes, &job, Some(1), chunk, Some(b"old"), None);
        assert_eq!(got.file, bytes, "chunk {chunk}");
        assert_eq!(
            got.backup.as_deref(),
            Some(&b"old"[..]),
            "the old backup stays"
        );
        assert_eq!(got.names, whole.names, "no temporary file, no copy");
        assert_eq!(got.outcome, whole.outcome);
        assert_eq!(got.outcome.hits, 0);
        assert_eq!(body(&got.text), body(&whole.text));
        assert!(
            got.text.contains("書き込めませんでした。 [<D>"),
            "{}",
            got.text
        );
    }
}

#[test]
fn the_old_backup_is_replaced_when_the_new_file_is_in_place() {
    let bytes = b"abc\r\ntail";
    let run = ran(
        bytes,
        &Job::new("abc", "d"),
        Some(1),
        4,
        Some(b"older"),
        None,
    );
    assert_eq!(run.backup.as_deref(), Some(&bytes[..]));
    assert_eq!(run.names, ["data.txt", "data.txt.skrold"]);
    let mut job = Job::new("abc", "d");
    job.backup = false;
    let run = ran(bytes, &job, Some(1), 4, Some(b"older"), None);
    assert_eq!(
        run.backup.as_deref(),
        Some(&b"older"[..]),
        "no backup was asked for"
    );
    assert_eq!(run.file, b"d\r\ntail");
}

#[test]
fn a_stop_in_the_middle_leaves_the_file_and_no_temporary_file() {
    // The result is sent on after 32 KiB: the first hits are written to the new file by then.
    let bytes = "abc line of the file\r\n".repeat(20_000);
    let job = Job::new("abc", "d");
    let run = ran(
        bytes.as_bytes(),
        &job,
        Some(1),
        4096,
        Some(b"old"),
        Some("abc line"),
    );
    assert!(run.outcome.stopped);
    assert_eq!(run.outcome.hits, 0);
    assert_eq!(run.file, bytes.as_bytes(), "the file is as it was");
    assert_eq!(run.backup.as_deref(), Some(&b"old"[..]), "the backup too");
    assert_eq!(
        run.names,
        ["data.txt", "data.txt.skrold"],
        "no temporary file"
    );
    assert!(
        run.seen.bytes < bytes.len() as u64,
        "the file was not read to its end: {} of {}",
        run.seen.bytes,
        bytes.len()
    );
}

#[test]
fn the_memory_is_that_of_a_chunk_and_a_line_not_of_the_file() {
    let bytes = "abc line of the file\r\nplain line\r\n".repeat(60_000);
    assert!(bytes.len() > 2_000_000);
    let job = Job::new("abc", "xyz");
    let got = ran(bytes.as_bytes(), &job, Some(1), 4096, None, None);
    assert_eq!(got.whole_reads, 0);
    assert_eq!(got.seen.files, 1);
    assert_eq!(got.seen.bytes, bytes.len() as u64, "read once, not twice");
    assert!(
        got.seen.peak <= 3 * 4096 + 64,
        "held {} bytes at once of a file of {}",
        got.seen.peak,
        bytes.len()
    );
    assert_eq!(got.outcome.hits, 60_000);
    assert_eq!(
        got.file,
        bytes.replace("abc line", "xyz line").as_bytes(),
        "and the file is right"
    );
}

#[test]
fn a_line_over_the_limit_is_copied_as_it_is_and_told() {
    let limit = 1000;
    let long = format!("{}abc{}", "x".repeat(2500), "y".repeat(2500));
    let cases: [(&str, String); 4] = [
        // before the first change: copied from the file when the new file begins
        ("first", format!("{long}\r\nabc\r\nabc\r\n")),
        // after it: written as it is dropped
        ("middle", format!("abc\r\n{long}\r\nabc\r\n")),
        ("last", format!("abc\r\nabc\r\n{long}\r\n")),
        // with no break at the end of the file
        ("no break", format!("abc\r\n{long}")),
    ];
    let job = Job::new("abc", "d");
    for (name, text) in cases {
        let bytes = text.as_bytes();
        let want = text
            .replace(&format!("{}abc", "x".repeat(2500)), "LONGX")
            .replace("abc", "d")
            .replace("LONGX", &format!("{}abc", "x".repeat(2500)));
        for chunk in [4, 64, 1000, 4096, 1024 * 1024] {
            let got = stream_probe::with_line_limit(limit, || {
                ran(bytes, &job, Some(1), chunk, None, None)
            });
            assert_eq!(got.file, want.as_bytes(), "{name}, chunk {chunk}");
            assert_eq!(got.backup.as_deref(), Some(bytes), "{name}, chunk {chunk}");
            let line = text
                .split("\r\n")
                .position(|l| l == long)
                .map_or(2, |n| n + 1);
            assert!(
                got.text.contains(&format!(
                    "{line}行目が長すぎる（1 MB。上限は 1 MB）ので検索も置換もせず、そのまま残しました。 [<D>"
                )),
                "{name}, chunk {chunk}: {}",
                got.text
            );
            assert_eq!(got.names, ["data.txt", "data.txt.skrold"], "{name}");
        }
    }
}

#[test]
fn a_file_of_a_stateful_encoding_is_told_and_left_alone() {
    // ISO-2022-JP cannot be cut anywhere: it is not read whole instead
    let bytes = b"\x1B$BF|K\\8l\x1B(B abc\r\n";
    let job = Job::new("abc", "d").charset(Encoding::Jis);
    let got = ran(bytes, &job, Some(1), 4096, None, None);
    assert_eq!(got.file, bytes);
    assert_eq!(got.whole_reads, 0);
    assert!(got.text.contains("file open error [<D>"), "{}", got.text);
    assert_eq!(got.names, ["data.txt"]);
}

#[test]
fn an_edit_that_ends_between_the_characters_of_one_invalid_sequence_is_cut_as_the_document_cuts_it()
{
    // One step of the codec over these bytes can decode to two characters of the text (stand-ins
    // for the invalid bytes): an edit that ends between them is rounded down, as the document
    // does, in the new file too.
    let sjis_bytes: Vec<&[u8]> = vec![
        b"\xED\x40 abc\r\n",
        b"\xED\x40\r\nab \xED\x40 c\r\n",
        b"\x81\x20 abc\r\n\x81",
        b"\xFD\xFE\xFF abc\r\n",
    ];
    for bytes in sjis_bytes {
        for key in ["^.", "."] {
            let job = Job::new(key, "X").regex().charset(Encoding::ShiftJis);
            same_as_the_whole_read(bytes, &job);
        }
    }
    // a lone surrogate in UTF-16, and one cut short at the end of the file
    let mut utf16: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0xD8, 0x41, 0x00, 0x20, 0x00];
    utf16.extend(utf16le("abc\r\n", false));
    utf16.extend([0x41, 0x00, 0x00, 0xDC, 0x42, 0x00]);
    let short: Vec<u8> = vec![0x41, 0x00, 0x20, 0x00, 0x00, 0xD8];
    for key in ["^.", "."] {
        let job = Job::new(key, "X").regex().charset(Encoding::Utf16Le);
        same_as_the_whole_read(&utf16, &job);
        same_as_the_whole_read(&short, &job);
    }
}

#[test]
fn a_stop_while_the_start_of_the_file_or_the_backup_is_copied_ends_the_replace_at_once() {
    // The first change is in the last line: the 6000 bytes before it are copied (to the backup,
    // 61 chunks, and to the new file, 60 chunks) in chunks of 100 bytes. The stop comes after the n-th chunk of any
    // copy, so that it comes in the backup and in the start of the new file in turn.
    let mut text = "plain line\r\n".repeat(500);
    text.push_str("abc\r\n");
    for backup in [true, false] {
        let stops: &[usize] = if backup {
            &[1, 5, 40, 61, 100]
        } else {
            &[1, 5, 40, 59]
        };
        for &after in stops {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("data.txt"), &text).unwrap();
            std::fs::write(dir.path().join("data.txt.skrold"), b"old").unwrap();
            let mut job = Job::new("abc", "d");
            job.backup = backup;
            let info = job.info(&dir, Some(1));
            let cancel = std::rc::Rc::new(AtomicBool::new(false));
            let seen = std::rc::Rc::new(std::cell::Cell::new(0usize));
            let hook = {
                let (cancel, seen) = (cancel.clone(), seen.clone());
                move || {
                    seen.set(seen.get() + 1);
                    if seen.get() == after {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
            };
            let mut sink = Collect {
                text: String::new(),
                stop_at: None,
            };
            let outcome = stream_probe::with_copy_hook(100, hook, || {
                run(&info, &cancel, &mut sink).expect("the search starts")
            });
            let what = format!("backup {backup}, stop after chunk {after}");
            assert!(outcome.stopped, "{what}");
            assert_eq!(outcome.hits, 0, "{what}");
            assert!(
                seen.get() == after,
                "{what}: the copy went on: {}",
                seen.get()
            );
            assert_eq!(
                std::fs::read(dir.path().join("data.txt")).unwrap(),
                text.as_bytes(),
                "{what}: the file is as it was"
            );
            assert_eq!(
                std::fs::read(dir.path().join("data.txt.skrold")).unwrap(),
                b"old",
                "{what}: so is the old backup"
            );
            let mut names: Vec<String> = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            assert_eq!(
                names,
                ["data.txt", "data.txt.skrold"],
                "{what}: nothing is left"
            );
        }
    }
}
