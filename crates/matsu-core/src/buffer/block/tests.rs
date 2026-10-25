use std::borrow::Cow;
use std::io;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::*;
use crate::buffer::storage::{EolIndex, LineIndex, MemorySource};
use crate::buffer::{BufferError, Document, EncodingCodec, Pos};
use crate::encoding::Encoding;
use crate::file::{RangedFile, RangedOptions, is_changed_outside};

fn codec(enc: Encoding) -> Arc<dyn Codec> {
    Arc::new(EncodingCodec(enc))
}

fn source(bytes: &[u8]) -> Arc<dyn Source> {
    Arc::new(MemorySource::new(bytes.to_vec()))
}

fn encode(enc: Encoding, text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    enc.codec().encode(text, &mut out).unwrap();
    out
}

fn options(block_size: usize, window_blocks: usize, ext_eol: bool) -> ScanOptions {
    ScanOptions {
        block_size,
        window_blocks,
        ext_eol,
        ..ScanOptions::default()
    }
}

fn scan(bytes: &[u8], enc: Encoding, o: ScanOptions) -> BlockIndex {
    scan_blocks(source(bytes), codec(enc), o, |_| ControlFlow::Continue(())).unwrap()
}

/// Checks the block index of `bytes` against the dense index, over every range, and the shape of
/// the blocks. Returns the block index.
fn check_same_as_dense(bytes: &[u8], enc: Encoding, o: ScanOptions) -> BlockIndex {
    let blocks = scan(bytes, enc, o);
    let mut dense = LineIndex::default();
    dense.scan(enc.codec(), bytes, 0, o.ext_eol).unwrap();
    let what = format!(
        "{enc:?} block {} window {} ext_eol {} bytes {bytes:02x?}",
        o.block_size, o.window_blocks, o.ext_eol
    );
    assert_eq!(blocks.covered_len(), bytes.len() as u64, "{what}");
    assert!(blocks.is_complete(), "{what}");
    assert_eq!(blocks.break_count(), dense.len() as u64, "{what}");
    assert_eq!(blocks.len(), dense.len(), "{what}");
    for j in 0..dense.len() {
        let (a, b) = (blocks.entry(j).unwrap(), dense.entry(j).unwrap());
        assert_eq!(
            (a.start, a.len, a.kind),
            (b.start, b.len, b.kind),
            "{what} entry {j}"
        );
    }
    for s in 0..=bytes.len() {
        assert_eq!(
            blocks.count_before(s).unwrap(),
            dense.count_before(s).unwrap(),
            "{what} count_before {s}"
        );
        for e in s..=bytes.len() {
            assert_eq!(
                blocks.agg(s, e).unwrap(),
                dense.agg(s, e).unwrap(),
                "{what} agg {s}..{e}"
            );
        }
    }
    let whole = (0, bytes.len());
    for n in 0..=dense.len() {
        let (a, b) = (
            blocks.nth_view(whole.0, whole.1, n).unwrap(),
            dense.nth_view(whole.0, whole.1, n).unwrap(),
        );
        let key = |v: Option<crate::buffer::storage::View>| v.map(|v| (v.off, v.len, v.kind));
        assert_eq!(key(a), key(b), "{what} nth_view {n}");
    }
    // The blocks cover the file without a gap, none is empty, and none cuts a line ending.
    let mut end = 0;
    for b in 0..blocks.block_count() {
        let (s, e) = blocks.block_range(b).unwrap();
        assert_eq!(s, end, "{what} block {b}");
        assert!(e > s, "{what} block {b} is empty");
        end = e;
        for j in 0..dense.len() {
            let x = dense.entry(j).unwrap();
            let (xs, xe) = (x.start as u64, (x.start + x.len) as u64);
            assert!(
                !(xs < s && s < xe),
                "{what}: block {b} starts inside a line ending"
            );
        }
    }
    assert_eq!(end, bytes.len() as u64, "{what}");
    blocks
}

fn sweep(bytes: &[u8], enc: Encoding) {
    for ext_eol in [false, true] {
        for block_size in [1, 2, 3, 4, 5, 7, 8, 16, 1000] {
            for window_blocks in [1, 3] {
                check_same_as_dense(bytes, enc, options(block_size, window_blocks, ext_eol));
            }
        }
    }
}

/// A deterministic pseudo-random sequence.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % n
    }
}

