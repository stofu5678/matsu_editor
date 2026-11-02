use std::borrow::Cow;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::buffer::{Document, Eol, MemorySource};
use crate::encoding::Encoding;

fn encode(enc: Encoding, text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    enc.codec().encode(text, &mut out).unwrap();
    out
}

/// A source that counts what is read from it.
#[derive(Debug)]
struct Counting {
    inner: MemorySource,
    total: AtomicU64,
    largest: AtomicU64,
    reads: AtomicU64,
}

impl Counting {
    fn new(bytes: &[u8]) -> Arc<Self> {
        Arc::new(Self {
            inner: MemorySource::new(bytes.to_vec()),
            total: AtomicU64::new(0),
            largest: AtomicU64::new(0),
            reads: AtomicU64::new(0),
        })
    }
}

impl Source for Counting {
    fn len(&self) -> u64 {
        self.inner.len()
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        self.total.fetch_add(end - start, Ordering::Relaxed);
        self.largest.fetch_max(end - start, Ordering::Relaxed);
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.read(start, end)
    }
}

fn reader(source: Arc<dyn Source>, enc: Encoding, ext_eol: bool, chunk: usize) -> LineReader {
    LineReader::new(source, Arc::new(EncodingCodec(enc)), ext_eol, chunk)
}

fn read_all(reader: &mut LineReader) -> Vec<Line> {
    let mut lines = Vec::new();
    while let Some(line) = reader.next_line().unwrap() {
        lines.push(line);
    }
    lines
}

/// The lines the document makes of the same bytes.
fn document_lines(bytes: &[u8], enc: Encoding, ext_eol: bool) -> Vec<Line> {
    let doc = Document::with_ext_eol(
        MemorySource::new(bytes.to_vec()),
        Arc::new(EncodingCodec(enc)),
        ext_eol,
    )
    .unwrap();
    doc.lines(0..doc.line_count())
        .map(|line| line.unwrap())
        .collect()
}

/// Checks the reader against the document over every chunk size from 4 up to past the length.
fn same_as_document(bytes: &[u8], enc: Encoding, ext_eol: bool) {
    let want = document_lines(bytes, enc, ext_eol);
    for chunk in (4..=bytes.len() + 8).step_by(4) {
        let source = Counting::new(bytes);
        let mut r = reader(source.clone(), enc, ext_eol, chunk);
        let got = read_all(&mut r);
        assert_eq!(
            got, want,
            "{enc:?} chunk {chunk} ext_eol {ext_eol} bytes {bytes:02x?}"
        );
        assert_eq!(r.bytes_read(), bytes.len() as u64);
        assert_eq!(
            source.total.load(Ordering::Relaxed),
            bytes.len() as u64,
            "every byte is read once ({enc:?} chunk {chunk})"
        );
        assert!(source.largest.load(Ordering::Relaxed) <= chunk as u64);
    }
}

const TEXTS: &[&str] = &[
    "",
    "one line",
    "a\nb\nc\n",
    "a\r\nb\r\nc",
    "a\rb\r\n\r\n\nc\r",
    "cr at the end\r",
    "\r\n\r\n\r\n",
    "line\u{85}nel\u{2028}ls\u{2029}ps\nend",
    "𠮷野家 😀 and 🎉\r\nsurrogate pairs\r\n𠮷\n",
];

#[test]
fn utf8_lines_are_the_lines_of_the_document() {
    for text in TEXTS {
        let bytes = encode(Encoding::Utf8, text);
        same_as_document(&bytes, Encoding::Utf8, false);
        same_as_document(&bytes, Encoding::Utf8, true);
    }
}

#[test]
fn utf16_lines_are_the_lines_of_the_document() {
    // A surrogate pair, and 0x0A in the high byte of a little-endian unit (U+0A00).
    for text in TEXTS.iter().copied().chain(["\u{0A00}x\r\n\u{0A0D}y\n"]) {
        for enc in [Encoding::Utf16Le, Encoding::Utf16Be] {
            let bytes = encode(enc, text);
            same_as_document(&bytes, enc, false);
            same_as_document(&bytes, enc, true);
        }
    }
}

