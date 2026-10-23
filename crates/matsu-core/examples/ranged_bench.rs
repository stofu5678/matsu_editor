//! Measures `RangedFile` (ADR-0020, section 8.1): sequential throughput against a plain read of the
//! whole file, random 4 KiB reads with a cold and a warm cache, and the memory of the process.
//!
//! It makes a test file S1 (UTF-8, 40 bytes per line, LF) in a temporary folder, measures, and
//! deletes the file. Nothing is written inside the repository.
//!
//! ```text
//! cargo run --release -p matsu-core --example ranged_bench -- [--size-mb 1024] [--dir PATH] [--random 200000]
//! ```
//!
//! `--dir` is where the file goes (default: the system temporary folder); use a folder on the disk
//! you want to measure. The operating system keeps recently read data in its own cache, so
//! "cold" here means "cold for `RangedFile`'s cache". Only Linux can drop the OS cache (it needs
//! root; the program tries and says what happened).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use matsu_core::buffer::Source;
use matsu_core::file::{RangedFile, RangedOptions};

const MIB: u64 = 1024 * 1024;
const LINE_LEN: usize = 40;
const CACHE_BYTES: u64 = 64 * MIB;
const READ_LEN: u64 = 4096;

struct Args {
    size_mb: u64,
    dir: PathBuf,
    random: usize,
}

fn parse_args() -> Args {
    let mut args = Args {
        size_mb: 1024,
        dir: std::env::temp_dir(),
        random: 200_000,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut value = || it.next().unwrap_or_else(|| panic!("{a} needs a value"));
        match a.as_str() {
            "--size-mb" => args.size_mb = value().parse().expect("--size-mb is a number"),
            "--dir" => args.dir = PathBuf::from(value()),
            "--random" => args.random = value().parse().expect("--random is a number"),
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

fn mb_per_s(bytes: u64, t: Duration) -> f64 {
    bytes as f64 / MIB as f64 / t.as_secs_f64()
}

/// S1: lines of 40 bytes: ten digits, a space, 28 letters, LF.
fn generate(path: &PathBuf, size: u64) -> Duration {
    let start = Instant::now();
    let mut out = File::create(path).expect("create the test file");
    let lines_per_chunk = 26_214; // about 1 MiB
    let mut buf = Vec::with_capacity(lines_per_chunk * LINE_LEN);
    let mut written = 0u64;
    let mut line_no = 0u64;
    while written < size {
        buf.clear();
        for _ in 0..lines_per_chunk {
            let mut digits = [b'0'; 10];
            let mut n = line_no;
            for d in digits.iter_mut().rev() {
                *d = b'0' + (n % 10) as u8;
                n /= 10;
            }
            buf.extend_from_slice(&digits);
            buf.push(b' ');
            buf.extend_from_slice(b"abcdefghijklmnopqrstuvwxyz.,");
            buf.push(b'\n');
            line_no += 1;
        }
        let take = (size - written).min(buf.len() as u64) as usize;
        out.write_all(&buf[..take]).expect("write the test file");
        written += take as u64;
    }
    out.sync_all().expect("sync the test file");
    start.elapsed()
}

fn drop_os_cache() -> &'static str {
    if cfg!(target_os = "linux") {
        match fs::write("/proc/sys/vm/drop_caches", "3") {
            Ok(()) => "dropped the OS cache",
            Err(_) => "could not drop the OS cache (needs root): reads may come from it",
        }
    } else {
        "the OS cache was not dropped (not possible without extra tools): reads may come from it"
    }
}

/// The resident memory of this process in MiB, and the peak where the OS tells it.
fn memory_mib() -> (Option<f64>, Option<f64>) {
    let pid = std::process::id();
    if cfg!(target_os = "linux") {
        let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
        let kib = |key: &str| {
            status
                .lines()
                .find_map(|l| l.strip_prefix(key))
                .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<f64>().ok())
                .map(|k| k / 1024.0)
        };
        (kib("VmRSS:"), kib("VmHWM:"))
    } else if cfg!(target_os = "macos") {
        let out = Command::new("ps")
            .args(["-o", "rss=", "-p", &pid.to_string()])
            .output()
            .ok();
        let rss = out
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<f64>().ok())
            .map(|k| k / 1024.0);
        (rss, None)
    } else if cfg!(windows) {
        let script =
            format!("$p = Get-Process -Id {pid}; \"$($p.WorkingSet64) $($p.PeakWorkingSet64)\"");
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        let mut it = out
            .split_whitespace()
            .filter_map(|v| v.parse::<f64>().ok())
            .map(|b| b / MIB as f64);
        (it.next(), it.next())
    } else {
        (None, None)
    }
}

fn show_memory(label: &str) {
    let (rss, peak) = memory_mib();
    let fmt = |v: Option<f64>| v.map_or("n/a".to_string(), |v| format!("{v:.1} MiB"));
    println!(
        "  memory {label}: resident {}, peak {}",
        fmt(rss),
        fmt(peak)
    );
}

/// A small deterministic random number generator (xorshift64*), so runs are comparable.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

fn percentile(sorted: &[u32], p: f64) -> f64 {
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    f64::from(sorted[i]) / 1000.0
}