#[test]
fn a_crlf_is_never_cut_by_a_block_edge() {
    sweep(b"ab\r\ncd\r\nef\r\n", Encoding::Utf8);
    sweep(b"\r\n\r\n\r\n\r\n", Encoding::Utf8);
    sweep(b"a\rb\r\r\nc\n\r\n", Encoding::Utf8);
    // With the block size 4 the first edge falls between the CR and the LF: it moves after the LF.
    let index = check_same_as_dense(b"abc\r\ndef", Encoding::Utf8, options(4, 2, true));
    assert_eq!(index.block_range(0), Some((0, 5)));
}

#[test]
fn a_multi_byte_character_is_never_cut_by_a_block_edge() {
    let text = "あいう\nえおかきく\nabc\u{1F600}d\nさしす";
    sweep(text.as_bytes(), Encoding::Utf8);
    let index = check_same_as_dense(text.as_bytes(), Encoding::Utf8, options(4, 1, false));
    // The first edge would fall inside the 2nd character (3 bytes each): it moves to its end.
    assert_eq!(index.block_range(0), Some((0, 6)));
}

#[test]
fn nel_ls_and_ps_end_lines_only_with_ext_eol() {
    let text = "a\u{85}b\u{2028}c\u{2029}d\r\ne";
    sweep(text.as_bytes(), Encoding::Utf8);
    let off = scan(text.as_bytes(), Encoding::Utf8, options(3, 2, false));
    let on = scan(text.as_bytes(), Encoding::Utf8, options(3, 2, true));
    assert_eq!(off.break_count(), 1);
    assert_eq!(on.break_count(), 4);
}

#[test]
fn a_line_over_many_blocks_is_one_line() {
    let mut text = "x".repeat(1000);
    text.push('\n');
    text.push_str(&"あ".repeat(300));
    let index = check_same_as_dense(text.as_bytes(), Encoding::Utf8, options(7, 3, false));
    assert_eq!(index.break_count(), 1);
    assert!(index.block_count() > 100);
    // The blocks of the long line break_count nothing and are cut at character boundaries.
    for b in 0..index.block_count() {
        let (s, _) = index.block_range(b).unwrap();
        if s > 1001 {
            assert_eq!((s - 1001) % 3, 0, "block {b} starts inside a character");
        }
    }
    let doc = Document::from_block_index(index).unwrap();
    assert_eq!(doc.line_count(), 2);
    assert_eq!(doc.line(0).unwrap().unwrap().text.len(), 1000);
    assert_eq!(doc.line(1).unwrap().unwrap().text, "あ".repeat(300));
}

#[test]
fn shift_jis_lead_and_trail_bytes_are_walked_from_a_known_boundary() {
    // 0x83 0x40 ("ァ") has a trail byte that looks like '@'; 0x95 0x5C ("表") one that looks like
    // a backslash.
    let bytes = [
        0x83, 0x40, 0x83, 0x40, 0x83, 0x40, b'\r', b'\n', 0x95, 0x5C, 0x95, 0x5C, 0x95, 0x5C, 0x95,
        0x5C, b'\n', 0x83, 0x40, 0x83, 0x40, 0x83, 0x40, 0x83, 0x40,
    ];
    sweep(&bytes, Encoding::ShiftJis);
    let text = "ソ表能申\n暴予ー\r\nアイウｱｲｳ\n十\\";
    sweep(&encode(Encoding::ShiftJis, text), Encoding::ShiftJis);
    // Edges of 5 bytes: [83 40 83 40 83] must end after the lead byte's trail.
    let index = check_same_as_dense(&bytes, Encoding::ShiftJis, options(5, 1, false));
    assert_eq!(index.block_range(0), Some((0, 6)));
    assert_eq!(index.char_boundary_at_or_before(3).unwrap(), 2);
    assert_eq!(index.char_boundary_at_or_before(9).unwrap(), 8);
    assert_eq!(index.char_boundary_at_or_before(5).unwrap(), 4);
    assert_eq!(index.char_boundary_at_or_before(0).unwrap(), 0);
    assert_eq!(
        index.char_boundary_at_or_before(100).unwrap(),
        bytes.len() as u64
    );
}

