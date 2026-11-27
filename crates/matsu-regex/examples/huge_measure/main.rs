//! Measures huge files without a window (ADR-0020, sections 8.1 and 8.3), through the code the
//! editor uses: `open_file` (a `RangedFile` and a background scan), the document read by ranges,
//! the search (`find` and `find_all` with the plain and the regular expression matchers), and
//! save. It is run by hand, or by the workflow `.github/workflows/huge-measure.yml`.
//!
//! ```text
//! cargo run --release -p matsu-regex --example huge_measure -- \
//!     --kinds s,c --size-mb 1024 --blocks 64,256 --dir <folder on the disk to measure> --out <report folder>
//! ```
//!
//! It makes each file in `--dir`, measures it for each block size (the block of the file's cache
//! and the block of the scan's index are the same; the cache is always `--cache-mb` in all),
//! deletes the file, and writes `report.md` and `report.json` into `--out`. The page
//! `docs/measurements/huge-files.md` says how to read them.
//!
//! The kinds are S (UTF-8 LF), C (CRLF), T (1-byte lines), L (one line), J (Shift_JIS), W
//! (UTF-16 LE) and E (EUC-JP); L and E are at most 1 GiB, as in ADR-0020.
//!
//! The program changes no product behavior: the block size reaches the code through
//! `OpenOptions::huge_ranged` and `huge_scan`, which only this program and the tests set.

mod checks;
mod generate;
mod report;
mod sys;

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use matsu_core::buffer::{Document, Pos, ScanOptions};
use matsu_core::file::{OpenOptions, RangedOptions, SaveOptions, no_progress, open_file};
use matsu_core::search::{
    Control, Direction, Matcher, PlainMatcher, PlainOptions, SearchError, SearchJob, find, find_all,
};
use matsu_regex::{RegexMatcher, RegexSearchOptions};

use generate::{COMMON_PLAIN, COMMON_REGEX, Kind, NEEDLE, write_file};
use report::{Machine, Report, Run};

const MIB: u64 = 1024 * 1024;
/// The rows of a screen that a jump reads.
const SCREEN_ROWS: usize = 60;
/// The bytes of a slice of a very long line that a screen shows.
const SLICE_BYTES: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
struct Args {
    kinds: Vec<Kind>,
    size_mb: u64,
    blocks_kib: Vec<u64>,
    cache_mb: u64,
    dir: PathBuf,
    out: PathBuf,
    search_limit: Duration,
    scan_limit: Duration,
    save: bool,
    search: bool,
    baseline: bool,
    checks_mb: Option<u64>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            kinds: vec![Kind::S],
            size_mb: 1024,
            blocks_kib: vec![64, 256],
            cache_mb: 64,
            dir: std::env::temp_dir(),
            out: PathBuf::from("."),
            search_limit: Duration::from_secs(120),
            scan_limit: Duration::from_secs(600),
            save: true,
            search: true,
            baseline: true,
            checks_mb: Some(32),
        }
    }
}

const USAGE: &str =
    "usage: huge_measure [--kinds s,c,t,l,j,w,e] [--size-mb 1024] [--blocks 64,256] [--cache-mb 64]
                    [--dir FOLDER] [--out FOLDER] [--search-limit-secs 120] [--scan-limit-secs 600]
                    [--no-save] [--no-search] [--no-baseline] [--checks-mb 32 | --no-checks]";

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{a} needs a value"))
        };
        let number = |text: String, name: &str| {
            text.parse::<u64>()
                .map_err(|_| format!("{name} needs a number, not {text:?}"))
        };
        match a.as_str() {
            "--kinds" => {
                let text = value()?;
                out.kinds = text
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| Kind::parse(s).ok_or_else(|| format!("unknown kind {s:?}")))
                    .collect::<Result<_, _>>()?;
            }
            "--size-mb" => out.size_mb = number(value()?, "--size-mb")?,
            "--blocks" => {
                out.blocks_kib = value()?
                    .split(',')
                    .map(|s| number(s.trim().to_owned(), "--blocks"))
                    .collect::<Result<_, _>>()?;
                if out.blocks_kib.contains(&0) {
                    return Err("--blocks must not have 0".into());
                }
            }
            "--cache-mb" => out.cache_mb = number(value()?, "--cache-mb")?.max(1),
            "--dir" => out.dir = PathBuf::from(value()?),
            "--out" => out.out = PathBuf::from(value()?),
            "--search-limit-secs" => out.search_limit = Duration::from_secs(number(value()?, a)?),
            "--scan-limit-secs" => out.scan_limit = Duration::from_secs(number(value()?, a)?),
            "--no-save" => out.save = false,
            "--no-search" => out.search = false,
            "--no-baseline" => out.baseline = false,
            "--checks-mb" => out.checks_mb = Some(number(value()?, "--checks-mb")?.max(1)),
            "--no-checks" => out.checks_mb = None,
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    if out.size_mb == 0 {
        return Err("--size-mb must not be 0".into());
    }
    Ok(out)
}

