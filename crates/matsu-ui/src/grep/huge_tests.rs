//! Tests of the Grep of huge files (ADR-0020, chapter 5, and 8.3): a file of `huge_file_len`
//! bytes or more is read in order, a chunk at a time, without an index. The chunk is made small
//! here, so that small files cross its edges in every place that matters: the CR LF of a line
//! ending, a UTF-16 surrogate pair, a Shift_JIS double byte. What is found must be what the whole
//! read finds, and the bytes held at once must stay near the size of the chunk.

use std::borrow::Cow;
use std::io;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use matsu_core::buffer::{
    BlockScanner, Document, EncodingCodec, MemorySource, ScanOptions, Source, scan_blocks,
};
use matsu_core::encoding::Encoding;

use super::engine::stream_probe::{self, Seen};
use super::engine::{GrepInfo, Outcome, Sink, WindowSource, run};
use super::format::{Layout, LineType};
use super::window::window_token;
use crate::search::state::SearchOptions;

struct Text(String);

impl Sink for Text {
    fn write(&mut self, text: &str) {
        self.0.push_str(text);
    }
}

fn info(dir: &tempfile::TempDir, key: &str) -> GrepInfo {
    GrepInfo {
        key: key.to_owned(),
        file: "*.*".to_owned(),
        folder: dir.path().to_string_lossy().into_owned(),
        options: SearchOptions::default(),
        sub_folder: false,
        charset: None,
        layout: Layout::default(),
        file_only: false,
        except_file_regex: false,
        replace: None,
        mime_decode: false,
        huge_file_len: None,
        window: None,
        window_gone: false,
    }
}

/// Runs the search with reads of `chunk` bytes: what it wrote, how it ended and what the files
/// that were read in order showed.
fn grep(info: &GrepInfo, chunk: usize) -> (String, Outcome, Seen) {
    let mut sink = Text(String::new());
    let (outcome, seen) = stream_probe::with_chunk(chunk, || {
        run(info, &AtomicBool::new(false), &mut sink).expect("the search starts")
    });
    (sink.0, outcome, seen)
}

/// The text after the heading.
fn body(text: &str) -> &str {
    text.split_once("\r\n\r\n\r\n").expect("a heading").1
}

fn encode(enc: Encoding, text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    enc.codec().encode(text, &mut out).unwrap();
    out
}

/// Checks that the search in order finds what the whole read finds, for chunks from 4 bytes up.
fn same_as_the_whole_read(bytes: &[u8], key: &str, charset: Option<Encoding>, layout: Layout) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("data.txt"), bytes).unwrap();
    let mut info = info(&dir, key);
    info.charset = charset;
    info.layout = layout;
    let (whole, whole_outcome, seen) = grep(&info, 4);
    assert_eq!(seen.files, 0, "no threshold: the file is read whole");
    assert!(whole_outcome.hits > 0, "the test finds something: {whole}");
    info.huge_file_len = Some(1);
    // the BOM is not part of the text that is read
    let bom = if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        2
    } else {
        0
    };
    for chunk in (4..=bytes.len() + 8).step_by(4) {
        let (text, outcome, seen) = grep(&info, chunk);
        assert_eq!(seen.files, 1, "chunk {chunk}: the file is read in order");
        assert_eq!(seen.bytes, (bytes.len() - bom) as u64, "chunk {chunk}");
        assert_eq!(outcome, whole_outcome, "chunk {chunk}");
        assert_eq!(body(&text), body(&whole), "chunk {chunk}");
    }
}

#[test]
fn utf8_with_crlf_across_the_edge_of_a_chunk() {
    let text = "first line\r\nsecond hit\r\n\r\nthird\rfourth hit\nlast hit";
    same_as_the_whole_read(text.as_bytes(), "hit", None, Layout::default());
}

#[test]
fn utf8_with_a_final_line_ending_has_no_empty_last_line() {
    // The lines that do not match are written: an empty line after the last ending would be one.
    let mut layout = Layout::default();
    layout.line_type = LineType::NoHitLine;
    same_as_the_whole_read(b"a\nhit\nb\n", "hit", None, layout);
}

#[test]
fn utf16_with_surrogate_pairs_across_the_edge_of_a_chunk() {
    let text = "𠮷野家 😀 hit\r\n🎉 and 😀 again\r\nplain\r\n";
    for enc in [Encoding::Utf16Le, Encoding::Utf16Be] {
        let mut bytes = enc.bom().to_vec();
        bytes.extend(encode(enc, text));
        same_as_the_whole_read(&bytes, "😀", None, Layout::default());
    }
}

