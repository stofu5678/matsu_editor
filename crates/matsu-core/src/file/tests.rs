//! Tests of opening and saving. They use real files in a private temporary folder.

use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use super::*;
use crate::buffer::{BufferError, Pos};
use crate::encoding::Certainty;

// ---- helpers ----

/// A folder that is removed at the end of the test.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("matsu-file-test-{}-{n}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let p = self.path(name);
        fs::write(&p, bytes).unwrap();
        p
    }

    /// The names of the files in the folder, sorted.
    fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn open_as(path: &Path, encoding: Encoding) -> TextFile {
    open_ext(path, encoding, false)
}

/// Opens with the setting `bEnableExtEol` (NEL, LS and PS as line endings) given.
fn open_ext(path: &Path, encoding: Encoding, ext_eol: bool) -> TextFile {
    let options = OpenOptions {
        encoding: Some(encoding),
        ext_eol,
        ..OpenOptions::default()
    };
    open_file(path, &options).unwrap()
}

fn text(file: &TextFile) -> String {
    file.document
        .text_between(Pos::new(0, 0), Pos::new(usize::MAX, 0))
        .unwrap()
}

fn save(file: &mut TextFile, options: &SaveOptions) -> SaveReport {
    file.save(options, &mut no_progress).unwrap()
}

fn save_keep(file: &mut TextFile) -> SaveReport {
    let options = SaveOptions::keep(file);
    save(file, &options)
}

fn encode(encoding: Encoding, text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    encoding.codec().encode(text, &mut out).unwrap();
    out
}

/// A text every encoding can write, with CR LF line endings and no ending on the last line.
fn sample(encoding: Encoding) -> &'static str {
    match encoding {
        Encoding::Latin1 => "\u{e9}t\u{e9} na\u{ef}ve\r\nsecond\r\nlast",
        _ => "\u{65e5}\u{672c}\u{8a9e} abc\r\n\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\r\nlast",
    }
}

// ---- opening ----

#[test]
fn every_encoding_opens_edits_and_saves() {
    for encoding in Encoding::ALL {
        let dir = TempDir::new();
        let text_in = sample(encoding);
        let mut bytes = if encoding.bom_default_on() {
            encoding.bom().to_vec()
        } else {
            Vec::new()
        };
        bytes.extend(encode(encoding, text_in));
        let path = dir.write("a.txt", &bytes);

        let mut file = open_as(&path, encoding);
        assert_eq!(file.encoding, encoding);
        assert_eq!(file.bom, encoding.bom_default_on(), "{encoding:?}");
        assert_eq!(text(&file), text_in, "{encoding:?}");
        assert_eq!(file.new_line, Eol::CrLf);

        // Saving without a change gives the same bytes.
        save_keep(&mut file);
        assert_eq!(fs::read(&path).unwrap(), bytes, "{encoding:?} unchanged");

        // Append one char; the original part is copied and the new part is encoded.
        let end = file.document.end_pos().unwrap();
        file.document.insert(end, "!").unwrap();
        save_keep(&mut file);
        let mut expected = if file.bom {
            encoding.bom().to_vec()
        } else {
            Vec::new()
        };
        expected.extend(encode(encoding, &format!("{text_in}!")));
        assert_eq!(fs::read(&path).unwrap(), expected, "{encoding:?} edited");

        let again = open_as(&path, encoding);
        assert_eq!(text(&again), format!("{text_in}!"), "{encoding:?} reopened");
        assert_eq!(dir.names(), ["a.txt"], "no temporary file is left");
    }
}

#[test]
fn unedited_bytes_survive_byte_for_byte() {
    // Bytes that are invalid in their encoding, and line endings of every kind mixed.
    let cases: [(Encoding, &[u8]); 3] = [
        (Encoding::Utf8, b"ab\xff\xfe\r\nc\xe3\x81\nd\rend\n"),
        (Encoding::ShiftJis, b"abc\xff\x80\r\n\xed\x40x\n\rend"),
        (Encoding::Utf16Le, b"a\0\x00\xd8b\0\r\0\n\0c\0\x01"),
    ];
    for (encoding, bytes) in cases {
        let dir = TempDir::new();
        let path = dir.write("a.txt", bytes);
        let mut file = open_as(&path, encoding);
        assert!(!file.bom);

        save_keep(&mut file);
        assert_eq!(fs::read(&path).unwrap(), bytes, "{encoding:?} unchanged");

        // An edit at the start leaves every other byte alone.
        file.document.insert(Pos::new(0, 0), "Z").unwrap();
        save_keep(&mut file);
        let mut expected = encode(encoding, "Z");
        expected.extend_from_slice(bytes);
        assert_eq!(fs::read(&path).unwrap(), expected, "{encoding:?} edited");
    }
}

#[test]
fn a_bom_is_removed_from_the_text_and_remembered() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"\xEF\xBB\xBFhello\n");
    let mut file = open_file(&path, &OpenOptions::default()).unwrap();
    assert_eq!(file.encoding, Encoding::Utf8);
    assert!(file.bom);
    assert_eq!(text(&file), "hello\n");
    assert_eq!(
        file.detection.as_ref().map(|d| d.certainty),
        Some(Certainty::Bom)
    );

    // The BOM is written again by a plain save.
    save_keep(&mut file);
    assert_eq!(fs::read(&path).unwrap(), b"\xEF\xBB\xBFhello\n");

    // It can be dropped.
    let mut options = SaveOptions::keep(&file);
    options.bom = false;
    save(&mut file, &options);
    assert_eq!(fs::read(&path).unwrap(), b"hello\n");
    assert!(!file.bom, "the document remembers the choice");

    // And added.
    options.bom = true;
    save(&mut file, &options);
    assert_eq!(fs::read(&path).unwrap(), b"\xEF\xBB\xBFhello\n");
}

#[test]
fn a_bom_is_not_written_for_an_encoding_without_one() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc");
    let mut file = open_as(&path, Encoding::ShiftJis);
    let mut options = SaveOptions::keep(&file);
    options.bom = true;
    save(&mut file, &options);
    assert_eq!(fs::read(&path).unwrap(), b"abc");
}

#[test]
fn a_bom_in_a_file_read_as_another_encoding_is_text() {
    // Forced as Shift_JIS, a UTF-8 BOM is three characters of text, not a BOM.
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"\xEF\xBB\xBFabc");
    let mut file = open_as(&path, Encoding::ShiftJis);
    assert!(!file.bom);
    save_keep(&mut file);
    assert_eq!(fs::read(&path).unwrap(), b"\xEF\xBB\xBFabc");
}

#[test]
fn the_encoding_can_be_forced_or_detected() {
    let dir = TempDir::new();
    let bytes = encode(
        Encoding::ShiftJis,
        &"\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{6587}\u{7ae0}\r\n".repeat(20),
    );
    let path = dir.write("a.txt", &bytes);
    let detected = open_file(&path, &OpenOptions::default()).unwrap();
    assert_eq!(detected.encoding, Encoding::ShiftJis);
    assert!(detected.detection.is_some());
    assert_eq!(detected.source_encoding(), Encoding::ShiftJis);

    let forced = open_as(&path, Encoding::Latin1);
    assert_eq!(forced.encoding, Encoding::Latin1);
    assert!(forced.detection.is_none());
    assert_ne!(text(&forced), text(&detected));
    // Whatever it is read as, the bytes come back.
    let mut forced = forced;
    save_keep(&mut forced);
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn the_line_ending_for_new_lines() {
    let dir = TempDir::new();
    let open = |bytes: &[u8], options: &OpenOptions| {
        open_file(dir.write("a.txt", bytes), options)
            .unwrap()
            .new_line
    };
    let lf_default = OpenOptions {
        default_eol: Eol::Lf,
        ..OpenOptions::default()
    };
    // The first line's ending wins.
    assert_eq!(open(b"a\nb\r\n", &lf_default), Eol::Lf);
    assert_eq!(open(b"a\r\nb\n", &lf_default), Eol::CrLf);
    assert_eq!(open(b"a\rb\n", &lf_default), Eol::Cr);
    // No ending: the default applies to the default encoding (UTF-8), the line ending of the OS to any other.
    assert_eq!(open(b"abc", &lf_default), Eol::Lf);
    assert_eq!(open(b"", &lf_default), Eol::Lf);
    let sjis = OpenOptions {
        encoding: Some(Encoding::ShiftJis),
        ..lf_default
    };
    assert_eq!(open(b"abc", &sjis), Eol::platform_default());
}

#[test]
fn a_file_without_a_line_ending_follows_the_os_by_default() {
    let dir = TempDir::new();
    let file = open_file(dir.write("a.txt", b"abc"), &OpenOptions::default()).unwrap();
    let expected = if cfg!(windows) { Eol::CrLf } else { Eol::Lf };
    assert_eq!(file.new_line, expected);
    assert_eq!(OpenOptions::default().default_eol, expected);
}

#[test]
fn an_empty_file_opens_and_saves() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"");
    let mut file = open_file(&path, &OpenOptions::default()).unwrap();
    assert_eq!(file.document.line_count(), 0);
    save_keep(&mut file);
    assert_eq!(fs::read(&path).unwrap(), b"");
}