/// Removes the file when dropped, also when a measurement panics.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn mb_per_s(bytes: u64, t: Duration) -> f64 {
    bytes as f64 / MIB as f64 / t.as_secs_f64().max(1e-9)
}

fn ms(t: Duration) -> f64 {
    t.as_secs_f64() * 1000.0
}

/// Which part of a document a screen is read from.
#[derive(Clone, Copy)]
enum At {
    Start,
    Middle,
    End,
}

/// Reads what a screen at `at` needs, as the editor does: a few rows, or, for a document that is
/// one very long line, a slice of it. Returns the bytes of text read.
fn read_screen(doc: &Document, at: At) -> Result<usize, String> {
    let count = doc.line_count();
    if count == 0 {
        return Ok(0);
    }
    if count == 1 && doc.is_long_line(0) {
        let raw = doc.line_raw_len(0).map_err(|e| e.to_string())?.unwrap_or(0);
        let from = match at {
            At::Start => 0,
            At::Middle => raw / 2,
            At::End => raw.saturating_sub(SLICE_BYTES),
        };
        let slice = doc
            .line_slice(0, from, from + SLICE_BYTES)
            .map_err(|e| e.to_string())?;
        return Ok(slice.map_or(0, |s| s.text.len()));
    }
    let first = match at {
        At::Start => 0,
        At::Middle => count / 2,
        At::End => count.saturating_sub(SCREEN_ROWS),
    };
    let mut bytes = 0;
    for line in doc.lines(first..(first + SCREEN_ROWS).min(count)) {
        bytes += line.map_err(|e| e.to_string())?.text.len();
    }
    Ok(bytes)
}

