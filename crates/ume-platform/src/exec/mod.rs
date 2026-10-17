//! Running an external command (`F_EXECMD`, 外部コマンド実行) in the background.
//!
//! Sakura: `CEditView::ExecCmd` (`sakura_core/view/CEditView_ExecCmd.cpp`). It runs the command
//! line with `CreateProcess` as it is; if that fails it assumes a command-line program and runs
//! `cmd.exe /C` (`/K` when the output is not taken), and it reads the standard output and the
//! standard error of the process (both go to the same pipe) while a dialog with a 中断 button
//! waits. The window of this program stays alive instead: [`start`] returns a [`Run`] at once,
//! and the caller asks it for [`Event`]s as they come.
//!
//! * The command line is tried as a program first and then through the shell of the OS: `cmd.exe`
//!   on Windows (as Sakura does), `/bin/sh -c` on macOS and Linux. On macOS and Linux a program
//!   is only tried directly when the line is plain words (no quote, `$`, `|`, `>` and the other
//!   characters a shell treats); anything else needs the shell, as `cmd.exe` is needed for `|`
//!   and `>` on Windows.
//! * The output and the error come as one stream of bytes, in the order they were written
//!   (Sakura gives both the same pipe). The caller decodes them (`ume_core::exec`).
//! * The standard input is a channel of byte pieces ([`stdin_channel`]); the process sees its end
//!   when the sender is dropped. Without one the process reads nothing (the output is taken) or
//!   what the program itself has (the output is not taken).
//! * [`Stopper::stop`] ends the process and the processes it started (the 中断 button): a process
//!   group on macOS and Linux, `taskkill /T` on Windows. Sakura ends only the first process.
//!
//! No `unsafe` code but the one call in `unix`.

use std::fmt;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
use unix as sys;
#[cfg(windows)]
use windows as sys;

#[cfg(test)]
mod tests;

/// How much is read from the process at a time (`MAX_WORK_READ`, 5 KiB in Sakura).
const READ_BYTES: usize = 8 * 1024;

/// How long the output is waited for after the process has ended. A process that started another
/// and left it running holds the pipe open; Sakura stops when the process has ended and the pipe
/// is empty (`PeekNamedPipe`), and so does this.
const GRACE: Duration = Duration::from_millis(300);

/// How many pieces of output may wait for the caller. The process is held back (its pipe fills)
/// when the caller does not keep up, instead of the memory of this program growing.
const OUTPUT_PIECES: usize = 64;

/// How many pieces the input may have waiting. The sender waits when the process does not read.
const STDIN_PIECES: usize = 8;

/// What to run.
#[derive(Debug)]
pub struct Spec {
    /// The command line, as the user typed it.
    pub command: String,
    /// The folder to run it in; the folder of this program when `None`.
    pub folder: Option<PathBuf>,
    /// Take the output and the error (`false`: the process keeps the ones of this program, or on
    /// Windows gets a console of its own).
    pub capture: bool,
    /// `cmd.exe /U`: the shell writes Unicode (Windows only; Sakura sets it for output as
    /// Unicode).
    pub unicode_shell: bool,
    /// The standard input.
    pub stdin: Option<StdinReceiver>,
}

impl Spec {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            folder: None,
            capture: true,
            unicode_shell: false,
            stdin: None,
        }
    }
}

/// The sending end of the standard input.
#[derive(Clone, Debug)]
pub struct StdinSender(SyncSender<Vec<u8>>);

impl StdinSender {
    /// Hands a piece over, waiting while the process is not reading. `false` when the process no
    /// longer reads (it ended or closed its input): stop sending.
    pub fn send(&self, piece: Vec<u8>) -> bool {
        self.0.send(piece).is_ok()
    }
}

/// The receiving end of the standard input, in [`Spec`].
#[derive(Debug)]
pub struct StdinReceiver(Receiver<Vec<u8>>);

/// The two ends of a standard input. Drop the sender to end the input.
pub fn stdin_channel() -> (StdinSender, StdinReceiver) {
    let (tx, rx) = mpsc::sync_channel(STDIN_PIECES);
    (StdinSender(tx), StdinReceiver(rx))
}

/// How the process ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exit {
    /// The exit code; on macOS and Linux a process killed by a signal has 128 plus its number
    /// (what a shell reports). `None` when the OS gives none.
    pub code: Option<i32>,
    /// It ended because [`Stopper::stop`] was called.
    pub stopped: bool,
}

/// What a [`Run`] reports.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// Bytes of the output and the error, in the order written.
    Output(Vec<u8>),
    /// The process has ended and its output has been given. The last event.
    Exited(Exit),
}

