//! The mark of a profile that is in use (the user has not decided how to treat a profile that runs; this
//! is option A of the PR: the editor says it is using the profile, and the manager refuses to rename or
//! delete it. If the user chooses B, Sakura's way of leaving it to the OS, this file and its callers are
//! removed).
//!
//! A running editor that has a named profile holds an exclusive lock on `in-use.lock` in the folder of
//! the profile (`matsu_platform::file_lock`). The lock belongs to the process: when the editor ends, or
//! crashes, the OS lets it go, so a mark that is left behind by a crash does not hold anything. The file
//! itself is removed when the editor ends in the usual way, and only an informal sign.
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

/// The name of the file in the folder of a profile.
const MARKER: &str = "in-use.lock";

/// What this process holds: it has one profile for its whole life, so there is one mark (a global is
/// right for a thing that exists once per application, ownership.md 3).
static HELD: Mutex<Option<(PathBuf, FileLock)>> = Mutex::new(None);

/// Makes the file in `dir` and locks it. The folder is made if it is not there.
pub(crate) fn hold(dir: &Path) -> io::Result<FileLock> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(MARKER);
    // the handle that makes the file is closed before the lock opens it (Windows shares nothing)
    OpenOptions::new().create(true).append(true).open(&path)?;
    FileLock::acquire(&path, ShareMode::DenyReadWrite)
}

/// Whether another holder has the mark of the folder. A folder without the file is not in use.
pub(crate) fn is_held(dir: &Path) -> bool {
    let path = dir.join(MARKER);
    if !path.exists() {
        return false;
    }
    // a lock that cannot be taken, for whatever reason, counts as in use: the safe answer
    FileLock::acquire(&path, ShareMode::DenyReadWrite).is_err()
}

/// The start of an editor whose profile has a name: marks the folder of the profile as in use for as
/// long as the process runs. Another editor of the same profile (it hands its launch over and ends)
/// does not get the mark, which is fine: the first one has it.
pub(crate) fn mark_this_process() {
    let env = Environment::from_process();
    let Some(name) = env.profile_name.clone().filter(|name| !name.is_empty()) else {
        return;
    };
    let Some(dir) = profile_manager::profile_dir(Os::current(), &env, &name) else {
        return;
    };
    if let Ok(lock) = hold(&dir)
        && let Ok(mut held) = HELD.lock()
    {
        *held = Some((dir, lock));
    }
}

/// The editor ends: lets go of the mark and removes the file.
pub(crate) fn release() {
    let Ok(mut held) = HELD.lock() else {
        return;
    };
    if let Some((dir, lock)) = held.take() {
        // the lock goes first: Windows does not delete a file that is open
        drop(lock);
        let _ = std::fs::remove_file(dir.join(MARKER));
    }
}
