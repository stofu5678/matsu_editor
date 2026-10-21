//! Sakura Editor as the oracle: run the same key macro on the same input in the real Sakura Editor
//! and in UME Editor, and compare the bytes of the saved files (the encoding, the line endings and
//! the BOM included).
//!
//! The agents read Sakura's behavior from its source, and a misreading shows late. The real Sakura
//! gives the expected result directly. The test is `#[ignore]` because it needs Windows, Sakura's
//! release binary and a display; `.github/workflows/sakura-oracle.yml` runs it.
//!
//! ```sh
//! cargo build --release --locked -p ume -p ume-script
//! SAKURA_DIR=C:/path/to/the/unzipped/sakura-release \
//!   cargo test --release --locked -p ume --test sakura_oracle -- --ignored --nocapture
//! ```
//!
//! # The cases
//!
//! `sakura_oracle/cases/<name>.mac` is a key macro that both editors run (a `.mac` file, started
//! with `-M=`). Its comment lines at the top say what to open:
//!
//! * `// input: <file>`: a file of `sakura_oracle/inputs/`, copied to `input.txt` for each editor;
//! * `// code: <n>`: Sakura's number of the encoding of the input (`-CODE=`; 0 is Shift_JIS, 2 EUC,
//!   3 UTF-16 LE, 4 UTF-8, 6 UTF-16 BE).
//!
//! The macro ends with `FileSaveAs('@OUT@', <encoding>, <line ending>);` and `ExitAll();`, and
//! `@OUT@` becomes the output file of each editor. The macros are ASCII only (a Japanese text is
//! written as `\uXXXX`), so that the encoding of the macro file does not matter.
//!
//! # Known differences
//!
//! `sakura_oracle/known_differences.txt` lists the cases in which UME differs from Sakura on
//! purpose or for now, each with the reason. A case that differs and is not listed fails the test,
//! and so does a listed case that no longer differs (delete the line then). So the job passes while
//! the known differences stand, and a new difference fails it.
//!
//! # Where the files go
//!
//! `$ORACLE_OUT` (default: this crate's `target/tmp`) gets `report.txt` and, for each case that
//! differs, `<case>/sakura.out`, `<case>/ume.out` and the macro; the workflow keeps it as an
//! artifact. `$ORACLE_ONLY` (a part of a case name) runs only the cases whose name contains it.
//!
//! # Safety
//!
//! Both editors run copies in a scratch folder, with their settings in the scratch folder
//! (a Sakura profile of its own, with its `sakura.ini`; `HOME`, `APPDATA` and so on for UME), so they never read the
//! settings of the user who runs the test. A Sakura that does not end (a dialog waits for a click)
//! is stopped by the process ids of the programs that run from the scratch folder, never by name.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// How long one editor may run for one case.
const TIME_LIMIT: Duration = Duration::from_secs(60);

/// UME ends at once when its macro works (about a second, a few more on a slow start); one that
/// stops at an error stays open, so do not wait as long as for Sakura.
const UME_TIME_LIMIT: Duration = Duration::from_secs(25);

/// `sakura.ini` of the test: nothing asks for a confirmation (`bExitConfirm`, `bCloseAllConfirm`
/// in `CShareData_IO.cpp`), and the search starts without options.
const SAKURA_INI: &str =
    "[Common]\r\nbExitConfirm=0\r\nbCloseAllConfirm=0\r\nbSearchRegularExp=0\r\n";

fn oracle_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sakura_oracle")
}

/// One case: the macro and what to open.
struct Case {
    name: String,
    input: String,
    code: String,
    mac: String,
}

fn load_cases() -> Vec<Case> {
    let only = std::env::var("ORACLE_ONLY").unwrap_or_default();
    let mut cases = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(oracle_dir().join("cases"))
        .expect("read the cases folder")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "mac"))
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        if !name.contains(&only) {
            continue;
        }
        let mac = fs::read_to_string(&path).unwrap();
        let header = |key: &str| {
            mac.lines()
                .find_map(|l| l.strip_prefix(&format!("// {key}: ")))
                .unwrap_or_else(|| panic!("{name}: no `// {key}:` line"))
                .trim()
                .to_string()
        };
        let input = header("input");
        assert!(
            oracle_dir().join("inputs").join(&input).is_file(),
            "{name}: the input {input} is not in sakura_oracle/inputs"
        );
        cases.push(Case {
            input,
            code: header("code"),
            mac,
            name,
        });
    }
    cases
}

/// `known_differences.txt`: `case-name | reason`, `#` starts a comment line.
fn load_known() -> BTreeMap<String, String> {
    let text = fs::read_to_string(oracle_dir().join("known_differences.txt")).unwrap();
    let mut known = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, reason) = line
            .split_once('|')
            .unwrap_or_else(|| panic!("known_differences.txt: no `|` in {line:?}"));
        let reason = reason.trim();
        assert!(!reason.is_empty(), "no reason for {}", name.trim());
        known.insert(name.trim().to_string(), reason.to_string());
    }
    known
}