/// Why the process could not be started.
#[derive(Debug)]
pub struct StartError {
    /// What the OS says.
    pub message: String,
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StartError {}

enum Raw {
    Output(Vec<u8>),
    ReaderDone,
    Exited(Exit),
}

/// Ends the process of a [`Run`]. Can be kept and used from another thread.
#[derive(Clone, Debug)]
pub struct Stopper(Arc<StopState>);

#[derive(Debug)]
struct StopState {
    pid: u32,
    stopped: AtomicBool,
    /// The process has ended (it may have left others running).
    exited: AtomicBool,
    /// The caller has been given [`Event::Exited`]: nothing is left to stop.
    ended: AtomicBool,
    /// Serializes the kill with the two flags, so that the id of a process that is gone is not
    /// used.
    lock: Mutex<()>,
}

impl Stopper {
    /// Ends the process and the ones it started. A process that has ended can have left others
    /// that still write to the output (`yes &`): on macOS and Linux they are ended through the
    /// process group, which lives as long as one of them does; on Windows the tree of a process
    /// that has ended is not known, and nothing is done. After [`Event::Exited`] nothing is done.
    pub fn stop(&self) {
        let state = &self.0;
        let _guard = state.lock.lock().unwrap_or_else(|e| e.into_inner());
        if state.ended.load(Ordering::SeqCst)
            || (state.exited.load(Ordering::SeqCst) && !sys::KILLS_AFTER_EXIT)
        {
            return;
        }
        state.stopped.store(true, Ordering::SeqCst);
        sys::kill_tree(state.pid);
    }
}

/// A command that runs. Dropping it does not stop the process.
pub struct Run {
    rx: Receiver<Raw>,
    stopper: Stopper,
    exit: Option<(Exit, Instant)>,
    reader_done: bool,
    over: bool,
}

impl Run {
    /// The id of the process.
    pub fn pid(&self) -> u32 {
        self.stopper.0.pid
    }

    /// The handle that ends the process.
    pub fn stopper(&self) -> Stopper {
        self.stopper.clone()
    }

    /// The next event, waiting at most `wait` for it. `None` when there is none yet, or when the
    /// run is over ([`Event::Exited`] was given).
    pub fn poll(&mut self, wait: Duration) -> Option<Event> {
        if self.over {
            return None;
        }
        let deadline = Instant::now() + wait;
        let mut disconnected = false;
        loop {
            // What is already queued comes first, whatever the clocks say.
            match self.rx.try_recv() {
                Ok(raw) => {
                    if let Some(event) = self.take(raw) {
                        return Some(event);
                    }
                    continue;
                }
                Err(TryRecvError::Disconnected) => disconnected = true,
                Err(TryRecvError::Empty) => {}
            }
            if let Some((exit, at)) = self.exit
                && (self.reader_done || disconnected || at.elapsed() >= GRACE)
            {
                return Some(self.finish(exit));
            }
            if disconnected {
                // the thread that waits for the process is gone without a word
                self.over = true;
                return None;
            }
            let mut left = deadline.saturating_duration_since(Instant::now());
            if let Some((_, at)) = self.exit {
                // do not sleep past the end of the grace period
                left = left.min(GRACE.saturating_sub(at.elapsed()));
            }
            if left.is_zero() {
                match self.exit {
                    // the grace period is over: the top of the loop gives the exit
                    Some((_, at)) if at.elapsed() >= GRACE => continue,
                    _ => return None,
                }
            }
            match self.rx.recv_timeout(left) {
                Ok(raw) => {
                    if let Some(event) = self.take(raw) {
                        return Some(event);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => disconnected = true,
            }
        }
    }

    /// The last event: from now on nothing can be stopped, and a stop that came after the process
    /// had exited (see [`Stopper::stop`]) is counted.
    fn finish(&mut self, mut exit: Exit) -> Event {
        let state = &self.stopper.0;
        let _guard = state.lock.lock().unwrap_or_else(|e| e.into_inner());
        state.ended.store(true, Ordering::SeqCst);
        exit.stopped |= state.stopped.load(Ordering::SeqCst);
        self.over = true;
        Event::Exited(exit)
    }

    /// Takes in what a thread sent; the event for the caller, if it is one.
    fn take(&mut self, raw: Raw) -> Option<Event> {
        match raw {
            Raw::Output(bytes) => {
                // The grace period after the exit counts from the last output.
                if let Some((_, at)) = &mut self.exit {
                    *at = Instant::now();
                }
                Some(Event::Output(bytes))
            }
            Raw::ReaderDone => {
                self.reader_done = true;
                None
            }
            Raw::Exited(exit) => {
                self.exit = Some((exit, Instant::now()));
                None
            }
        }
    }
}

/// Starts the command and returns at once.
pub fn start(spec: Spec) -> Result<Run, StartError> {
    if spec.command.trim().is_empty() {
        return Err(StartError {
            message: "the command is empty".to_owned(),
        });
    }
    let pipe = if spec.capture {
        Some(io::pipe().map_err(|e| StartError {
            message: e.to_string(),
        })?)
    } else {
        None
    };
    let mut failure = None;
    let mut spawned = None;
    // the program as it is, then the shell
    for shell in [false, true] {
        let command = if shell {
            Some(sys::shell_command(
                &spec.command,
                spec.capture,
                spec.unicode_shell,
            ))
        } else {
            sys::direct_command(&spec.command)
        };
        let Some(mut command) = command else { continue };
        match launch(&mut command, &spec, pipe.as_ref().map(|(_, w)| w)) {
            Ok(child) => {
                spawned = Some(child);
                break;
            }
            Err(error) => failure = Some(error),
        }
    }
    let Some(mut child) = spawned else {
        return Err(StartError {
            message: failure.map_or_else(|| "cannot start".to_owned(), |e| e.to_string()),
        });
    };
    let reader = pipe.map(|(reader, _writer)| reader);
    let (tx, rx) = mpsc::sync_channel(OUTPUT_PIECES);
    let state = Arc::new(StopState {
        pid: child.id(),
        stopped: AtomicBool::new(false),
        exited: AtomicBool::new(false),
        ended: AtomicBool::new(false),
        lock: Mutex::new(()),
    });
    if let (Some(input), Some(stdin)) = (spec.stdin, child.stdin.take()) {
        feed(input, stdin);
    }
    match reader {
        Some(reader) => read_output(reader, tx.clone()),
        None => {
            let _ = tx.send(Raw::ReaderDone);
        }
    }
    wait_for(child, Arc::clone(&state), tx);
    Ok(Run {
        rx,
        stopper: Stopper(state),
        exit: None,
        reader_done: false,
        over: false,
    })
}

fn launch(
    command: &mut Command,
    spec: &Spec,
    writer: Option<&io::PipeWriter>,
) -> io::Result<Child> {
    if let Some(folder) = &spec.folder {
        command.current_dir(folder);
    }
    command.stdin(match (&spec.stdin, spec.capture) {
        (Some(_), _) => Stdio::piped(),
        (None, true) => Stdio::null(),
        (None, false) => Stdio::inherit(),
    });
    match writer {
        Some(writer) => {
            command.stdout(writer.try_clone()?);
            command.stderr(writer.try_clone()?);
        }
        None => {
            command.stdout(Stdio::inherit());
            command.stderr(Stdio::inherit());
        }
    }
    sys::prepare(command, spec.capture);
    command.spawn()
}

/// Writes the pieces of the input to the process, then closes its input.
fn feed(input: StdinReceiver, mut stdin: std::process::ChildStdin) {
    thread::spawn(move || {
        for piece in input.0 {
            if stdin.write_all(&piece).is_err() {
                // the process does not read any more; dropping the receiver tells the sender
                break;
            }
        }
    });
}

fn read_output(mut reader: io::PipeReader, tx: SyncSender<Raw>) {
    thread::spawn(move || {
        let mut buf = vec![0; READ_BYTES];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(Raw::Output(buf[..n].to_vec())).is_err() {
                        return;
                    }
                }
            }
        }
        let _ = tx.send(Raw::ReaderDone);
    });
}

