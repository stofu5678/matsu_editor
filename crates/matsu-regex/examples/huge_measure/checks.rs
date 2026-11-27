//! The checks of the operating system's behavior of ADR-0020, 8.1: what happens to a file that is
//! open by ranges when another program replaces it by a rename, deletes it, truncates it or
//! appends to it. They run on a small file, through `RangedFile` (the source of the editor), and
//! say what the OS did and what a read gave afterwards. The editor never maps the file into
//! memory, so there is no SIGBUS to see (ADR-0020, 1).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use matsu_core::buffer::Source;
use matsu_core::file::{RangedFile, RangedOptions, is_changed_outside};

use crate::generate::{Kind, write_file};
use crate::report::Check;

const PIECE: u64 = 4096;

/// A folder of this program's own, removed when dropped.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// What a read of `start..end` of the source gave.
fn read_result(source: &RangedFile, start: u64, end: u64) -> String {
    match source.read(start, end) {
        Ok(bytes) => format!("ok ({} bytes)", bytes.len()),
        Err(e) if is_changed_outside(&e) => "changed outside (detected)".to_owned(),
        Err(e) => format!("error: {:?}: {e}", e.kind()),
    }
}

fn call_result<T>(result: std::io::Result<T>) -> String {
    match result {
        Ok(_) => "ok".to_owned(),
        Err(e) => format!("error: {:?}: {e}", e.kind()),
    }
}

/// Runs the four checks in `dir` with a file of about `size` bytes.
pub fn run(dir: &Path, size: u64) -> Vec<Check> {
    let scratch = Scratch(dir.join(format!("huge-measure-checks-{}", std::process::id())));
    if let Err(e) = fs::create_dir_all(&scratch.0) {
        return vec![failed("setup", format!("cannot make a folder: {e}"))];
    }
    let mut checks = Vec::new();
    for (name, op) in [
        ("replace the open file by a rename", Op::Rename),
        ("delete the open file", Op::Delete),
        ("truncate the open file to half", Op::Truncate),
        ("append to the open file", Op::Append),
    ] {
        checks.push(one(&scratch.0, name, op, size));
    }
    checks
}

fn failed(name: &str, message: String) -> Check {
    Check {
        name: name.to_owned(),
        action: message,
        read_uncached: "-".to_owned(),
        read_cached: "-".to_owned(),
    }
}

#[derive(Clone, Copy)]
enum Op {
    Rename,
    Delete,
    Truncate,
    Append,
}

fn one(dir: &Path, name: &str, op: Op, size: u64) -> Check {
    let path = dir.join("open.txt");
    let made = File::create(&path)
        .and_then(|mut f| write_file(Kind::S, size, &mut f).and_then(|_| f.sync_all()));
    if let Err(e) = made {
        return failed(name, format!("cannot make the file: {e}"));
    }
    let len = fs::metadata(&path).map_or(0, |m| m.len());
    let options = RangedOptions {
        block_size: 64 * 1024,
        cache_blocks: 16,
    };
    // Two sources of their own, so that one read cannot change what the other one sees: the first
    // read of a source after the change may be the one that finds it (`RangedFile` checks the
    // file's length and time when it loads a block, and then remembers the change for good).
    // `cached` has its first piece in its cache and is read only there; `uncached` has loaded
    // nothing of the place it is read at.
    let open = || RangedFile::open_with(&path, options);
    let (cached, uncached) = match (open(), open()) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return failed(name, format!("cannot open the file: {e}")),
    };
    if let Err(e) = cached.read(0, PIECE) {
        return failed(name, format!("cannot read the file: {e}"));
    }

    let action = match op {
        Op::Rename => {
            let other = dir.join("other.txt");
            let made = File::create(&other).and_then(|mut f| {
                // Another file of about the same size, with other bytes in it.
                write_file(Kind::T, len / 2 + 1, &mut f).map(|_| ())
            });
            match made {
                Ok(()) => call_result(fs::rename(&other, &path)),
                Err(e) => format!("cannot make the new file: {e}"),
            }
        }
        Op::Delete => call_result(fs::remove_file(&path)),
        Op::Truncate => call_result(
            OpenOptions::new()
                .write(true)
                .open(&path)
                .and_then(|f| f.set_len(len / 2)),
        ),
        Op::Append => call_result(
            OpenOptions::new()
                .append(true)
                .open(&path)
                .and_then(|mut f| f.write_all(&[b'x'; 4096])),
        ),
    };
    Check {
        name: name.to_owned(),
        action,
        read_uncached: read_result(&uncached, len - PIECE, len),
        read_cached: read_result(&cached, 0, PIECE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("matsu-huge-measure-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    #[test]
    fn the_four_checks_run_and_say_what_happened() {
        let dir = temp_dir("checks");
        let checks = run(&dir.0, 256 * 1024);
        assert_eq!(checks.len(), 4);
        for c in &checks {
            assert!(!c.name.is_empty());
            assert!(!c.action.is_empty() && !c.read_uncached.is_empty());
            assert!(!c.action.starts_with("cannot"), "{c:?}");
        }
        // The folder of the checks is gone when they are done.
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
    }

    #[test]
    fn a_truncated_file_is_never_read_as_if_nothing_happened() {
        // Not OS specific: reading the part that is gone must not give bytes.
        let dir = temp_dir("truncate");
        let checks = run(&dir.0, 256 * 1024);
        let truncate = &checks[2];
        assert_eq!(truncate.action, "ok", "{truncate:?}");
        assert!(
            !truncate.read_uncached.starts_with("ok"),
            "a read past the new end gave bytes: {truncate:?}"
        );
    }
}