/// A scratch folder that is removed at the end, also when the test fails.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        // The long form of the path (no `RUNNER~1`, no `\\?\`): `processes_in` compares it with
        // the path Windows reports for a running program.
        let base = fs::canonicalize(std::env::temp_dir()).unwrap();
        let base = base.to_str().unwrap();
        let base = PathBuf::from(base.strip_prefix(r"\\?\").unwrap_or(base));
        let dir = base.join(format!("ume-oracle-{name}-{}", std::process::id()));
        // The path goes into a macro string and a PowerShell string, both in single quotes.
        assert!(
            !dir.to_str().unwrap().contains('\''),
            "the temporary folder {} has an apostrophe in its path: set TEMP to another folder",
            dir.display()
        );
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A path in a macro string: forward slashes, so that no backslash needs an escape.
fn macro_path(path: &Path) -> String {
    path.to_str().unwrap().replace('\\', "/")
}

/// The ids of the processes that run a program from inside `dir`. Sakura starts a resident process
/// and the editor in another process, and the first process ends at once, so the child handle is
/// not enough to find them. The folder is private to the case, so this finds only our own
/// processes (and nothing by name).
fn processes_in(dir: &Path) -> Vec<u32> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let prefix = format!("{}\\", dir.display());
    let script = format!(
        "Get-CimInstance Win32_Process | Where-Object {{ $_.ExecutablePath -and $_.ExecutablePath.StartsWith('{prefix}', [System.StringComparison]::OrdinalIgnoreCase) }} | ForEach-Object {{ $_.ProcessId }}"
    );
    let Ok(output) = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect()
}

/// Stops the programs of `dir` by process id. Returns what was running (process id, name and the
/// title of its window, to see which dialog waited), or `None` if nothing was.
fn stop_all(dir: &Path) -> Option<String> {
    let ids = processes_in(dir);
    if ids.is_empty() {
        return None;
    }
    let list = ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    let script = format!(
        "Get-Process -Id {list} -ErrorAction SilentlyContinue | ForEach-Object {{ \"$($_.Id) $($_.ProcessName) [$($_.MainWindowTitle)]\" }}"
    );
    let what = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).replace(['\r', '\n'], " "))
        .unwrap_or_default();
    for id in &ids {
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &id.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    thread::sleep(Duration::from_millis(500));
    Some(what)
}

/// What one editor did.
enum Outcome {
    Saved(Vec<u8>),
    /// It did not write the file (the macro stopped, a dialog waited, ...), with the reason.
    Failed(String),
}

/// Prepares `<scratch>/<side>` for `case` and returns the folder, the macro path and the output.
fn prepare(scratch: &Scratch, side: &str, case: &Case) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let dir = scratch.0.join(side);
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.txt");
    fs::copy(oracle_dir().join("inputs").join(&case.input), &input).unwrap();
    let out = dir.join("out.bin");
    let mac = dir.join("case.mac");
    fs::write(
        &mac,
        case.mac
            .replace("@OUT@", &macro_path(&out))
            .replace("\r\n", "\n"),
    )
    .unwrap();
    (dir, input, mac, out)
}