/// Reads `count` ranges of 4 KiB at random places in `base..base + span`, returning the time of
/// every read in nanoseconds (sorted) and the total.
fn random_reads(f: &RangedFile, base: u64, span: u64, count: usize, seed: u64) -> (Vec<u32>, u64) {
    let mut rng = Rng(seed);
    let mut times = Vec::with_capacity(count);
    let mut sum = 0u64;
    for _ in 0..count {
        let start = base + rng.next() % (span - READ_LEN);
        let t = Instant::now();
        let bytes = f.read(start, start + READ_LEN).expect("random read");
        times.push(t.elapsed().as_nanos().min(u128::from(u32::MAX)) as u32);
        sum = sum.wrapping_add(u64::from(bytes[0]) + bytes.len() as u64);
    }
    times.sort_unstable();
    (times, sum)
}

fn report_random(label: &str, times: &[u32], f: &RangedFile) {
    let total: u64 = times.iter().map(|&t| u64::from(t)).sum();
    let stats = f.cache_stats();
    println!(
        "  {label}: mean {:.1} us, p50 {:.1} us, p99 {:.1} us, max {:.1} us; cache hits {} misses {}",
        total as f64 / times.len() as f64 / 1000.0,
        percentile(times, 0.5),
        percentile(times, 0.99),
        percentile(times, 1.0),
        stats.hits,
        stats.misses,
    );
}

fn main() {
    let args = parse_args();
    let size = args.size_mb * MIB;
    assert!(
        size > 64 * MIB,
        "--size-mb must be larger than the 64 MiB cache"
    );
    let path = args
        .dir
        .join(format!("matsu-ranged-bench-{}.txt", std::process::id()));
    let _cleanup = Cleanup(path.clone());

    println!(
        "ranged_bench: {} on {} {}, {} MiB file, file: {}",
        if cfg!(debug_assertions) {
            "DEBUG build (do not trust the numbers)"
        } else {
            "release build"
        },
        std::env::consts::OS,
        std::env::consts::ARCH,
        args.size_mb,
        path.display()
    );
    show_memory("at start");
    let t = generate(&path, size);
    println!(
        "make the file: {:.1} s ({:.0} MB/s)",
        t.as_secs_f64(),
        mb_per_s(size, t)
    );
    println!("{}", drop_os_cache());

    // The disk's speed: a plain read of the whole file in 1 MiB pieces.
    let plain_read = || {
        let mut file = File::open(&path).unwrap();
        let mut buf = vec![0u8; MIB as usize];
        let mut sum = 0u64;
        let t = Instant::now();
        loop {
            let n = file.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            sum = sum.wrapping_add(u64::from(buf[0]) + n as u64);
        }
        let t = t.elapsed();
        println!(
            "  baseline, plain read in 1 MiB pieces: {:.2} s, {:.0} MB/s (check {sum})",
            t.as_secs_f64(),
            mb_per_s(size, t)
        );
    };

    for block in [64 * 1024u64, 256 * 1024, MIB] {
        let options = RangedOptions {
            block_size: block as usize,
            cache_blocks: (CACHE_BYTES / block) as usize,
        };
        println!(
            "\nblock {} KiB x {} blocks = {} MiB cache",
            block / 1024,
            options.cache_blocks,
            CACHE_BYTES / MIB
        );

        plain_read();
        // Sequential: the whole file in ranges (what a scan does). Ranges of 2 blocks or more
        // skip the cache; small ones go through it.
        for range in [32 * 1024u64, MIB] {
            let f = RangedFile::open_with(&path, options).unwrap();
            let mut sum = 0u64;
            let t = Instant::now();
            let mut pos = 0;
            while pos < size {
                let end = (pos + range).min(size);
                let bytes = f.read(pos, end).unwrap();
                sum = sum.wrapping_add(u64::from(bytes[0]) + bytes.len() as u64);
                pos = end;
            }
            let t = t.elapsed();
            println!(
                "  sequential, ranges of {} KiB: {:.2} s, {:.0} MB/s (check {sum})",
                range / 1024,
                t.as_secs_f64(),
                mb_per_s(size, t)
            );
            show_memory("after the sequential read");
        }

        // Random 4 KiB over the whole file, cache cold (a new RangedFile).
        let f = RangedFile::open_with(&path, options).unwrap();
        let (times, _) = random_reads(&f, 0, size, args.random, 1);
        report_random(
            &format!("random 4 KiB over the whole file x {}, cold", args.random),
            &times,
            &f,
        );
        show_memory("after the random reads");
        drop(f);

        // Random 4 KiB inside a 16 MiB working set, after one pass to fill the cache (warm).
        let f = RangedFile::open_with(&path, options).unwrap();
        let base = size / 2;
        let span = 16 * MIB;
        let _ = random_reads(&f, base, span, args.random, 2);
        let before = f.cache_stats();
        let (times, _) = random_reads(&f, base, span, args.random, 3);
        let after = f.cache_stats();
        let total: u64 = times.iter().map(|&t| u64::from(t)).sum();
        println!(
            "  random 4 KiB inside 16 MiB x {}, warm: mean {:.1} us, p50 {:.1} us, p99 {:.1} us, max {:.1} us; cache hits {} misses {}",
            args.random,
            total as f64 / times.len() as f64 / 1000.0,
            percentile(&times, 0.5),
            percentile(&times, 0.99),
            percentile(&times, 1.0),
            after.hits - before.hits,
            after.misses - before.misses,
        );
        println!("  cache holds {} MiB", f.cached_bytes() as u64 / MIB);
    }
    println!("\ndone; the test file is deleted");
}
