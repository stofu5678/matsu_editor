//! Runs a macro as a child process. All the I/O happens on threads of its own, so a caller that
//! lives on a UI thread never blocks: it gets `HostEvent`s from a sink and answers a request
//! with `Session::send`.

use std::ffi::{OsStr, OsString};
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::boot;
use crate::protocol::{self, InitializeInfo, ScriptSource};

/// The time that a macro may run if the editor does not say otherwise.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How long to wait for the rest of the output after the child has ended. A grandchild that
/// inherited the pipes can hold them open for much longer.
const OUTPUT_GRACE: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    JavaScript,
    Python,
    Ruby,
    Php,
    /// Run by the bundled script host (`ume-script --engine <name>`), which sits next to the
    /// editor's executable.
    Bundled(Engine),
}

/// The engines of the bundled script host. A new engine adds a variant here, its names below
/// and its extension in [`Language::of_file`]; the rest of the code handles every engine alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Lua,
    QuickJs,
    Mruby,
    MicroPython,
}

impl Engine {
    /// The name that `ume-script --engine` takes.
    pub fn name(self) -> &'static str {
        match self {
            Self::Lua => "lua",
            Self::QuickJs => "quickjs",
            Self::Mruby => "mruby",
            Self::MicroPython => "micropython",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Lua => "Lua (ume-script)",
            Self::QuickJs => "QuickJS (ume-script)",
            Self::Mruby => "mruby (ume-script)",
            Self::MicroPython => "MicroPython (ume-script)",
        }
    }
}

/// The file name of the bundled script host.
pub const SCRIPT_HOST: &str = if cfg!(windows) {
    "ume-script.exe"
} else {
    "ume-script"
};

impl Language {
    /// The language of a macro file, by its extension.
    pub fn of_file(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "mjs" | "cjs" => Some(Self::JavaScript),
            "py" => Some(Self::Python),
            "rb" => Some(Self::Ruby),
            "php" => Some(Self::Php),
            "lua" => Some(Self::Bundled(Engine::Lua)),
            "js" => Some(Self::Bundled(Engine::QuickJs)),
            "mruby" => Some(Self::Bundled(Engine::Mruby)),
            "upy" => Some(Self::Bundled(Engine::MicroPython)),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::JavaScript => "Node.js",
            Self::Python => "Python",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::Bundled(engine) => engine.display_name(),
        }
    }

    /// The usual name of the interpreter on this OS, which the system finds on the `PATH`.
    pub fn default_interpreter(self) -> OsString {
        match self {
            // Next to the editor; if that fails, the system looks on the `PATH`.
            Self::Bundled(_) => std::env::current_exe()
                .ok()
                .map(|exe| exe.with_file_name(SCRIPT_HOST))
                .filter(|host| host.is_file())
                .map_or_else(|| SCRIPT_HOST.into(), PathBuf::into_os_string),
            Self::JavaScript => "node".into(),
            Self::Python if cfg!(windows) => "python".into(),
            Self::Python => "python3".into(),
            Self::Ruby => "ruby".into(),
            Self::Php => "php".into(),
        }
    }

    /// Asks the interpreter for the path of its real executable, once. On Windows the `python`
    /// of the Install Manager starts through a launcher process and costs about 100 ms at every
    /// start (docs/poc/json-rpc.md); the editor may call this once, keep the result and put it
    /// in `MacroConfig::interpreter`. Nothing calls it by default.
    pub fn real_interpreter_path(self, interpreter: &OsStr) -> io::Result<PathBuf> {
        let (flag, code) = match self {
            // The host is the executable itself; there is no launcher to see through.
            Self::Bundled(_) => return Ok(PathBuf::from(interpreter)),
            Self::JavaScript => ("-p", "process.execPath"),
            Self::Python => ("-c", "import sys; print(sys.executable)"),
            // A version manager (asdf, rbenv) answers with a shim; this is the real one.
            Self::Ruby => ("-e", "require 'rbconfig'; puts RbConfig.ruby"),
            Self::Php => ("-r", "echo PHP_BINARY;"),
        };
        let output = Command::new(interpreter)
            .arg(flag)
            .arg(code)
            // A pipe gets the ANSI code page on Windows by default (the same as in `start`).
            .env("PYTHONUTF8", "1")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let line = protocol::trim_line_end(text.lines().next().unwrap_or_default());
        if !output.status.success() || line.is_empty() {
            return Err(io::Error::other(format!(
                "the interpreter did not tell its path ({})",
                output.status
            )));
        }
        Ok(PathBuf::from(line))
    }
}

