use std::ffi::OsString;
use std::io::{self, Cursor, Read, Write};
use std::sync::mpsc;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use super::*;

/// A key that no other test and no other run uses.
fn unique_key(test: &str) -> Key {
    Key::from_name(&format!("ume-test-{test}-{}", std::process::id()))
}

fn request(name: &str) -> Request {
    let cwd = std::env::current_dir().unwrap();
    Request {
        args: vec![cwd.join(name).into_os_string()],
        cwd,
    }
}

/// A handler that passes each request to the returned channel.
fn collecting() -> (Handler, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    (
        Box::new(move |request| {
            let _ = tx.send(request);
        }),
        rx,
    )
}

fn nothing() -> Handler {
    Box::new(|_| {})
}

// The key

#[test]
fn the_same_settings_file_gives_the_same_key_however_it_is_written() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    let file = dir.path().join("ume.ini");
    let direct = Key::for_settings_file(&file);
    // Before the file exists, and after: the folder is resolved either way.
    std::fs::write(&file, "").unwrap();
    assert_eq!(Key::for_settings_file(&file), direct);
    let indirect = dir.path().join("sub").join("..").join("ume.ini");
    assert_eq!(Key::for_settings_file(&indirect), direct);
    #[cfg(unix)]
    {
        let link = dir.path().join("link.ini");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert_eq!(Key::for_settings_file(&link), direct);
    }
}

#[test]
fn another_settings_file_gives_another_key() {
    let dir = tempfile::tempdir().unwrap();
    let a = Key::for_settings_file(&dir.path().join("a").join("ume.ini"));
    let b = Key::for_settings_file(&dir.path().join("b").join("ume.ini"));
    assert_ne!(a, b);
    assert_eq!(a.as_str().len(), 16);
    assert!(a.as_str().bytes().all(|c| c.is_ascii_hexdigit()));
}

#[cfg(any(windows, target_os = "macos"))]
#[test]
fn the_case_of_the_path_does_not_matter_where_the_file_system_ignores_it() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Ume.ini");
    std::fs::write(&file, "").unwrap();
    let upper = std::path::PathBuf::from(file.to_string_lossy().to_uppercase());
    assert_eq!(
        Key::for_settings_file(&file),
        Key::for_settings_file(&upper)
    );
}

#[test]
fn the_key_does_not_change_between_builds() {
    // FNV-1a of "ume": a pinned value, so that a change of the function is noticed.
    assert_eq!(Key::from_name("ume").as_str(), "4caa84193e171146");
}

// The messages on a connection, without an OS channel

/// A connection made of the bytes the client sends, and the bytes the server wrote back.
struct Fake {
    input: Cursor<Vec<u8>>,
    output: Vec<u8>,
}

impl Read for Fake {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.input.read(buf)
    }
}

impl Write for Fake {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn serve(bytes: Vec<u8>) -> (Vec<u8>, Vec<Request>) {
    let (handler, rx) = collecting();
    let mut stream = Fake {
        input: Cursor::new(bytes),
        output: Vec::new(),
    };
    let _ = serve_connection(&mut stream, &handler);
    (stream.output, rx.try_iter().collect())
}

#[test]
fn a_readable_request_is_handed_over_and_accepted() {
    let sent = request("a.txt");
    let (output, handled) = serve(sent.encode());
    assert_eq!(output, [Reply::Accepted.to_byte()]);
    assert_eq!(handled, [sent]);
}

#[test]
fn a_request_of_another_version_is_refused_and_not_handed_over() {
    let mut message = request("a.txt").encode();
    message[4..8].copy_from_slice(&(VERSION + 1).to_le_bytes());
    let (output, handled) = serve(message);
    assert_eq!(output, [Reply::Refused.to_byte()]);
    assert!(handled.is_empty());
}

#[test]
fn a_message_that_is_too_long_is_refused_without_reading_it() {
    let length = u32::try_from(MAX_MESSAGE + 1).unwrap();
    let (output, handled) = serve(length.to_le_bytes().to_vec());
    assert_eq!(output, [Reply::Refused.to_byte()]);
    assert!(handled.is_empty());
}

#[test]
fn a_cut_message_is_not_handed_over() {
    let message = request("a.txt").encode();
    let (output, handled) = serve(message[..message.len() - 1].to_vec());
    assert!(output.is_empty());
    assert!(handled.is_empty());
}

// The OS channel

#[test]
fn the_second_launch_reaches_the_first() {
    let key = unique_key("deliver");
    let (handler, rx) = collecting();
    let first = join(&key, &request("first.txt"), handler);
    let Joined::Receiver(_receiver) = first else {
        panic!("the first launch must be the receiver");
    };
    let sent = request("second.txt");
    assert!(matches!(join(&key, &sent, nothing()), Joined::Delivered));
    assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(sent));
    // And again: the receiver goes on after a request.
    let sent = request("third.txt");
    assert!(matches!(join(&key, &sent, nothing()), Joined::Delivered));
    assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(sent));
}

#[test]
fn another_key_is_another_receiver() {
    let a = join(&unique_key("key-a"), &request("a"), nothing());
    let b = join(&unique_key("key-b"), &request("b"), nothing());
    assert!(matches!(a, Joined::Receiver(_)));
    assert!(matches!(b, Joined::Receiver(_)));
}

