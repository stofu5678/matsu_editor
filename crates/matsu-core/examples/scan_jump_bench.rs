//! Measures a move to the end (and the middle) of a huge file while the scan is still running
//! (ADR-0020, chapter 2: 「走査していない場所への移動」): the time to make the window on the file
//! there and to read the last screen from it, and the memory of the process.
//!
//! It makes a test file in a temporary folder, opens it by ranges as the editor does, starts the
//! scan, jumps while the scan is running, waits for the scan to end, and deletes the file. Nothing
//! is written inside the repository.
//!
//! ```text
//! cargo run --release -p matsu-core --example scan_jump_bench -- [--shape s1|t1] [--size-mb 1024] [--dir PATH]
//! ```
//!
//! Shapes (ADR-0020, 8.1 and the addendum of 2026-10-29): `s1` is UTF-8 lines of 56 bytes, `t1` is
//! lines that are only a line ending (the worst case for the number of lines).

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use matsu_core::buffer::{EncodingCodec, ScanJob, ScanOptions, Source, WINDOW_BYTES};
use matsu_core::encoding::Encoding;
use matsu_core::file::{RangedFile, RangedOptions};

const MIB: u64 = 1024 * 1024;
/// The rows of a screen the jump reads.
const SCREEN_ROWS: usize = 60;

struct Args {
    shape: String,
    size_mb: u64,
    dir: PathBuf,
}

fn parse_args() -> Args {
    let mut args = Args {
        shape: "s1".to_owned(),
        size_mb: 1024,
        dir: std::env::temp_dir(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut value = || it.next().unwrap_or_else(|| panic!("{a} needs a value"));
        match a.as_str() {
            "--shape" => args.shape = value(),
            "--size-mb" => args.size_mb = value().parse().expect("--size-mb is a number"),
            "--dir" => args.dir = PathBuf::from(value()),
            other => panic!("unknown argument {other}"),
        }
    }
    args
}

/// Removes the file when dropped, also when a measurement panics.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn generate(path: &PathBuf, size: u64, shape: &str) {
    let mut out = File::create(path).expect("create the test file");
    let mut buf = Vec::with_capacity(MIB as usize + 64);
    let mut written = 0u64;
    let mut line_no = 0u64;
    while written < size {
        buf.clear();
        while buf.len() < MIB as usize {
            if shape == "t1" {
                buf.push(b'\n');
            } else {
                // 56 bytes: "line " + 12 digits + " " + 37 letters + LF
                buf.extend_from_slice(format!("line {line_no:012} ").as_bytes());
                buf.extend_from_slice(b"abcdefghijklmnopqrstuvwxyz.,abcdefghi");
                buf.push(b'\n');
            }
            line_no += 1;
        }
        let take = (size - written).min(buf.len() as u64) as usize;
        out.write_all(&buf[..take]).expect("write the test file");
        written += take as u64;
    }
    out.sync_all().expect("sync the test file");
}

/// The resident memory now and the peak, in MiB (Linux; `None` elsewhere).
fn memory_mib() -> (Option<f64>, Option<f64>) {
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let kib = |key: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<f64>().ok())
            .map(|k| k / 1024.0)
    };
    (kib("VmRSS:"), kib("VmHWM:"))
}

fn show(m: Option<f64>) -> String {
    m.map_or("-".to_owned(), |m| format!("{m:.1} MB"))
}

fn ms(t: Duration) -> String {
    format!("{:.2} ms", t.as_secs_f64() * 1000.0)
}

fn main() {
    let args = parse_args();
    let size = args.size_mb * MIB;
    let path = args.dir.join(format!("matsu-scan-jump-{}.txt", args.shape));
    let _cleanup = Cleanup(path.clone());
    let made = Instant::now();
    generate(&path, size, &args.shape);
    println!(
        "shape {} size {} MiB made in {:.1} s",
        args.shape,
        args.size_mb,
        made.elapsed().as_secs_f64()
    );

    let opened = Instant::now();
    let source: Arc<dyn Source> =
        Arc::new(RangedFile::open_with(&path, RangedOptions::default()).expect("open by ranges"));
    // A slow scan, so that the jump happens while it runs (the disk and the cache of this machine
    // are fast, and the whole scan takes a second or so).
    let options = ScanOptions {
        step_delay: Some(Duration::from_millis(20)),
        ..ScanOptions::default()
    };
    let job = ScanJob::spawn(
        source.clone(),
        Arc::new(EncodingCodec(Encoding::Utf8)),
        options,
    )
    .expect("start the scan");
    while job.progress().scanned == 0 {
        std::thread::sleep(Duration::from_millis(1));
    }
    let open_time = opened.elapsed();
    let (rss_open, _) = memory_mib();

    println!("| move | scanned at the move | time | window | rows read | resident after |");
    println!("|---|---|---|---|---|---|");
    let len = source.len();
    for (name, target) in [
        ("end (Ctrl+End)", len),
        ("middle", len / 2),
        ("90 %", len / 10 * 9),
    ] {
        let scanned = job.progress().scanned;
        let t = Instant::now();
        let window = job
            .open_window(target, WINDOW_BYTES)
            .expect("read the window")
            .expect("a window can be cut");
        let doc = &window.document;
        let last = doc.line_count().saturating_sub(SCREEN_ROWS);
        let rows = doc
            .lines(last..doc.line_count())
            .filter_map(Result::ok)
            .count();
        let end = doc.end_pos().expect("end");
        let _ = doc.pos_to_offset(end).expect("offset");
        let time = t.elapsed();
        println!(
            "| {name} | {:.1} % | {} | {:.2} MiB, {} lines | {rows} | {} |",
            scanned as f64 * 100.0 / len as f64,
            ms(time),
            (window.end - window.start) as f64 / MIB as f64,
            doc.line_count(),
            show(memory_mib().0),
        );
    }

    let waited = Instant::now();
    while !job.is_finished() {
        std::thread::sleep(Duration::from_millis(10));
    }
    let (rss_done, peak) = memory_mib();
    println!(
        "opened and first block in {}; scan done after {:.1} s more (the delay of 20 ms per read included)",
        ms(open_time),
        waited.elapsed().as_secs_f64()
    );
    println!(
        "resident: after opening {}, after the scan {}, peak {}",
        show(rss_open),
        show(rss_done),
        show(peak)
    );
}