/// How a search ended.
enum Outcome {
    Done {
        elapsed: Duration,
        hits: usize,
    },
    Limit {
        after: Duration,
        lines_done: usize,
        lines_total: usize,
    },
    Failed(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    First,
    All,
}

fn run_search(doc: &Document, matcher: &Arc<dyn Matcher>, mode: Mode, limit: Duration) -> Outcome {
    let doc = doc.clone();
    let matcher = Arc::clone(matcher);
    let job = SearchJob::spawn(move |ctl: &mut Control<'static>| {
        let start = Instant::now();
        let hits = match mode {
            Mode::First => {
                let found = find(
                    &doc,
                    &*matcher,
                    Pos::new(0, 0),
                    Direction::Forward,
                    false,
                    ctl,
                )?;
                usize::from(found.is_some())
            }
            Mode::All => {
                let mut n = 0;
                for item in find_all(&doc, &*matcher, None, std::mem::take(ctl)) {
                    item?;
                    n += 1;
                }
                n
            }
        };
        Ok::<_, SearchError>((start.elapsed(), hits))
    });
    let start = Instant::now();
    while !job.is_finished() {
        if start.elapsed() > limit {
            job.cancel();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let progress = job.progress();
    match job.join() {
        Ok((elapsed, hits)) => Outcome::Done { elapsed, hits },
        Err(SearchError::Cancelled) => Outcome::Limit {
            after: start.elapsed(),
            lines_done: progress.lines_done,
            lines_total: progress.lines_total,
        },
        Err(e) => Outcome::Failed(e.to_string()),
    }
}

fn add_search(
    run: &mut Run,
    label: &str,
    doc: &Document,
    matcher: &Arc<dyn Matcher>,
    limit: Duration,
) {
    for (mode, what) in [(Mode::First, "first hit"), (Mode::All, "whole file")] {
        let unit = if mode == Mode::First { "ms" } else { "s" };
        let name = format!("{label} search: {what}");
        match run_search(doc, matcher, mode, limit) {
            Outcome::Done { elapsed, hits } => {
                let t = if mode == Mode::First {
                    ms(elapsed)
                } else {
                    elapsed.as_secs_f64()
                };
                run.num(&name, unit, t);
                run.num(&format!("{label} search: {what}, hits"), "", hits as f64);
            }
            Outcome::Limit {
                after,
                lines_done,
                lines_total,
            } => {
                run.text(
                    &name,
                    format!(
                        "stopped after {:.0} s at {:.2} % of the lines",
                        after.as_secs_f64(),
                        lines_done as f64 * 100.0 / lines_total.max(1) as f64
                    ),
                );
                run.missing(&format!("{label} search: {what}, hits"), "");
            }
            Outcome::Failed(e) => {
                run.text(&name, format!("failed: {e}"));
                run.missing(&format!("{label} search: {what}, hits"), "");
            }
        }
    }
}

/// A plain read of the whole file in pieces of 1 MiB: the speed of the disk.
fn baseline(path: &Path) -> Result<f64, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; MIB as usize];
    let (mut total, mut sum) = (0u64, 0u64);
    let start = Instant::now();
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total += n as u64;
        sum = sum.wrapping_add(u64::from(buf[0]));
    }
    let t = start.elapsed();
    std::hint::black_box(sum);
    Ok(mb_per_s(total, t))
}

fn mib(bytes: usize) -> f64 {
    bytes as f64 / MIB as f64
}

/// Measures one file with one block size. A step that fails ends the run with a note; the
/// numbers measured before stay.
fn measure(path: &Path, kind: Kind, file_bytes: u64, block_kib: u64, args: &Args) -> Run {
    let mut run = Run::new(kind.letter(), file_bytes, block_kib);
    if let Err(message) = measure_steps(&mut run, path, kind, file_bytes, block_kib, args) {
        run.notes.push(format!("stopped: {message}"));
    }
    run
}