#[test]
fn a_boundary_in_a_long_shift_jis_line_comes_from_the_block() {
    // One line of 2-byte characters whose trail bytes are 0x40..0x7E: no ASCII-looking byte can
    // be told from a trail byte from the middle, only from the start of a block.
    let line: Vec<u8> = (0..500).flat_map(|_| [0x83, 0x40]).collect();
    let index = check_same_as_dense(&line, Encoding::ShiftJis, options(33, 4, false));
    for pos in 0..line.len() as u64 {
        assert_eq!(
            index.char_boundary_at_or_before(pos).unwrap(),
            pos & !1,
            "pos {pos}"
        );
    }
}

#[test]
fn euc_jp_characters_are_not_cut() {
    let text = "日本語のテキスト\nＥＵＣ\r\nｱｲｳ\n";
    sweep(&encode(Encoding::EucJp, text), Encoding::EucJp);
}

#[test]
fn utf16_units_surrogates_and_crlf_are_not_cut() {
    let text = "ab\u{1F600}c\r\nd\u{1F600}\u{1F600}\ne\u{85}f\u{2028}g\u{2029}\r\r\n\u{1F600}";
    for enc in [Encoding::Utf16Le, Encoding::Utf16Be] {
        let bytes = encode(enc, text);
        sweep(&bytes, enc);
    }
    // A UTF-16 LE 0x0A00 unit ("\u{0A00}") has the byte 0x0A, which is not a line ending.
    let bytes = encode(Encoding::Utf16Le, "a\u{0A00}b\n\u{0D00}c");
    assert!(bytes.contains(&0x0A));
    let index = check_same_as_dense(&bytes, Encoding::Utf16Le, options(3, 2, true));
    assert_eq!(index.break_count(), 1);
    // An edge never falls inside a surrogate pair.
    let bytes = encode(Encoding::Utf16Le, "\u{1F600}\u{1F600}\u{1F600}");
    let index = check_same_as_dense(&bytes, Encoding::Utf16Le, options(1, 1, false));
    assert_eq!(index.block_count(), 3);
}

#[test]
fn utf32_is_scanned_too() {
    let text = "a\u{1F600}\r\nb\nc";
    for enc in [Encoding::Utf32Le, Encoding::Utf32Be] {
        sweep(&encode(enc, text), enc);
    }
}

#[test]
fn an_odd_tail_and_invalid_bytes_do_not_break_the_scan() {
    let mut bytes = encode(Encoding::Utf16Le, "ab\ncd");
    bytes.push(0x41);
    sweep(&bytes, Encoding::Utf16Le);
    // A lone high surrogate at the end and a lone Shift_JIS lead byte before a line ending.
    sweep(&[0x00, 0xD8, 0x0A, 0x00, 0x00, 0xD8], Encoding::Utf16Le);
    sweep(&[0x81, b'\n', 0x81, b'\r', b'\n', 0x81], Encoding::ShiftJis);
    sweep(
        &[0xE3, b'\n', 0xE3, 0x81, b'\r', 0xFF, 0xFF],
        Encoding::Utf8,
    );
}

#[test]
fn empty_files_and_files_without_a_final_line_ending() {
    let index = scan(b"", Encoding::Utf8, options(4, 1, false));
    assert_eq!(index.block_count(), 0);
    assert!(index.is_complete());
    let doc = Document::from_block_index(index).unwrap();
    assert_eq!(doc.line_count(), 0);
    assert_eq!(doc.row_count(), 1);
    sweep(b"no ending", Encoding::Utf8);
    sweep(b"one\nlast", Encoding::Utf8);
    sweep(b"\n", Encoding::Utf8);
    sweep(b"\r", Encoding::Utf8);
    sweep(b"x", Encoding::Utf8);
}

