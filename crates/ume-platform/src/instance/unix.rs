//! The receiver and the sender on macOS and Linux: a Unix domain socket and a lock file.
//!
//! The files are in a directory of the user that nobody else can enter (mode 0700, owned by the
//! user), so only that user can connect. Who is the receiver is decided by the lock file (`flock`),
//! not by the socket: the first process to take the lock removes any socket that is left, binds a
//! new one and listens. The OS drops the lock when its holder dies, however it dies, so a socket
//! left behind is stale by definition, and two launches at the same moment cannot both win.

// `geteuid` is an `unsafe` call (ADR-0013: only the OS modules of this crate have unsafe code).
#![allow(unsafe_code)]

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::{Handler, Key, Request, SendError, Timeouts, bounded, exchange, serve_connection};

/// How long a receiver waits for a client that has connected and not sent its request.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(5);

/// Keeps the lock and the socket. Dropping it stops the listener and removes the socket.
pub(super) struct Guard {
    socket: PathBuf,
    stop: Arc<AtomicBool>,
    _lock: File,
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wakes the thread that waits in `accept`, so it sees the flag and ends.
        let _ = UnixStream::connect(&self.socket);
        let _ = fs::remove_file(&self.socket);
    }
}

fn euid() -> u32 {
    // SAFETY: `geteuid` takes nothing and cannot fail.
    unsafe { libc::geteuid() }
}

/// The directory for the sockets and locks of this user, made if it is not there.
///
/// It must be the same for every launch of the user, so it is not taken from the environment
/// (`$XDG_RUNTIME_DIR` and `$TMPDIR` differ between a desktop session, `ssh` and `cron`): on Linux it
/// is `/run/user/<uid>` (what `$XDG_RUNTIME_DIR` is on a system with systemd) when that is the
/// user's own folder, and `/tmp/ume-editor-<uid>` otherwise and on macOS.
pub(super) fn private_dir() -> io::Result<PathBuf> {
    let uid = euid();
    if cfg!(target_os = "linux") {
        let dir = PathBuf::from(format!("/run/user/{uid}/ume-editor"));
        if ensure_private_dir(&dir).is_ok() {
            return Ok(dir);
        }
    }
    let dir = PathBuf::from(format!("/tmp/ume-editor-{uid}"));
    ensure_private_dir(&dir)?;
    Ok(dir)
}

/// Makes `dir` (mode 0700) if it is not there; refuses one that someone else owns or that others
/// can enter, or that is a link.
fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    match fs::DirBuilder::new().mode(0o700).create(dir) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    let meta = fs::symlink_metadata(dir)?;
    if !meta.is_dir() || meta.uid() != euid() || meta.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} is not a private directory of the user", dir.display()),
        ));
    }
    Ok(())
}

pub(super) fn socket_path(dir: &Path, key: &Key) -> PathBuf {
    dir.join(format!("ume-{}.sock", key.as_str()))
}

pub(super) fn lock_path(dir: &Path, key: &Key) -> PathBuf {
    dir.join(format!("ume-{}.lock", key.as_str()))
}

/// Takes the key if nobody holds it: `Ok(Some)` is the receiver, `Ok(None)` means someone else is.
pub(super) fn try_receive(key: &Key, handler: Arc<Handler>) -> io::Result<Option<Guard>> {
    let dir = private_dir()?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(lock_path(&dir, key))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Ok(None),
        Err(TryLockError::Error(e)) => return Err(e),
    }
    let socket = socket_path(&dir, key);
    // Whatever is there was left by a process that died: it did not hold the lock.
    match fs::remove_file(&socket) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let listener = UnixListener::bind(&socket)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    std::thread::Builder::new()
        .name("ume-instance-accept".to_owned())
        .spawn(move || accept_loop(&listener, &stopped, &handler))?;
    Ok(Some(Guard {
        socket,
        stop,
        _lock: lock,
    }))
}

fn accept_loop(listener: &UnixListener, stop: &AtomicBool, handler: &Arc<Handler>) {
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let Ok(mut stream) = stream else { continue };
        let handler = handler.clone();
        // One thread for each client: one that stalls must not keep the others waiting.
        let _ = std::thread::Builder::new()
            .name("ume-instance-client".to_owned())
            .spawn(move || {
                let _ = stream.set_read_timeout(Some(CLIENT_TIMEOUT));
                let _ = stream.set_write_timeout(Some(CLIENT_TIMEOUT));
                let _ = serve_connection(&mut stream, &handler);
            });
    }
}

/// Sends `request` to the receiver of `key`.
pub(super) fn send(
    key: &Key,
    request: &Request,
    timeouts: Timeouts,
) -> Result<super::Reply, SendError> {
    let socket = socket_path(&private_dir()?, key);
    let request = request.clone();
    // `connect` has no timeout, and waits for ever when the queue of the listener is full.
    bounded(timeouts.receiver + timeouts.reply, move || {
        let mut stream = connect(&socket, timeouts.receiver)?;
        stream.set_read_timeout(Some(timeouts.reply))?;
        stream.set_write_timeout(Some(timeouts.reply))?;
        Ok(exchange(&mut stream, &request)?)
    })
}

/// Connects, trying again for `wait` while the receiver holds the key but has not bound yet.
fn connect(socket: &Path, wait: Duration) -> Result<UnixStream, SendError> {
    let start = Instant::now();
    loop {
        match UnixStream::connect(socket) {
            Ok(stream) => return Ok(stream),
            Err(e) if is_not_listening(&e) && start.elapsed() < wait => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(e.into()),
        }
    }
}

fn is_not_listening(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
    )
}