/// What the editor tells the host about one run. `for_script` fills in the defaults; the
/// editor then changes the fields it has settings for.
pub struct MacroConfig {
    pub script: PathBuf,
    pub language: Language,
    /// The interpreter to start: a name found on the `PATH`, or a path.
    pub interpreter: OsString,
    /// The longest time that the macro may run.
    pub timeout: Duration,
    /// The directory in which the boot files get their own folder. `None` is the user's
    /// temporary directory.
    pub temp_dir: Option<PathBuf>,
    /// The path of the document for `initialize`, `None` when it is not a file.
    pub document_path: Option<String>,
}

impl MacroConfig {
    /// A configuration from the file's extension: the usual interpreter and the default timeout.
    pub fn for_script(script: &Path, document_path: Option<String>) -> Result<Self, StartError> {
        let language = Language::of_file(script).ok_or_else(|| StartError::UnknownLanguage {
            script: script.to_path_buf(),
        })?;
        Ok(Self {
            script: script.to_path_buf(),
            language,
            interpreter: language.default_interpreter(),
            timeout: DEFAULT_TIMEOUT,
            temp_dir: None,
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
    /// The script could not be read (the bundled host gets its text from the editor).
    ReadScript {
        script: PathBuf,
        source: std::io::Error,
    },
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
                "{}: マクロの拡張子は .mjs、.js、.cjs、.py、.rb、.php、.lua、.mruby、.upy のどれかにしてください",
                script.display()
            ),
            Self::Boot(err) => write!(f, "マクロの準備に失敗しました: {err}"),
            Self::ReadScript { script, source } => write!(
                f,
                "{} を読めませんでした (UTF-8 で保存してください): {source}",
                script.display()
            ),
            Self::Spawn {
                language,
                interpreter,
                source,
            } => write!(
                f,
                "{} を起動できませんでした ({}): {source}。インストールされているか、場所の指定が合っているか確かめてください",
                language.display_name(),
                interpreter.to_string_lossy()
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

/// The time that the macro has spent waiting for the window (a dialog, `Sleep`), which does not
/// count against its timeout.
#[derive(Debug, Default)]
struct Waiting {
    /// How many calls are being waited for now.
    holds: usize,
    /// When the oldest wait began.
    since: Option<Instant>,
    /// The time of the waits that are over.
    spent: Duration,
}

/// A running macro: answer requests with `send` and stop it with `cancel`.
pub struct Session {
    writer: mpsc::Sender<String>,
    cancel: Arc<AtomicBool>,
    waiting: Arc<Mutex<Waiting>>,
}

impl Session {
    /// The window starts to wait for the user on behalf of the macro (a dialog is open): the
    /// timeout stops running until the matching [`release`](Self::release). A macro that is
    /// stopped by the timeout while the user reads its message box would be a bug.
    pub fn hold(&self) {
        let mut waiting = self.waiting.lock().unwrap_or_else(PoisonError::into_inner);
        waiting.holds += 1;
        waiting.since.get_or_insert_with(Instant::now);
    }

    /// Ends one [`hold`](Self::hold). The time of the wait is added to the time the macro may
    /// run.
    pub fn release(&self) {
        let mut waiting = self.waiting.lock().unwrap_or_else(PoisonError::into_inner);
        waiting.holds = waiting.holds.saturating_sub(1);
        if waiting.holds == 0
            && let Some(since) = waiting.since.take()
        {
            waiting.spent += since.elapsed();
        }
    }

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
    let script = match config.language {
        Language::Bundled(_) => Some(read_script(&config.script)?),
        Language::JavaScript | Language::Python | Language::Ruby | Language::Php => None,
    };
    // The bundled host needs no boot files.
    let boot_dir = if matches!(config.language, Language::Bundled(_)) {
        None
    } else {
        Some(boot::write_boot_files(config.temp_dir.as_deref()).map_err(StartError::Boot)?)
    };
    let boot_files = |name: &str| {
        boot_dir
            .as_ref()
            .map(|dir| dir.path().join(name))
            .unwrap_or_default()
    };
    let mut command = Command::new(&config.interpreter);
    match config.language {
        Language::Bundled(engine) => {
            // The text goes in `initialize`, so there is no path to open.
            command.arg("--engine").arg(engine.name());
        }
        Language::JavaScript => {
            command
                .arg("--require")
                .arg(boot_files(boot::JS_BOOT))
                .arg(&config.script);
        }
        Language::Python => {
            // -u: no buffering, so the user's output and the requests stay in order.
            command
                .arg("-u")
                .arg(boot_files(boot::PY_BOOT))
                .arg(&config.script);
        }
        Language::Ruby => {
            // -E UTF-8: the user's strings and files are UTF-8 whatever the OS locale is.
            command
                .arg("-E")
                .arg("UTF-8")
                .arg(boot_files(boot::RB_BOOT))
                .arg(&config.script);
        }
        Language::Php => {
            // The errors go to the standard error (the output panel shows them as the macro's
            // output, with the line number), once, and as plain text.
            command
                .args([
                    "-d",
                    "display_errors=stderr",
                    "-d",
                    "log_errors=0",
                    "-d",
                    "html_errors=0",
                    "-d",
                    "error_reporting=E_ALL",
                    "-d",
                    "output_buffering=0",
                ])
                .arg(boot_files(boot::PHP_BOOT))
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
            script,
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

    // `Finished` must be the last event. A grandchild that inherited the pipes can keep a reader
    // running after the grace period, so the readers send through a gate that the supervisor
    // closes (while it sends `Finished`) and that drops what comes later.
    let ended = Arc::new(Mutex::new(false));
    let gated: Sink = {
        let sink = sink.clone();
        let ended = ended.clone();
        Arc::new(move |event| {
            let ended = ended.lock().unwrap_or_else(PoisonError::into_inner);
            if !*ended {
                sink(event);
            }
        })
    };

    let (done_tx, done_rx) = mpsc::channel::<()>();
    {
        let sink = gated.clone();
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
        let sink = gated;
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
    let waiting = Arc::new(Mutex::new(Waiting::default()));
    let supervisor_waiting = waiting.clone();
    thread::spawn(move || {
        // The boot files live as long as the child.
        let mut boot_dir = boot_dir;
        let mut reason = EndReason::Exited;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {}
                Err(_) => break None,
            }
            if supervisor_cancel.load(Ordering::SeqCst) {
                reason = EndReason::Cancelled;
            } else {
                // The time spent waiting for the window does not count.
                let waiting = supervisor_waiting
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                if waiting.holds == 0 && Instant::now() >= deadline + waiting.spent {
                    reason = EndReason::TimedOut;
                }
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
        // The folder goes before the end is reported. The caller may exit the process as soon as
        // it has seen `Finished`, which would cut the removal short and leave the folder.
        // A folder that cannot be removed is tried once more when this thread drops it.
        if let Some(dir) = &mut boot_dir {
            let _ = dir.remove();
        }
        let mut ended = ended.lock().unwrap_or_else(PoisonError::into_inner);
        *ended = true;
        sink(HostEvent::Finished(Finished {
            reason,
            exit_code: status.and_then(|s| s.code()),
            status: status.map_or_else(|| "unknown".into(), |s| s.to_string()),
        }));
    });

    Ok(Session {
        writer,
        cancel,
        waiting,
    })
}

/// Reads the script for a host that gets its text in `initialize`. It must be UTF-8; a BOM is
/// dropped.
fn read_script(path: &Path) -> Result<ScriptSource, StartError> {
    let fail = |source| StartError::ReadScript {
        script: path.to_path_buf(),
        source,
    };
    let bytes = std::fs::read(path).map_err(fail)?;
    let source = String::from_utf8(bytes)
        .map_err(|e| fail(io::Error::new(io::ErrorKind::InvalidData, e)))?;
    Ok(ScriptSource {
        name: path.display().to_string(),
        source: source
            .strip_prefix('\u{feff}')
            .unwrap_or(&source)
            .to_string(),
    })
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
    fn a_script_for_the_host_is_read_as_utf8_without_its_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("あ.lua");
        std::fs::write(&path, "\u{feff}print('日本語')").unwrap();
        let script = read_script(&path).unwrap();
        assert_eq!(script.source, "print('日本語')");
        assert_eq!(script.name, path.display().to_string());

        std::fs::write(&path, b"print('\x82\xa0')").unwrap(); // Shift_JIS
        assert!(matches!(
            read_script(&path),
            Err(StartError::ReadScript { .. })
        ));
        assert!(read_script(&dir.path().join("none.lua")).is_err());
    }

    #[test]
    fn language_by_extension() {
        assert_eq!(
            Language::of_file(Path::new("a.cjs")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::of_file(Path::new("a.js")),
            Some(Language::Bundled(Engine::QuickJs))
        );
        assert_eq!(Language::of_file(Path::new("a.PY")), Some(Language::Python));
        assert_eq!(
            Language::of_file(Path::new("a.mjs")),
            Some(Language::JavaScript)
        );
        assert_eq!(Language::of_file(Path::new("a.rb")), Some(Language::Ruby));
        assert_eq!(Language::of_file(Path::new("a.PHP")), Some(Language::Php));
        assert_eq!(
            Language::of_file(Path::new("a.Lua")),
            Some(Language::Bundled(Engine::Lua))
        );
        assert_eq!(
            Language::of_file(Path::new("a.UPY")),
            Some(Language::Bundled(Engine::MicroPython))
        );
        assert_eq!(Language::of_file(Path::new("a.txt")), None);
        assert_eq!(Language::of_file(Path::new("noext")), None);
    }
}