#[test]
fn random_files_match_the_dense_index() {
    let alphabets: [(Encoding, &[&[u8]]); 4] = [
        (
            Encoding::Utf8,
            &[
                b"a",
                b"\n",
                b"\r",
                b"\r\n",
                "あ".as_bytes(),
                "\u{85}".as_bytes(),
                "\u{2028}".as_bytes(),
                b"\xFF",
                "😀".as_bytes(),
            ],
        ),
        (
            Encoding::ShiftJis,
            &[
                b"a",
                b"\n",
                b"\r",
                b"\r\n",
                &[0x83, 0x40],
                &[0x95, 0x5C],
                &[0xB1],
                &[0x81],
            ],
        ),
        (
            Encoding::EucJp,
            &[
                b"a",
                b"\n",
                b"\r",
                b"\r\n",
                &[0xA4, 0xA2],
                &[0x8E, 0xB1],
                &[0x8F],
            ],
        ),
        (
            Encoding::Utf16Le,
            &[
                b"a\0",
                b"\n\0",
                b"\r\0",
                b"\r\0\n\0",
                &[0x3D, 0xD8, 0x00, 0xDE],
                &[0x00, 0xD8],
                &[0x85, 0x00],
                &[0x28, 0x20],
            ],
        ),
    ];
    let mut rng = Lcg(7);
    for (enc, alphabet) in alphabets {
        for _ in 0..40 {
            let mut bytes = Vec::new();
            for _ in 0..rng.next(40) {
                bytes.extend_from_slice(alphabet[rng.next(alphabet.len())]);
            }
            for ext_eol in [false, true] {
                for block_size in [1, 2, 3, 5, 6, 11] {
                    check_same_as_dense(&bytes, enc, options(block_size, 1 + rng.next(3), ext_eol));
                }
            }
        }
    }
}

#[test]
fn stateful_encodings_are_refused() {
    for enc in [Encoding::Jis, Encoding::Utf7] {
        let r = scan_blocks(source(b"abc"), codec(enc), ScanOptions::default(), |_| {
            ControlFlow::Continue(())
        });
        assert!(matches!(r, Err(ScanError::Stateful)), "{enc:?}");
        assert!(matches!(
            ScanJob::spawn(source(b"abc"), codec(enc), ScanOptions::default()),
            Err(ScanError::Stateful)
        ));
    }
}

#[test]
fn the_index_costs_16_bytes_per_block() {
    // The worst case for a dense index: nothing but line endings (5 bytes each).
    let bytes = vec![b'\n'; 1 << 20];
    let index = scan(&bytes, Encoding::Utf8, options(4096, 16, false));
    assert_eq!(index.break_count(), 1 << 20);
    assert_eq!(index.block_count(), 256);
    assert!(index.heap_bytes() <= 256 * 16 * 2);
    let mut dense = LineIndex::default();
    dense
        .scan(Encoding::Utf8.codec(), &bytes, 0, false)
        .unwrap();
    assert!(index.heap_bytes() * 100 < dense.heap_bytes());
    // Reading around keeps only a few tables.
    let doc = Document::from_block_index(index.clone()).unwrap();
    for i in [0, 1_000_000, 500_000, 7, 1_048_575] {
        assert_eq!(doc.line(i).unwrap().unwrap().text, "");
    }
    assert!(index.cached_table_bytes() < 32 * 4096 * 6);
}

fn lines_of(doc: &Document) -> Vec<(String, Option<crate::buffer::Eol>)> {
    doc.lines(0..doc.line_count())
        .map(|l| l.unwrap())
        .map(|l| (l.text, l.eol))
        .collect()
}

fn assert_docs_equal(a: &Document, b: &Document, what: &str) {
    assert_eq!(a.line_count(), b.line_count(), "{what}");
    assert_eq!(a.row_count(), b.row_count(), "{what}");
    assert_eq!(a.raw_len(), b.raw_len(), "{what}");
    assert_eq!(a.end_pos().unwrap(), b.end_pos().unwrap(), "{what}");
    assert_eq!(lines_of(a), lines_of(b), "{what}");
    for i in 0..a.line_count() {
        assert_eq!(
            a.line_len(i).unwrap(),
            b.line_len(i).unwrap(),
            "{what} line {i}"
        );
    }
    let end = a.end_pos().unwrap();
    assert_eq!(
        a.text_between(Pos { line: 0, col: 0 }, end).unwrap(),
        b.text_between(Pos { line: 0, col: 0 }, end).unwrap(),
        "{what}"
    );
}

fn block_doc(bytes: &[u8], enc: Encoding, o: ScanOptions) -> Document {
    Document::from_block_index(scan(bytes, enc, o)).unwrap()
}