#[test]
fn shift_jis_double_bytes_across_the_edge_of_a_chunk() {
    // The second byte of ソ, 表 and 能 is 0x5C (a backslash in ASCII); that of ｱ-like kana and of
    // 　 is in 0x40..=0x7E. A chunk that starts or ends in the middle must not take them for text.
    let text = "ソース表能\r\n表のソース\r\n\\ソ\r\n能力ソース\r\n";
    let bytes = encode(Encoding::ShiftJis, text);
    same_as_the_whole_read(
        &bytes,
        "ソース",
        Some(Encoding::ShiftJis),
        Layout::default(),
    );
}

#[test]
fn a_regular_expression_matches_within_the_line_as_in_the_whole_read() {
    let dir = tempfile::tempdir().unwrap();
    let text = "key=1\r\nkey=22\r\nno\r\nkey=333\r\n";
    std::fs::write(dir.path().join("data.txt"), text).unwrap();
    let mut info = info(&dir, r"key=(\d+)$");
    info.options.regex = true;
    let (whole, _, _) = grep(&info, 4);
    info.huge_file_len = Some(1);
    for chunk in [4, 8, 12, 16, 64] {
        let (text, _, seen) = grep(&info, chunk);
        assert_eq!(seen.files, 1);
        assert_eq!(body(&text), body(&whole), "chunk {chunk}");
    }
}

#[test]
fn a_big_file_is_read_in_chunks_and_never_whole() {
    // About 3 MiB of 100 byte lines, 64 KiB chunks. The reader may hold a chunk and one line.
    let dir = tempfile::tempdir().unwrap();
    let mut text = String::new();
    for n in 0..30_000 {
        text.push_str(&format!("{n:08} {}\r\n", "x".repeat(80)));
        if n % 7_000 == 3 {
            text.push_str("a needle in the line\r\n");
        }
    }
    std::fs::write(dir.path().join("big.txt"), &text).unwrap();
    let mut info = info(&dir, "needle");
    info.huge_file_len = Some(1024 * 1024);
    let chunk = 64 * 1024;
    let (out, outcome, seen) = grep(&info, chunk);
    assert_eq!(outcome.hits, 5, "{}", body(&out));
    assert_eq!(seen.files, 1);
    assert_eq!(seen.bytes, text.len() as u64);
    assert!(
        seen.peak <= chunk + 128,
        "the reader held {} bytes for chunks of {chunk}",
        seen.peak
    );
    // the line numbers are counted while reading
    assert!(body(&out).contains("big.txt(7006,3)"), "{}", body(&out));
}

#[test]
fn a_file_below_the_threshold_is_read_whole() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hit\n").unwrap();
    let mut info = info(&dir, "hit");
    info.huge_file_len = Some(5);
    let (_, outcome, seen) = grep(&info, 4);
    assert_eq!((outcome.hits, seen.files), (1, 0));
    info.huge_file_len = Some(4);
    let (_, outcome, seen) = grep(&info, 4);
    assert_eq!((outcome.hits, seen.files), (1, 1));
}

#[test]
fn a_file_with_a_state_in_its_encoding_is_read_whole() {
    // ISO-2022-JP cannot be cut anywhere: the whole read converts it.
    let dir = tempfile::tempdir().unwrap();
    let bytes = encode(Encoding::Jis, "日本語のhit\r\nつぎ\r\n");
    std::fs::write(dir.path().join("jis.txt"), bytes).unwrap();
    let mut info = info(&dir, "hit");
    info.charset = Some(Encoding::Jis);
    info.huge_file_len = Some(1);
    let (out, outcome, seen) = grep(&info, 4);
    assert_eq!((outcome.hits, seen.files), (1, 0), "{}", body(&out));
}

#[test]
fn the_search_stops_reading_at_the_first_hit_of_a_file_when_asked() {
    let dir = tempfile::tempdir().unwrap();
    let text = format!("hit\r\n{}", "filler line of text\r\n".repeat(50_000));
    std::fs::write(dir.path().join("a.txt"), &text).unwrap();
    let mut info = info(&dir, "hit");
    info.file_only = true;
    info.huge_file_len = Some(1);
    let (_, outcome, seen) = grep(&info, 4096);
    assert_eq!(outcome.hits, 1);
    assert!(
        seen.bytes <= 8192,
        "read {} of {} bytes",
        seen.bytes,
        text.len()
    );
}