#[test]
fn a_directory_is_not_a_file() {
    let dir = TempDir::new();
    assert!(matches!(
        open_file(&dir.0, &OpenOptions::default()),
        Err(FileError::Io(_))
    ));
    assert!(matches!(
        open_file(dir.path("missing"), &OpenOptions::default()),
        Err(FileError::Io(_))
    ));
}

// ---- the size limit ----

#[test]
fn a_file_over_the_limit_is_refused_without_being_read() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"0123456789");
    let mut options = OpenOptions {
        max_len: Some(5),
        ..OpenOptions::default()
    };
    let mut calls = 0;
    let err = open_file_with(&path, &options, &mut |_, _| {
        calls += 1;
        ControlFlow::Continue(())
    })
    .unwrap_err();
    assert!(
        matches!(err, FileError::TooLarge { len: 10, max: 5 }),
        "{err}"
    );
    assert!(err.to_string().contains("too large"));
    assert_eq!(calls, 0, "nothing was read");
    // Exactly at the limit is fine.
    options.max_len = Some(10);
    assert!(open_file(&path, &options).is_ok());
}

#[test]
fn there_is_no_size_limit_by_default() {
    // ADR-0012: no limit, as 64-bit Sakura.
    assert_eq!(OpenOptions::default().max_len, None);
}

#[cfg(unix)]
#[test]
fn a_sparse_file_over_an_explicit_limit_is_refused_without_being_read() {
    // `set_len` makes a sparse file on Unix file systems, so this does not write 5 GiB. If the
    // size check did not come first, the open would try to read 5 GiB.
    let dir = TempDir::new();
    let path = dir.path("big.txt");
    let file = fs::File::create(&path).unwrap();
    file.set_len(5 * 1024 * 1024 * 1024).unwrap();
    drop(file);
    let options = OpenOptions {
        max_len: Some(4 * 1024 * 1024 * 1024),
        ..OpenOptions::default()
    };
    let err = open_file(&path, &options).unwrap_err();
    assert!(
        matches!(err, FileError::TooLarge { len, max } if len == 5 << 30 && max == 4 << 30),
        "{err}"
    );
}

#[test]
fn opening_can_be_cancelled() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &vec![b'a'; 3 * READ_CHUNK + 5]);
    let mut seen = Vec::new();
    let err = open_file_with(&path, &OpenOptions::default(), &mut |done, total| {
        seen.push((done, total));
        if seen.len() == 2 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .unwrap_err();
    assert!(matches!(err, FileError::Cancelled));
    let total = (3 * READ_CHUNK + 5) as u64;
    assert_eq!(
        seen,
        [(READ_CHUNK as u64, total), (2 * READ_CHUNK as u64, total)]
    );
}

// ---- line endings ----

#[test]
fn line_endings_are_kept_or_converted() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"a\r\nb\nc\rd");
    let mut file = open_file(&path, &OpenOptions::default()).unwrap();
    assert!(file.has_mixed_line_endings().unwrap());

    // A plain save keeps them.
    save_keep(&mut file);
    assert_eq!(fs::read(&path).unwrap(), b"a\r\nb\nc\rd");

    // Conversion changes every ending but does not add one at the end.
    let mut options = SaveOptions::keep(&file);
    options.eol = Some(Eol::Lf);
    save(&mut file, &options);
    assert_eq!(fs::read(&path).unwrap(), b"a\nb\nc\nd");
    assert_eq!(file.new_line, Eol::Lf);
    // The document itself was not changed.
    assert_eq!(text(&file), "a\r\nb\nc\rd");

    // A later plain save does not bring the old endings back.
    save_keep(&mut file);
    assert_eq!(fs::read(&path).unwrap(), b"a\nb\nc\nd");
    // A save that chose not to convert is what the next plain save repeats.
    let mut no_conversion = SaveOptions::keep(&file);
    no_conversion.eol = None;
    save(&mut file, &no_conversion);
    assert_eq!(fs::read(&path).unwrap(), b"a\r\nb\nc\rd");
    save_keep(&mut file);
    assert_eq!(fs::read(&path).unwrap(), b"a\r\nb\nc\rd");
    options.eol = Some(Eol::CrLf);
    save(&mut file, &options);
    assert_eq!(fs::read(&path).unwrap(), b"a\r\nb\r\nc\r\nd");
    options.eol = Some(Eol::Cr);
    save(&mut file, &options);
    assert_eq!(fs::read(&path).unwrap(), b"a\rb\rc\rd");

    let same = TempDir::new();
    let path = same.write("b.txt", b"a\nb\nc\n");
    let file = open_file(&path, &OpenOptions::default()).unwrap();
    assert!(!file.has_mixed_line_endings().unwrap());
}

#[test]
fn line_endings_are_converted_in_the_files_encoding() {
    let dir = TempDir::new();
    let mut bytes = Encoding::Utf16Le.bom().to_vec();
    bytes.extend(encode(Encoding::Utf16Le, "a\r\nb\n"));
    let path = dir.write("a.txt", &bytes);
    let mut file = open_as(&path, Encoding::Utf16Le);
    let mut options = SaveOptions::keep(&file);
    options.eol = Some(Eol::Lf);
    save(&mut file, &options);
    let mut expected = Encoding::Utf16Le.bom().to_vec();
    expected.extend(encode(Encoding::Utf16Le, "a\nb\n"));
    assert_eq!(fs::read(&path).unwrap(), expected);
}

#[test]
fn a_file_with_one_kind_of_ending_round_trips() {
    for (name, bytes) in [
        ("crlf", &b"a\r\nb\r\nc"[..]),
        ("lf", b"a\nb\nc\n"),
        ("cr", b"a\rb\rc\r"),
    ] {
        let dir = TempDir::new();
        let path = dir.write(name, bytes);
        let mut file = open_file(&path, &OpenOptions::default()).unwrap();
        assert!(!file.has_mixed_line_endings().unwrap());
        file.document.insert(Pos::new(0, 1), "!").unwrap();
        save_keep(&mut file);
        let got = fs::read(&path).unwrap();
        assert_eq!(got.len(), bytes.len() + 1, "{name}");
        assert_eq!(&got[..2], b"a!", "{name}");
        assert_eq!(&got[2..], &bytes[1..], "{name}");
    }
}

// ---- characters that cannot be written ----

#[test]
fn unmappable_characters_are_reported_before_anything_is_written() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "a\u{65e5}\u{1F600}\nb\u{1F600}".as_bytes());
    let mut file = open_as(&path, Encoding::Utf8);

    let options = SaveOptions::for_encoding(Encoding::ShiftJis);
    let report = file.check_save(&options, &mut no_progress).unwrap();
    assert_eq!(report.total, 2);
    assert_eq!(
        report.first,
        [
            UnmappableChar {
                pos: Pos::new(0, 4),
                col_utf16: 2,
                ch: '\u{1F600}',
            },
            UnmappableChar {
                pos: Pos::new(1, 1),
                col_utf16: 1,
                ch: '\u{1F600}',
            },
        ]
    );

    // Saving to the same path is refused, and neither the file nor the folder changes.
    let before = fs::read(&path).unwrap();
    let err = file.save(&options, &mut no_progress).unwrap_err();
    assert!(
        matches!(&err, FileError::Unmappable(r) if r.total == 2),
        "{err}"
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(dir.names(), ["a.txt"]);
    assert_eq!(file.encoding, Encoding::Utf8, "the document did not change");

    // "Save anyway" writes a question mark for each (Sakura: CCodeBase, `?`).
    let target = dir.path("b.txt");
    let mut anyway = options.clone();
    anyway.allow_unmappable = true;
    let saved = file.save_as(&target, &anyway, &mut no_progress).unwrap();
    assert_eq!(saved.substituted, 2);
    let mut expected = encode(Encoding::ShiftJis, "a\u{65e5}");
    expected.extend_from_slice(b"?\nb?");
    assert_eq!(fs::read(&target).unwrap(), expected);
    assert_eq!(saved.bytes_written, expected.len() as u64);
    assert_eq!(file.encoding, Encoding::ShiftJis);
    assert_eq!(file.path.as_deref(), Some(target.as_path()));
}

