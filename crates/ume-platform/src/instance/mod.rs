//! One UME Editor process per settings file ([ADR-0016]).
//!
//! A launch calls [`join`] with the [`Key`] of its settings file. Either it becomes the receiver of
//! the key (the first launch: from then on each [`Request`] that a later launch sends is handed to
//! the handler), or it finds the receiver, sends its request and is told to exit
//! ([`Joined::Delivered`]), or it can do neither and runs on its own ([`Joined::Standalone`]).
//!
//! * Windows: a named pipe. The first instance of the pipe (`FILE_FLAG_FIRST_PIPE_INSTANCE`) makes
//!   the receiver, so two launches at the same moment end with one receiver. The pipe is for the
//!   current user only (its security descriptor names the user's SID) and refuses remote clients.
//!   The sender lets the receiver take the foreground (`AllowSetForegroundWindow`), so that its
//!   window can come to the front.
//! * macOS and Linux: a Unix domain socket in a directory that only the user can read, and a lock
//!   file next to it. The lock decides who is the receiver; the OS drops it when its holder dies,
//!   so the socket that a dead process left behind is known to be stale and is taken over.
//!
//! No call waits for long: a receiver that does not answer costs the sender [`Timeouts::reply`],
//! and then the sender runs on its own.
//!
//! [ADR-0016]: https://github.com/stofu5678/ume_editor/blob/main/docs/adr/0016-single-instance.md

mod request;
#[cfg(test)]
mod tests;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::fmt;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use request::{DecodeError, MAX_MESSAGE, Reply, Request, VERSION};

#[cfg(unix)]
use unix as os;
#[cfg(windows)]
use windows as os;

/// What a process shares with the others that use the same settings file. The OS user is not in it:
/// the transport is per user (the pipe name has the user's SID, the socket directory is the user's).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(String);

impl Key {
    /// The key of the settings file `path`: the file the process reads, or will write when it does
    /// not exist yet. The same file gives the same key however the path is written (relative,
    /// through a link, in another case on a file system that ignores case); another file gives
    /// another key, as Sakura's profiles do.
    pub fn for_settings_file(path: &Path) -> Key {
        let (canonical, existing) = canonical_path(path);
        // The case is asked of the nearest folder that exists, so that making the settings folder
        // later does not change the key.
        match canonical.to_str() {
            Some(text) if ignores_case(&existing) => {
                Key::from_bytes(text.to_lowercase().as_bytes())
            }
            // A name that is not text cannot be lowered: it is taken as it is.
            _ => Key::from_bytes(&native_bytes(&canonical)),
        }
    }

    /// A key from any text (tests use names of their own).
    pub fn from_name(name: &str) -> Key {
        Key::from_bytes(name.as_bytes())
    }

    fn from_bytes(bytes: &[u8]) -> Key {
        // FNV-1a, 64 bits: the key must not change between builds (`DefaultHasher` may).
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Key(format!("{hash:016x}"))
    }

    /// 16 hexadecimal digits, safe in a file name and a pipe name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The path with links and `..` resolved, also when the file or its folders do not exist yet: the
/// nearest folder that exists is resolved and the rest is put after it. (The settings folder is made
/// when the first file is written, and the key must not change then.) Also returns that nearest
/// folder (or file) that exists.
fn canonical_path(path: &Path) -> (PathBuf, PathBuf) {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    let mut rest = Vec::new();
    let mut existing = absolute.as_path();
    loop {
        if let Ok(resolved) = existing.canonicalize() {
            let full = rest
                .iter()
                .rev()
                .fold(resolved.clone(), |p, name| p.join(name));
            return (full, resolved);
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name);
                existing = parent;
            }
            _ => return (absolute.clone(), absolute),
        }
    }
}

/// The path as the bytes the OS stores, so that names that are not text stay different.
#[cfg(unix)]
fn native_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

/// Windows ignores case in every folder, so this is not used there.
#[cfg(windows)]
fn native_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()
}

/// Whether the file system of the folder `dir` ignores the case of names: Windows does in every
/// folder.
#[cfg(windows)]
fn ignores_case(_dir: &Path) -> bool {
    true
}

/// Whether the file system of the folder `dir` ignores the case of names (macOS does on its default
/// volumes). It is asked of the file system: the name of the folder in the other case is the same
/// folder, or not.
#[cfg(unix)]
fn ignores_case(dir: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let name = dir.to_string_lossy();
    let mut other = name.to_uppercase();
    if other == name {
        other = name.to_lowercase();
    }
    if other == name {
        // No letters in the name: nothing to tell by.
        return false;
    }
    match (std::fs::metadata(dir), std::fs::metadata(&other)) {
        (Ok(a), Ok(b)) => (a.dev(), a.ino()) == (b.dev(), b.ino()),
        _ => false,
    }
}

/// How long the calls of [`join`] wait.
#[derive(Clone, Copy, Debug)]
pub struct Timeouts {
    /// How long a sender waits for a receiver that has been started (it holds the key) but is not
    /// listening yet.
    pub receiver: Duration,
    /// How long a sender waits to write its request and to be answered.
    pub reply: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Timeouts {
            receiver: Duration::from_secs(1),
            reply: Duration::from_secs(2),
        }
    }
}

