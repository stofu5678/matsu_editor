//! Starting a one-liner and following it until it ends.
//!
//! [`start`] picks the shell ([`crate::shell`]), starts it with its arguments and the standard
//! error apart from the standard output, and feeds the text to its standard input from a thread of
//! its own: sending the input and reading the output never wait for each other, so a command that
//! does not read its input, or one that writes a lot before it reads, cannot lock up. The input is
//! closed when the text is sent.
//!
//! The output is kept until the process ends (a failure must change nothing, ADR-0029), so
//! [`Run::poll`] has one result: [`Event::Finished`] with everything, or [`Event::Failed`] when the
//! output is over its cap or the input could not be read. The environment is the one of this
//! program and nothing is added (ADR-0029), except the `PATH` the caller gives for macOS.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use matsu_core::oneliner::{
    Collector, ERROR_CAP_BYTES, OUTPUT_CAP_BYTES, Overflow, TextDecoder, Utf8Decoder,
    for_next_line, for_replace,
};
use matsu_platform::exec::{self, Exit, Program, Spec, StdinSender, Stopper, TrySend};

use crate::shell::{self, ShellError, TextEncoding};

/// The least time one [`Run::poll`] spends taking in output, even when it is asked not to wait.
const WORK_SLICE: Duration = Duration::from_millis(20);

/// How long [`Run::poll`] sleeps between looks at the thread that sends the input.
const POLL_SLEEP: Duration = Duration::from_millis(2);

/// Where the output goes (ADR-0029, 結果の置き場所). It only decides how the output is finished
/// off ([`matsu_core::oneliner::for_replace`] and
/// [`matsu_core::oneliner::for_next_line`]); what is done with the document is for the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputKind {
    /// 置き換える: a line break at the end that the input did not have is dropped, once.
    Replace,
    /// 次の行に入れる: the output ends with a line break.
    NextLine,
    /// アウトプットに出す: the output as it is.
    Window,
}

/// The text of the input: calls the sink with chunks of text (line breaks LF) until it returns
/// `false` or the text is over. It is called once to check that the text can be written in the
/// encoding of the shell (`cmd`), and once to send it, from another thread; so it must read the
/// same text both times. An `Err` is a text that cannot be read: nothing may be done with the
/// output then.
///
/// For a document: a closure that clones or shares a snapshot of it and calls
/// `matsu_core::oneliner::feed_lf`.
pub type Input = Arc<dyn Fn(&mut dyn FnMut(&str) -> bool) -> Result<(), String> + Send + Sync>;

/// What to run.
pub struct Request {
    /// The shell tab's setting (`[Terminal]` `Shell=`); only its program part is used. Empty: the
    /// default of the tab.
    pub shell: String,
    /// The one-liner.
    pub line: String,
    /// The folder to run in: the folder of the document, else the home folder.
    pub folder: Option<PathBuf>,
    /// The text to send.
    pub input: Input,
    /// Where the output goes.
    pub output: OutputKind,
    /// The `PATH` for the shell, `None` for the one of this program. macOS started from Finder or
    /// the Dock: the login shell's ([`matsu_platform::login_path`]).
    pub path: Option<OsString>,
}

/// Why a one-liner did not start.
#[derive(Debug)]
pub enum StartError {
    /// The line is empty.
    EmptyLine,
    /// The shell setting is not a command line.
    Shell(ShellError),
    /// A character of the input cannot be written in the encoding of the shell (an emoji for
    /// `cmd` on Japanese Windows). Nothing was started.
    CannotEncode,
    /// The input could not be read.
    Input(String),
    /// The OS could not start the shell; what it says.
    Process { program: String, message: String },
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLine => f.write_str("the one-liner is empty"),
            Self::Shell(error) => error.fmt(f),
            Self::CannotEncode => f.write_str(
                "the text has a character that the shell cannot read (the code page of cmd)",
            ),
            Self::Input(message) => write!(f, "the text could not be read: {message}"),
            Self::Process { program, message } => {
                write!(f, "cannot start {program}: {message}")
            }
        }
    }
}