#[test]
fn added_text_that_the_encoding_cannot_write_is_found() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &encode(Encoding::ShiftJis, "\u{65e5}\u{672c}\n"));
    let mut file = open_as(&path, Encoding::ShiftJis);
    let options = SaveOptions::keep(&file);
    assert_eq!(
        file.check_save(&options, &mut no_progress).unwrap().total,
        0
    );
    file.document.insert(Pos::new(0, 3), "\u{1F600}").unwrap();
    let report = file.check_save(&options, &mut no_progress).unwrap();
    assert_eq!(report.total, 1);
    assert_eq!(report.first[0].pos, Pos::new(0, 3));
    assert!(matches!(
        file.save(&options, &mut no_progress),
        Err(FileError::Unmappable(_))
    ));
}

#[test]
fn a_line_ending_the_encoding_cannot_write_is_reported() {
    // U+0085 is a line ending in UTF-8 (with the setting on) but has no Shift_JIS form.
    let dir = TempDir::new();
    let path = dir.write("a.txt", "ab\u{85}cd".as_bytes());
    let file = open_ext(&path, Encoding::Utf8, true);
    assert_eq!(file.document.line_count(), 2);
    let report = file
        .check_save(
            &SaveOptions::for_encoding(Encoding::ShiftJis),
            &mut no_progress,
        )
        .unwrap();
    assert_eq!(report.total, 1);
    assert_eq!(report.first[0].pos, Pos::new(0, 2));
    assert_eq!(report.first[0].ch, '\u{85}');
    // Converting every ending to CR LF makes it writable.
    let mut options = SaveOptions::for_encoding(Encoding::ShiftJis);
    options.eol = Some(Eol::CrLf);
    assert_eq!(
        file.check_save(&options, &mut no_progress).unwrap().total,
        0
    );
}

#[test]
fn the_report_lists_only_the_first_hundred() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "\u{1F600}\n".repeat(150).as_bytes());
    let file = open_as(&path, Encoding::Utf8);
    let report = file
        .check_save(
            &SaveOptions::for_encoding(Encoding::ShiftJis),
            &mut no_progress,
        )
        .unwrap();
    assert_eq!(report.total, 150);
    assert_eq!(report.first.len(), MAX_REPORTED);
    assert_eq!(report.first[99].pos, Pos::new(99, 0));
}

// ---- saving in another encoding ----

#[test]
fn saving_in_another_encoding_converts_everything() {
    let dir = TempDir::new();
    let path = dir.write(
        "a.txt",
        &encode(Encoding::ShiftJis, "\u{65e5}\u{672c}\r\nabc"),
    );
    let mut file = open_as(&path, Encoding::ShiftJis);
    let target = dir.path("b.txt");
    save_as(
        &mut file,
        &target,
        &SaveOptions::for_encoding(Encoding::Utf8),
    );
    assert_eq!(
        fs::read(&target).unwrap(),
        "\u{65e5}\u{672c}\r\nabc".as_bytes()
    );
    assert_eq!(file.encoding, Encoding::Utf8);
    // The original of the document is still Shift_JIS: a later plain save in UTF-8 converts again.
    assert_eq!(file.source_encoding(), Encoding::ShiftJis);
    file.document.insert(Pos::new(1, 3), "!").unwrap();
    save_keep(&mut file);
    assert_eq!(
        fs::read(&target).unwrap(),
        "\u{65e5}\u{672c}\r\nabc!".as_bytes()
    );
    // The first file is as it was.
    assert_eq!(
        fs::read(&path).unwrap(),
        encode(Encoding::ShiftJis, "\u{65e5}\u{672c}\r\nabc")
    );

    // With a BOM by default for UTF-16.
    save_as(
        &mut file,
        &dir.path("c.txt"),
        &SaveOptions::for_encoding(Encoding::Utf16Le),
    );
    let mut expected = b"\xFF\xFE".to_vec();
    expected.extend(encode(Encoding::Utf16Le, "\u{65e5}\u{672c}\r\nabc!"));
    assert_eq!(fs::read(dir.path("c.txt")).unwrap(), expected);
}

fn save_as(file: &mut TextFile, path: &Path, options: &SaveOptions) -> SaveReport {
    file.save_as(path, options, &mut no_progress).unwrap()
}

#[test]
fn invalid_bytes_are_written_as_their_byte_in_another_encoding() {
    // Find a byte that Shift_JIS cannot decode.
    let codec = Encoding::ShiftJis.codec();
    let bad = (0x80u8..=0xFF)
        .find(|&b| {
            let mut s = String::new();
            codec.decode(&[b'x', b], &mut s);
            s.chars()
                .any(|c| c as u32 >= crate::encoding::INVALID_BYTE_BASE)
        })
        .expect("Shift_JIS has invalid bytes");
    let dir = TempDir::new();
    let path = dir.write("a.txt", &[b'a', bad, b'b']);
    let mut file = open_as(&path, Encoding::ShiftJis);
    let target = dir.path("b.txt");
    let report = save_as(
        &mut file,
        &target,
        &SaveOptions::for_encoding(Encoding::Utf8),
    );
    // The raw byte is written (as Sakura does); it is not an unmappable character.
    assert_eq!(report.substituted, 0);
    assert_eq!(fs::read(&target).unwrap(), [b'a', bad, b'b']);
}

#[test]
fn a_large_amount_of_added_text_is_written_in_pieces() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"head\nTAIL");
    let mut file = open_as(&path, Encoding::Utf8);
    let big = "\u{3042}".repeat(READ_CHUNK); // 3 MiB of UTF-8
    file.document.insert(Pos::new(0, 4), &big).unwrap();
    save_keep(&mut file);
    let got = fs::read(&path).unwrap();
    assert_eq!(got.len(), 9 + 3 * READ_CHUNK);
    assert!(got.starts_with(b"head"));
    assert!(got.ends_with(b"\nTAIL"));
    // The same through another encoding.
    save_as(
        &mut file,
        &dir.path("b.txt"),
        &SaveOptions::for_encoding(Encoding::Utf16Le),
    );
    assert_eq!(
        fs::read(dir.path("b.txt")).unwrap().len(),
        2 + 2 * (9 + READ_CHUNK)
    );
}

// ---- failure paths ----

#[test]
fn a_new_document_needs_a_path() {
    let dir = TempDir::new();
    let mut file = TextFile::new_untitled(Encoding::Utf8, false, Eol::Lf);
    file.document.insert(Pos::new(0, 0), "new\n").unwrap();
    let err = file
        .save(&SaveOptions::keep(&file), &mut no_progress)
        .unwrap_err();
    assert!(matches!(err, FileError::NoPath));
    let path = dir.path("n.txt");
    let keep = SaveOptions::keep(&file);
    save_as(&mut file, &path, &keep);
    assert_eq!(fs::read(&path).unwrap(), b"new\n");
    // Now a plain save works.
    file.document.insert(Pos::new(1, 0), "x").unwrap();
    save_keep(&mut file);
    assert_eq!(fs::read(&path).unwrap(), b"new\nx");
}

#[test]
fn a_cancelled_save_leaves_the_file_and_the_folder_as_they_were() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"original\n");
    let mut file = open_as(&path, Encoding::Utf8);
    file.document.insert(Pos::new(0, 0), "edit ").unwrap();
    let err = file
        .save(
            &SaveOptions::keep(&file),
            &mut |_, _| ControlFlow::Break(()),
        )
        .unwrap_err();
    assert!(matches!(err, FileError::Cancelled));
    assert_eq!(fs::read(&path).unwrap(), b"original\n");
    assert_eq!(dir.names(), ["a.txt"], "the temporary file is removed");

    // The same on the path that converts line by line.
    let err = file
        .save(
            &SaveOptions::for_encoding(Encoding::Utf16Le),
            &mut |_, _| ControlFlow::Break(()),
        )
        .unwrap_err();
    assert!(matches!(err, FileError::Cancelled));
    assert_eq!(fs::read(&path).unwrap(), b"original\n");
    assert_eq!(dir.names(), ["a.txt"]);
    assert_eq!(file.encoding, Encoding::Utf8, "the document did not change");
}

