//! What the program asks the operating system: memory, the machine, the OS cache. No dependency:
//! `/proc` on Linux, `ps` on macOS and PowerShell on Windows.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::report::Machine;

const MIB: f64 = 1024.0 * 1024.0;

/// The resident memory of this process now and its peak, in MiB (`None` where the OS does not tell).
pub fn memory_mib() -> (Option<f64>, Option<f64>) {
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
        let rss = output("ps", &["-o", "rss=", "-p", &std::process::id().to_string()])
            .and_then(|s| s.trim().parse::<f64>().ok())
            .map(|k| k / 1024.0);
        (rss, None)
    } else if cfg!(windows) {
        let script = format!(
            "$p = Get-Process -Id {}; \"$($p.WorkingSet64) $($p.PeakWorkingSet64)\"",
            std::process::id()
        );
        let out = output("powershell", &["-NoProfile", "-Command", &script]).unwrap_or_default();
        let mut it = out
            .split_whitespace()
            .filter_map(|v| v.parse::<f64>().ok())
            .map(|b| b / MIB);
        (it.next(), it.next())
    } else {
        (None, None)
    }
}

/// Starts the peak of the resident memory again from now (Linux only: `clear_refs` 5). Returns
/// whether it worked; where it did not, the peak is the peak of the whole process so far.
pub fn reset_peak_memory() -> bool {
    cfg!(target_os = "linux") && fs::write("/proc/self/clear_refs", "5").is_ok()
}

/// Drops the OS's cache of file data, so that a read comes from the disk. Only Linux as root can.
/// Returns whether it did.
pub fn drop_os_cache() -> bool {
    if cfg!(target_os = "linux") {
        // Dirty pages cannot be dropped: write them out first.
        let _ = Command::new("sync").status();
        fs::write("/proc/sys/vm/drop_caches", "3").is_ok()
    } else {
        false
    }
}

/// What the report says about the OS cache, from the answer of [`drop_os_cache`].
pub fn drop_note(dropped: bool) -> &'static str {
    if dropped {
        "dropped before each run and before each cold read (so a first read comes from the disk)"
    } else if cfg!(target_os = "linux") {
        "not dropped (needs root): reads may come from the OS cache"
    } else {
        "not dropped (no way without extra tools): reads may come from the OS cache"
    }
}

fn output(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The machine, the OS, the commit. Nothing that names the user or the computer.
pub fn machine(dir: &Path, cache_dropped: &str) -> Machine {
    let (os, cpu, memory_mib, filesystem) = if cfg!(target_os = "linux") {
        let release = fs::read_to_string("/etc/os-release").unwrap_or_default();
        let name = release
            .lines()
            .find_map(|l| l.strip_prefix("PRETTY_NAME="))
            .map(|v| v.trim_matches('"').to_owned())
            .unwrap_or_else(|| "linux".to_owned());
        let kernel = fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
        let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let cpu = cpuinfo
            .lines()
            .find_map(|l| l.strip_prefix("model name"))
            .and_then(|v| v.split(':').nth(1))
            .map(|v| v.trim().to_owned())
            .unwrap_or_else(|| "unknown".to_owned());
        let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let memory = meminfo
            .lines()
            .find_map(|l| l.strip_prefix("MemTotal:"))
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            .map(|k| k / 1024);
        let fs_type = output("stat", &["-f", "-c", "%T", &dir.to_string_lossy()])
            .unwrap_or_else(|| "unknown".to_owned());
        (
            format!("{name}, kernel {}", kernel.trim()),
            cpu,
            memory,
            fs_type,
        )
    } else if cfg!(windows) {
        let cpu = std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_owned());
        let memory = output(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
            ],
        )
        .and_then(|s| s.parse::<u64>().ok())
        .map(|b| b / (1024 * 1024));
        let version = output("cmd", &["/C", "ver"]).unwrap_or_else(|| "windows".to_owned());
        let drive = dir
            .canonicalize()
            .ok()
            .and_then(|p| p.to_string_lossy().chars().find(char::is_ascii_alphabetic))
            .map(|c| c.to_string());
        let fs_type = drive
            .and_then(|d| {
                output(
                    "powershell",
                    &[
                        "-NoProfile",
                        "-Command",
                        &format!("(Get-Volume -DriveLetter {d}).FileSystemType"),
                    ],
                )
            })
            .unwrap_or_else(|| "unknown".to_owned());
        (version, cpu, memory, fs_type)
    } else {
        let name = output("sw_vers", &["-productVersion"]).unwrap_or_default();
        let cpu = output("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_default();
        let memory = output("sysctl", &["-n", "hw.memsize"])
            .and_then(|s| s.parse::<u64>().ok())
            .map(|b| b / (1024 * 1024));
        (
            format!("{} {name}", std::env::consts::OS),
            cpu,
            memory,
            "unknown".to_owned(),
        )
    };
    let commit = std::env::var("GITHUB_SHA")
        .ok()
        .or_else(|| output("git", &["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_owned());
    Machine {
        os,
        arch: std::env::consts::ARCH.to_owned(),
        cpu,
        cpus: std::thread::available_parallelism().map_or(1, usize::from),
        memory_mib,
        commit,
        rustc: output("rustc", &["--version"]).unwrap_or_else(|| "unknown".to_owned()),
        // Only the last part: the full path may hold a user's name.
        dir: dir
            .file_name()
            .map_or_else(|| ".".to_owned(), |n| n.to_string_lossy().into_owned()),
        filesystem,
        started: iso_utc(SystemTime::now()),
        cache_dropped: cache_dropped.to_owned(),
        release_build: !cfg!(debug_assertions),
    }
}

/// `2026-11-28T12:34:56Z`.
pub fn iso_utc(time: SystemTime) -> String {
    let secs = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    // Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn the_date_is_iso() {
        assert_eq!(iso_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        // 2026-11-28 12:34:56 UTC.
        let t = UNIX_EPOCH + Duration::from_secs(1_795_869_296);
        assert_eq!(iso_utc(t), "2026-11-28T12:34:56Z");
        // A leap day.
        let t = UNIX_EPOCH + Duration::from_secs(951_782_400);
        assert_eq!(iso_utc(t), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn the_machine_has_an_os_and_a_commit() {
        let m = machine(Path::new("."), "no");
        assert!(!m.os.is_empty() && !m.commit.is_empty());
        assert!(m.cpus >= 1);
    }
}
