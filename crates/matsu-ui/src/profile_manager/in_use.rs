//! The mark of a profile that is in use (the user has not decided how to treat a profile that runs; this
//! is option A of the PR: the editor says it is using the profile, and the manager refuses to rename or
//! delete it. If the user chooses B, Sakura's way of leaving it to the OS, this file and its callers are
//! removed).
//!
//! A running editor that has a named profile holds an exclusive lock on its own file,
//! `in-use-<process number>.lock`, in the folder of the profile (`matsu_platform::file_lock`). Several
//! editors can run with one profile (`--separate-process`, a Grep to the standard output), and each has a
//! file of its own, so the profile is in use as long as any of them is. The lock belongs to the process:
//! when the editor ends, or crashes, the OS lets it go. A file that a crashed process left is found
//! unlocked by [`is_held`] and does not count; it is not deleted (see there). A process removes only its
//! own file when it ends, after its last write ([`release_after`]).
//!
//! * Windows: the lock is a share mode that refuses every other open, so it holds against every program.
//! * macOS and Linux: an advisory `flock`, seen by every MATSU Editor (and the manager), not by a program
//!   that does not ask for a lock.

use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use matsu_config::location::{Environment, Os};
use matsu_config::profile_manager;
use matsu_platform::file_lock::{FileLock, ShareMode};

/// The names of the files: `in-use-` and the number of the process, `.lock`.
const PREFIX: &str = "in-use-";
const SUFFIX: &str = ".lock";

/// What this process holds: it has one profile for its whole life, so there is one mark (a global is
/// right for a thing that exists once per application, ownership.md 3).
static HELD: Mutex<Option<(PathBuf, FileLock)>> = Mutex::new(None);

fn file_of(dir: &Path, id: u32) -> PathBuf {
    dir.join(format!("{PREFIX}{id}{SUFFIX}"))
}

/// How often a mark is tried, and the wait between two tries. A check ([`is_held`]) holds a file for an
/// instant while it tries its lock, and the mark of a process that starts then is refused for that
/// instant.
const TRIES: u32 = 10;
const TRY_WAIT: Duration = Duration::from_millis(10);

/// Makes the file of the holder `id` in `dir` and locks it. The folder is made if it is not there. A
/// file that an earlier process with the same number left is used again. A second holder with the same
/// `id` is refused (the process number is unique among the running ones). Every failure is tried again a
/// few times.
pub(crate) fn hold_as(dir: &Path, id: u32) -> io::Result<FileLock> {
    std::fs::create_dir_all(dir)?;
    let path = file_of(dir, id);
    let mut tries = 0;
    loop {
        // the handle that makes the file is closed before the lock opens it (Windows shares nothing)
        let locked = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|_| FileLock::acquire(&path, ShareMode::DenyReadWrite));
        match locked {
            Ok(lock) => return Ok(lock),
            Err(error) => {
                tries += 1;
                if tries >= TRIES {
                    return Err(error);
                }
                std::thread::sleep(TRY_WAIT);
            }
        }
    }
}

/// Whether any editor holds a mark in the folder: some `in-use-*.lock` that cannot be locked. A file that
/// can be locked is let go at once and does not count (the file of a process that ended or crashed; it
/// stays, which does no harm, and a process with the same number uses it again). Nothing is deleted
/// here: a file removed between the lock and its release could be the one a starting editor locks in that
/// instant, and that editor would hold a file that no one can find. A file that is gone meanwhile is not a
/// holder. Any other error counts as in use: the safe answer.
pub(crate) fn is_held(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with(PREFIX) && name.ends_with(SUFFIX)) {
            return false;
        }
        match FileLock::acquire(&entry.path(), ShareMode::DenyReadWrite) {
            Ok(_lock) => false,
            Err(error) => error.kind() != io::ErrorKind::NotFound,
        }
    })
}

/// The start of an editor whose profile has a name: marks the folder of the profile as in use for as
/// long as the process runs.
pub(crate) fn mark_this_process() {
    let env = Environment::from_process();
    let Some(name) = env.profile_name.clone().filter(|name| !name.is_empty()) else {
        return;
    };
    let Some(dir) = profile_manager::profile_dir(Os::current(), &env, &name) else {
        return;
    };
    let id = std::process::id();
    match hold_as(&dir, id) {
        Ok(lock) => {
            if let Ok(mut held) = HELD.lock() {
                *held = Some((file_of(&dir, id), lock));
            }
        }
        // the editor runs, but the profile manager would not know that it is in use: say so
        Err(error) => eprintln!(
            "matsu: the profile {name} could not be marked as in use ({}): {error}",
            dir.display()
        ),
    }
}

/// Runs the writes of the end of the process, and only then lets go of the mark: another process must not
/// move the folder of the profile while they still write to it.
pub(crate) fn release_after<T>(writes: impl FnOnce() -> T) -> T {
    let result = writes();
    release();
    result
}

/// The editor ends: lets go of its own mark and removes its own file (no other).
pub(crate) fn release() {
    let Ok(mut held) = HELD.lock() else {
        return;
    };
    if let Some((path, lock)) = held.take() {
        // the lock goes first: Windows does not delete a file that is open
        drop(lock);
        let _ = std::fs::remove_file(path);
    }
}

/// Makes `lock` the mark of this process, as `mark_this_process` does, for the tests.
#[cfg(test)]
pub(crate) fn set_held_for_test(dir: &Path, lock: FileLock) {
    *HELD.lock().unwrap() = Some((file_of(dir, std::process::id()), lock));
}