#[test]
fn a_short_document_can_be_cancelled_too() {
    // Nothing but a little added text: no progress is reported while writing, but the cancel
    // is still honored before the target is replaced.
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"old");
    let mut file = TextFile::new_untitled(Encoding::Utf8, false, Eol::Lf);
    file.document.insert(Pos::new(0, 0), "new").unwrap();
    let err = file
        .save_as(&path, &SaveOptions::keep(&file), &mut |_, _| {
            ControlFlow::Break(())
        })
        .unwrap_err();
    assert!(matches!(err, FileError::Cancelled));
    assert_eq!(fs::read(&path).unwrap(), b"old");
    assert_eq!(dir.names(), ["a.txt"]);
    assert_eq!(file.path, None);
}

#[test]
fn a_failure_while_creating_the_file_changes_nothing() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"original");
    let mut file = open_as(&path, Encoding::Utf8);
    // The folder does not exist.
    let err = file
        .save_as(
            dir.path("missing").join("b.txt"),
            &SaveOptions::keep(&file),
            &mut no_progress,
        )
        .unwrap_err();
    assert!(matches!(err, FileError::Io(_)), "{err}");
    assert_eq!(file.path.as_deref(), Some(path.as_path()));
    // The target is a folder.
    let sub = dir.path("sub");
    fs::create_dir(&sub).unwrap();
    let err = file
        .save_as(&sub, &SaveOptions::keep(&file), &mut no_progress)
        .unwrap_err();
    assert!(matches!(err, FileError::Io(_)), "{err}");
    assert_eq!(dir.names(), ["a.txt", "sub"]);
    assert_eq!(fs::read_dir(&sub).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn a_pipe_is_not_replaced() {
    let dir = TempDir::new();
    let fifo = dir.path("pipe");
    let status = std::process::Command::new("mkfifo").arg(&fifo).status();
    if !matches!(status, Ok(s) if s.success()) {
        return; // no mkfifo on this machine
    }
    let mut file = TextFile::new_untitled(Encoding::Utf8, false, Eol::Lf);
    let keep = SaveOptions::keep(&file);
    let err = file.save_as(&fifo, &keep, &mut no_progress).unwrap_err();
    assert!(matches!(err, FileError::Io(_)), "{err}");
    // Opening it must not block either.
    assert!(matches!(
        open_file(&fifo, &OpenOptions::default()),
        Err(FileError::Io(_))
    ));
    assert_eq!(dir.names(), ["pipe"]);
    assert!(!fs::metadata(&fifo).unwrap().is_file());
}

#[test]
fn many_unmappable_characters_in_one_line_are_counted() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "\u{1F600}".repeat(50_000).as_bytes());
    let file = open_as(&path, Encoding::Utf8);
    let report = file
        .check_save(
            &SaveOptions::for_encoding(Encoding::ShiftJis),
            &mut no_progress,
        )
        .unwrap();
    assert_eq!(report.total, 50_000);
    assert_eq!(report.first.len(), MAX_REPORTED);
    assert_eq!(report.first[1].col_utf16, 2);
}

#[test]
fn a_read_only_file_is_not_replaced() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"original");
    let mut file = open_as(&path, Encoding::Utf8);
    file.document.insert(Pos::new(0, 0), "x").unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&path, perms.clone()).unwrap();
    let result = file.save(&SaveOptions::keep(&file), &mut no_progress);
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    fs::set_permissions(&path, perms).unwrap();
    assert!(matches!(result, Err(FileError::ReadOnly)));
    assert_eq!(fs::read(&path).unwrap(), b"original");
    assert_eq!(dir.names(), ["a.txt"]);
}

#[cfg(unix)]
#[test]
fn permissions_are_kept_and_symlinks_are_followed() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let dir = TempDir::new();
    let real = dir.write("real.txt", b"one");
    fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
    let link = dir.path("link.txt");
    symlink(&real, &link).unwrap();

    let mut file = open_as(&link, Encoding::Utf8);
    file.document.insert(Pos::new(0, 3), "two").unwrap();
    save_keep(&mut file);

    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the link is still a link"
    );
    assert_eq!(fs::read(&real).unwrap(), b"onetwo");
    assert_eq!(
        fs::metadata(&real).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(dir.names(), ["link.txt", "real.txt"]);
}

#[test]
fn write_bytes_writes_a_new_file_and_replaces_an_old_one() {
    let dir = TempDir::new();
    let new = dir.path("new.bin");
    write_bytes(&new, b"\x00\xFFabc", Preserve::default()).unwrap();
    assert_eq!(fs::read(&new).unwrap(), b"\x00\xFFabc");
    let old = dir.write("old.bin", b"much longer old contents");
    write_bytes(&old, b"x", Preserve::default()).unwrap();
    assert_eq!(fs::read(&old).unwrap(), b"x");
    assert_eq!(
        dir.names(),
        ["new.bin", "old.bin"],
        "no temporary file is left"
    );
}

#[test]
fn write_bytes_refuses_a_folder_and_a_read_only_file() {
    let dir = TempDir::new();
    let sub = dir.path("sub");
    fs::create_dir(&sub).unwrap();
    assert!(write_bytes(&sub, b"x", Preserve::default()).is_err());
    let file = dir.write("ro.bin", b"keep");
    let mut permissions = fs::metadata(&file).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&file, permissions).unwrap();
    assert!(matches!(
        write_bytes(&file, b"x", Preserve::default()),
        Err(FileError::ReadOnly)
    ));
    assert_eq!(fs::read(&file).unwrap(), b"keep");
    // Let the folder be removed on every OS.
    let mut permissions = fs::metadata(&file).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&file, permissions).unwrap();
}

#[cfg(unix)]
#[test]
fn write_bytes_keeps_the_mode_and_follows_a_symlink() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let dir = TempDir::new();
    let real = dir.write("real.bin", b"one");
    fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
    let link = dir.path("link.bin");
    symlink(&real, &link).unwrap();
    write_bytes(&link, b"two", Preserve::default()).unwrap();
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&real).unwrap(), b"two");
    assert_eq!(
        fs::metadata(&real).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(dir.names(), ["link.bin", "real.bin"]);
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_is_followed_not_replaced() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new();
    let link = dir.path("link.txt");
    symlink("real.txt", &link).unwrap(); // relative, and real.txt does not exist
    let mut file = TextFile::new_untitled(Encoding::Utf8, false, Eol::Lf);
    file.document.insert(Pos::new(0, 0), "data").unwrap();
    let keep = SaveOptions::keep(&file);
    file.save_as(&link, &keep, &mut no_progress).unwrap();
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(dir.path("real.txt")).unwrap(), b"data");
}

#[cfg(unix)]
#[test]
fn a_private_file_stays_private() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new();
    let path = dir.write("secret.txt", b"x");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let mut file = open_as(&path, Encoding::Utf8);
    file.document.insert(Pos::new(0, 1), "y").unwrap();
    save_keep(&mut file);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    // The temporary file is created with that mode from the start.
    let (_file, temp) =
        super::save::create_temp_for_test(&dir.0, Some(&fs::Permissions::from_mode(0o600)));
    assert_eq!(
        fs::metadata(&temp).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn a_new_file_is_created_and_an_old_one_replaced() {
    let dir = TempDir::new();
    let mut file = TextFile::new_untitled(Encoding::ShiftJis, false, Eol::CrLf);
    file.document.insert(Pos::new(0, 0), "\u{65e5}").unwrap();
    let path = dir.path("new.txt");
    let keep = SaveOptions::keep(&file);
    save_as(&mut file, &path, &keep);
    assert_eq!(
        fs::read(&path).unwrap(),
        encode(Encoding::ShiftJis, "\u{65e5}")
    );
    // Saving over an existing, different file replaces its content completely.
    fs::write(&path, vec![b'x'; 1000]).unwrap();
    save_keep(&mut file);
    assert_eq!(
        fs::read(&path).unwrap(),
        encode(Encoding::ShiftJis, "\u{65e5}")
    );
    assert_eq!(dir.names(), ["new.txt"]);
}

#[test]
fn progress_is_reported_while_saving() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &vec![b'a'; 2 * READ_CHUNK + 1]);
    let mut file = open_as(&path, Encoding::Utf8);
    let mut seen = Vec::new();
    file.save(&SaveOptions::keep(&file), &mut |done, total| {
        seen.push((done, total));
        ControlFlow::Continue(())
    })
    .unwrap();
    let total = (2 * READ_CHUNK + 1) as u64;
    assert_eq!(
        seen,
        [
            (READ_CHUNK as u64, total),
            (2 * READ_CHUNK as u64, total),
            (total, total),
            // The last chance to cancel, before the target is replaced.
            (total, total)
        ]
    );
}

