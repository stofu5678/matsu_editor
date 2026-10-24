//! The folder `unsaved/` and the operations on it (ADR-0022).

use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::file::{self, DocFile, FormatError};
use super::info::DocInfo;
use super::lock::{DirLock, LockFailure};
use super::session::{Session, is_valid_id};

const SESSION_FILE: &str = "session.ini";
const LOCK_FILE: &str = "lock";
const REJECTED_DIR: &str = "rejected";
const DOC_EXTENSION: &str = "doc";
const TMP_EXTENSION: &str = "tmp";

/// The folder for the unsaved documents next to the settings file `matsu.ini` at `ini_path`. With `-PROF`
/// the settings file is already in the folder of the profile, so each profile has its own.
pub fn unsaved_dir(ini_path: &Path) -> PathBuf {
    ini_path
        .parent()
        .map_or_else(|| PathBuf::from("unsaved"), |dir| dir.join("unsaved"))
}

/// Why [`Store::open`] did not give a store.
#[derive(Debug)]
pub enum StoreError {
    /// Another process holds the lock. The caller does not use the feature and says so once.
    Locked,
    /// The folder or the lock could not be made or used (no permission, a drive without locks). The caller
    /// does not use the feature either.
    Io(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Locked => write!(f, "the folder of unsaved documents is in use"),
            StoreError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// What is in the folder besides what the session lists.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    /// Documents whose file exists but the session does not list: a new document whose write was cut off
    /// before `session.ini` was written. Restore them (after the listed ones).
    pub orphans: Vec<String>,
    /// Ids the session lists but whose file is gone: a removal cut off before `session.ini` was written.
    /// Skip them.
    pub dangling: Vec<String>,
}

/// The folder, locked by this process.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    _lock: DirLock,
}

