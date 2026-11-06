//! Running the login shell (macOS; the tests run it on Linux with a shell of their own).

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::{
    OUTPUT_LIMIT, RESOLVING_VARIABLE, between_marks, make_mark, mix, parse_output, shell_script,
};

/// Asks `shell -l -i -c` for its `PATH` and mixes it with `startup`; gives up at `deadline`.
/// `-i` is needed: zsh reads `~/.zshrc` only when it is interactive. The standard input is
/// `/dev/null`, the error is thrown away, and the shell leads a process group so that it and
/// what it started end together.
pub(super) fn read(
    shell: &Path,
    deadline: Instant,
    startup: Option<&OsStr>,
) -> Result<OsString, String> {
    let mark = make_mark();
    let mut child = Command::new(shell)
        .args(["-l", "-i", "-c"])
        .arg(shell_script(&mark))
        .env(RESOLVING_VARIABLE, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stdout = child.stdout.take().ok_or("no output")?;

    // The reader keeps taking what the shell writes (a full pipe would hold the shell back), but
    // only the first `OUTPUT_LIMIT` bytes are kept. It ends when the pipe is closed; a process
    // that the shell left behind can keep it open, so nobody waits for the reader's end.
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut kept = 0;
        loop {
            match stdout.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if kept < OUTPUT_LIMIT {
                        let take = n.min(OUTPUT_LIMIT - kept);
                        kept += take;
                        // The receiver may be gone; reading goes on to the end, so that a
                        // process the shell left behind does not die of SIGPIPE when it writes.
                        let _ = tx.send(buf[..take].to_vec());
                    }
                }
            }
        }
    });

    let mut output = Vec::new();
    let mut timed_out = false;
    loop {
        // the marks are all that is needed: the shell may still be finishing its work
        if between_marks(&output, &mark).is_some() {
            break;
        }
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(piece) => output.extend_from_slice(&piece),
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                timed_out = true;
                break;
            }
        }
    }
    // Nothing has waited for the shell yet, so its id is still its own (a process that has ended
    // keeps it until it is waited for) and the group can be ended.
    let read_it = between_marks(&output, &mark).is_some();
    if timed_out && !read_it {
        // the deadline came before the marks: end the shell and what it started, which can hold
        // the pipe
        crate::exec::kill_tree(child.id());
        let _ = child.wait();
    } else {
        // The marks came (or the output ended without them): the shell is only finishing
        // (`.zlogout`), and what its configuration started on purpose (`rbenv rehash &`) must
        // not be killed. It is reaped on another thread; one that is still there at the
        // deadline is ended with its group.
        reap(child, deadline);
    }

    match parse_output(&output, &mark) {
        Ok(path) => mix(&OsString::from_vec(path), startup)
            .ok_or_else(|| "the PATH of the shell has no absolute folder".to_owned()),
        Err(_) if timed_out => Err("timed out".to_owned()),
        Err(why) => Err(why.to_owned()),
    }
}

/// Waits for the shell to end, in the background; ends it and its group at `deadline` if it is
/// still running. A shell that has been waited for is not killed: its id may be used again.
fn reap(mut child: Child, deadline: Instant) {
    std::thread::spawn(move || {
        loop {
            match child.try_wait() {
                Ok(None) => {}
                _ => return,
            }
            if Instant::now() >= deadline {
                crate::exec::kill_tree(child.id());
                let _ = child.wait();
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
}
