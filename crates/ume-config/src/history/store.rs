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
//! * A lock older than [`STALE_LOCK`] is left by a process that died and is taken over.

use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::History;
use crate::profile::Profile;

/// A lock this old is taken over.
const STALE_LOCK: Duration = Duration::from_secs(20);
/// How long to wait for the lock before giving up.
const LOCK_TIMEOUT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(5);

/// The history in a settings file.
#[derive(Clone, Debug)]
pub struct HistoryStore {
    path: PathBuf,
    case_insensitive: bool,
}

/// Removes the lock file when it goes out of scope.
struct Lock(PathBuf);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
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

    pub(super) fn lock_path(&self) -> PathBuf {
        let mut name = self.path.as_os_str().to_owned();
        name.push(".mru-lock");
        PathBuf::from(name)
    }

    fn read_profile(&self) -> io::Result<Profile> {
        match Profile::load(&self.path) {
            Ok(profile) => Ok(profile),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Profile::new()),
            Err(e) => Err(e),
        }
    }

    /// The history as the file has it now. A file that is missing or cannot be read is an empty
    /// history.
    pub fn load(&self) -> History {
        let profile = self.read_profile().unwrap_or_default();
        History::from_profile(&profile, self.case_insensitive)
    }

    fn lock(&self) -> io::Result<Lock> {
        let path = self.lock_path();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let start = std::time::Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => return Ok(Lock(path)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    let age = std::fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| SystemTime::now().duration_since(t).ok());
                    if age.is_some_and(|a| a > STALE_LOCK) {
                        let _ = std::fs::remove_file(&path);
                        continue;
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

    /// Applies `change` to the history in the file and writes it back. Nothing is written when the
    /// history does not change. Returns the history as it is after the change.
    pub fn update(&self, change: impl FnOnce(&mut History)) -> io::Result<History> {
        let _lock = self.lock()?;
        let mut profile = self.read_profile()?;
        let before = History::from_profile(&profile, self.case_insensitive);
        let mut history = before.clone();
        change(&mut history);
        if history != before {
            history
                .write_to(&mut profile)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            profile.save(&self.path)?;
        }
        Ok(history)
    }
}