#[test]
fn a_block_document_reads_and_edits_like_a_dense_one() {
    let text = "first line\r\nsecond あいう\nthird\r\n\r\nfifth \u{1F600} line without end";
    for enc in [
        Encoding::Utf8,
        Encoding::ShiftJis,
        Encoding::Utf16Le,
        Encoding::EucJp,
    ] {
        let bytes = if enc == Encoding::Utf8 {
            text.as_bytes().to_vec()
        } else {
            encode(enc, &text.replace('\u{1F600}', ""))
        };
        for block_size in [1, 3, 8, 64] {
            let o = options(block_size, 2, false);
            let c = codec(enc);
            let mut dense = Document::new(MemorySource::new(bytes.clone()), c).unwrap();
            let mut blocks = block_doc(&bytes, enc, o);
            let what = format!("{enc:?} block {block_size}");
            assert_docs_equal(&dense, &blocks, &what);
            let edits: [(Pos, Pos, &str); 6] = [
                (Pos { line: 1, col: 3 }, Pos { line: 1, col: 3 }, "XY\r\nZ"),
                (Pos { line: 0, col: 2 }, Pos { line: 3, col: 0 }, ""),
                (Pos { line: 0, col: 0 }, Pos { line: 0, col: 0 }, "\n"),
                (Pos { line: 2, col: 1 }, Pos { line: 2, col: 2 }, "\r"),
                (Pos { line: 0, col: 0 }, Pos { line: 99, col: 0 }, "tail\r"),
                (Pos { line: 1, col: 0 }, Pos { line: 1, col: 0 }, "\nz"),
            ];
            for (n, (from, to, ins)) in edits.into_iter().enumerate() {
                let (d, b) = (dense.replace(from, to, ins), blocks.replace(from, to, ins));
                assert_eq!(d.unwrap(), b.unwrap(), "{what} edit {n}");
                assert_docs_equal(&dense, &blocks, &format!("{what} edit {n}"));
                blocks.check_invariants();
            }
        }
    }
}

#[test]
fn saving_chunks_of_a_block_document_are_the_original_bytes() {
    let bytes: Vec<u8> = "abc\r\ndef\nあいう\n".repeat(50).into_bytes();
    let doc = block_doc(&bytes, Encoding::Utf8, options(16, 2, false));
    let mut out = Vec::new();
    for chunk in doc.chunks_of(10) {
        match chunk.unwrap() {
            crate::buffer::Chunk::Original(b) => out.extend_from_slice(&b),
            crate::buffer::Chunk::Added(_) => panic!("nothing was added"),
        }
    }
    assert_eq!(out, bytes);
}

#[test]
fn progress_reaches_100_percent_and_can_cancel() {
    let bytes = vec![b'a'; 10_000];
    let mut seen = Vec::new();
    let o = options(100, 5, false);
    scan_blocks(source(&bytes), codec(Encoding::Utf8), o, |p| {
        seen.push(p);
        ControlFlow::Continue(())
    })
    .unwrap();
    assert!(seen.windows(2).all(|w| w[0].scanned < w[1].scanned));
    let last = seen.last().unwrap();
    assert_eq!(
        (last.scanned, last.total, last.percent()),
        (10_000, 10_000, 100)
    );
    assert_eq!(last.state, ScanState::Done);
    assert!(seen.iter().any(|p| (1..100).contains(&p.percent())));
    assert!(
        seen[..seen.len() - 1]
            .iter()
            .all(|p| p.state == ScanState::Running)
    );

    let mut calls = 0;
    let r = scan_blocks(source(&bytes), codec(Encoding::Utf8), o, |_| {
        calls += 1;
        if calls == 3 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    });
    assert!(matches!(r, Err(ScanError::Cancelled)));
    assert_eq!(calls, 3);
    // A window of any size is fine: it is the whole file.
    let index = scan(&bytes, Encoding::Utf8, options(100, usize::MAX, false));
    assert_eq!(index.covered_len(), 10_000);
    // An empty file is 100% at once.
    let p = BlockScanner::new(source(b""), codec(Encoding::Utf8), o)
        .unwrap()
        .progress();
    assert_eq!((p.percent(), p.state), (100, ScanState::Done));
}

/// A source that blocks the scan thread until the test lets it go on.
#[derive(Debug)]
struct Gate {
    inner: MemorySource,
    reads: AtomicUsize,
    /// Reads after this many wait until `open` is set.
    allow: AtomicUsize,
    open: Mutex<bool>,
    cv: std::sync::Condvar,
}