/// What a receiver does with each request. It runs on a thread of the receiver, so it should only
/// pass the request on (to the UI thread) and return. It returns `false` when the request could not
/// be passed on (the receiver is ending): the sender is then told "refused" and runs on its own,
/// instead of believing that its files are being opened.
pub type Handler = Box<dyn Fn(Request) -> bool + Send + Sync + 'static>;

/// What [`join`] made of this process.
#[must_use]
pub enum Joined {
    /// This process is the receiver of the key. Keep the value for as long as the process runs.
    Receiver(Receiver),
    /// The request was handed to the running UME Editor: the process has nothing more to do.
    Delivered,
    /// There is a receiver but it could not be used (or none could be made): run as an editor of
    /// its own.
    Standalone(Standalone),
}

/// Why a process runs on its own although another may be running.
#[derive(Debug)]
pub enum Standalone {
    /// The receiver cannot read this request (another version).
    Refused,
    /// The receiver did not answer in time.
    TimedOut,
    /// The channel could not be set up or used.
    Failed(io::Error),
}

impl fmt::Display for Standalone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Standalone::Refused => f.write_str("the running UME Editor cannot read this request"),
            Standalone::TimedOut => f.write_str("the running UME Editor did not answer in time"),
            Standalone::Failed(e) => write!(f, "cannot reach the running UME Editor: {e}"),
        }
    }
}

/// The receiver of a key. While it is alive, later launches with the key reach the handler.
/// Dropping it (on Unix) stops listening and removes the socket; on Windows the pipe lives until
/// the process ends.
pub struct Receiver {
    _guard: os::Guard,
}

/// Becomes the receiver of `key`, or sends `request` to the process that is. See the module
/// documentation.
pub fn join(key: &Key, request: &Request, handler: Handler) -> Joined {
    join_with(key, request, handler, Timeouts::default())
}

/// [`join`] with timeouts of your own.
pub fn join_with(key: &Key, request: &Request, handler: Handler, timeouts: Timeouts) -> Joined {
    let handler = std::sync::Arc::new(handler);
    // The receiver may die between "someone holds the key" and our connection: look again.
    let mut last = Standalone::Failed(io::Error::new(
        io::ErrorKind::NotFound,
        "the running UME Editor went away",
    ));
    for _ in 0..3 {
        match os::try_receive(key, handler.clone()) {
            Ok(Some(guard)) => return Joined::Receiver(Receiver { _guard: guard }),
            Ok(None) => {}
            Err(e) => return Joined::Standalone(Standalone::Failed(e)),
        }
        match os::send(key, request, timeouts) {
            Ok(Reply::Accepted) => return Joined::Delivered,
            Ok(Reply::Refused) => return Joined::Standalone(Standalone::Refused),
            Err(SendError::NoReceiver) => continue,
            Err(SendError::TimedOut) => {
                last = Standalone::TimedOut;
                break;
            }
            Err(SendError::Io(e)) => {
                last = Standalone::Failed(e);
                break;
            }
        }
    }
    Joined::Standalone(last)
}

/// Runs `send` on a thread and waits for it for `limit`. A call that blocks without a timeout of
/// its own (a `connect` to a socket whose queue is full) cannot hold the sender longer than that;
/// the thread is left behind in that case and ends with the process.
#[cfg(unix)]
pub(crate) fn bounded<T: Send + 'static>(
    limit: Duration,
    send: impl FnOnce() -> Result<T, SendError> + Send + 'static,
) -> Result<T, SendError> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("ume-instance-send".to_owned())
        .spawn(move || {
            let _ = tx.send(send());
        })?;
    rx.recv_timeout(limit).unwrap_or(Err(SendError::TimedOut))
}

/// Why a request could not be sent.
#[derive(Debug)]
pub(crate) enum SendError {
    /// Nobody listens (any more).
    NoReceiver,
    TimedOut,
    Io(io::Error),
}

impl From<io::Error> for SendError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => SendError::TimedOut,
            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => SendError::NoReceiver,
            _ => SendError::Io(e),
        }
    }
}

/// Reads one message from a connection and answers it. The handler gets the request only when it
/// is readable.
pub(crate) fn serve_connection<S: Read + Write>(
    stream: &mut S,
    handler: &Handler,
) -> io::Result<()> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    if length > MAX_MESSAGE {
        // Not read, so the stream is left as it is: answer and let the caller close it.
        return stream.write_all(&[Reply::Refused.to_byte()]);
    }
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body)?;
    let handed_over = Request::decode(&body).is_ok_and(handler);
    let reply = if handed_over {
        Reply::Accepted
    } else {
        Reply::Refused
    };
    stream.write_all(&[reply.to_byte()])?;
    stream.flush()
}

/// Writes the request and reads the answer.
pub(crate) fn exchange<S: Read + Write>(stream: &mut S, request: &Request) -> io::Result<Reply> {
    stream.write_all(&request.encode())?;
    stream.flush()?;
    let mut byte = [0u8; 1];
    stream.read_exact(&mut byte)?;
    Ok(Reply::from_byte(byte[0]))
}
