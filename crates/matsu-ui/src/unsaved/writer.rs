//! The writer: one thread that does every operation on the folder of unsaved documents (ADR-0022, sections 2
//! and 3).
//!
//! The window never writes a file. It hands the thread a job: a snapshot of a document to write, the list of
//! windows and tabs, a document to discard. The thread does them in the order they were queued, so
//!
//! * a discard that is queued after a write of the same document always ends with the file gone: a save
//!   cannot be followed by an older text coming back (the generation of ADR-0022 is the order of the queue,
//!   because one thread does both the writes and the deletions, so a snapshot cannot be renamed into place
//!   after the deletion that retired it);
//! * the window can go on typing while a large document is written: the snapshot is a clone of the document
//!   (it shares everything with the live one), and the live one is never locked.
//!
//! A write that fails (disk full, no permission) leaves the old file. The thread records the failure in
//! [`Shared`] and in [`Writer::failed`]; the window tries again later and tells the user once.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use matsu_config::unsaved::{DocInfo, Session, Store, StoreError};
use matsu_core::buffer::{Document, Eol};
use matsu_core::encoding::Encoding;

/// What makes the copy of a document out of date: the text (its version), the encoding and BOM, and the line
/// ending of new lines. Equal revisions have equal copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Revision {
    pub version: u64,
    pub encoding: Encoding,
    pub bom: bool,
    pub eol: Eol,
}

/// What the window and the thread share about one document.
#[derive(Debug, Default)]
pub(crate) struct Shared {
    /// A write of this document is queued or running; no second one is queued meanwhile.
    pub in_flight: AtomicBool,
    /// The last write failed.
    pub failed: AtomicBool,
    /// The revision of the last write that succeeded.
    pub written: Mutex<Option<Revision>>,
}