impl Gate {
    fn new(bytes: Vec<u8>, allow: usize) -> Arc<Self> {
        Arc::new(Self {
            inner: MemorySource::new(bytes),
            reads: AtomicUsize::new(0),
            allow: AtomicUsize::new(allow),
            open: Mutex::new(false),
            cv: std::sync::Condvar::new(),
        })
    }

    fn open(&self) {
        *self.open.lock().unwrap() = true;
        self.cv.notify_all();
    }

    fn wait_for_reads(&self, n: usize) {
        while self.reads.load(Ordering::Acquire) < n {
            std::thread::yield_now();
        }
    }
}

impl Source for Gate {
    fn len(&self) -> u64 {
        self.inner.len()
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        // Only the scan thread waits: the test reads the document from its own thread.
        if std::thread::current().name() == Some("matsu-block-scan") {
            let n = self.reads.fetch_add(1, Ordering::AcqRel);
            if n >= self.allow.load(Ordering::Acquire) {
                let mut open = self.open.lock().unwrap();
                while !*open {
                    open = self.cv.wait(open).unwrap();
                }
            }
        }
        self.inner.read(start, end)
    }
}

#[test]
fn a_job_shows_the_scanned_start_read_only_and_then_the_whole_file() {
    let text = "line\r\n".repeat(2000);
    let gate = Gate::new(text.clone().into_bytes(), 3);
    let o = options(64, 4, false);
    let job = ScanJob::spawn(gate.clone(), codec(Encoding::Utf8), o).unwrap();
    gate.wait_for_reads(4);
    // The thread waits in its 4th read; three windows are scanned.
    let p = job.progress();
    assert_eq!(p.state, ScanState::Running);
    assert!(p.scanned > 0 && p.scanned < p.total);
    assert!((1..100).contains(&p.percent()));
    assert!(!job.is_finished());

    let mut doc = job.snapshot().unwrap();
    assert!(doc.is_read_only());
    assert!(doc.line_count() > 10 && doc.line_count() < 2000);
    assert_eq!(doc.line(0).unwrap().unwrap().text, "line");
    let err = doc.insert(Pos { line: 0, col: 0 }, "x").unwrap_err();
    assert!(matches!(err, BufferError::ReadOnly));
    assert_eq!(doc.line(0).unwrap().unwrap().text, "line");

    gate.allow.store(usize::MAX, Ordering::Release);
    gate.open();
    let mut doc = job.finish().unwrap();
    assert!(!doc.is_read_only());
    assert_eq!(doc.line_count(), 2000);
    doc.insert(Pos { line: 5, col: 2 }, "XX").unwrap();
    assert_eq!(doc.line(5).unwrap().unwrap().text, "liXXne");
}

#[test]
fn a_job_can_be_cancelled() {
    let gate = Gate::new(vec![b'a'; 100_000], 1);
    let job = ScanJob::spawn(gate.clone(), codec(Encoding::Utf8), options(100, 2, false)).unwrap();
    gate.wait_for_reads(2);
    job.cancel();
    gate.allow.store(usize::MAX, Ordering::Release);
    gate.open();
    let progress = loop {
        let p = job.progress();
        if p.state != ScanState::Running {
            break p;
        }
        std::thread::yield_now();
    };
    assert_eq!(progress.state, ScanState::Cancelled);
    assert!(progress.scanned < progress.total);
    assert!(job.failure().is_none());
    assert!(matches!(job.finish(), Err(ScanError::Cancelled)));
}

#[test]
fn a_job_over_an_empty_file_is_done() {
    let job = ScanJob::spawn(source(b""), codec(Encoding::Utf8), ScanOptions::default()).unwrap();
    let doc = job.finish().unwrap();
    assert!(!doc.is_read_only());
    assert_eq!(doc.line_count(), 0);
}

fn ranged(bytes: &[u8]) -> (tempfile::NamedTempFile, Arc<RangedFile>) {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(bytes).unwrap();
    file.flush().unwrap();
    let opts = RangedOptions {
        block_size: 32,
        cache_blocks: 4,
    };
    let ranged = RangedFile::open_with(file.path(), opts).unwrap();
    (file, Arc::new(ranged))
}

#[test]
fn a_ranged_file_is_scanned_and_read_back() {
    let text = "row one\r\nrow two あ\nrow three\r\n".repeat(40);
    let (_file, source) = ranged(text.as_bytes());
    let o = options(50, 3, false);
    let index = scan_blocks(source, codec(Encoding::Utf8), o, |_| {
        ControlFlow::Continue(())
    })
    .unwrap();
    let doc = Document::from_block_index(index).unwrap();
    let dense = Document::from_utf8(text.into_bytes()).unwrap();
    assert_docs_equal(&dense, &doc, "ranged");
}

