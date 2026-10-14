//! Runs a macro as a child process. All the I/O happens on threads of its own, so a caller that
//! lives on a UI thread never blocks: it gets `HostEvent`s from a sink and answers a request
//! with `Session::send`.

use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::boot;
use super::protocol::{self, InitializeInfo};

/// The time that a macro may run if `POC_MACRO_TIMEOUT` does not say otherwise.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How long to wait for the rest of the output after the child has ended. A grandchild that
/// inherited the pipes can hold them open for much longer.
const OUTPUT_GRACE: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    JavaScript,
    Python,
}

impl Language {
    /// The language of a macro file, by its extension.
    pub fn of_file(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "js" | "cjs" => Some(Self::JavaScript),
            "py" => Some(Self::Python),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::JavaScript => "Node.js",
            Self::Python => "Python",
        }
    }

    pub fn env_var(self) -> &'static str {
        match self {
            Self::JavaScript => "POC_NODE",
            Self::Python => "POC_PYTHON",
        }
    }

    /// The interpreter: the environment variable, or the usual name on this OS.
    pub fn interpreter(self) -> OsString {
        if let Some(path) = std::env::var_os(self.env_var()).filter(|p| !p.is_empty()) {
            return path;
        }
        match self {
            Self::JavaScript => "node".into(),
            Self::Python if cfg!(windows) => "python".into(),
            Self::Python => "python3".into(),
        }
    }
}

/// The timeout from `POC_MACRO_TIMEOUT` (seconds, may have a fraction), or the default.
pub fn timeout_from_env() -> Duration {
    parse_timeout(std::env::var("POC_MACRO_TIMEOUT").ok().as_deref())
}

fn parse_timeout(text: Option<&str>) -> Duration {
    text.and_then(|text| text.trim().parse::<f64>().ok())
        .filter(|seconds| seconds.is_finite() && *seconds > 0.0 && *seconds < 1.0e9)
        .map_or(DEFAULT_TIMEOUT, Duration::from_secs_f64)
}

pub struct MacroConfig {
    pub script: PathBuf,
    pub language: Language,
    pub interpreter: OsString,
    pub timeout: Duration,
    /// The path of the document for `initialize`, `None` when it is not a file.
    pub document_path: Option<String>,
}

impl MacroConfig {
    /// A configuration from the file's extension and the environment variables.
    pub fn for_script(script: &Path, document_path: Option<String>) -> Result<Self, StartError> {
        let language = Language::of_file(script).ok_or_else(|| StartError::UnknownLanguage {
            script: script.to_path_buf(),
        })?;
        Ok(Self {
            script: script.to_path_buf(),
            language,
            interpreter: language.interpreter(),
            timeout: timeout_from_env(),
            document_path,
        })
    }
}

#[derive(Debug)]
pub enum StartError {
    UnknownLanguage {
        script: PathBuf,
    },
    Boot(std::io::Error),
    Spawn {
        language: Language,
        interpreter: OsString,
        source: std::io::Error,
    },
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownLanguage { script } => write!(
                f,
                "{}: マクロの拡張子は .js か .py にしてください",
                script.display()
            ),
            Self::Boot(err) => write!(f, "マクロの準備に失敗しました: {err}"),
            Self::Spawn {
                language,
                interpreter,
                source,
            } => write!(
                f,
                "{} を起動できませんでした ({}): {source}。環境変数 {} で場所を指定できます",
                language.display_name(),
                interpreter.to_string_lossy(),
                language.env_var()
            ),
        }
    }
}

impl std::error::Error for StartError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    /// The macro ended by itself (with any exit code).
    Exited,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub reason: EndReason,
    /// `None` when the process was killed by a signal.
    pub exit_code: Option<i32>,
    /// The exit status in words, such as "exit status: 1".
    pub status: String,
}

#[derive(Debug)]
pub enum HostEvent {
    /// A line of user output, without its line break. Output without a line break is held
    /// until its line ends or the macro exits.
    Output { stream: Stream, text: String },
    /// A JSON-RPC message from the macro (the text after the RS).
    Message(String),
    /// The last event.
    Finished(Finished),
}

/// A running macro: answer requests with `send` and stop it with `cancel`.
pub struct Session {
    writer: mpsc::Sender<String>,
    cancel: Arc<AtomicBool>,
}

impl Session {
    /// Queues a line for the macro's standard input. It does not block.
    pub fn send(&self, line: String) {
        let _ = self.writer.send(line);
    }

