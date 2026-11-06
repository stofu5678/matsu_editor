//! Running the login shell (macOS; the tests run it on Linux with a shell of their own).

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Instant;

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
                        if tx.send(buf[..take].to_vec()).is_err() {
                            break;
                        }
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
    // A shell that is still there is ended with its group. One that has been waited for is left
    // alone: its id may be used again.
    if matches!(child.try_wait(), Ok(None)) {
        crate::exec::kill_tree(child.id());
    }
    let _ = child.wait();

    match parse_output(&output, &mark) {
        Ok(path) => mix(&OsString::from_vec(path), startup)
            .ok_or_else(|| "the PATH of the shell has no absolute folder".to_owned()),
        Err(_) if timed_out => Err("timed out".to_owned()),
        Err(why) => Err(why.to_owned()),
    }
}