impl std::error::Error for StartError {}

/// Why a started one-liner has no result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// The output is over [`OUTPUT_CAP_BYTES`]. The process was stopped.
    OutputTooLarge,
    /// The input could not be read while it was sent. The process was stopped.
    Input(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputTooLarge => write!(
                f,
                "the output is larger than {} MiB",
                OUTPUT_CAP_BYTES / (1024 * 1024)
            ),
            Self::Input(message) => write!(f, "the text could not be read: {message}"),
        }
    }
}

impl std::error::Error for RunError {}

/// The end of a one-liner that ran.
#[derive(Debug, PartialEq, Eq)]
pub struct Finished {
    /// The exit code and whether [`Stopper::stop`] ended it. A nonzero code is for the caller to
    /// report: it changes nothing in the document and shows [`Finished::error`].
    pub exit: Exit,
    /// The output as text in pieces, all line breaks LF (the caller makes them the ones of the
    /// document), finished off for the [`OutputKind`].
    pub output: Vec<String>,
    /// The standard error as text, line breaks LF, at most [`ERROR_CAP_BYTES`] of its first part.
    pub error: String,
    /// More of the standard error came than is kept.
    pub error_truncated: bool,
    /// The input ended with a line break (false for no input).
    pub input_ended_with_break: bool,
}

/// What [`Run::poll`] gives; the run is over after it.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Finished(Finished),
    Failed(RunError),
}

/// What the thread that sends the input tells.
struct InputState {
    /// The process has ended: nothing reads the input any more, so sending it is given up (a
    /// descendant of the shell may still hold the pipe open, and then a send would wait for ever).
    /// The text is still read to its end, to know how it ends.
    abandon: AtomicBool,
    ends_with_break: AtomicBool,
    done: AtomicBool,
    error: Mutex<Option<String>>,
}

/// A one-liner that runs. Dropping it does not stop the process; [`Run::stopper`] does.
pub struct Run {
    inner: exec::Run,
    stopper: Stopper,
    output: Option<Collector>,
    error: Option<Collector>,
    kind: OutputKind,
    input: Arc<InputState>,
    exit: Option<Exit>,
    over: bool,
}

/// Starts the one-liner and returns at once (but for the check of the text for `cmd`, which reads
/// it once: call this from a thread that may spend that time on a large text).
pub fn start(request: Request) -> Result<Run, StartError> {
    if request.line.trim().is_empty() {
        return Err(StartError::EmptyLine);
    }
    let program = shell::program(&request.shell).map_err(StartError::Shell)?;
    let invocation = shell::invocation(&program, &request.line);
    let encoding = invocation.kind.text_encoding();

    if encoding == TextEncoding::Oem {
        check_encodable(&request.input, encoding)?;
    }

    let (sender, receiver) = exec::stdin_channel();
    let mut spec = Spec::new(String::new());
    spec.folder = request.folder;
    spec.stdin = Some(receiver);
    spec.path = request.path;
    spec.separate_error = true;
    spec.program = Some(program_of(invocation));
    let inner = exec::start(spec).map_err(|e| StartError::Process {
        program: program.clone(),
        message: e.message,
    })?;
    let stopper = inner.stopper();

    let input = Arc::new(InputState {
        abandon: AtomicBool::new(false),
        ends_with_break: AtomicBool::new(false),
        done: AtomicBool::new(false),
        error: Mutex::new(None),
    });
    feed(request.input, encoding, sender, Arc::clone(&input));

    Ok(Run {
        inner,
        stopper,
        output: Some(Collector::new(
            decoder(encoding),
            OUTPUT_CAP_BYTES,
            Overflow::Fail,
        )),
        error: Some(Collector::new(
            decoder(encoding),
            ERROR_CAP_BYTES,
            Overflow::Truncate,
        )),
        kind: request.output,
        input,
        exit: None,
        over: false,
    })
}