fn measure_steps(
    run: &mut Run,
    path: &Path,
    kind: Kind,
    file_bytes: u64,
    block_kib: u64,
    args: &Args,
) -> Result<(), String> {
    let block = (block_kib * 1024) as usize;
    let options = OpenOptions {
        encoding: Some(kind.encoding()),
        huge_file_len: Some(1),
        huge_scan: ScanOptions {
            block_size: block,
            ..ScanOptions::default()
        },
        huge_ranged: RangedOptions {
            block_size: block,
            cache_blocks: ((args.cache_mb * MIB) as usize / block).max(1),
        },
        ..OpenOptions::default()
    };
    sys::drop_os_cache();
    if !sys::reset_peak_memory() {
        run.notes.push(
            "the peak of the memory is the peak of the whole process so far (it could not be reset)"
                .to_owned(),
        );
    }

    // The memory the process holds already (the allocator keeps what the earlier runs used): the
    // numbers below are to be read against it.
    run.num(
        "resident before opening",
        "MiB",
        sys::memory_mib().0.unwrap_or(f64::NAN),
    );

    // The first window: open the file (the first block of the scan is waited for) and read a screen.
    let t0 = Instant::now();
    let mut file = open_file(path, &options).map_err(|e| format!("open: {e}"))?;
    let opened = t0.elapsed();
    if !file.is_ranged() {
        return Err("the file was not opened by ranges".to_owned());
    }
    read_screen(&file.document, At::Start)?;
    run.num("open, first block scanned", "ms", ms(opened));
    run.num("first window readable", "ms", ms(t0.elapsed()));
    run.num(
        "resident after opening",
        "MiB",
        sys::memory_mib().0.unwrap_or(f64::NAN),
    );

    // The scan, in the background as in the editor; the time includes opening.
    if let Some(job) = file.take_scan() {
        let mut cancelled = false;
        while !job.is_finished() {
            if !cancelled && t0.elapsed() > args.scan_limit {
                job.cancel();
                cancelled = true;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let progress = job.progress();
        if cancelled {
            run.num(
                "scan stopped at the limit, share of the file",
                "%",
                progress.scanned as f64 * 100.0 / progress.total.max(1) as f64,
            );
            return Err(format!(
                "the scan did not end in {} s",
                args.scan_limit.as_secs()
            ));
        }
        file.document = job.finish().map_err(|e| format!("scan: {e}"))?;
    }
    let scan = t0.elapsed();
    run.num("scan, with opening", "s", scan.as_secs_f64());
    run.num("scan throughput", "MB/s", mb_per_s(file_bytes, scan));
    run.num("lines", "", file.document.line_count() as f64);
    let (rss, peak) = sys::memory_mib();
    run.num("resident after the scan", "MiB", rss.unwrap_or(f64::NAN));
    run.num("peak resident until now", "MiB", peak.unwrap_or(f64::NAN));
    run.num(
        "index in memory",
        "MiB",
        mib(file.document.index_heap_bytes()),
    );

    // Jumps: the first read of a place is cold for the file's cache (and for the OS cache when it
    // was dropped), the second is warm.
    for (name, at) in [("middle", At::Middle), ("end", At::End)] {
        let t = Instant::now();
        read_screen(&file.document, at)?;
        let cold = t.elapsed();
        let t = Instant::now();
        read_screen(&file.document, at)?;
        run.num(&format!("jump to the {name}, cold"), "ms", ms(cold));
        run.num(&format!("jump to the {name}, warm"), "ms", ms(t.elapsed()));
    }

    if args.search && !kind.is_one_line() {
        let plain: Arc<dyn Matcher> = Arc::new(PlainMatcher::new(
            NEEDLE,
            PlainOptions {
                match_case: true,
                whole_word: false,
            },
        ));
        add_search(
            run,
            "plain (needle)",
            &file.document,
            &plain,
            args.search_limit,
        );
        let regex: Arc<dyn Matcher> = Arc::new(
            RegexMatcher::new(&needle_regex(), RegexSearchOptions { match_case: true })
                .map_err(|e| format!("regex: {e:?}"))?,
        );
        add_search(
            run,
            "regex (needle)",
            &file.document,
            &regex,
            args.search_limit,
        );
        let plain: Arc<dyn Matcher> = Arc::new(PlainMatcher::new(
            COMMON_PLAIN,
            PlainOptions {
                match_case: true,
                whole_word: false,
            },
        ));
        add_search(
            run,
            "plain (common text)",
            &file.document,
            &plain,
            args.search_limit,
        );
        let regex: Arc<dyn Matcher> = Arc::new(
            RegexMatcher::new(COMMON_REGEX, RegexSearchOptions { match_case: true })
                .map_err(|e| format!("regex: {e:?}"))?,
        );
        add_search(
            run,
            "regex (common text)",
            &file.document,
            &regex,
            args.search_limit,
        );
    } else if args.search {
        run.text(
            "search",
            "left out: a search decodes the whole line, and this file is one line",
        );
    }

    if args.save {
        let keep = SaveOptions::keep(&file);
        let t = Instant::now();
        file.save(&keep, &mut no_progress)
            .map_err(|e| format!("save without changes: {e}"))?;
        let t = t.elapsed();
        run.num("save without changes", "s", t.as_secs_f64());
        run.num(
            "save without changes, throughput",
            "MB/s",
            mb_per_s(file_bytes, t),
        );
        file.document
            .insert(Pos::new(0, 0), "x")
            .map_err(|e| format!("edit: {e}"))?;
        let keep = SaveOptions::keep(&file);
        let t = Instant::now();
        file.save(&keep, &mut no_progress)
            .map_err(|e| format!("save after an edit: {e}"))?;
        let t = t.elapsed();
        run.num("save after one edit", "s", t.as_secs_f64());
        run.num(
            "save after one edit, throughput",
            "MB/s",
            mb_per_s(file_bytes, t),
        );
    }
    let (rss, peak) = sys::memory_mib();
    run.num("resident at the end", "MiB", rss.unwrap_or(f64::NAN));
    run.num("peak resident of the run", "MiB", peak.unwrap_or(f64::NAN));
    Ok(())
}

/// A regular expression for the needle that is not the needle itself written out (a character
/// class makes the engine compile a real pattern).
fn needle_regex() -> String {
    NEEDLE.replacen("MATSU", "MAT[S]U", 1)
}

fn write_reports(report: &Report, out: &Path) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    fs::write(out.join("report.md"), report.markdown()).map_err(|e| e.to_string())?;
    fs::write(out.join("report.json"), report.json()).map_err(|e| e.to_string())
}

fn run_all(args: &Args, argument_text: &str) -> Result<Report, String> {
    fs::create_dir_all(&args.dir).map_err(|e| format!("{}: {e}", args.dir.display()))?;
    let cache = sys::drop_os_cache();
    let mut report = Report {
        machine: Machine {
            ..sys::machine(&args.dir, cache)
        },
        arguments: argument_text.to_owned(),
        ..Report::default()
    };
    for &kind in &args.kinds {
        let bytes = kind.size_for(args.size_mb * MIB);
        let path = args.dir.join(format!(
            "huge-measure-{}-{}.bin",
            kind.letter(),
            std::process::id()
        ));
        // Never over a file that is there: the guard below deletes the file it is given.
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                format!(
                    "{}: {e} (a file of this name must not be there)",
                    path.display()
                )
            })?;
        let _cleanup = Cleanup(path.clone());
        let started = Instant::now();
        let mut file = file;
        let made = write_file(kind, bytes, &mut file)
            .map_err(|e| format!("make {}: {e}", kind.letter()))?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        let made_in = started.elapsed();
        eprintln!(
            "{}: made {} MiB in {:.1} s",
            kind.letter(),
            made / MIB,
            made_in.as_secs_f64()
        );
        if args.baseline {
            sys::drop_os_cache();
            match baseline(&path) {
                Ok(speed) => report.baselines.push((kind.letter().to_owned(), speed)),
                Err(e) => eprintln!("{}: the baseline failed: {e}", kind.letter()),
            }
        }
        for (i, &block_kib) in args.blocks_kib.iter().enumerate() {
            eprintln!("{}: block {block_kib} KiB ...", kind.letter());
            let mut run = measure(&path, kind, made, block_kib, args);
            if i == 0 {
                run.notes.push(format!(
                    "the file ({}) was made in {:.1} s ({:.0} MB/s)",
                    kind.describe(),
                    made_in.as_secs_f64(),
                    mb_per_s(made, made_in)
                ));
            }
            report.runs.push(run);
            // After every run, so that a job that is cut off leaves what it has.
            write_reports(&report, &args.out)?;
        }
        // The file goes now: the next kind may need the room (the runners have little).
        drop(_cleanup);
    }
    if let Some(mb) = args.checks_mb {
        eprintln!("OS checks ...");
        report.checks = checks::run(&args.dir, mb * MIB);
    }
    write_reports(&report, &args.out)?;
    Ok(report)
}