// ---- speed ----

/// Run with `cargo test -p matsu-core --release -- --ignored --nocapture file::tests::large`.
#[test]
#[ignore = "slow in debug builds; run with --release --ignored --nocapture to see the numbers"]
fn large_file_open_and_save_times() {
    for (encoding, line) in [
        (
            Encoding::Utf8,
            "The quick brown fox \u{65e5}\u{672c}\u{8a9e} jumps over the lazy dog\r\n",
        ),
        (
            Encoding::ShiftJis,
            "The quick brown fox \u{65e5}\u{672c}\u{8a9e} jumps over the lazy dog\r\n",
        ),
    ] {
        let dir = TempDir::new();
        let unit = encode(encoding, line);
        let count = (500 * 1024 * 1024) / unit.len();
        let mut bytes = Vec::with_capacity(count * unit.len());
        for _ in 0..count {
            bytes.extend_from_slice(&unit);
        }
        let path = dir.write("big.txt", &bytes);
        let size_mib = bytes.len() / (1024 * 1024);
        drop(bytes);

        let t = Instant::now();
        let mut file = open_file(&path, &OpenOptions::default()).unwrap();
        println!(
            "{encoding:?} {size_mib} MiB: open {:?}, {} lines",
            t.elapsed(),
            file.document.line_count()
        );
        assert_eq!(file.encoding, encoding);

        let t = Instant::now();
        let options = SaveOptions::keep(&file);
        file.save(&options, &mut no_progress).unwrap();
        println!("{encoding:?}: unedited save {:?}", t.elapsed());

        file.document.insert(Pos::new(1000, 0), "edit").unwrap();
        let t = Instant::now();
        file.save(&options, &mut no_progress).unwrap();
        println!("{encoding:?}: edited save {:?}", t.elapsed());

        let t = Instant::now();
        let mut options = SaveOptions::keep(&file);
        options.eol = Some(Eol::Lf);
        file.save(&options, &mut no_progress).unwrap();
        println!(
            "{encoding:?}: save with line ending conversion {:?}",
            t.elapsed()
        );
    }
}

#[test]
fn the_check_can_be_cancelled_too() {
    let mut file = TextFile::new_untitled(Encoding::Utf8, false, Eol::Lf);
    file.document.insert(Pos::new(0, 0), "added text").unwrap();
    let keep = SaveOptions::keep(&file);
    let err = file
        .check_save(&keep, &mut |_, _| ControlFlow::Break(()))
        .unwrap_err();
    assert!(matches!(err, FileError::Cancelled));
}

// ---- very long lines ----

#[test]
fn a_line_of_several_chunks_is_converted_piece_by_piece() {
    let dir = TempDir::new();
    let line = "abc\u{65e5}".repeat(READ_CHUNK); // about 6 MiB of UTF-8 in one line
    let path = dir.write("a.txt", line.as_bytes());
    let mut file = open_as(&path, Encoding::Utf8);
    assert_eq!(file.document.line_count(), 1);
    let target = dir.path("b.txt");
    save_as(
        &mut file,
        &target,
        &SaveOptions::for_encoding(Encoding::Utf16Le),
    );
    let mut expected = b"\xFF\xFE".to_vec();
    expected.extend(encode(Encoding::Utf16Le, &line));
    assert_eq!(fs::read(&target).unwrap(), expected);

    // Shift_JIS source with multibyte characters cut into pieces.
    let sjis = encode(Encoding::ShiftJis, &"\u{65e5}\u{672c}".repeat(READ_CHUNK));
    let path = dir.write("c.txt", &sjis);
    let mut file = open_as(&path, Encoding::ShiftJis);
    let target = dir.path("d.txt");
    let mut options = SaveOptions::for_encoding(Encoding::Utf8);
    options.eol = Some(Eol::Lf);
    save_as(&mut file, &target, &options);
    assert_eq!(
        fs::read(&target).unwrap(),
        "\u{65e5}\u{672c}".repeat(READ_CHUNK).as_bytes()
    );
}

#[test]
fn line_endings_at_the_edge_of_a_piece_are_converted_once() {
    // A CR LF whose CR is the last byte of the first 1 MiB piece.
    let mut bytes = vec![b'a'; READ_CHUNK - 1];
    bytes.extend_from_slice(b"\r\nb\r\nc");
    let dir = TempDir::new();
    let path = dir.write("a.txt", &bytes);
    let mut file = open_as(&path, Encoding::Utf8);
    assert!(!file.has_mixed_line_endings().unwrap());
    let mut options = SaveOptions::keep(&file);
    options.eol = Some(Eol::Lf);
    save(&mut file, &options);
    let mut expected = vec![b'a'; READ_CHUNK - 1];
    expected.extend_from_slice(b"\nb\nc");
    assert_eq!(fs::read(&path).unwrap(), expected);
}

#[test]
fn cancelling_works_inside_one_long_line() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &vec![b'a'; 4 * READ_CHUNK]);
    let mut file = open_as(&path, Encoding::Utf8);
    let mut calls = 0;
    let err = file
        .save(
            &SaveOptions::for_encoding(Encoding::Utf16Le),
            &mut |_, _| {
                calls += 1;
                if calls == 2 {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            },
        )
        .unwrap_err();
    assert!(matches!(err, FileError::Cancelled));
    // It stopped in the pre-save check, long before the end of the line.
    assert_eq!(calls, 2);
    assert_eq!(fs::read(&path).unwrap(), vec![b'a'; 4 * READ_CHUNK]);
    assert_eq!(dir.names(), ["a.txt"]);

    // And while writing (the check is skipped when the caller allows the loss).
    let mut options = SaveOptions::for_encoding(Encoding::Utf16Le);
    options.allow_unmappable = true;
    let mut calls = 0;
    let err = file
        .save(&options, &mut |_, _| {
            calls += 1;
            if calls == 2 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .unwrap_err();
    assert!(matches!(err, FileError::Cancelled));
    assert_eq!(calls, 2);
    assert_eq!(dir.names(), ["a.txt"]);
}

#[test]
fn an_unmappable_character_in_a_later_piece_has_the_right_columns() {
    let dir = TempDir::new();
    // 600,000 chars of 3 bytes: the emoji is in the second 1 MiB piece.
    let n = 600_000;
    let mut text = "\u{65e5}".repeat(n);
    text.push('\u{1F600}');
    text.push_str("\r\n\u{1F600}");
    let path = dir.write("a.txt", text.as_bytes());
    let file = open_as(&path, Encoding::Utf8);
    let report = file
        .check_save(
            &SaveOptions::for_encoding(Encoding::ShiftJis),
            &mut no_progress,
        )
        .unwrap();
    assert_eq!(report.total, 2);
    assert_eq!(
        report.first[0],
        UnmappableChar {
            pos: Pos::new(0, 3 * n),
            col_utf16: n,
            ch: '\u{1F600}',
        }
    );
    assert_eq!(report.first[1].pos, Pos::new(1, 0));
}

#[test]
fn code_pages_open_edit_and_save_with_invalid_bytes() {
    use crate::encoding::CodePage;
    // Bytes made with Python's codecs: text, an invalid byte, CR LF, text.
    let cases: [(u16, &[u8], &str); 4] = [
        (936, b"\xbc\xf2\xcc\xe5\xff\r\n\xd6\xd0\xce\xc4", "\u{4e2d}"),
        (950, b"\xc1\x63\xff\r\n\xa4\xa4\xa4\xe5", "\u{4e2d}"),
        (949, b"\xc7\xd1\xff\r\n\xb1\xb9\xbe\xee", "\u{d55c}"),
        (1251, b"\xcf\xf0\xe8\x98\r\n\xe2\xe5\xf2", "\u{43f}"),
    ];
    for (number, bytes, added) in cases {
        let encoding = Encoding::CodePage(CodePage::new(number).unwrap());
        let dir = TempDir::new();
        let path = dir.write("a.txt", bytes);
        let mut file = open_as(&path, encoding);
        assert_eq!(file.encoding, encoding);
        assert!(!file.bom);
        assert_eq!(file.new_line, Eol::CrLf);
        save_keep(&mut file);
        assert_eq!(fs::read(&path).unwrap(), bytes, "CP{number} unchanged");

        let end = file.document.end_pos().unwrap();
        file.document.insert(end, added).unwrap();
        save_keep(&mut file);
        let mut expected = bytes.to_vec();
        expected.extend(encode(encoding, added));
        assert_eq!(fs::read(&path).unwrap(), expected, "CP{number} edited");
    }
}

#[test]
fn preserve_defaults_to_permissions_only() {
    assert!(!Preserve::default().owner_and_group);
    assert!(
        Preserve::default()
            .with_owner_and_group(true)
            .owner_and_group
    );
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"x");
    let file = open_as(&path, Encoding::Utf8);
    assert_eq!(SaveOptions::keep(&file).preserve, Preserve::default());
    assert_eq!(
        SaveOptions::for_encoding(Encoding::Utf8).preserve,
        Preserve::default()
    );
}

#[cfg(unix)]
#[test]
fn owner_and_group_can_be_kept_and_the_mode_still_is() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    for keep_owner in [false, true] {
        let dir = TempDir::new();
        let path = dir.write("a.txt", b"one");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let before = fs::metadata(&path).unwrap();
        let mut file = open_as(&path, Encoding::Utf8);
        file.document.insert(Pos::new(0, 3), "two").unwrap();
        let mut options = SaveOptions::keep(&file);
        options.preserve = Preserve::default().with_owner_and_group(keep_owner);
        save(&mut file, &options);

        let after = fs::metadata(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"onetwo");
        assert_eq!(after.permissions().mode() & 0o777, 0o640);
        // The test creates the file itself, so the owner and group are the same either way; this
        // checks that asking for them does not break the save. Changing them needs root.
        assert_eq!((after.uid(), after.gid()), (before.uid(), before.gid()));
        assert_eq!(dir.names(), ["a.txt"]);
    }
}