/// The platform's program with its arguments. `cmd` is a Windows program: its command line goes
/// as it is; elsewhere (a `cmd` that is not the one of Windows) it gets the same words as arguments.
fn program_of(invocation: shell::Invocation) -> Program {
    let mut args: Vec<OsString> = invocation.args.into_iter().map(OsString::from).collect();
    let mut raw_args = None;
    if let Some(raw) = invocation.raw_args {
        if cfg!(windows) {
            raw_args = Some(OsString::from(raw));
        } else if let Some(line) = raw
            .strip_prefix("/D /S /C \"")
            .and_then(|s| s.strip_suffix('"'))
        {
            args.extend(["/D", "/S", "/C"].map(OsString::from));
            args.push(OsString::from(line));
        }
    }
    Program {
        program: OsString::from(invocation.program),
        args,
        raw_args,
    }
}

/// Reads the whole text once to see that the encoding can write it.
fn check_encodable(input: &Input, encoding: TextEncoding) -> Result<(), StartError> {
    let mut bad = false;
    let result = input(&mut |chunk| {
        if encode(encoding, chunk).is_none() {
            bad = true;
        }
        !bad
    });
    match result {
        _ if bad => Err(StartError::CannotEncode),
        Err(message) => Err(StartError::Input(message)),
        Ok(()) => Ok(()),
    }
}

/// Sends the text, encoded, from a thread of its own, then closes the input of the process (the
/// sender is dropped). When the process stops reading, the text is still read to its end: the
/// caller needs to know how it ends.
fn feed(input: Input, encoding: TextEncoding, sender: StdinSender, state: Arc<InputState>) {
    thread::spawn(move || {
        let mut sending = true;
        let result = input(&mut |chunk| {
            if !chunk.is_empty() {
                state
                    .ends_with_break
                    .store(chunk.ends_with('\n'), Ordering::SeqCst);
            }
            if sending {
                match encode(encoding, chunk) {
                    // the process does not read any more: stop sending, go on reading
                    Some(bytes) => sending = bytes.is_empty() || send(&sender, bytes, &state),
                    None => {
                        sending = false;
                        let mut error = state.error.lock().unwrap_or_else(|e| e.into_inner());
                        error.get_or_insert_with(|| "cannot encode the text".to_owned());
                    }
                }
            }
            true
        });
        drop(sender);
        if let Err(message) = result {
            let mut error = state.error.lock().unwrap_or_else(|e| e.into_inner());
            error.get_or_insert(message);
        }
        state.done.store(true, Ordering::SeqCst);
    });
}

/// Hands a piece to the process, waiting while it does not read; `false` when it does not read
/// any more, or has ended ([`InputState::abandon`]).
fn send(sender: &StdinSender, mut piece: Vec<u8>, state: &InputState) -> bool {
    let mut pause = Duration::from_micros(50);
    loop {
        match sender.try_send(piece) {
            Ok(()) => return true,
            Err(TrySend::Closed) => return false,
            Err(TrySend::Full(back)) => {
                if state.abandon.load(Ordering::SeqCst) {
                    return false;
                }
                piece = back;
                thread::sleep(pause);
                pause = (pause * 2).min(Duration::from_millis(2));
            }
        }
    }
}

fn encode(encoding: TextEncoding, text: &str) -> Option<Vec<u8>> {
    match encoding {
        TextEncoding::Utf8 => Some(text.as_bytes().to_vec()),
        #[cfg(windows)]
        TextEncoding::Oem => exec::oem::encode(text),
        #[cfg(not(windows))]
        TextEncoding::Oem => Some(text.as_bytes().to_vec()),
    }
}

fn decoder(encoding: TextEncoding) -> Box<dyn TextDecoder + Send> {
    match encoding {
        TextEncoding::Utf8 => Box::new(Utf8Decoder::new()),
        #[cfg(windows)]
        TextEncoding::Oem => Box::new(OemDecoder::default()),
        #[cfg(not(windows))]
        TextEncoding::Oem => Box::new(Utf8Decoder::new()),
    }
}

/// The OEM code page in pieces: the first half of a two-byte character waits for the second.
#[cfg(windows)]
#[derive(Default)]
struct OemDecoder {
    waiting: Vec<u8>,
}

