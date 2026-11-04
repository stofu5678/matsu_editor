//! Windows on a file the scan has not reached (`window.rs`), over a source that makes its bytes
//! from the number of the line: a file of 10 GB without a file on the disk (ADR-0020, 8.3).

use std::borrow::Cow;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::*;
use crate::buffer::{Document, EncodingCodec, Pos};
use crate::encoding::Encoding;

/// A file of `lines` lines that are all `width` bytes long; line `i` is `make(i)`.
#[derive(Debug)]
struct Fake {
    lines: u64,
    width: u64,
    make: fn(u64) -> Vec<u8>,
    /// How many bytes have been read.
    read_bytes: Arc<AtomicU64>,
}

impl Source for Fake {
    fn len(&self) -> u64 {
        self.lines * self.width
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        assert!(start <= end && end <= self.len(), "{start}..{end}");
        self.read_bytes.fetch_add(end - start, Ordering::Relaxed);
        let mut out = Vec::with_capacity((end - start) as usize);
        let mut i = start / self.width;
        while (out.len() as u64) < end - start {
            let line = (self.make)(i);
            assert_eq!(line.len() as u64, self.width);
            let from = start.saturating_sub(i * self.width) as usize;
            let want = (end - start) as usize - out.len();
            out.extend_from_slice(&line[from..(from + want).min(line.len())]);
            i += 1;
        }
        Ok(Cow::Owned(out))
    }
}

fn lf(i: u64) -> Vec<u8> {
    format!("line {i:011}\n").into_bytes()
}

fn crlf(i: u64) -> Vec<u8> {
    format!("line {i:011}\r\n").into_bytes()
}