#[test]
fn shift_jis_double_bytes_are_not_cut_by_the_chunk_edge() {
    // The second byte of ソ (83 5C), 表 (95 5C) and 能 (94 5C) is 0x5C, and that of 　 and the
    // letters below is in 0x40..=0x7E: no chunk edge may make the reader start in the middle.
    let text = "ソ表能\r\n表現ソ\nカタカナ漢字\r\nＡＢＣ\r\n\\ソ";
    let bytes = encode(Encoding::ShiftJis, text);
    assert!(bytes.windows(2).any(|w| w == [0x83, 0x5C]));
    same_as_document(&bytes, Encoding::ShiftJis, false);
}

#[test]
fn euc_jp_lines_are_the_lines_of_the_document() {
    let bytes = encode(Encoding::EucJp, "日本語\r\nテキスト\nｱｲｳ\r\n");
    same_as_document(&bytes, Encoding::EucJp, false);
}

#[test]
fn a_crlf_at_the_edge_of_a_chunk_is_one_ending() {
    // Chunks of 4 bytes cut "ab\r|\ncd" between the CR and the LF in UTF-8, and
    // "a\r|\nb" between the units in UTF-16.
    let mut r = reader(Counting::new(b"abc\r\ndef"), Encoding::Utf8, false, 4);
    let lines = read_all(&mut r);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].eol, Some(Eol::CrLf));
    assert_eq!(lines[0].text, "abc");
    assert_eq!(lines[1].text, "def");
    let bytes = encode(Encoding::Utf16Le, "ab\r\ncd");
    let mut r = reader(Counting::new(&bytes), Encoding::Utf16Le, false, 8);
    let lines = read_all(&mut r);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].eol, Some(Eol::CrLf));
}

#[test]
fn the_last_line_without_an_ending_is_a_line_and_an_empty_rest_is_not() {
    let mut r = reader(Counting::new(b"a\nb"), Encoding::Utf8, false, 4);
    assert_eq!(read_all(&mut r).len(), 2);
    let mut r = reader(Counting::new(b"a\nb\n"), Encoding::Utf8, false, 4);
    assert_eq!(read_all(&mut r).len(), 2);
    let mut r = reader(Counting::new(b""), Encoding::Utf8, false, 4);
    assert!(r.next_line().unwrap().is_none());
}

#[test]
fn the_reader_holds_one_chunk_and_the_unfinished_line() {
    // 2000 lines of 40 bytes: with chunks of 1 KiB the reader never holds much more than that.
    let line = format!("{}\n", "x".repeat(39));
    let bytes = line.repeat(2000).into_bytes();
    let source = Counting::new(&bytes);
    let mut r = reader(source.clone(), Encoding::Utf8, false, 1024);
    assert_eq!(read_all(&mut r).len(), 2000);
    assert!(
        r.peak_buffered() <= 1024 + 40 + LOOKAHEAD,
        "peak {}",
        r.peak_buffered()
    );
    assert_eq!(source.total.load(Ordering::Relaxed), bytes.len() as u64);
    assert!(source.largest.load(Ordering::Relaxed) <= 1024);
}

#[test]
fn a_long_line_is_found_in_linear_time() {
    // One line of 3 MiB with 4 KiB chunks: the ending is looked for only in the new bytes.
    let mut bytes = vec![b'x'; 3 * 1024 * 1024];
    bytes.extend_from_slice(b"\nend");
    let source = Counting::new(&bytes);
    let mut r = reader(source.clone(), Encoding::Utf8, false, 4096);
    let lines = read_all(&mut r);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text.len(), 3 * 1024 * 1024);
    assert_eq!(source.total.load(Ordering::Relaxed), bytes.len() as u64);
}

/// A source made of numbered lines, `line 000000000042\n`, which nothing holds in memory.
#[derive(Debug)]
struct Numbered {
    lines: u64,
    reads: AtomicU64,
    bytes: AtomicU64,
    largest: AtomicU64,
}

const NUMBERED_LINE: u64 = 18;

impl Source for Numbered {
    fn len(&self) -> u64 {
        self.lines * NUMBERED_LINE
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(end - start, Ordering::Relaxed);
        self.largest.fetch_max(end - start, Ordering::Relaxed);
        let mut out = Vec::with_capacity((end - start) as usize);
        for at in start..end {
            let (n, k) = (at / NUMBERED_LINE, at % NUMBERED_LINE);
            let text = format!("line {n:012}\n");
            out.push(text.as_bytes()[k as usize]);
        }
        Ok(Cow::Owned(out))
    }
}