impl Store {
    /// Makes the folder if needed (private to the user on macOS and Linux) and takes its lock.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let dir = dir.into();
        fs::create_dir_all(&dir).map_err(StoreError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).map_err(StoreError::Io)?;
        }
        match DirLock::acquire(&dir.join(LOCK_FILE)) {
            Ok(lock) => Ok(Store { dir, _lock: lock }),
            Err(LockFailure::Held) => Err(StoreError::Locked),
            Err(LockFailure::Io(e)) => Err(StoreError::Io(e)),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The path of the file of document `id`.
    pub fn doc_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{DOC_EXTENSION}"))
    }

    /// A new id: 32 lower-case hex digits made from the time, the process id and a counter (so that no random
    /// number generator is needed).
    pub fn new_id() -> String {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!(
            "{:016x}{:08x}{:08x}",
            nanos as u64,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    }

    /// Writes the file of document `id`. `write_text` writes the text (UTF-8 without a BOM, with the line
    /// endings the document has) to the writer it is given; the text is never held as one string here.
    /// Written to a temporary file, flushed, renamed over the target, and the folder flushed. If anything
    /// fails the temporary file is removed and the old file stays.
    pub fn write_doc(
        &self,
        id: &str,
        info: &DocInfo,
        write_text: &mut dyn FnMut(&mut dyn Write) -> io::Result<()>,
    ) -> io::Result<()> {
        check_id(id)?;
        let header = file::header_bytes(info)?;
        self.write_atomic(&format!("{id}.{DOC_EXTENSION}"), &mut |w| {
            w.write_all(&header)?;
            write_text(w)
        })
    }

    /// Opens the file of document `id`: the information is read, the text is left to be read.
    pub fn open_doc(&self, id: &str) -> Result<DocFile, FormatError> {
        check_id(id)?;
        file::open(File::open(self.doc_path(id))?)
    }

    /// Deletes the file of document `id` (no error if it is already gone). It does not touch `session.ini`:
    /// see [`Store::discard_doc`].
    pub fn remove_doc(&self, id: &str) -> io::Result<()> {
        check_id(id)?;
        match fs::remove_file(self.doc_path(id)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => {
                sync_dir(&self.dir);
                Ok(())
            }
        }
    }

    /// Removes a document the way the order of ADR-0022 says: the file first, then `session.ini` without it.
    /// Returns the new session.
    pub fn discard_doc(&self, id: &str, session: &Session) -> io::Result<Session> {
        self.remove_doc(id)?;
        let next = session.without(id);
        self.write_session(&next)?;
        Ok(next)
    }

    /// Reads `session.ini`. A missing or unreadable file is an empty session.
    pub fn read_session(&self) -> Session {
        fs::read(self.dir.join(SESSION_FILE))
            .map(|bytes| Session::from_bytes(&bytes))
            .unwrap_or_default()
    }

    /// Writes `session.ini` the same safe way as a document.
    pub fn write_session(&self, session: &Session) -> io::Result<()> {
        let bytes = session
            .to_bytes()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        self.write_atomic(SESSION_FILE, &mut |w| w.write_all(&bytes))
    }

    /// Compares the folder with `session`.
    pub fn scan(&self, session: &Session) -> io::Result<Scan> {
        let mut on_disk = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some(DOC_EXTENSION) {
                continue;
            }
            if let Some(stem) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| is_valid_id(s))
            {
                on_disk.push(stem.to_owned());
            }
        }
        on_disk.sort();
        let orphans = on_disk
            .iter()
            .filter(|id| !session.contains(id))
            .cloned()
            .collect();
        let dangling = session
            .ids()
            .filter(|id| on_disk.binary_search_by(|d| d.as_str().cmp(id)).is_err())
            .map(str::to_owned)
            .collect();
        Ok(Scan { orphans, dangling })
    }

    /// Deletes the temporary files a cut-off write left. Only the process that holds the lock calls this, so
    /// no write of its own is running. Returns how many were deleted.
    pub fn remove_tmp(&self) -> io::Result<usize> {
        let mut removed = 0;
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some(TMP_EXTENSION)
                && path.is_file()
                && fs::remove_file(&path).is_ok()
            {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Moves the file of `id` to `rejected/`, where it stays for [`REJECTED_KEEP`]: for a document that could not
    /// be restored, so the user's text is not deleted without a word.
    ///
    /// The day it was rejected is in the name, `rejected/<id>.<seconds since the epoch>.doc`, because the
    /// modification time of the file is the day of its last write (a rename keeps it) and cannot always be
    /// changed (a read-only file). The name is moved by the same rename, so a restart cannot revert it.
    pub fn reject(&self, id: &str) -> io::Result<()> {
        self.reject_at(id, SystemTime::now())
    }

    /// [`Store::reject`] on the day `now`; the tests give it.
    pub fn reject_at(&self, id: &str, now: SystemTime) -> io::Result<()> {
        check_id(id)?;
        let rejected = self.dir.join(REJECTED_DIR);
        fs::create_dir_all(&rejected)?;
        let secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let moved = rejected.join(format!("{id}.{secs}.{DOC_EXTENSION}"));
        fs::rename(self.doc_path(id), &moved)?;
        // Both folders changed: the entry that went and the entry that came.
        sync_dir(&rejected);
        sync_dir(&self.dir);
        Ok(())
    }

    /// Deletes what is in `rejected/` and was last changed `max_age` before `now`. Returns how many.
    pub fn purge_rejected(&self, now: SystemTime, max_age: Duration) -> io::Result<usize> {
        let rejected = self.dir.join(REJECTED_DIR);
        let entries = match fs::read_dir(&rejected) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e),
        };
        let mut removed = 0;
        for entry in entries {
            let entry = entry?;
            // The day in the name; for a file that has none, the last write.
            let since = rejected_day(&entry.file_name().to_string_lossy())
                .or_else(|| entry.metadata().and_then(|m| m.modified()).ok());
            let old = since
                .and_then(|t| now.duration_since(t).ok())
                .is_some_and(|age| age >= max_age);
            if old && entry.path().is_file() && fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Deletes everything this module made: `session.ini`, the documents, the temporary files and `rejected/`.
    /// The folder and the `lock` file stay (see the module comment of `lock`). The caller has stopped its
    /// writer first.
    pub fn clear(&self) -> io::Result<()> {
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == LOCK_FILE {
                continue;
            }
            if path.is_dir() {
                if name == REJECTED_DIR {
                    fs::remove_dir_all(&path)?;
                }
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str());
            let ours = name == SESSION_FILE
                || (ext == Some(DOC_EXTENSION)
                    && path
                        .file_stem()
                        .is_some_and(|s| is_valid_id(&s.to_string_lossy())))
                || ext == Some(TMP_EXTENSION);
            if ours {
                fs::remove_file(&path)?;
            }
        }
        sync_dir(&self.dir);
        Ok(())
    }

    /// `<name>.<n>.tmp` in the folder, written by `produce`, flushed, renamed over `<name>`; then the folder is
    /// flushed. The temporary file is removed when anything fails.
    fn write_atomic(
        &self,
        name: &str,
        produce: &mut dyn FnMut(&mut dyn Write) -> io::Result<()>,
    ) -> io::Result<()> {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let target = self.dir.join(name);
        let tmp = self.dir.join(format!(
            "{name}.{}.{TMP_EXTENSION}",
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options.open(&tmp)?;
        let result = (|| {
            let mut writer = BufWriter::new(file);
            produce(&mut writer)?;
            let file = writer
                .into_inner()
                .map_err(io::IntoInnerError::into_error)?;
            crate::sync::sync_file(&file)?;
            drop(file);
            fs::rename(&tmp, &target)
        })();
        match result {
            Ok(()) => {
                sync_dir(&self.dir);
                Ok(())
            }
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
    }
}

/// How long a file stays in `rejected/` (ADR-0022, section 3).
pub const REJECTED_KEEP: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// The day in the name `<id>.<seconds>.doc` of a rejected document.
fn rejected_day(name: &str) -> Option<SystemTime> {
    let stem = name.strip_suffix(&format!(".{DOC_EXTENSION}"))?;
    let (_, secs) = stem.rsplit_once('.')?;
    UNIX_EPOCH.checked_add(Duration::from_secs(secs.parse().ok()?))
}

fn check_id(id: &str) -> io::Result<()> {
    if is_valid_id(id) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a document id",
        ))
    }
}

/// Asks the OS to record a rename or a delete. Best effort, like the save of a document: not every OS
/// lets a folder be opened.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(d) = File::open(dir) {
        let _ = crate::sync::sync_file(&d);
    }
    #[cfg(not(unix))]
    let _ = dir;
}