fn wait_for(mut child: Child, state: Arc<StopState>, tx: SyncSender<Raw>) {
    thread::spawn(move || {
        let status = child.wait();
        {
            // a stop in progress finishes first; later ones see a process that has ended
            let _guard = state.lock.lock().unwrap_or_else(|e| e.into_inner());
            state.exited.store(true, Ordering::SeqCst);
        }
        let code = status.ok().and_then(sys::exit_code);
        let _ = tx.send(Raw::Exited(Exit {
            code,
            stopped: state.stopped.load(Ordering::SeqCst),
        }));
    });
}

/// Splits a command line into the program and the rest, as `CreateProcess` reads it: a program
/// in double quotes ends at the closing quote, else it ends at the first space or tab. The rest
/// keeps its spaces. `None` for a line with no program.
pub fn split_program(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start_matches([' ', '\t']);
    if let Some(quoted) = line.strip_prefix('"') {
        let end = quoted.find('"')?;
        let (program, rest) = quoted.split_at(end);
        return (!program.is_empty()).then_some((program, &rest[1..]));
    }
    let end = line.find([' ', '\t']).unwrap_or(line.len());
    let (program, rest) = line.split_at(end);
    (!program.is_empty()).then_some((program, rest))
}

/// Whether a shell would treat something in `line`: macOS and Linux run such a line through
/// `/bin/sh -c` without trying it as a program first.
pub fn needs_shell(line: &str) -> bool {
    line.chars().any(|c| {
        matches!(
            c,
            '"' | '\''
                | '`'
                | '$'
                | '\\'
                | '|'
                | '&'
                | ';'
                | '<'
                | '>'
                | '('
                | ')'
                | '*'
                | '?'
                | '['
                | ']'
                | '{'
                | '}'
                | '~'
                | '#'
                | '!'
                | '\n'
                | '\r'
        )
    })
}