#[cfg(windows)]
impl TextDecoder for OemDecoder {
    fn push(&mut self, bytes: &[u8]) -> String {
        self.waiting.extend_from_slice(bytes);
        let ready = self.waiting.len() - exec::oem::held_len(&self.waiting);
        let text = exec::oem::decode_lossy(&self.waiting[..ready]);
        self.waiting.drain(..ready);
        text
    }

    fn finish(&mut self) -> String {
        exec::oem::decode_lossy(&std::mem::take(&mut self.waiting))
    }
}

impl Run {
    /// The handle that ends the process (the cancel button). The result then has
    /// `exit.stopped`; the output is for the caller to throw away.
    pub fn stopper(&self) -> Stopper {
        self.stopper.clone()
    }

    /// The result, waiting at most `wait` for it; `None` when it is not there yet, and after it
    /// has been given. Output is kept inside until the process has ended.
    ///
    /// The call takes in output for the time of `wait`, but at least `WORK_SLICE` (20 ms): a caller
    /// that polls with a zero wait still gets the output through. It returns within about that
    /// time, however fast the command writes.
    pub fn poll(&mut self, wait: Duration) -> Option<Event> {
        if self.over {
            return None;
        }
        let deadline = Instant::now() + wait.max(WORK_SLICE);
        let wait_until = Instant::now() + wait;
        loop {
            // `done` first: an error that the thread records is stored before it says it is done
            let input_done = self.input.done.load(Ordering::SeqCst);
            if let Some(message) = self.input_error() {
                return Some(self.fail(RunError::Input(message)));
            }
            if let Some(exit) = self.exit
                && input_done
            {
                return Some(self.finished(exit));
            }
            let now = Instant::now();
            if self.exit.is_some() {
                // only the thread that reads the text is left: a moment
                if now >= wait_until {
                    return None;
                }
                thread::sleep(wait_until.saturating_duration_since(now).min(POLL_SLEEP));
                continue;
            }
            match self.inner.poll(wait_until.saturating_duration_since(now)) {
                Some(exec::Event::Output(bytes)) => {
                    let pushed = self.output.as_mut().map(|c| c.push(&bytes));
                    if matches!(pushed, Some(Err(_))) {
                        return Some(self.fail(RunError::OutputTooLarge));
                    }
                }
                Some(exec::Event::Error(bytes)) => {
                    if let Some(collector) = &mut self.error {
                        // the error keeps its first part and never fails
                        let _ = collector.push(&bytes);
                    }
                }
                Some(exec::Event::Exited(exit)) => {
                    self.exit = Some(exit);
                    self.input.abandon.store(true, Ordering::SeqCst);
                }
                // nothing is there: do not spin until the deadline, which is for a stream of output
                None if Instant::now() >= wait_until => return None,
                None => {}
            }
            if Instant::now() >= deadline {
                return None;
            }
        }
    }

    fn input_error(&self) -> Option<String> {
        self.input
            .error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// The process is ended and the run is over.
    fn fail(&mut self, error: RunError) -> Event {
        // stopping can take a moment (Windows starts `taskkill`): not on the thread that polls, which is the window's
        let stopper = self.stopper.clone();
        thread::spawn(move || stopper.stop());
        self.over = true;
        Event::Failed(error)
    }

    fn finished(&mut self, exit: Exit) -> Event {
        self.over = true;
        let output = self.output.take().map(Collector::finish);
        let error = self.error.take().map(Collector::finish);
        let Some(Ok(mut output)) = output else {
            return Event::Failed(RunError::OutputTooLarge);
        };
        let ended = self.input.ends_with_break.load(Ordering::SeqCst);
        match self.kind {
            OutputKind::Replace => for_replace(&mut output.pieces, ended),
            OutputKind::NextLine => for_next_line(&mut output.pieces),
            OutputKind::Window => {}
        }
        let error = error.and_then(Result::ok).unwrap_or_default();
        Event::Finished(Finished {
            exit,
            output: output.pieces,
            error: error.pieces.concat(),
            error_truncated: error.truncated,
            input_ended_with_break: ended,
        })
    }
}