#[test]
fn a_file_over_4_gib_is_read_in_chunks_of_the_given_size() {
    // 300 million lines of 18 bytes would be 5.4 GB; the reader is stopped after a few lines
    // from the start, and then from a position past 4 GiB.
    let source = Arc::new(Numbered {
        lines: 300_000_000,
        reads: AtomicU64::new(0),
        bytes: AtomicU64::new(0),
        largest: AtomicU64::new(0),
    });
    assert!(source.len() > 4 * 1024 * 1024 * 1024);
    let mut r = reader(source.clone(), Encoding::Utf8, false, 4096);
    for n in 0..1000u64 {
        let line = r.next_line().unwrap().unwrap();
        assert_eq!(line.text, format!("line {n:012}"));
    }
    assert!(source.bytes.load(Ordering::Relaxed) <= 4096 * 6);
    assert_eq!(source.largest.load(Ordering::Relaxed), 4096);
    assert_eq!(r.total(), 300_000_000 * NUMBERED_LINE);
}

fn write_file(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn huge(from: u64) -> OpenOptions {
    OpenOptions {
        huge_file_len: Some(from),
        ..OpenOptions::default()
    }
}

fn stream_lines(stream: &mut LineStream) -> Vec<Line> {
    let mut lines = Vec::new();
    while let Some(line) = stream.next_line().unwrap() {
        lines.push(line);
    }
    lines
}

#[test]
fn a_file_is_streamed_from_the_threshold_and_not_below_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(&dir, "a.txt", b"one\r\ntwo\r\nthree");
    assert!(
        open_line_stream(&path, &OpenOptions::default(), 8)
            .unwrap()
            .is_none()
    );
    assert!(open_line_stream(&path, &huge(16), 8).unwrap().is_none());
    let mut stream = open_line_stream(&path, &huge(15), 8).unwrap().unwrap();
    assert_eq!(stream.encoding, Encoding::Utf8);
    let lines = stream_lines(&mut stream);
    assert_eq!(
        lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["one", "two", "three"]
    );
    assert_eq!(lines[2].eol, None);
    assert_eq!(stream.reader().bytes_read(), 15);
}

#[test]
fn the_bom_is_not_part_of_the_first_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice("日本語\nabc\n".as_bytes());
    let path = write_file(&dir, "bom.txt", &bytes);
    let mut stream = open_line_stream(&path, &huge(1), 4).unwrap().unwrap();
    assert!(stream.bom);
    assert_eq!(stream_lines(&mut stream)[0].text, "日本語");

    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(encode(Encoding::Utf16Le, "ab\r\ncd\r\n"));
    let path = write_file(&dir, "utf16.txt", &bytes);
    let mut stream = open_line_stream(&path, &huge(1), 4).unwrap().unwrap();
    assert_eq!(stream.encoding, Encoding::Utf16Le);
    let lines = stream_lines(&mut stream);
    assert_eq!(lines[0].text, "ab");
    assert_eq!(lines[1].text, "cd");
}

#[test]
fn a_given_encoding_is_used_and_a_stateful_one_is_left_to_the_whole_read() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = encode(Encoding::ShiftJis, "ソース\r\n表\r\n");
    let path = write_file(&dir, "sjis.txt", &bytes);
    let mut options = huge(1);
    options.encoding = Some(Encoding::ShiftJis);
    let mut stream = open_line_stream(&path, &options, 4).unwrap().unwrap();
    let lines = stream_lines(&mut stream);
    assert_eq!(lines[0].text, "ソース");
    assert_eq!(lines[1].text, "表");

    let mut options = huge(1);
    options.encoding = Some(Encoding::Jis);
    assert!(open_line_stream(&path, &options, 4).unwrap().is_none());
    // an empty file is opened whole
    let empty = write_file(&dir, "empty.txt", b"");
    assert!(open_line_stream(&empty, &huge(0), 4).unwrap().is_none());
}

#[test]
fn a_file_that_shrinks_while_it_is_read_is_an_error_not_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    // more than one cache block of the ranged file, so that a later read goes to the disk
    let bytes = "0123456789\n".repeat(60_000).into_bytes();
    let path = write_file(&dir, "shrink.txt", &bytes);
    let mut stream = open_line_stream(&path, &huge(1), 64).unwrap().unwrap();
    assert!(stream.next_line().unwrap().is_some());
    std::fs::write(&path, b"short").unwrap();
    let mut failed = false;
    while let Some(result) = stream.next_line().transpose() {
        if result.is_err() {
            failed = true;
            break;
        }
    }
    assert!(failed);
}
