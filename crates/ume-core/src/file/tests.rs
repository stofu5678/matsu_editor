//! Tests of opening and saving. They use real files in a private temporary folder.

use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use super::*;
use crate::buffer::Pos;
use crate::encoding::Certainty;

// ---- helpers ----

/// A folder that is removed at the end of the test.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ume-file-test-{}-{n}", std::process::id()));
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
    let options = OpenOptions {
        encoding: Some(encoding),
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
        max_len: 5,
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
    options.max_len = 10;
    assert!(open_file(&path, &options).is_ok());
}

#[cfg(unix)]
#[test]
fn a_sparse_file_over_2_gib_is_refused() {
    // `set_len` makes a sparse file on Unix file systems, so this does not write 2 GiB. If the
    // size check did not come first, the open would try to read 2 GiB.
    let dir = TempDir::new();
    let path = dir.path("big.txt");
    let file = fs::File::create(&path).unwrap();
    file.set_len(2 * 1024 * 1024 * 1024 + 1).unwrap();
    drop(file);
    let err = open_file(&path, &OpenOptions::default()).unwrap_err();
    assert!(
        matches!(err, FileError::TooLarge { len, max } if len == 2_147_483_649 && max == 2_147_483_648),
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
    // U+0085 is a line ending in UTF-8 but has no Shift_JIS form.
    let dir = TempDir::new();
    let path = dir.write("a.txt", "ab\u{85}cd".as_bytes());
    let file = open_as(&path, Encoding::Utf8);
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

/// Run with `cargo test -p ume-core --release -- --ignored --nocapture file::tests::large`.
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