#[test]
fn a_stop_is_seen_while_a_big_file_is_read() {
    struct Stop<'a>(&'a AtomicBool);
    impl Sink for Stop<'_> {
        fn write(&mut self, _: &str) {
            self.0.store(true, Ordering::Relaxed);
        }
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hit\r\n".repeat(100_000)).unwrap();
    let mut info = info(&dir, "hit");
    info.huge_file_len = Some(1);
    let cancel = AtomicBool::new(false);
    let mut sink = Stop(&cancel);
    let (outcome, seen) = stream_probe::with_chunk(4096, || {
        run(&info, &cancel, &mut sink).expect("the search starts")
    });
    // the heading is written first and sets the flag: nothing of the file is read far
    assert!(outcome.stopped);
    assert!(seen.bytes < 500_000);
}

// ---- the text of a window (`:HWND:`) ----

/// A source that counts what is read from it.
#[derive(Debug)]
struct Counting {
    inner: MemorySource,
    total: AtomicU64,
    largest: AtomicU64,
}

impl Counting {
    fn new(bytes: &[u8]) -> Arc<Self> {
        Arc::new(Self {
            inner: MemorySource::new(bytes.to_vec()),
            total: AtomicU64::new(0),
            largest: AtomicU64::new(0),
        })
    }

    fn reset(&self) {
        self.total.store(0, Ordering::Relaxed);
        self.largest.store(0, Ordering::Relaxed);
    }
}

impl Source for Counting {
    fn len(&self) -> u64 {
        self.inner.len()
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        self.total.fetch_add(end - start, Ordering::Relaxed);
        self.largest.fetch_max(end - start, Ordering::Relaxed);
        self.inner.read(start, end)
    }
}

fn window_info(key: &str, window: WindowSource) -> GrepInfo {
    GrepInfo {
        key: key.to_owned(),
        file: window_token(7),
        folder: "/no/such/folder".to_owned(),
        options: SearchOptions::default(),
        sub_folder: true,
        charset: None,
        layout: Layout::default(),
        file_only: false,
        except_file_regex: false,
        replace: None,
        mime_decode: false,
        huge_file_len: Some(1),
        window: Some(window),
        window_gone: false,
    }
}

fn window(
    document: Document,
    name: &str,
    scanning_file: Option<std::path::PathBuf>,
) -> WindowSource {
    WindowSource {
        document,
        encoding: Encoding::Utf8,
        folder: String::new(),
        name: name.to_owned(),
        title: name.to_owned(),
        scanning_file,
    }
}

fn scan_options() -> ScanOptions {
    ScanOptions {
        block_size: 4096,
        window_blocks: 4,
        ..ScanOptions::default()
    }
}

fn text_of_lines(count: usize) -> String {
    let mut text = String::new();
    for n in 0..count {
        text.push_str(&format!(
            "{n:06} some text on the line {}\n",
            "x".repeat(40)
        ));
        if n % 1000 == 7 {
            text.push_str("a needle here\n");
        }
    }
    text
}