fn run_sakura(scratch: &Scratch, case: &Case, sakura_dir: &Path) -> Outcome {
    let (dir, input, mac, out) = prepare(scratch, "sakura", case);
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for file in ["sakura.exe", "bregonig.dll", "sakura_lang_en_US.dll"] {
        let from = sakura_dir.join(file);
        if from.is_file() {
            fs::copy(&from, bin.join(file)).unwrap();
        }
    }
    // A profile of its own: Sakura names its shared memory and its resident process after the
    // profile (`CShareData::InitShareData`, `CControlProcess::InitializeProcess`), not after the
    // folder of the exe, so without it this run would join a Sakura the user has open, and its
    // `ExitAll` would close that one. The settings of a profile are in `<exe folder>/<profile>/`
    // (`CControlProcess::GetIniFileName`).
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let profile = format!(
        "umeoracle{}x{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    );
    fs::create_dir_all(bin.join(&profile)).unwrap();
    fs::write(bin.join(&profile).join("sakura.ini"), SAKURA_INI).unwrap();

    let result = Command::new(bin.join("sakura.exe"))
        .arg(format!("-M={}", mac.display()))
        .arg(format!("-PROF={profile}"))
        .arg(format!("-CODE={}", case.code))
        .arg(&input)
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match result {
        Ok(child) => child,
        Err(e) => return Outcome::Failed(format!("could not start sakura.exe: {e}")),
    };
    // The first process hands the work to the resident one and ends, so the child handle says
    // little. Wait until the editor has saved and every program of the folder has ended (`ExitAll`
    // ends the resident one too; the next case needs it to be gone), or until nothing runs any
    // more and nothing was saved.
    let started = Instant::now();
    let mut idle = 0;
    let stopped = loop {
        let running = !processes_in(&bin).is_empty();
        if !running && out.is_file() {
            break None;
        }
        // Nothing runs and no file after a few looks: Sakura ended without saving.
        idle = if running { 0 } else { idle + 1 };
        if idle >= 4 {
            break None;
        }
        if started.elapsed() > TIME_LIMIT {
            break stop_all(&bin);
        }
        thread::sleep(Duration::from_millis(250));
    };
    let _ = child.try_wait();
    match (fs::read(&out), stopped) {
        (Ok(bytes), None) => Outcome::Saved(bytes),
        (Ok(_), Some(what)) => Outcome::Failed(format!(
            "sakura did not end by itself; stopped by process id: {what}"
        )),
        (Err(_), Some(what)) => Outcome::Failed(format!(
            "sakura did not save the file and did not end; stopped by process id: {what}"
        )),
        (Err(_), None) => Outcome::Failed("sakura ended without saving the file".into()),
    }
}

fn run_ume(scratch: &Scratch, case: &Case) -> Outcome {
    let (dir, input, mac, out) = prepare(scratch, "ume", case);
    let built = PathBuf::from(env!("CARGO_BIN_EXE_ume"));
    let host_name = if cfg!(windows) {
        "ume-script.exe"
    } else {
        "ume-script"
    };
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let exe = bin.join(built.file_name().unwrap());
    fs::copy(&built, &exe).unwrap();
    let host = built.with_file_name(host_name);
    if host.is_file() {
        fs::copy(&host, bin.join(host_name)).unwrap();
    }

    let log = dir.join("output.log");
    let log_file = fs::File::create(&log).unwrap();
    let mut child = Command::new(&exe)
        .arg("--separate-process")
        .arg(format!("-M={}", mac.display()))
        .arg(format!("-CODE={}", case.code))
        .arg(&input)
        .current_dir(&dir)
        .env("HOME", &dir)
        .env("USERPROFILE", &dir)
        .env("APPDATA", dir.join("appdata"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("RUST_BACKTRACE", "1")
        .stdin(Stdio::null())
        .stdout(log_file.try_clone().unwrap())
        .stderr(log_file)
        .spawn()
        .expect("start ume");
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if started.elapsed() > UME_TIME_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(100));
    };
    let mut log_text = String::new();
    if let Ok(mut f) = fs::File::open(&log) {
        let mut bytes = Vec::new();
        let _ = f.read_to_end(&mut bytes);
        log_text = String::from_utf8_lossy(&bytes).into_owned();
    }
    let tail: String = log_text
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .join(" / ");
    match (fs::read(&out), status) {
        (Ok(bytes), Some(s)) if s.success() => Outcome::Saved(bytes),
        (Ok(_), Some(s)) => Outcome::Failed(format!("ume ended with {s}: {tail}")),
        (Ok(_), None) => Outcome::Failed(format!("ume did not end in time: {tail}")),
        (Err(_), Some(s)) => Outcome::Failed(format!("ume ended with {s} without saving: {tail}")),
        (Err(_), None) => Outcome::Failed(format!("ume did not save and did not end: {tail}")),
    }
}

/// The bytes as text for a log: printable ASCII as it is, the rest as `\xNN`, with the line ends
/// marked, cut around `at`.
fn show(bytes: &[u8], at: usize) -> String {
    let from = at.saturating_sub(40);
    let to = (at + 80).min(bytes.len());
    let mut s = String::new();
    if from > 0 {
        s.push_str("...");
    }
    for &b in &bytes[from..to] {
        match b {
            b'\r' => s.push_str("\\r"),
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => {
                let _ = write!(s, "\\x{b:02x}");
            }
        }
    }
    if to < bytes.len() {
        s.push_str("...");
    }
    s
}

/// A readable account of how two outputs differ.
fn describe(sakura: &[u8], ume: &[u8]) -> String {
    let at = sakura
        .iter()
        .zip(ume)
        .position(|(a, b)| a != b)
        .unwrap_or(sakura.len().min(ume.len()));
    format!(
        "first difference at byte {at} (Sakura {} bytes, UME {} bytes)\n    Sakura: {}\n    UME:    {}",
        sakura.len(),
        ume.len(),
        show(sakura, at),
        show(ume, at)
    )
}

#[test]
#[ignore = "starts Sakura Editor and UME Editor: needs Windows, SAKURA_DIR and a display (see the module comment)"]
fn ume_saves_the_same_bytes_as_sakura() {
    let sakura_dir = PathBuf::from(
        std::env::var("SAKURA_DIR").expect("SAKURA_DIR: the folder with the unzipped sakura.exe"),
    );
    assert!(
        sakura_dir.join("sakura.exe").is_file(),
        "{} has no sakura.exe",
        sakura_dir.display()
    );
    let out_dir = std::env::var_os("ORACLE_OUT")
        .map_or_else(|| PathBuf::from(env!("CARGO_TARGET_TMPDIR")), PathBuf::from);
    fs::create_dir_all(&out_dir).unwrap();

    let cases = load_cases();
    let known = load_known();
    assert!(!cases.is_empty(), "no case to run");

    let mut report = String::new();
    let mut new_differences = Vec::new();
    let mut stale = Vec::new();
    let mut sakura_failures = Vec::new();
    let (mut same, mut known_count) = (0, 0);
    let mut sakura_ran = false;
    let mut processed = 0;
    for case in &cases {
        processed += 1;
        let scratch = Scratch::new(&case.name);
        let sakura = run_sakura(&scratch, case, &sakura_dir);
        let ume = run_ume(&scratch, case);
        sakura_ran |= matches!(sakura, Outcome::Saved(_));
        let (status, detail) = match (&sakura, &ume) {
            (Outcome::Failed(why), _) => {
                // The test itself is wrong (or the machine): never allow-listed.
                sakura_failures.push(format!("{}: {why}", case.name));
                ("SAKURA-FAILED", why.clone())
            }
            (Outcome::Saved(s), Outcome::Saved(u)) if s == u => ("same", String::new()),
            (Outcome::Saved(s), Outcome::Saved(u)) => ("DIFF", describe(s, u)),
            (Outcome::Saved(s), Outcome::Failed(why)) => (
                "DIFF",
                format!("UME wrote no file: {why} (Sakura wrote {} bytes)", s.len()),
            ),
        };
        let is_diff = status == "DIFF";
        let reason = known.get(&case.name);
        let label = match (is_diff, reason) {
            (false, None) => {
                if status == "same" {
                    same += 1;
                }
                status.to_string()
            }
            (true, Some(_)) => {
                known_count += 1;
                "DIFF (known)".to_string()
            }
            (true, None) => {
                new_differences.push(case.name.clone());
                "DIFF (NEW)".to_string()
            }
            (false, Some(_)) => {
                if status == "same" {
                    stale.push(case.name.clone());
                }
                format!("{status} (listed as known!)")
            }
        };
        let _ = writeln!(report, "[{label}] {}", case.name);
        if !detail.is_empty() {
            let _ = writeln!(report, "    {}", detail.replace('\n', "\n    "));
        }
        if let Some(reason) = reason {
            let _ = writeln!(report, "    known: {reason}");
        }
        if is_diff || status == "SAKURA-FAILED" {
            let case_out = out_dir.join(&case.name);
            let _ = fs::create_dir_all(&case_out);
            let _ = fs::write(case_out.join("case.mac"), &case.mac);
            let _ = fs::copy(
                oracle_dir().join("inputs").join(&case.input),
                case_out.join("input"),
            );
            if let Outcome::Saved(b) = &sakura {
                let _ = fs::write(case_out.join("sakura.out"), b);
            }
            if let Outcome::Saved(b) = &ume {
                let _ = fs::write(case_out.join("ume.out"), b);
            }
        }
        // Sakura fails on the first cases: a setup problem, not a result; do not wait for the rest.
        if !sakura_ran && sakura_failures.len() >= 3 {
            println!(
                "Sakura failed on the first {} cases: stopping",
                sakura_failures.len()
            );
            break;
        }
        // Print as we go: a hang in a later case must not hide the earlier results.
        println!("[{label}] {}", case.name);
        if !detail.is_empty() {
            println!("    {}", detail.replace('\n', "\n    "));
        }
    }
    let summary = format!(
        "{processed} of {} cases run: {same} same, {known_count} known differences, {} new differences, {} stale list entries, {} Sakura failures",
        cases.len(),
        new_differences.len(),
        stale.len(),
        sakura_failures.len()
    );
    let _ = writeln!(report, "\n{summary}");
    fs::write(out_dir.join("report.txt"), &report).unwrap();
    println!("\n{summary}");

    // Names in the list that match no case are typos (only checked when every case ran).
    let mut unknown = Vec::new();
    if std::env::var("ORACLE_ONLY").is_err() {
        unknown = known
            .keys()
            .filter(|k| !cases.iter().any(|c| &c.name == *k))
            .cloned()
            .collect();
    }
    assert!(
        new_differences.is_empty()
            && stale.is_empty()
            && sakura_failures.is_empty()
            && unknown.is_empty(),
        "{summary}\nnew differences (add to known_differences.txt with a reason, or fix UME): {new_differences:?}\nlisted but no longer different (delete the line): {stale:?}\nSakura did not run: {sakura_failures:?}\nlisted but no such case: {unknown:?}"
    );
}