// ---- MIME headers (bAutoMIMEdecode) ----

/// "日本語" as a Base64 encoded word of ISO-2022-JP.
const MIME_WORD: &[u8] = b"Subject: =?ISO-2022-JP?B?GyRCRnxLXDhsGyhC?=\r\n";

fn open_mime(path: &Path, encoding: Encoding, mime_decode: bool) -> TextFile {
    let options = OpenOptions {
        encoding: Some(encoding),
        mime_decode,
        ..OpenOptions::default()
    };
    open_file(path, &options).unwrap()
}

#[test]
fn a_jis_file_has_its_mime_words_decoded_when_the_option_is_on() {
    let dir = TempDir::new();
    let path = dir.write("mail.txt", MIME_WORD);
    assert_eq!(
        text(&open_mime(&path, Encoding::Jis, true)),
        "Subject: 日本語\r\n"
    );
}

#[test]
fn mime_decoding_is_off_by_default() {
    assert!(!OpenOptions::default().mime_decode);
    let dir = TempDir::new();
    let path = dir.write("mail.txt", MIME_WORD);
    assert_eq!(
        text(&open_mime(&path, Encoding::Jis, false)),
        "Subject: =?ISO-2022-JP?B?GyRCRnxLXDhsGyhC?=\r\n"
    );
}

#[test]
fn only_jis_files_are_mime_decoded() {
    let dir = TempDir::new();
    let path = dir.write("mail.txt", MIME_WORD);
    assert_eq!(
        text(&open_mime(&path, Encoding::Utf8, true)),
        String::from_utf8_lossy(MIME_WORD)
    );
}

#[test]
fn a_detected_jis_file_is_mime_decoded_too() {
    let dir = TempDir::new();
    let mut bytes = b"\x1B$BF|K\\8l\x1B(B\r\n".to_vec();
    bytes.extend_from_slice(MIME_WORD);
    let path = dir.write("mail.txt", &bytes);
    let options = OpenOptions {
        mime_decode: true,
        ..OpenOptions::default()
    };
    let file = open_file(&path, &options).unwrap();
    assert_eq!(file.encoding, Encoding::Jis);
    assert_eq!(text(&file), "日本語\r\nSubject: 日本語\r\n");
}

// ---- NEL, LS and PS (`m_bEnableExtEol`) ----

const EXT_TEXT: &str = "a\u{85}b\u{2028}c\u{2029}d\r\ne";

#[test]
fn a_file_with_nel_ls_and_ps_is_split_only_when_the_setting_is_on() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", EXT_TEXT.as_bytes());
    let off = open_ext(&path, Encoding::Utf8, false);
    assert_eq!(off.document.line_count(), 2);
    assert_eq!(text(&off), EXT_TEXT);
    // The first line ending is the CR LF, not the NEL (`CDocEditor::OnAfterLoad`).
    assert_eq!(off.new_line, Eol::CrLf);
    let on = open_ext(&path, Encoding::Utf8, true);
    assert_eq!(on.document.line_count(), 5);
    assert_eq!(on.new_line, Eol::Nel);
    assert!(on.document.ext_eol());
    assert!(!off.document.ext_eol());
}

#[test]
fn utf16_follows_the_setting_too() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &encode(Encoding::Utf16Le, EXT_TEXT));
    let off = open_ext(&path, Encoding::Utf16Le, false);
    assert_eq!(off.document.line_count(), 2);
    assert_eq!(off.new_line, Eol::CrLf);
    assert_eq!(text(&off), EXT_TEXT);
    let on = open_ext(&path, Encoding::Utf16Le, true);
    assert_eq!(on.document.line_count(), 5);
    assert_eq!(on.new_line, Eol::Nel);
}

#[test]
fn shift_jis_has_no_nel_so_the_setting_changes_nothing_there() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", &encode(Encoding::ShiftJis, "\u{65e5}\r\n\u{672c}"));
    for ext in [false, true] {
        let file = open_ext(&path, Encoding::ShiftJis, ext);
        assert_eq!(file.document.line_count(), 2);
    }
    // A NEL typed into a Shift_JIS document is a character with no Shift_JIS form when the
    // setting is off, and a line ending (which cannot be written either) when it is on: both are
    // reported.
    for ext in [false, true] {
        let mut file = open_ext(&path, Encoding::ShiftJis, ext);
        file.document.insert(Pos::new(0, 0), "\u{85}").unwrap();
        let report = file
            .check_save(&SaveOptions::keep(&file), &mut no_progress)
            .unwrap();
        assert_eq!(report.total, 1, "ext {ext}");
        assert_eq!(report.first[0].ch, '\u{85}');
    }
}

#[test]
fn converting_the_line_endings_leaves_the_characters_alone_when_the_setting_is_off() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", EXT_TEXT.as_bytes());
    let mut options = SaveOptions::for_encoding(Encoding::Utf8);
    options.eol = Some(Eol::Lf);
    let mut off = open_ext(&path, Encoding::Utf8, false);
    let target = dir.0.join("off.txt");
    off.save_as(&target, &options, &mut no_progress).unwrap();
    assert_eq!(
        fs::read(&target).unwrap(),
        "a\u{85}b\u{2028}c\u{2029}d\ne".as_bytes()
    );
    let mut on = open_ext(&path, Encoding::Utf8, true);
    let target = dir.0.join("on.txt");
    on.save_as(&target, &options, &mut no_progress).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"a\nb\nc\nd\ne");
}

#[test]
fn added_text_follows_the_setting_when_it_is_saved() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"x");
    let mut options = SaveOptions::for_encoding(Encoding::Utf8);
    options.eol = Some(Eol::CrLf);
    for (ext, want) in [(false, "x\u{85}y\r\nz"), (true, "x\r\ny\r\nz")] {
        let mut file = open_ext(&path, Encoding::Utf8, ext);
        file.document.insert(Pos::new(0, 1), "\u{85}y\nz").unwrap();
        let target = dir.0.join(format!("added-{ext}.txt"));
        file.save_as(&target, &options, &mut no_progress).unwrap();
        assert_eq!(fs::read(&target).unwrap(), want.as_bytes(), "ext {ext}");
    }
}

#[test]
fn the_original_keeps_its_endings_when_the_setting_changes_before_a_save() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "a\u{85}b\r\nc".as_bytes());
    let mut options = SaveOptions::for_encoding(Encoding::Utf8);
    options.eol = Some(Eol::Lf);
    // Read with the setting off, then turned on: the NEL is still a character.
    let mut file = open_ext(&path, Encoding::Utf8, false);
    file.document.set_ext_eol(true);
    let target = dir.0.join("off-then-on.txt");
    file.save_as(&target, &options, &mut no_progress).unwrap();
    assert_eq!(fs::read(&target).unwrap(), "a\u{85}b\nc".as_bytes());
    // Read with it on, then turned off: the NEL is still a line ending.
    let mut file = open_ext(&path, Encoding::Utf8, true);
    file.document.set_ext_eol(false);
    let target = dir.0.join("on-then-off.txt");
    file.save_as(&target, &options, &mut no_progress).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"a\nb\nc");
}