impl Shared {
    pub(crate) fn written(&self) -> Option<Revision> {
        *self.written.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// A document as it was when the window asked for its copy.
pub(crate) struct Snapshot {
    pub doc: Document,
    pub info: DocInfo,
}

enum Job {
    Write {
        id: String,
        shared: Arc<Shared>,
        revision: Revision,
        snapshot: Box<Snapshot>,
    },
    Session(Session),
    Discard(String),
    Clear,
    Barrier(Sender<()>),
}

/// The thread and the way to it. Dropping it finishes the queued jobs, ends the thread and lets go of the lock
/// on the folder.
pub(crate) struct Writer {
    tx: Option<Sender<Job>>,
    failed: Arc<AtomicBool>,
    /// The last write of `session.ini` failed.
    session_failed: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Writer {
    /// Opens the folder (taking its lock) and starts the thread. The temporary files a cut-off write left
    /// are removed first: this process holds the lock, so none of them is being written.
    pub(crate) fn start(dir: &std::path::Path) -> Result<Self, StoreError> {
        Self::start_retrying(dir, RETRY_STUCK)
    }

    /// [`Writer::start`] with the time between the tries of a removal that failed; the tests make it short.
    pub(crate) fn start_retrying(
        dir: &std::path::Path,
        retry: std::time::Duration,
    ) -> Result<Self, StoreError> {
        let store = Store::open(dir)?;
        let _ = store.remove_tmp();
        let failed = Arc::new(AtomicBool::new(false));
        let session_failed = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel::<Job>();
        let thread = std::thread::Builder::new()
            .name("unsaved-writer".to_owned())
            .spawn({
                let failed = Arc::clone(&failed);
                let session_failed = Arc::clone(&session_failed);
                move || run(&store, &rx, &failed, &session_failed, retry)
            })
            .map_err(StoreError::Io)?;
        Ok(Self {
            tx: Some(tx),
            failed,
            session_failed,
            thread: Some(thread),
        })
    }

    fn send(&self, job: Job) {
        if let Some(tx) = &self.tx {
            // The thread only ends when this is dropped.
            let _ = tx.send(job);
        }
    }

    /// Queues the write of the copy of document `id`. The caller has checked `shared.in_flight`.
    pub(crate) fn write(
        &self,
        id: String,
        shared: &Arc<Shared>,
        revision: Revision,
        snapshot: Snapshot,
    ) {
        shared.in_flight.store(true, Ordering::SeqCst);
        self.send(Job::Write {
            id,
            shared: Arc::clone(shared),
            revision,
            snapshot: Box::new(snapshot),
        });
    }

    /// Queues the write of `session.ini`.
    pub(crate) fn session(&self, session: Session) {
        self.send(Job::Session(session));
    }

    /// Queues the removal of the copy of document `id` (its file first, then its entry in `session.ini`).
    pub(crate) fn discard(&self, id: String) {
        self.send(Job::Discard(id));
    }

    /// Queues the removal of everything in the folder.
    pub(crate) fn clear(&self) {
        self.send(Job::Clear);
    }

    /// Whether any operation of the thread failed since the last one that worked.
    pub(crate) fn failed(&self) -> bool {
        self.failed.load(Ordering::SeqCst)
    }

    /// Whether the last write of `session.ini` failed.
    pub(crate) fn session_failed(&self) -> bool {
        self.session_failed.load(Ordering::SeqCst)
    }

    /// Waits until every job queued so far is done.
    pub(crate) fn flush(&self) {
        let (tx, rx) = mpsc::channel();
        self.send(Job::Barrier(tx));
        let _ = rx.recv();
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.tx = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(
    store: &Store,
    rx: &mpsc::Receiver<Job>,
    failed: &AtomicBool,
    session_failed: &AtomicBool,
    retry: std::time::Duration,
) {
    let mut session = store.read_session();
    // What failed to be removed: tried again until it is gone, so a discarded text does not come back.
    let mut pending = Pending::default();
    // The time of the next try, checked after every job too: a queue that is never idle must not starve it.
    let mut next_retry = std::time::Instant::now() + retry;
    loop {
        let job = match rx.recv_timeout(retry) {
            Ok(job) => job,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Only a removal that is tried again can say that things work again.
                if !pending.is_empty() {
                    pending.retry(store, &mut session);
                    next_retry = std::time::Instant::now() + retry;
                    failed.store(!pending.is_empty(), Ordering::SeqCst);
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                pending.retry(store, &mut session);
                return;
            }
        };
        let result = match job {
            Job::Write {
                id,
                shared,
                revision,
                snapshot,
            } => {
                let result = write_doc(store, &id, &snapshot);
                match &result {
                    Ok(()) => {
                        *shared.written.lock().unwrap_or_else(|e| e.into_inner()) = Some(revision);
                        shared.failed.store(false, Ordering::SeqCst);
                    }
                    Err(_) => shared.failed.store(true, Ordering::SeqCst),
                }
                shared.in_flight.store(false, Ordering::SeqCst);
                result
            }
            Job::Session(next) => {
                let result = store.write_session(&next);
                session_failed.store(result.is_err(), Ordering::SeqCst);
                session = next;
                result
            }
            Job::Discard(id) => {
                let result = store.discard_doc(&id, &session).map(|next| session = next);
                if result.is_err() {
                    pending.stuck.push(id);
                }
                result
            }
            Job::Clear => {
                session = Session::default();
                // What is there now is removed file by file, and what cannot be removed is remembered by name
                // and tried again. Not the whole folder again: by then the feature may be on, and the copies
                // written since must stay.
                pending.stuck.clear();
                pending.leftovers.clear();
                clear_now(store, &mut pending)
            }
            Job::Barrier(tx) => {
                let _ = tx.send(());
                continue;
            }
        };
        if !pending.is_empty() && std::time::Instant::now() >= next_retry {
            pending.retry(store, &mut session);
            next_retry = std::time::Instant::now() + retry;
        }
        failed.store(result.is_err() || !pending.is_empty(), Ordering::SeqCst);
    }
}

fn write_doc(store: &Store, id: &str, snapshot: &Snapshot) -> io::Result<()> {
    store.write_doc(id, &snapshot.info, &mut |w| write_text(&snapshot.doc, w))
}

/// The text of the document as UTF-8, line by line with the line endings it has. The characters that stand for
/// bytes that were not valid in the document's encoding (U+10FF00..) are written as the ordinary 4-byte UTF-8 of
/// those characters, not as the raw bytes the saver of the document writes: the copy keeps what the editor
/// holds, and the encoding of the document applies when the user saves.
pub(crate) fn write_text(doc: &Document, w: &mut dyn Write) -> io::Result<()> {
    for line in doc.lines(0..doc.line_count()) {
        let line = line.map_err(io::Error::other)?;
        w.write_all(line.text.as_bytes())?;
        if let Some(eol) = line.eol {
            w.write_all(eol.as_str().as_bytes())?;
        }
    }
    Ok(())
}

/// How often the removals that failed are tried again.
const RETRY_STUCK: std::time::Duration = std::time::Duration::from_secs(5);

/// The removals that did not work yet.
#[derive(Default)]
struct Pending {
    /// Copies that could not be removed.
    stuck: Vec<String>,
    /// Files of a clear that could not be removed: documents, temporary files, the rejected ones.
    leftovers: Vec<std::path::PathBuf>,
}

impl Pending {
    fn is_empty(&self) -> bool {
        self.stuck.is_empty() && self.leftovers.is_empty()
    }

    fn retry(&mut self, store: &Store, session: &mut Session) {
        self.leftovers.retain(|path| remove_file(path).is_err());
        if self.leftovers.is_empty() {
            let _ = std::fs::remove_dir(store.dir().join("rejected"));
        }
        self.stuck
            .retain(|id| match store.discard_doc(id, session) {
                Ok(next) => {
                    *session = next;
                    false
                }
                Err(_) => true,
            });
    }
}

/// Removes a file; a file that is not there is removed.
fn remove_file(path: &std::path::Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// The files `clear` removes, listed now: the documents, the temporary files, `session.ini`, and what is in
/// `rejected/`. `lock` and anything else stay.
pub(crate) fn clear_targets(store: &Store) -> Vec<std::path::PathBuf> {
    let mut targets = Vec::new();
    let mut list = |dir: &std::path::Path, all: bool| {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let ext = path.extension().and_then(|e| e.to_str());
            let stem_is_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(matsu_config::unsaved::is_valid_id);
            if all
                || name == "session.ini"
                || ext == Some("tmp")
                || (ext == Some("doc") && stem_is_id)
            {
                targets.push(path);
            }
        }
    };
    list(store.dir(), false);
    list(&store.dir().join("rejected"), true);
    targets
}

/// Removes everything [`clear_targets`] lists. What cannot be removed goes to `pending.leftovers`, and the
/// result is an error then.
fn clear_now(store: &Store, pending: &mut Pending) -> io::Result<()> {
    let mut first_error = None;
    for path in clear_targets(store) {
        if let Err(e) = remove_file(&path) {
            first_error.get_or_insert(e);
            // `session.ini` is written again by the keeper; the others are asked for again.
            if path.file_name().and_then(|n| n.to_str()) != Some("session.ini") {
                pending.leftovers.push(path);
            }
        }
    }
    if first_error.is_none() {
        let _ = std::fs::remove_dir(store.dir().join("rejected"));
    }
    first_error.map_or(Ok(()), Err)
}