#[test]
fn a_window_of_a_huge_file_is_searched_by_its_block_index_not_read_whole() {
    let text = text_of_lines(10_000);
    let source = Counting::new(text.as_bytes());
    let index = scan_blocks(
        source.clone(),
        Arc::new(EncodingCodec(Encoding::Utf8)),
        scan_options(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    let document = Document::from_block_index(index).unwrap();
    assert!(document.is_ranged() && !document.is_read_only());
    source.reset();
    let info = window_info("needle", window(document, ":HWND:[7]big", None));
    let mut sink = Text(String::new());
    let outcome = run(&info, &AtomicBool::new(false), &mut sink).unwrap();
    assert_eq!(outcome.hits, 10, "{}", body(&sink.0));
    // no read of the document was bigger than a few blocks, and the bytes read are a few times
    // the size of the file at most (a block is read again for the lines near its edges)
    assert!(
        source.largest.load(Ordering::Relaxed) <= 16 * 4096,
        "largest read {}",
        source.largest.load(Ordering::Relaxed)
    );
    assert!(
        source.total.load(Ordering::Relaxed) <= 4 * text.len() as u64,
        "read {} of {}",
        source.total.load(Ordering::Relaxed),
        text.len()
    );
}

#[test]
fn a_window_still_being_scanned_is_searched_in_the_file_not_in_its_snapshot() {
    let text = text_of_lines(3_000);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.txt");
    std::fs::write(&path, &text).unwrap();
    // The snapshot has only the first window of the scan.
    let source: Arc<dyn Source> = Arc::new(MemorySource::new(text.as_bytes().to_vec()));
    let mut scanner = BlockScanner::new(
        source,
        Arc::new(EncodingCodec(Encoding::Utf8)),
        scan_options(),
    )
    .unwrap();
    scanner.step().unwrap();
    assert!(!scanner.is_done());
    let snapshot = Document::from_block_index(scanner.shared().lock().unwrap().clone()).unwrap();
    assert!(snapshot.is_read_only());

    let run_for = |scanning_file| {
        let info = window_info(
            "needle",
            window(snapshot.clone(), ":HWND:[7]big", scanning_file),
        );
        let mut sink = Text(String::new());
        stream_probe::with_chunk(8192, || {
            let outcome = run(&info, &AtomicBool::new(false), &mut sink).unwrap();
            (outcome.hits, sink.0.clone())
        })
    };
    let ((partial, _), seen) = run_for(None);
    assert_eq!(seen.files, 0);
    assert!(partial < 3, "the snapshot has only the start: {partial}");
    let ((all, out), seen) = run_for(Some(path));
    assert_eq!(seen.files, 1);
    assert_eq!(all, 3, "{}", body(&out));
    assert!(seen.peak <= 8192 + 128);
}

// ---- the measurement of the PR ----

/// The peak resident memory of this process so far, in MB (Linux), or 0.
fn peak_rss_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|l| l.strip_prefix("VmHWM:"))
                .and_then(|l| l.split_whitespace().next()?.parse::<u64>().ok())
        })
        .map_or(0, |kb| kb / 1024)
}

/// Not a test: the time and the memory of a Grep of a big file, for the PR. Run it in a release
/// build with the file in `MATSU_GREP_MEASURE_FILE`:
/// `cargo test -p matsu-ui --release --lib measure_a_big_file -- --ignored --nocapture`.
#[test]
#[ignore = "measures the file in MATSU_GREP_MEASURE_FILE; run in a release build"]
fn measure_a_big_file() {
    let Ok(path) = std::env::var("MATSU_GREP_MEASURE_FILE") else {
        return;
    };
    let key = std::env::var("MATSU_GREP_MEASURE_KEY").unwrap_or_else(|_| "needle".to_owned());
    let path = std::path::PathBuf::from(path);
    let len = std::fs::metadata(&path).unwrap().len();
    let mb = len as f64 / (1024.0 * 1024.0);
    println!("file {} MB, memory before {} MB", mb as u64, peak_rss_mb());

    // the file, read in order
    let dir = tempfile::tempdir().unwrap();
    let mut info = info(&dir, &key);
    info.folder = path.parent().unwrap().to_string_lossy().into_owned();
    info.file = path.file_name().unwrap().to_string_lossy().into_owned();
    info.huge_file_len = Some(256 * 1024 * 1024);
    let started = std::time::Instant::now();
    let mut sink = Text(String::new());
    let outcome = run(&info, &AtomicBool::new(false), &mut sink).unwrap();
    let secs = started.elapsed().as_secs_f64();
    println!(
        "grep in order: {} hits, {secs:.2} s, {:.0} MB/s, peak memory {} MB",
        outcome.hits,
        mb / secs,
        peak_rss_mb()
    );

    // the window of the same file, once its scan is done: the lines come from the block index
    let options = matsu_core::file::OpenOptions {
        huge_file_len: Some(256 * 1024 * 1024),
        ..Default::default()
    };
    let started = std::time::Instant::now();
    let mut opened = matsu_core::file::open_file(&path, &options).unwrap();
    let document = match opened.take_scan() {
        Some(job) => job.finish().unwrap(),
        None => opened.document.clone(),
    };
    println!(
        "open and scan: {:.2} s, {} lines, peak memory {} MB",
        started.elapsed().as_secs_f64(),
        document.line_count(),
        peak_rss_mb()
    );
    let info = window_info(&key, window(document, ":HWND:[7]big", None));
    let started = std::time::Instant::now();
    let mut sink = Text(String::new());
    let outcome = run(&info, &AtomicBool::new(false), &mut sink).unwrap();
    let secs = started.elapsed().as_secs_f64();
    println!(
        "grep of the window: {} hits, {secs:.2} s, {:.0} MB/s, peak memory {} MB",
        outcome.hits,
        mb / secs,
        peak_rss_mb()
    );
}