#[test]
fn added_text_keeps_its_endings_when_the_setting_changes_before_a_save() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"");
    let mut options = SaveOptions::for_encoding(Encoding::Utf8);
    options.eol = Some(Eol::Lf);
    // Typed with the setting off, then it is turned on: the NEL stays a character. The text
    // typed next to it after the change is scanned under the new setting.
    let mut file = open_ext(&path, Encoding::Utf8, false);
    file.document.insert(Pos::new(0, 0), "a\u{85}b").unwrap();
    file.document.set_ext_eol(true);
    file.document.insert(Pos::new(0, 4), "c\u{85}d").unwrap();
    assert_eq!(file.document.line_count(), 2);
    let target = dir.0.join("off-then-on.txt");
    file.save_as(&target, &options, &mut no_progress).unwrap();
    assert_eq!(fs::read(&target).unwrap(), "a\u{85}bc\nd".as_bytes());
    // The other way round: the NEL typed with the setting on is still a line ending.
    let mut file = open_ext(&path, Encoding::Utf8, true);
    file.document.insert(Pos::new(0, 0), "a\u{85}b").unwrap();
    file.document.set_ext_eol(false);
    file.document.insert(Pos::new(1, 1), "c\u{85}d").unwrap();
    assert_eq!(file.document.line_count(), 2);
    let target = dir.0.join("on-then-off.txt");
    file.save_as(&target, &options, &mut no_progress).unwrap();
    assert_eq!(fs::read(&target).unwrap(), "a\nbc\u{85}d".as_bytes());
}

// ---- saving with small chunks (ADR-0020 section 4) ----

/// Everything a save writes for one file, as bytes and reports: the plain save, the save after
/// edits, saves in other encodings and with converted line endings, and the check before saving.
fn saved_outputs(encoding: Encoding, original: &[u8]) -> Vec<String> {
    let dir = TempDir::new();
    let path = dir.write("a.txt", original);
    let mut file = open_as(&path, encoding);
    let mut out = Vec::new();
    let mut record = |label: &str, file: &mut TextFile, options: &SaveOptions, name: &str| {
        let check = file.check_save(options, &mut no_progress).unwrap();
        let target = dir.path(name);
        let report = save_as(file, &target, options);
        out.push(format!(
            "{label}: {check:?} {report:?} {:?}",
            fs::read(&target).unwrap()
        ));
    };

    let keep = SaveOptions::keep(&file);
    record("plain", &mut file, &keep, "plain.txt");
    for (label, target) in [
        ("utf8", Encoding::Utf8),
        ("utf16", Encoding::Utf16Le),
        ("sjis", Encoding::ShiftJis),
        ("latin1", Encoding::Latin1),
    ] {
        let mut options = SaveOptions::for_encoding(target);
        options.allow_unmappable = true;
        record(label, &mut file, &options, &format!("{label}.txt"));
    }

    // Edits in the middle of the first original piece, at its start and at its end.
    file.document.insert(Pos::new(1, 1), "\u{e9}").unwrap();
    file.document.insert(Pos::new(0, 0), "\u{e9}\r\n").unwrap();
    file.document
        .delete(Pos::new(2, 0), Pos::new(2, 1))
        .unwrap();
    let end = file.document.end_pos().unwrap();
    file.document.insert(end, "\u{e9}\r\n!").unwrap();
    let mut keep = SaveOptions::keep(&file);
    keep.allow_unmappable = true;
    record("edited", &mut file, &keep, "edited.txt");
    for (label, eol) in [("lf", Eol::Lf), ("crlf", Eol::CrLf), ("cr", Eol::Cr)] {
        let mut options = keep.clone();
        options.eol = Some(eol);
        record(label, &mut file, &options, &format!("{label}.txt"));
    }
    for (label, target) in [("e-utf8", Encoding::Utf8), ("e-utf16be", Encoding::Utf16Be)] {
        let mut options = SaveOptions::for_encoding(target);
        options.eol = Some(Eol::CrLf);
        record(label, &mut file, &options, &format!("{label}.txt"));
    }
    out
}

#[test]
fn saving_with_tiny_chunks_gives_the_same_bytes() {
    use crate::encoding::CodePage;
    // CRLF, a lone CR, a lone LF, a surrogate pair, double-byte characters and a last line
    // without an ending, so that a cut at any byte falls inside something.
    let text = "ab\r\n\u{65e5}\u{672c}\u{8a9e}\u{1f600}\rx\u{e9}\n\r\n\u{3053}\u{3093}\r\nlast";
    let mut encodings: Vec<Encoding> = Encoding::ALL.to_vec();
    encodings.extend([Encoding::Utf32Le, Encoding::Utf32Be]);
    encodings.extend(
        [936u16, 950, 949, 1251]
            .into_iter()
            .map(|n| Encoding::CodePage(CodePage::new(n).unwrap())),
    );
    for encoding in encodings {
        let mut original = encoding.bom().to_vec();
        // Characters that the encoding cannot write become `?`; the file is read back anyway.
        let _ = encoding.codec().encode(text, &mut original);
        let normal = saved_outputs(encoding, &original);
        for len in [1, 2, 3, 4, 5, 7, 8, 13] {
            let tiny = crate::buffer::with_chunk_len(len, || saved_outputs(encoding, &original));
            assert_eq!(tiny.len(), normal.len());
            for (a, b) in tiny.iter().zip(&normal) {
                assert_eq!(a, b, "{encoding:?} with chunks of {len} bytes");
            }
        }
    }
}

#[test]
fn saving_with_tiny_chunks_keeps_invalid_bytes() {
    let cases: [(Encoding, &[u8]); 3] = [
        (
            Encoding::Utf8,
            b"ab\xff\xfe\r\nc\xe3\x81\nd\rend\xf0\x9f\x98\x80\n",
        ),
        (
            Encoding::ShiftJis,
            b"abc\xff\x80\r\n\xed\x40x\n\rend\x82\xa0",
        ),
        (
            Encoding::Utf16Le,
            b"a\0\x00\xd8b\0\r\0\n\0c\0\x3d\xd8\x00\xde\x01",
        ),
    ];
    for (encoding, bytes) in cases {
        let normal = saved_outputs(encoding, bytes);
        for len in 1..=9 {
            let tiny = crate::buffer::with_chunk_len(len, || saved_outputs(encoding, bytes));
            assert_eq!(tiny, normal, "{encoding:?} with chunks of {len} bytes");
        }
    }
}

// ---- opening by ranges (ADR-0020) ----

fn huge_options(from: u64) -> OpenOptions {
    OpenOptions {
        huge_file_len: Some(from),
        huge_scan: ScanOptions {
            block_size: 16,
            window_blocks: 2,
            ..ScanOptions::default()
        },
        ..OpenOptions::default()
    }
}

fn all_lines(doc: &Document) -> Vec<String> {
    doc.lines(0..doc.line_count())
        .map(|l| l.unwrap().text)
        .collect()
}

/// Waits until the scan of `file` is done and returns the complete document.
fn finish_scan(file: &mut TextFile) -> Document {
    file.take_scan()
        .expect("the scan is running")
        .finish()
        .unwrap()
}

#[test]
fn a_file_over_the_threshold_opens_by_ranges_and_reads_like_a_whole_one() {
    let dir = TempDir::new();
    let text = "first line\r\nsecond あいう\r\nthird\r\n".repeat(30);
    let path = dir.write("big.txt", text.as_bytes());
    let whole = open_file(&path, &OpenOptions::default()).unwrap();
    let mut options = huge_options(100);
    options.huge_scan.step_delay = Some(std::time::Duration::from_millis(20));
    let mut ranged = open_file(&path, &options).unwrap();
    assert_eq!(ranged.encoding, whole.encoding);
    assert_eq!(ranged.new_line, whole.new_line);
    assert!(ranged.document.is_read_only(), "the scan is still running");
    assert!(ranged.document.line_count() < whole.document.line_count());
    assert_eq!(ranged.document.line(0).unwrap().unwrap().text, "first line");
    let done = finish_scan(&mut ranged);
    assert!(!done.is_read_only());
    assert_eq!(all_lines(&done), all_lines(&whole.document));
}

#[test]
fn a_file_under_the_threshold_is_read_whole() {
    let dir = TempDir::new();
    let path = dir.write("small.txt", b"a\nb\n");
    let mut file = open_file(&path, &huge_options(1000)).unwrap();
    assert!(file.take_scan().is_none());
    assert!(!file.document.is_read_only());
    assert_eq!(file.document.line_count(), 2);
}