    /// Kills the macro. `Finished` follows with `EndReason::Cancelled`.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

impl Drop for Session {
    /// A macro whose owner has gone (the window closed, for one) is stopped.
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

pub type Sink = Arc<dyn Fn(HostEvent) + Send + Sync>;

/// Starts the macro. The events go to `sink` from other threads, in order, ending with
/// `Finished`.
pub fn start(config: MacroConfig, sink: Sink) -> Result<Session, StartError> {
    let boot_dir = boot::write_boot_files().map_err(StartError::Boot)?;
    let mut command = Command::new(&config.interpreter);
    match config.language {
        Language::JavaScript => {
            command
                .arg("--require")
                .arg(boot_dir.path().join(boot::JS_BOOT))
                .arg(&config.script);
        }
        Language::Python => {
            // -u: no buffering, so the user's output and the requests stay in order.
            command
                .arg("-u")
                .arg(boot_dir.path().join(boot::PY_BOOT))
                .arg(&config.script);
        }
    }
    command
        // Python writes to a pipe in the ANSI code page of Windows by default.
        .env("PYTHONUTF8", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console window for the interpreter.
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|source| StartError::Spawn {
        language: config.language,
        interpreter: config.interpreter.clone(),
        source,
    })?;
    let stdin = child.stdin.take().expect("stdin is piped");
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");

    // The writer: the lines go out in order, and a macro that does not read never blocks the
    // caller. When the child is gone the writes fail and the rest is dropped.
    let (writer, lines) = mpsc::channel::<String>();
    writer
        .send(protocol::initialize_message(&InitializeInfo {
            editor_name: "UME Editor".into(),
            editor_version: env!("CARGO_PKG_VERSION").into(),
            process_id: std::process::id(),
            document_path: config.document_path.clone(),
        }))
        .expect("the receiver is alive");
    thread::spawn(move || {
        let mut stdin = stdin;
        for line in lines {
            // Only LF ends a line (Emacs on Linux reads a CR as a line end too).
            if stdin
                .write_all(line.as_bytes())
                .and_then(|()| stdin.write_all(b"\n"))
                .and_then(|()| stdin.flush())
                .is_err()
            {
                break;
            }
        }
        // Dropping stdin closes it: the macro sees EOF.
    });

    let (done_tx, done_rx) = mpsc::channel::<()>();
    {
        let sink = sink.clone();
        let done = done_tx.clone();
        thread::spawn(move || {
            read_lines(stdout, |line| {
                let split = protocol::split_output_line(line);
                if let Some(text) = split.user {
                    sink(HostEvent::Output {
                        stream: Stream::Stdout,
                        text: text.to_string(),
                    });
                }
                if let Some(message) = split.message {
                    sink(HostEvent::Message(message.to_string()));
                }
            });
            let _ = done.send(());
        });
    }
    {
        let sink = sink.clone();
        thread::spawn(move || {
            read_lines(stderr, |line| {
                sink(HostEvent::Output {
                    stream: Stream::Stderr,
                    text: line.to_string(),
                });
            });
            let _ = done_tx.send(());
        });
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let supervisor_cancel = cancel.clone();
    let deadline = Instant::now() + config.timeout;
    thread::spawn(move || {
        // The boot files live as long as the child.
        let _boot_dir = boot_dir;
        let mut reason = EndReason::Exited;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {}
                Err(_) => break None,
            }
            if supervisor_cancel.load(Ordering::SeqCst) {
                reason = EndReason::Cancelled;
            } else if Instant::now() >= deadline {
                reason = EndReason::TimedOut;
            }
            if reason != EndReason::Exited {
                let _ = child.kill();
                break child.wait().ok();
            }
            thread::sleep(Duration::from_millis(10));
        };
        // The readers end when the pipes close. Wait for the rest of the output, but not
        // forever.
        let grace_end = Instant::now() + OUTPUT_GRACE;
        for _ in 0..2 {
            let left = grace_end.saturating_duration_since(Instant::now());
            let _ = done_rx.recv_timeout(left);
        }
        sink(HostEvent::Finished(Finished {
            reason,
            exit_code: status.and_then(|s| s.code()),
            status: status.map_or_else(|| "unknown".into(), |s| s.to_string()),
        }));
    });

    Ok(Session { writer, cancel })
}

/// Reads lines until the end of the stream. The bytes are read as UTF-8, and anything that is
/// not (a console code page, for example) becomes U+FFFD. A line's CR LF or LF is dropped.
fn read_lines(stream: impl std::io::Read, mut on_line: impl FnMut(&str)) {
    let mut reader = BufReader::new(stream);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buffer);
                on_line(protocol::trim_line_end(&text));
            }
        }
    }
}

/// What a finished macro left behind.
#[derive(Debug)]
pub struct Report {
    pub output: Vec<(Stream, String)>,
    pub finished: Finished,
}

impl Report {
    /// The lines of one stream.
    #[cfg(test)]
    pub fn lines(&self, stream: Stream) -> Vec<&str> {
        self.output
            .iter()
            .filter(|(s, _)| *s == stream)
            .map(|(_, text)| text.as_str())
            .collect()
    }
}

/// Runs a macro and waits for it, answering each request with `handler` (which returns the
/// response line, or `None` for a notification). This blocks the calling thread: the editor
/// uses `start` instead. The tests and the command line mode use this.
pub fn run_blocking(
    config: MacroConfig,
    mut handler: impl FnMut(&str) -> Option<String>,
) -> Result<Report, StartError> {
    let (tx, rx) = mpsc::channel();
    let sink: Sink = Arc::new(move |event| {
        let _ = tx.send(event);
    });
    let session = start(config, sink)?;
    let mut output = Vec::new();
    for event in rx {
        match event {
            HostEvent::Output { stream, text } => output.push((stream, text)),
            HostEvent::Message(text) => {
                if let Some(response) = handler(&text) {
                    session.send(response);
                }
            }
            HostEvent::Finished(finished) => return Ok(Report { output, finished }),
        }
    }
    unreachable!("the supervisor always sends Finished")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_parsing() {
        assert_eq!(parse_timeout(None), DEFAULT_TIMEOUT);
        assert_eq!(parse_timeout(Some("5")), Duration::from_secs(5));
        assert_eq!(parse_timeout(Some(" 0.5 ")), Duration::from_millis(500));
        for bad in ["", "abc", "0", "-3", "NaN", "inf"] {
            assert_eq!(parse_timeout(Some(bad)), DEFAULT_TIMEOUT, "{bad}");
        }
    }

    #[test]
    fn language_by_extension() {
        assert_eq!(
            Language::of_file(Path::new("a.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(Language::of_file(Path::new("a.PY")), Some(Language::Python));
        assert_eq!(Language::of_file(Path::new("a.txt")), None);
        assert_eq!(Language::of_file(Path::new("noext")), None);
    }
}
