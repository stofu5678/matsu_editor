//! The history in the settings file, read and written so that several windows can use it at once.
//!
//! Sakura keeps the history in shared memory that every window of the editor maps, and writes
//! `sakura.ini` when the last window closes. UME Editor windows are not tied to one process, and
//! the file is also where the user's settings are, so the history is written like this:
//!
//! * [`HistoryStore::update`] takes a lock (a file `<settings file>.mru-lock` made with
//!   `create_new`), reads the file again, applies the change to what is in the file now, writes
//!   the section `[MRU]` and everything else as it was, and releases the lock. Two windows that
//!   add a file at the same moment both end up in the history; neither overwrites the other with
//!   an old copy.
//! * The file is replaced as a whole ([`Profile::save`]), so a crash leaves the old file or the
//!   new one.
//! * A lock older than 20 seconds is left by a process that died and is taken over.

use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::History;
use crate::profile::Profile;

/// A lock this old is taken over.
const STALE_LOCK: Duration = Duration::from_secs(20);
/// How long to wait for the lock before giving up.
const LOCK_TIMEOUT: Duration = Duration::from_secs(3);
const POLL: Duration = Duration::from_millis(5);

/// The history in a settings file.
#[derive(Clone, Debug)]
pub struct HistoryStore {
    path: PathBuf,
    case_insensitive: bool,
}

/// Whether an error only says "try again in a moment". On Windows a file that another handle has
/// open or that is waiting to be deleted cannot be replaced, deleted or created again: that is
/// `ERROR_ACCESS_DENIED` (5), `ERROR_SHARING_VIOLATION` (32) or `ERROR_LOCK_VIOLATION` (33). Other
/// systems report a real permission problem, which retrying does not cure.
fn is_busy(e: &io::Error) -> bool {
    cfg!(windows)
        && (e.kind() == io::ErrorKind::PermissionDenied
            || matches!(e.raw_os_error(), Some(5 | 32 | 33)))
}

/// Runs `op` again while it fails with a busy error, until [`LOCK_TIMEOUT`] has passed.
fn retry<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let start = std::time::Instant::now();
    loop {
        match op() {
            Err(e) if is_busy(&e) && start.elapsed() < LOCK_TIMEOUT => std::thread::sleep(POLL),
            other => return other,
        }
    }
}

/// Removes the lock file when it goes out of scope.
struct Lock(PathBuf);

impl Drop for Lock {
    fn drop(&mut self) {
        // On Windows a file that was just read may not be deleted for a moment.
        for _ in 0..200 {
            match std::fs::remove_file(&self.0) {
                Err(e) if is_busy(&e) => std::thread::sleep(POLL),
                _ => break,
            }
        }
    }
}

impl HistoryStore {
    /// The history in `path`. `case_insensitive` says whether two paths that differ in case are the
    /// same file (Windows and macOS: yes).
    pub fn new(path: impl Into<PathBuf>, case_insensitive: bool) -> Self {
        Self {
            path: path.into(),
            case_insensitive,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(test)]
    pub(super) fn lock_path(&self) -> PathBuf {
        lock_path(&self.path)
    }

    /// The history as the file has it now. A file that is missing or cannot be read is an empty
    /// history.
    pub fn load(&self) -> History {
        let profile = read_profile(&self.path).unwrap_or_default();
        History::from_profile(&profile, self.case_insensitive)
    }

    /// Applies `change` to the history in the file and writes it back. Nothing is written when the
    /// history does not change. Returns the history as it is after the change.
    pub fn update(&self, change: impl FnOnce(&mut History)) -> io::Result<History> {
        let mut result = None;
        update_profile(&self.path, |profile| {
            let before = History::from_profile(profile, self.case_insensitive);
            let mut history = before.clone();
            change(&mut history);
            let changed = history != before;
            if changed {
                history
                    .write_to(profile)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            }
            result = Some(history);
            Ok(changed)
        })?;
        result.ok_or_else(|| io::Error::other("the change was not applied"))
    }
}

/// The lock file of a settings file. One lock covers the whole file, whichever section is written.
fn lock_path(settings: &Path) -> PathBuf {
    let mut name = settings.as_os_str().to_owned();
    name.push(".mru-lock");
    PathBuf::from(name)
}

fn read_profile(path: &Path) -> io::Result<Profile> {
    match retry(|| Profile::load(path)) {
        Ok(profile) => Ok(profile),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Profile::new()),
        Err(e) => Err(e),
    }
}

fn lock(settings: &Path) -> io::Result<Lock> {
    let path = lock_path(settings);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let start = std::time::Instant::now();
    loop {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => return Ok(Lock(path)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists || is_busy(&e) => {
                let age = std::fs::metadata(&path)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| SystemTime::now().duration_since(t).ok());
                if age.is_some_and(|a| a > STALE_LOCK) {
                    // Two processes may take over the same dead lock; that is rare (the lock
                    // is held for milliseconds) and the cost is one lost addition.
                    match std::fs::remove_file(&path) {
                        Ok(()) => continue,
                        Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                        // Busy: the owner is deleting it right now; wait below.
                        Err(e) if is_busy(&e) => {}
                        Err(e) => return Err(e),
                    }
                }
                if start.elapsed() > LOCK_TIMEOUT {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "the history is locked by another window",
                    ));
                }
                std::thread::sleep(POLL);
            }
            Err(e) => return Err(e),
        }
    }
}

/// The one way to change the settings file while other windows may do the same: takes the lock,
/// reads the file again, lets `change` edit its [`Profile`] (the history, the search strings, ...)
/// and writes it back with [`Profile::save`] if `change` returns `true`. Every writer of a section
/// of the file goes through here, so that two windows that write different sections at the same
/// moment do not overwrite each other with an old copy. A file that does not exist is an empty
/// profile; one that cannot be read is an error (and is not touched).
pub fn update_profile(
    path: &Path,
    change: impl FnOnce(&mut Profile) -> io::Result<bool>,
) -> io::Result<()> {
    let _lock = lock(path)?;
    let mut profile = read_profile(path)?;
    if change(&mut profile)? {
        retry(|| profile.save(path))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_busy_error_is_tried_again_only_where_windows_makes_it() {
        let mut calls = 0;
        let result = retry(|| {
            calls += 1;
            if calls < 3 {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            } else {
                Ok(calls)
            }
        });
        if cfg!(windows) {
            assert_eq!(result.unwrap(), 3);
        } else {
            // A real permission error is returned at once.
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
            assert_eq!(calls, 1);
        }
        assert!(is_busy(&io::Error::from_raw_os_error(32)) == cfg!(windows));
        assert!(!is_busy(&io::Error::from(io::ErrorKind::NotFound)));
    }

    #[test]
    fn other_errors_are_not_retried() {
        let mut calls = 0;
        let result: io::Result<()> = retry(|| {
            calls += 1;
            Err(io::Error::from(io::ErrorKind::InvalidData))
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }
}