#[test]
fn a_small_file_scanned_at_once_comes_back_complete() {
    let dir = TempDir::new();
    let path = dir.write("quick.txt", b"a\nb\nc");
    let mut options = huge_options(1);
    options.huge_scan = ScanOptions::default();
    let mut file = open_file(&path, &options).unwrap();
    // The scan of 5 bytes is done when the open returns, or soon after; either way the text is
    // there.
    if let Some(job) = file.take_scan() {
        file.document = job.finish().unwrap();
    }
    assert!(!file.document.is_read_only());
    assert_eq!(all_lines(&file.document), ["a", "b", "c"]);
}

#[test]
fn the_bom_is_not_part_of_the_text_of_a_file_opened_by_ranges() {
    let dir = TempDir::new();
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend_from_slice("héllo\nwörld\n".repeat(20).as_bytes());
    let path = dir.write("bom.txt", &bytes);
    let mut options = huge_options(10);
    options.huge_scan.step_delay = Some(std::time::Duration::from_millis(10));
    let mut file = open_file(&path, &options).unwrap();
    assert!(file.bom && file.encoding == Encoding::Utf8);
    assert_eq!(file.document.line(0).unwrap().unwrap().text, "héllo");
    let done = finish_scan(&mut file);
    assert_eq!(done.line_count(), 40);
    assert_eq!(done.line(1).unwrap().unwrap().text, "wörld");
    // UTF-16 with a BOM.
    let mut utf16 = vec![0xFF, 0xFE];
    for unit in "ab\r\ncd\r\n".repeat(40).encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    let path = dir.write("bom16.txt", &utf16);
    let file = open_file(&path, &huge_options(10)).unwrap();
    assert_eq!(file.encoding, Encoding::Utf16Le);
    assert_eq!(file.new_line, Eol::CrLf);
}

#[test]
fn iso_2022_jp_over_the_threshold_is_read_whole() {
    let dir = TempDir::new();
    let path = dir.write("jis.txt", b"\x1b$B$\"$$\x1b(B\nabc\n");
    let mut options = huge_options(1);
    options.encoding = Some(Encoding::Jis);
    let mut file = open_file(&path, &options).unwrap();
    assert!(file.take_scan().is_none());
    assert!(!file.document.is_read_only());
    assert_eq!(file.document.line(0).unwrap().unwrap().text, "あい");
    let mut options = huge_options(1);
    options.encoding = Some(Encoding::Utf7);
    let mut file = open_file(&path, &options).unwrap();
    assert!(file.take_scan().is_none());
}

#[test]
fn an_empty_file_is_never_opened_by_ranges() {
    let dir = TempDir::new();
    let path = dir.write("empty.txt", b"");
    let mut file = open_file(&path, &huge_options(0)).unwrap();
    assert!(file.take_scan().is_none());
    assert_eq!(file.document.line_count(), 0);
}

#[test]
fn a_file_opened_by_ranges_saves_like_a_whole_one() {
    let dir = TempDir::new();
    let text = "alpha\r\nbeta あ\r\ngamma\r\n".repeat(20);
    let path = dir.write("save.txt", text.as_bytes());
    let mut options = huge_options(10);
    options.huge_scan.step_delay = Some(std::time::Duration::from_millis(5));
    let mut file = open_file(&path, &options).unwrap();
    file.document = finish_scan(&mut file);
    file.document.insert(Pos::new(1, 0), "X").unwrap();
    let out = dir.path("out.txt");
    let save = SaveOptions::keep(&file);
    file.save_as(&out, &save, &mut no_progress).unwrap();
    let expected = text.replacen("beta", "Xbeta", 1);
    assert_eq!(fs::read(&out).unwrap(), expected.as_bytes());
}

#[test]
fn a_document_that_shows_only_the_scanned_start_is_never_saved() {
    let dir = TempDir::new();
    let text = "alpha\r\nbeta\r\n".repeat(200);
    let path = dir.write("partial.txt", text.as_bytes());
    let mut options = huge_options(10);
    options.huge_scan.step_delay = Some(std::time::Duration::from_millis(50));
    let mut file = open_file(&path, &options).unwrap();
    assert!(file.document.is_read_only());
    let save = SaveOptions::keep(&file);
    let out = dir.path("out.txt");
    let error = file.save_as(&out, &save, &mut no_progress).unwrap_err();
    assert!(
        matches!(error, FileError::Buffer(BufferError::ReadOnly)),
        "{error}"
    );
    assert!(file.check_save(&save, &mut no_progress).is_err());
    assert!(!out.exists());
    drop(file.take_scan());
}

/// `MATSU_OPEN_FILE=<path> cargo test --release -p matsu-core --lib measure_open_by_ranges --
/// --ignored --nocapture`: the time of `open_file` for a big file read whole and by ranges (the
/// first lines the view draws, the scan to the end). `MATSU_OPEN_WHOLE=1` also reads it whole.
#[test]
#[ignore = "measures the file named by MATSU_OPEN_FILE"]
fn measure_open_by_ranges() {
    let Ok(path) = std::env::var("MATSU_OPEN_FILE") else {
        return;
    };
    let path = PathBuf::from(path);
    let status = |key: &str| {
        fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| s.lines().find(|l| l.starts_with(key)).map(str::to_owned))
            .unwrap_or_default()
    };
    if std::env::var("MATSU_OPEN_WHOLE").is_ok() {
        let t = Instant::now();
        let whole = open_file(&path, &OpenOptions::default()).unwrap();
        println!(
            "whole read: open_file {:?}, {}",
            t.elapsed(),
            status("VmHWM")
        );
        drop(whole);
    }
    let options = OpenOptions {
        huge_file_len: Some(1),
        ..OpenOptions::default()
    };
    let t = Instant::now();
    let mut file = open_file(&path, &options).unwrap();
    println!("by ranges: open_file {:?}", t.elapsed());
    let t = Instant::now();
    let lines: Vec<_> = file
        .document
        .lines(0..60)
        .map(|l| l.unwrap().text.len())
        .collect();
    println!(
        "the first {} lines (a screen): {:?}",
        lines.len(),
        t.elapsed()
    );
    let job = file.take_scan();
    let t = Instant::now();
    if let Some(job) = job {
        let doc = job.finish().unwrap();
        println!(
            "scan to the end: {:?}, {} lines, {}",
            t.elapsed(),
            doc.line_count(),
            status("VmHWM")
        );
        let t = Instant::now();
        let n = doc.line_count();
        let _ = doc.line(n / 2).unwrap().map(|l| l.text.len());
        println!("the middle line: {:?}", t.elapsed());
    }
}

#[test]
fn a_cancel_while_opening_by_ranges_is_honored() {
    let dir = TempDir::new();
    let path = dir.write("cancel.txt", "line\n".repeat(100).as_bytes());
    let error =
        open_file_with(&path, &huge_options(10), &mut |_, _| ControlFlow::Break(())).unwrap_err();
    assert!(matches!(error, FileError::Cancelled), "{error}");
}

#[test]
fn a_document_that_can_be_edited_never_comes_with_a_running_scan() {
    // The scan of a small file ends about when the open looks at it: whichever way it falls, a
    // complete document is not kept with a job that would put the same text over an edit.
    let dir = TempDir::new();
    let path = dir.write("race.txt", "line\n".repeat(40).as_bytes());
    let mut options = huge_options(1);
    options.huge_scan.block_size = 8;
    options.huge_scan.window_blocks = 1;
    for _ in 0..300 {
        let mut file = open_file(&path, &options).unwrap();
        let scan = file.take_scan();
        assert!(
            file.document.is_read_only() || scan.is_none(),
            "a document that can be edited came with a scan"
        );
    }
}

#[test]
fn a_crlf_at_the_edge_of_the_probe_for_the_first_line_ending_is_seen_whole() {
    let dir = TempDir::new();
    // The first line is 1 MiB - 1 bytes: its CR is the last byte of the probe.
    let mut bytes = vec![b'a'; 1024 * 1024 - 1];
    bytes.extend_from_slice(b"\r\nnext\r\n");
    let path = dir.write("edge.txt", &bytes);
    let mut options = huge_options(1);
    options.huge_scan = ScanOptions::default();
    let mut file = open_file(&path, &options).unwrap();
    assert_eq!(file.new_line, Eol::CrLf);
    drop(file.take_scan());
}
