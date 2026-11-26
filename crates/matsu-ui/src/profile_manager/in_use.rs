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
//! unlocked by [`is_held`], which removes it. A process removes only its own file when it ends.
//!
//! * Windows: the lock is a share mode that refuses every other open, so it holds against every program.
//! * macOS and Linux: an advisory `flock`, seen by every MATSU Editor (and the manager), not by a program
//!   that does not ask for a lock.

use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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

/// Makes the file of the holder `id` in `dir` and locks it. The folder is made if it is not there. A
/// second holder with the same `id` is refused (the process number is unique among the running ones).
pub(crate) fn hold_as(dir: &Path, id: u32) -> io::Result<FileLock> {
    std::fs::create_dir_all(dir)?;
    let path = file_of(dir, id);
    // `is_held` may remove the file between the two steps (it takes it for the file of a process that
    // is gone): make it again, a few times
    let mut last = None;
    for _ in 0..3 {
        // the handle that makes the file is closed before the lock opens it (Windows shares nothing)
        OpenOptions::new().create(true).append(true).open(&path)?;
        match FileLock::acquire(&path, ShareMode::DenyReadWrite) {
            Ok(lock) => return Ok(lock),
            Err(error) if path.exists() => return Err(error),
            Err(error) => last = Some(error),
        }
    }
    Err(last.unwrap_or_else(|| io::Error::other("the mark could not be made")))
}

/// Whether any editor holds a mark in the folder. A file that can be locked belongs to a process that is
/// gone: it is removed, so that the files of crashed processes do not pile up. A lock that cannot be
/// taken, for whatever reason, counts as in use: the safe answer.
pub(crate) fn is_held(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut held = false;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with(PREFIX) && name.ends_with(SUFFIX)) {
            continue;
        }
        let path = entry.path();
        match FileLock::acquire(&path, ShareMode::DenyReadWrite) {
            Ok(lock) => {
                // the lock goes first: Windows does not delete a file that is open
                drop(lock);
                let _ = std::fs::remove_file(&path);
            }
            Err(_) => held = true,
        }
    }
    held
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
    if let Ok(lock) = hold_as(&dir, id)
        && let Ok(mut held) = HELD.lock()
    {
        *held = Some((file_of(&dir, id), lock));
    }
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