#[test]
fn a_file_that_shrinks_during_the_scan_is_reported() {
    let text = "0123456789\n".repeat(100);
    let (file, source) = ranged(text.as_bytes());
    let o = options(40, 2, false);
    let mut steps = 0;
    let r = scan_blocks(source, codec(Encoding::Utf8), o, |_| {
        steps += 1;
        if steps == 2 {
            file.as_file().set_len(100).unwrap();
        }
        ControlFlow::Continue(())
    });
    match r {
        Err(ScanError::Io(e)) => assert!(is_changed_outside(&e), "{e}"),
        other => panic!("expected the changed-outside error, got {other:?}"),
    }
}

#[test]
fn a_job_reports_a_file_that_shrinks() {
    let text = "0123456789\n".repeat(100);
    let (file, source) = ranged(text.as_bytes());
    file.as_file().set_len(50).unwrap();
    let job = ScanJob::spawn(source, codec(Encoding::Utf8), options(40, 2, false)).unwrap();
    while !job.is_finished() {
        std::thread::yield_now();
    }
    let p = job.progress();
    assert_eq!(p.state, ScanState::Failed);
    let failure = job.failure().unwrap();
    assert!(failure.changed_outside, "{}", failure.message);
    assert!(matches!(job.finish(), Err(ScanError::Io(e)) if is_changed_outside(&e)));
}

#[test]
fn a_file_that_shrinks_after_the_scan_fails_the_reads() {
    let text = "0123456789\n".repeat(100);
    let (file, source) = ranged(text.as_bytes());
    let index = scan_blocks(source, codec(Encoding::Utf8), options(40, 2, false), |_| {
        ControlFlow::Continue(())
    })
    .unwrap();
    let doc = Document::from_block_index(index).unwrap();
    file.as_file().set_len(10).unwrap();
    // A block that was never read fails with the changed-outside error, not a crash.
    match doc.line(90) {
        Err(BufferError::Io(e)) => assert!(is_changed_outside(&e), "{e}"),
        other => panic!("expected an I/O error, got {other:?}"),
    }
    // `line_count` does not fail even then; `try_line_count` reports it.
    assert!(doc.line_count() >= 99);
    assert!(doc.try_line_count().is_err() || doc.try_line_count().unwrap() == 100);
}

/// A source whose bytes can be replaced under the index.
#[derive(Debug)]
struct Changing(Mutex<Vec<u8>>);

impl Source for Changing {
    fn len(&self) -> u64 {
        self.0.lock().unwrap().len() as u64
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        let data = self.0.lock().unwrap();
        data.get(start as usize..end as usize)
            .map(|b| Cow::Owned(b.to_vec()))
            .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))
    }
}