fn main() -> std::process::ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return std::process::ExitCode::SUCCESS;
    }
    let args = match parse_args(&raw) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    match run_all(&args, &raw.join(" ")) {
        Ok(report) => {
            println!("{}", report.markdown());
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("matsu-huge-measure-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_arguments_are_read() {
        let a = parse_args(&strings(&[
            "--kinds",
            "s10,L1,j",
            "--size-mb",
            "16",
            "--blocks",
            "64, 256",
            "--no-save",
            "--no-checks",
            "--search-limit-secs",
            "5",
        ]))
        .unwrap();
        assert_eq!(a.kinds, vec![Kind::S, Kind::L, Kind::J]);
        assert_eq!(a.size_mb, 16);
        assert_eq!(a.blocks_kib, vec![64, 256]);
        assert!(!a.save && a.search && a.checks_mb.is_none());
        assert_eq!(a.search_limit, Duration::from_secs(5));
        assert_eq!(parse_args(&[]).unwrap(), Args::default());
    }

    #[test]
    fn bad_arguments_are_refused() {
        for bad in [
            &["--kinds", "q"][..],
            &["--size-mb", "x"],
            &["--size-mb", "0"],
            &["--blocks", "0"],
            &["--size-mb"],
            &["--nope"],
        ] {
            assert!(parse_args(&strings(bad)).is_err(), "{bad:?}");
        }
    }

    /// Every kind goes through the whole measurement on a small file, with a small block, and every
    /// step gives a number.
    #[test]
    fn every_kind_is_measured_end_to_end() {
        let dir = scratch("measure");
        for kind in Kind::ALL {
            let path = dir.join(format!("{}.bin", kind.letter()));
            let mut file = File::create(&path).unwrap();
            let bytes = write_file(kind, 600 * 1024, &mut file).unwrap();
            drop(file);
            let args = Args {
                dir: dir.clone(),
                search_limit: Duration::from_secs(60),
                scan_limit: Duration::from_secs(60),
                ..Args::default()
            };
            let run = measure(&path, kind, bytes, 64, &args);
            assert!(
                !run.notes.iter().any(|n| n.starts_with("stopped")),
                "{kind:?}: {:?}",
                run.notes
            );
            let value = |name: &str| {
                run.metrics
                    .iter()
                    .find(|m| m.0 == name)
                    .unwrap_or_else(|| panic!("{kind:?}: no {name}: {:?}", run.metrics))
                    .2
                    .clone()
            };
            assert!(matches!(value("scan throughput"), report::Value::Num(n) if n > 0.0));
            assert!(matches!(
                value("jump to the end, cold"),
                report::Value::Num(_)
            ));
            assert!(matches!(
                value("save after one edit"),
                report::Value::Num(_)
            ));
            if !kind.is_one_line() {
                // The needle is once in the file; the regex finds it too.
                assert_eq!(
                    value("plain (needle) search: whole file, hits"),
                    report::Value::Num(1.0)
                );
                assert_eq!(
                    value("regex (needle) search: whole file, hits"),
                    report::Value::Num(1.0)
                );
                assert_eq!(
                    value("plain (needle) search: first hit, hits"),
                    report::Value::Num(1.0)
                );
            }
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_search_over_the_limit_is_reported_as_stopped() {
        let dir = scratch("limit");
        let path = dir.join("t.bin");
        let mut file = File::create(&path).unwrap();
        write_file(Kind::T, 2 * MIB, &mut file).unwrap();
        drop(file);
        let options = OpenOptions {
            huge_file_len: Some(1),
            ..OpenOptions::default()
        };
        let mut f = open_file(&path, &options).unwrap();
        let doc = match f.take_scan() {
            Some(job) => job.finish().unwrap(),
            None => f.document.clone(),
        };
        let matcher: Arc<dyn Matcher> = Arc::new(PlainMatcher::new("zzz", PlainOptions::default()));
        // A limit of zero: the cancel comes at the first look.
        match run_search(&doc, &matcher, Mode::All, Duration::ZERO) {
            Outcome::Limit { .. } | Outcome::Done { hits: 0, .. } => {}
            Outcome::Done { hits, .. } => panic!("hits: {hits}"),
            Outcome::Failed(e) => panic!("{e}"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_reports_are_written() {
        let dir = scratch("reports");
        let args = Args {
            kinds: vec![Kind::S],
            size_mb: 1,
            blocks_kib: vec![64, 256],
            dir: dir.clone(),
            out: dir.join("out"),
            checks_mb: Some(1),
            save: false,
            ..Args::default()
        };
        let report = run_all(&args, "--test").unwrap();
        assert_eq!(report.runs.len(), 2);
        assert_eq!(report.checks.len(), 4);
        let md = fs::read_to_string(dir.join("out/report.md")).unwrap();
        let json = fs::read_to_string(dir.join("out/report.json")).unwrap();
        assert!(md.contains("block 64 KiB") && md.contains("block 256 KiB"));
        assert!(md.contains("## OS checks"));
        assert!(json.contains("\"block_kib\": 256"));
        // The test file is deleted.
        let left: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with("huge-measure-"))
            .collect();
        assert!(left.is_empty(), "{left:?}");
        let _ = fs::remove_dir_all(&dir);
    }
}