#[test]
fn launches_at_the_same_moment_end_with_one_receiver() {
    const LAUNCHES: usize = 8;
    let key = Arc::new(unique_key("race"));
    let (tx, rx) = mpsc::channel();
    let barrier = Arc::new(Barrier::new(LAUNCHES));
    let threads: Vec<_> = (0..LAUNCHES)
        .map(|i| {
            let (key, barrier, tx) = (key.clone(), barrier.clone(), tx.clone());
            std::thread::spawn(move || {
                barrier.wait();
                let handler: Handler = Box::new(move |request| {
                    let _ = tx.send(request);
                });
                join(&key, &request(&format!("{i}.txt")), handler)
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    let receivers = results
        .iter()
        .filter(|j| matches!(j, Joined::Receiver(_)))
        .count();
    let delivered = results
        .iter()
        .filter(|j| matches!(j, Joined::Delivered))
        .count();
    assert_eq!(receivers, 1);
    assert_eq!(delivered, LAUNCHES - 1);
    // Every request that was delivered reached the one receiver.
    let mut seen = 0;
    while seen < LAUNCHES - 1 {
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        seen += 1;
    }
}

#[test]
fn a_receiver_that_does_not_answer_costs_the_sender_its_timeout_only() {
    let key = unique_key("hang");
    let hung: Handler = Box::new(|_| std::thread::sleep(Duration::from_secs(3)));
    let Joined::Receiver(_receiver) = join(&key, &request("first"), hung) else {
        panic!("the first launch must be the receiver");
    };
    let timeouts = Timeouts {
        receiver: Duration::from_millis(200),
        reply: Duration::from_millis(200),
    };
    let start = Instant::now();
    let joined = join_with(&key, &request("second"), nothing(), timeouts);
    assert!(
        matches!(joined, Joined::Standalone(Standalone::TimedOut)),
        "{}",
        match &joined {
            Joined::Standalone(why) => why.to_string(),
            _ => "not standalone".to_owned(),
        }
    );
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[cfg(unix)]
mod unix_only {
    use std::fs::OpenOptions;
    use std::os::unix::net::UnixListener;

    use super::*;
    use crate::instance::unix::{lock_path, private_dir, socket_path};

    #[test]
    fn the_socket_of_a_process_that_died_is_taken_over() {
        let key = unique_key("stale");
        let dir = private_dir().unwrap();
        // A socket file with nobody behind it, and a lock file that nobody holds.
        let socket = socket_path(&dir, &key);
        drop(UnixListener::bind(&socket).unwrap());
        assert!(socket.exists());
        std::fs::write(lock_path(&dir, &key), "").unwrap();
        let (handler, rx) = collecting();
        let Joined::Receiver(_receiver) = join(&key, &request("first"), handler) else {
            panic!("a stale socket must not keep the key");
        };
        let sent = request("second");
        assert!(matches!(join(&key, &sent, nothing()), Joined::Delivered));
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(sent));
    }

    #[test]
    fn a_receiver_that_was_dropped_leaves_the_key_free() {
        let key = unique_key("dropped");
        let first = join(&key, &request("first"), nothing());
        let Joined::Receiver(receiver) = first else {
            panic!("the first launch must be the receiver");
        };
        let socket = socket_path(&private_dir().unwrap(), &key);
        assert!(socket.exists());
        drop(receiver);
        assert!(!socket.exists());
        assert!(matches!(
            join(&key, &request("second"), nothing()),
            Joined::Receiver(_)
        ));
    }

    #[test]
    fn a_holder_that_never_listens_costs_the_sender_its_timeout_only() {
        let key = unique_key("lock-only");
        let dir = private_dir().unwrap();
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(lock_path(&dir, &key))
            .unwrap();
        lock.lock().unwrap();
        let timeouts = Timeouts {
            receiver: Duration::from_millis(200),
            reply: Duration::from_millis(200),
        };
        let start = Instant::now();
        let joined = join_with(&key, &request("second"), nothing(), timeouts);
        assert!(matches!(joined, Joined::Standalone(_)));
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_listener_that_never_accepts_costs_the_sender_its_timeout_only() {
        let key = unique_key("no-accept");
        let dir = private_dir().unwrap();
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(lock_path(&dir, &key))
            .unwrap();
        lock.lock().unwrap();
        let socket = socket_path(&dir, &key);
        let _ = std::fs::remove_file(&socket);
        let _listener = UnixListener::bind(&socket).unwrap();
        let timeouts = Timeouts {
            receiver: Duration::from_millis(200),
            reply: Duration::from_millis(200),
        };
        let joined = join_with(&key, &request("second"), nothing(), timeouts);
        assert!(matches!(joined, Joined::Standalone(Standalone::TimedOut)));
        std::fs::remove_file(&socket).ok();
    }

    #[test]
    fn the_directory_is_the_users_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = private_dir().unwrap();
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0);
        let key = unique_key("mode");
        let Joined::Receiver(_receiver) = join(&key, &request("a"), nothing()) else {
            panic!("the first launch must be the receiver");
        };
        let mode = std::fs::metadata(socket_path(&dir, &key))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0);
    }
}

#[test]
fn the_arguments_of_a_request_keep_their_order() {
    let key = unique_key("order");
    let (handler, rx) = collecting();
    let Joined::Receiver(_receiver) = join(&key, &request("first"), handler) else {
        panic!("the first launch must be the receiver");
    };
    let cwd = std::env::current_dir().unwrap();
    let sent = Request {
        cwd: cwd.clone(),
        args: ["c.txt", "日本語 b.txt", "a.txt"]
            .into_iter()
            .map(OsString::from)
            .collect(),
    };
    assert!(matches!(join(&key, &sent, nothing()), Joined::Delivered));
    assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(sent));
}