#[test]
fn a_block_that_changed_at_the_same_length_is_caught_by_its_line_count() {
    let src = Arc::new(Changing(Mutex::new(b"a\nb\nc\nd\ne\nf\ng\nh\n".to_vec())));
    let index = scan_blocks(
        src.clone(),
        codec(Encoding::Utf8),
        options(6, 1, false),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    let doc = Document::from_block_index(index).unwrap();
    assert_eq!(doc.line(1).unwrap().unwrap().text, "b");
    *src.0.lock().unwrap() = b"abcdefghijklmnop".to_vec();
    // The first block's table is cached; the later ones are read and do not match.
    match doc.line(6) {
        Err(BufferError::Io(e)) => assert_eq!(e.kind(), io::ErrorKind::InvalidData),
        other => panic!("expected an error, got {other:?}"),
    }
}

#[test]
fn a_short_read_is_an_error() {
    #[derive(Debug)]
    struct Short(MemorySource);
    impl Source for Short {
        fn len(&self) -> u64 {
            self.0.len()
        }
        fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
            self.0.read(start, end.saturating_sub(1).max(start))
        }
    }
    let r = scan_blocks(
        Arc::new(Short(MemorySource::new(b"abc\ndef".to_vec()))),
        codec(Encoding::Utf8),
        options(3, 1, false),
        |_| ControlFlow::Continue(()),
    );
    assert!(matches!(r, Err(ScanError::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof));
}

#[test]
fn a_partial_index_covers_the_start_and_counts_exactly() {
    let text = "ab\r\ncd\r\nef\r\ngh\r\n".repeat(20);
    let mut scanner = BlockScanner::new(
        source(text.as_bytes()),
        codec(Encoding::Utf8),
        options(10, 2, false),
    )
    .unwrap();
    scanner.step().unwrap();
    let snapshot = scanner.shared().lock().unwrap().clone();
    assert!(!snapshot.is_complete());
    let covered = snapshot.covered_len() as usize;
    assert!(covered > 0 && covered < text.len());
    // The scanned part ends at a block edge, so its count is that of the bytes before it.
    let expected = text.as_bytes()[..covered]
        .windows(2)
        .filter(|w| w == b"\r\n")
        .count();
    assert_eq!(snapshot.break_count(), expected as u64);
    let doc = Document::from_block_index(snapshot).unwrap();
    assert!(doc.is_read_only());
    assert_eq!(doc.raw_len(), covered);
    scanner.step().unwrap();
    assert!(scanner.progress().scanned > covered as u64);
}

/// Measures the scan of a real file (ADR-0020, chapter 8.1): `MATSU_SCAN_FILE=<path> cargo test
/// --release -p matsu-core --lib measure_scan_of_a_file -- --ignored --nocapture`. `MATSU_SCAN_ENC`
/// is `utf8` (default), `sjis` or `utf16le`.
#[test]
#[ignore = "measures the file named by MATSU_SCAN_FILE"]
fn measure_scan_of_a_file() {
    use std::time::Instant;

    fn status(key: &str) -> String {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with(key))
                    .map(|l| l.trim().to_owned())
            })
            .unwrap_or_default()
    }

    let Ok(path) = std::env::var("MATSU_SCAN_FILE") else {
        return;
    };
    let enc = match std::env::var("MATSU_SCAN_ENC").as_deref() {
        Ok("sjis") => Encoding::ShiftJis,
        Ok("utf16le") => Encoding::Utf16Le,
        _ => Encoding::Utf8,
    };
    if std::env::var("MATSU_SCAN_BASELINE").is_ok() {
        // The dense index over the same bytes, for comparison.
        let bytes = std::fs::read(&path).unwrap();
        let t = Instant::now();
        let mut dense = LineIndex::default();
        dense.scan(enc.codec(), &bytes, 0, false).unwrap();
        println!(
            "dense scan in memory: {:?}, {} breaks",
            t.elapsed(),
            dense.len()
        );
    }
    let source = Arc::new(RangedFile::open(&path).unwrap());
    let len = source.len();
    let started = Instant::now();
    let job = ScanJob::spawn(source, codec(enc), ScanOptions::default()).unwrap();
    // Until the first lines can be read.
    while job.progress().scanned == 0 && !job.is_finished() {
        std::thread::yield_now();
    }
    let doc = job.snapshot().unwrap();
    let first: Vec<_> = doc.lines(0..40).map(|l| l.unwrap().text.len()).collect();
    println!(
        "first {} lines readable after {:?}",
        first.len(),
        started.elapsed()
    );
    let mut last = Instant::now();
    while !job.is_finished() {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if last.elapsed().as_secs() >= 5 {
            last = Instant::now();
            let p = job.progress();
            println!(
                "  {}% ({} breaks) {}",
                p.percent(),
                p.breaks,
                status("VmRSS")
            );
        }
    }
    let scan_time = started.elapsed();
    let doc = job.finish().unwrap();
    println!(
        "scan of {} MiB: {:?} = {:.0} MiB/s; {} breaks, index {} bytes",
        len >> 20,
        scan_time,
        (len >> 20) as f64 / scan_time.as_secs_f64(),
        doc.row_count() - 1,
        doc.index_heap_bytes()
    );
    println!("{} {}", status("VmRSS"), status("VmHWM"));
    // Jumps: the last lines and a line in the middle.
    let t = Instant::now();
    let n = doc.line_count();
    let tail = doc.line(n - 1).unwrap().map(|l| l.text.len());
    let mid = doc.line(n / 2).unwrap().map(|l| l.text.len());
    println!(
        "line_count + last + middle line: {:?} ({tail:?}, {mid:?})",
        t.elapsed()
    );
}