fn utf16(i: u64) -> Vec<u8> {
    format!("line {i:011}\r\n")
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// One line of 101 bytes, with characters of two bytes (Shift_JIS).
fn sjis(i: u64) -> Vec<u8> {
    let mut v = format!("line {i:011} xx").into_bytes();
    // 5C is a trail byte of Shift_JIS ("ソ" is 83 5C), which looks like a backslash.
    for _ in 0..40 {
        v.extend_from_slice(&[0x83, 0x5C]);
    }
    v.extend_from_slice(b"\r\n");
    v
}

const TEN_GB: u64 = 10 * 1000 * 1000 * 1000;

fn fake(width: u64, make: fn(u64) -> Vec<u8>) -> (Arc<Fake>, Arc<AtomicU64>) {
    let read_bytes = Arc::new(AtomicU64::new(0));
    let source = Arc::new(Fake {
        lines: TEN_GB / width,
        width,
        make,
        read_bytes: read_bytes.clone(),
    });
    (source, read_bytes)
}

/// A scan of the fake file that stays in its first window: it sleeps long after the first read.
fn slow_job(source: Arc<Fake>, enc: Encoding) -> ScanJob {
    let options = ScanOptions {
        window_blocks: 4,
        step_delay: Some(Duration::from_secs(600)),
        ..ScanOptions::default()
    };
    let job = ScanJob::spawn(source, Arc::new(EncodingCodec(enc)), options).unwrap();
    // The first window is in the index when the thread sleeps.
    for _ in 0..3000 {
        if job.progress().scanned > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(job.progress().scanned > 0);
    job
}

fn first_line(window: &ScanWindow) -> String {
    window.document.line(0).unwrap().unwrap().text
}

#[test]
fn a_window_far_down_a_file_of_10_gb_reads_only_its_stretch() {
    let (source, read) = fake(17, lf);
    let job = slow_job(source.clone(), Encoding::Utf8);
    let scanned = job.progress().scanned;
    assert!(scanned < 8 * 1024 * 1024);
    let before = read.load(Ordering::Relaxed);
    let target = 7_000_000_000;
    let window = job.open_window(target, WINDOW_BYTES).unwrap().unwrap();
    let read_now = read.load(Ordering::Relaxed) - before;
    // The stretch, the search for its start and the check of its end: a few times the window.
    assert!(read_now < 4 * WINDOW_BYTES as u64, "{read_now} bytes read");
    // From line start to line end, around the target.
    assert_eq!(window.start % 17, 0);
    assert_eq!(window.end % 17, 0);
    assert!(window.start <= target && target < window.end, "{window:?}");
    assert!(window.end - window.start <= WINDOW_BYTES as u64);
    assert!(window.end - window.start > WINDOW_BYTES as u64 - 17);
    let first = window.start / 17;
    assert_eq!(first_line(&window), format!("line {first:011}"));
    assert_eq!(
        window.document.line_count() as u64,
        (window.end - window.start) / 17
    );
    assert!(window.document.is_read_only());
    // The lines before are estimated from the part scanned (every line is as long as the others,
    // so the estimate is right to a few lines).
    assert!(!window.exact);
    assert!(
        window.lines_before.abs_diff(first) < first / 10_000,
        "{window:?}"
    );
}

#[test]
fn a_window_in_the_scanned_part_counts_its_lines_exactly() {
    let (source, _) = fake(17, lf);
    let job = slow_job(source, Encoding::Utf8);
    let scanned = job.progress().scanned;
    let window = job.open_window(scanned / 2, WINDOW_BYTES).unwrap().unwrap();
    assert!(window.exact);
    assert_eq!(window.lines_before, window.start / 17);
    // The scan has passed a place: the number of its line is exact for the job too.
    assert_eq!(job.exact_lines_before(1700), Some(100));
    assert_eq!(job.exact_lines_before(scanned + 1), None);
    assert_eq!(job.line_start_byte(100), Some(1700));
    assert_eq!(job.line_start_byte(0), Some(0));
    assert_eq!(job.line_start_byte(TEN_GB), None);
}

#[test]
fn a_window_at_the_start_and_at_the_end_of_the_file() {
    let (source, _) = fake(17, lf);
    let len = source.len();
    let job = slow_job(source, Encoding::Utf8);
    let top = job.open_window(0, WINDOW_BYTES).unwrap().unwrap();
    assert_eq!((top.start, top.lines_before, top.exact), (0, 0, true));
    let bottom = job.open_window(len, WINDOW_BYTES).unwrap().unwrap();
    assert_eq!(bottom.end, len);
    assert!(bottom.start > len - WINDOW_BYTES as u64 - 17);
    // The file ends with a line ending: the last line is the one before the empty end.
    let last = len / 17 - 1;
    let count = bottom.document.line_count();
    assert_eq!(
        bottom.document.line(count - 1).unwrap().unwrap().text,
        format!("line {last:011}")
    );
    // A target past the end is the end.
    let past = job.open_window(len + 5, WINDOW_BYTES).unwrap().unwrap();
    assert_eq!(past.end, len);
}

#[test]
fn a_window_never_starts_in_the_middle_of_a_line_or_a_character() {
    type Make = fn(u64) -> Vec<u8>;
    let cases: [(Encoding, u64, Make); 3] = [
        (Encoding::Utf8, 18, crlf),
        (Encoding::Utf16Le, 36, utf16),
        (Encoding::ShiftJis, 101, sjis),
    ];
    for (enc, width, make) in cases {
        let (source, _) = fake(width, make);
        let job = slow_job(source, enc);
        // Targets at every offset in a line, so the edges fall everywhere in the line endings
        // and the two-byte characters.
        for k in 0..width {
            let target = 3_000_000_000 + k;
            let window = job.open_window(target, 64 * 1024).unwrap().unwrap();
            let what = format!("{enc:?} target {target}: {window:?}");
            assert_eq!(window.start % width, 0, "{what}");
            assert_eq!(window.end % width, 0, "{what}");
            let line = window.start / width;
            assert!(
                first_line(&window).starts_with(&format!("line {line:011}")),
                "{what}"
            );
            // Every line of the window is whole: none has the stand-in of a cut character.
            let n = window.document.line_count();
            let last = window.document.line(n - 1).unwrap().unwrap();
            assert!(last.text.starts_with("line "), "{what}");
            assert!(!last.text.contains('\u{FFFD}'), "{what}");
        }
    }
}

#[test]
fn a_line_longer_than_the_window_gives_no_window() {
    #[derive(Debug)]
    struct LongLine(Vec<u8>);
    impl Source for LongLine {
        fn len(&self) -> u64 {
            self.0.len() as u64
        }
        fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
            Ok(Cow::Borrowed(&self.0[start as usize..end as usize]))
        }
    }
    // One line of 3 MiB, then a short one: no line start is near the middle of the long line.
    let mut bytes = vec![b'x'; 3 * 1024 * 1024];
    bytes.push(b'\n');
    bytes.extend_from_slice(b"end\n");
    let options = ScanOptions {
        window_blocks: 1,
        step_delay: Some(Duration::from_secs(600)),
        ..ScanOptions::default()
    };
    let job = ScanJob::spawn(
        Arc::new(LongLine(bytes)),
        Arc::new(EncodingCodec(Encoding::Utf8)),
        options,
    )
    .unwrap();
    assert!(job.open_window(1_500_000, WINDOW_BYTES).unwrap().is_none());
    // From the start the stretch would be the long line itself.
    assert!(job.open_window(0, 64 * 1024).unwrap().is_none());
}

#[test]
fn the_position_of_a_byte_in_a_window_and_back() {
    let (source, _) = fake(17, lf);
    let job = slow_job(source, Encoding::Utf8);
    let window = job
        .open_window(5_000_000_016 + 5, WINDOW_BYTES)
        .unwrap()
        .unwrap();
    let doc: &Document = &window.document;
    let offset = (5_000_000_016 + 5 - window.start) as usize;
    let pos = doc.offset_to_pos(offset).unwrap();
    assert_eq!(pos.col, 5);
    assert_eq!(doc.pos_to_offset(pos).unwrap(), offset);
    assert_eq!(doc.pos_to_offset(Pos::new(0, 0)).unwrap(), 0);
    assert_eq!(
        doc.offset_to_pos(usize::MAX).unwrap(),
        doc.end_pos().unwrap()
    );
}

#[test]
fn no_window_is_made_when_a_long_line_crosses_the_target_and_the_edge_of_the_window() {
    #[derive(Debug)]
    struct Bytes(Vec<u8>);
    impl Source for Bytes {
        fn len(&self) -> u64 {
            self.0.len() as u64
        }
        fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
            Ok(Cow::Borrowed(&self.0[start as usize..end as usize]))
        }
    }
    // Short lines, then a line of 200 KiB, then short lines again: a window of 64 KiB around the
    // middle of the long line would end before the target (at the last short line).
    let mut bytes = b"ab\n".repeat(40_000);
    let long_start = bytes.len();
    bytes.extend(std::iter::repeat_n(b'x', 200 * 1024));
    bytes.push(b'\n');
    bytes.extend_from_slice(&b"cd\n".repeat(40_000));
    let options = ScanOptions {
        window_blocks: 1,
        step_delay: Some(Duration::from_secs(600)),
        ..ScanOptions::default()
    };
    let job = ScanJob::spawn(
        Arc::new(Bytes(bytes)),
        Arc::new(EncodingCodec(Encoding::Utf8)),
        options,
    )
    .unwrap();
    let target = (long_start + 100 * 1024) as u64;
    assert!(job.open_window(target, 64 * 1024).unwrap().is_none());
    // The very start of the long line: the window would end where the line begins, with none of
    // the line in it (a jump there would show the empty end instead of the line).
    assert!(
        job.open_window(long_start as u64, 64 * 1024)
            .unwrap()
            .is_none()
    );
    // Short lines before the long line have a window.
    assert!(
        job.open_window(long_start as u64 - 1000, 64 * 1024)
            .unwrap()
            .is_some()
    );
}
