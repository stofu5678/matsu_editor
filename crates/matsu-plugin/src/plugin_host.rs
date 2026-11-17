//! A resident host for one plugin (ADR-0026, chapter 2).
//!
//! A macro is a process that runs once. A plugin is called again and again, by the events it
//! declared, so its process can stay: the editor sends `plugin/invoke` (the jack, the text of
//! the handler, the arguments), the plugin runs the handler, asks the editor for what it needs
//! while it runs (`Editor.*`, `Plugin.*`), and answers the invoke. The messages are those of the
//! macro protocol (`docs/poc/json-rpc.md`); the host is `matsu-script --engine quickjs` for
//! `.js`, and the boot of the macro (`boot.cjs`, `boot.py`, `boot.rb`, `boot.php`) in plugin mode for Node.js,
//! Python, Ruby and PHP.
//!
//! * One host for one plugin and one language ([`PluginHost`]); it starts when the first call is
//!   made (lazily), never before.
//! * Resident (`UseCache=1` in `plugin.def`) keeps the process for the next call; otherwise the
//!   process ends after each call, as Sakura's plugin that is not cached is loaded again.
//! * A call has a time limit by its jack ([`LIMITS`], one table). A process that does not answer
//!   in time is killed. A process that dies, a handler that throws and a timeout are failures;
//!   [`MAX_FAILURES`] in a row stop the plugin until the user enables it again.
//!
//! The host does not block: [`PluginHost::poll`] gives what happened, and the caller answers the
//! plugin's requests. [`PluginHost::run_blocking`] is the loop for tests and the command line.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::api;
use crate::protocol::{self, PluginIdentity, RpcError};
use crate::session::{self, HostEvent, Language, MacroConfig, Session, Sink, StartError, Stream};

/// How many failures in a row stop a plugin (ADR-0026: three).
pub const MAX_FAILURES: u32 = 3;

/// The time that a call may take, by its jack (ADR-0026, chapter 2). The numbers are the
/// proposal of the ADR; the user decides them by using the plugins, so they are here and nowhere
/// else. A call that waits for the user (a dialog) does not count while it waits.
pub const LIMITS: &[(&str, Duration)] = &[
    ("Command", Duration::from_secs(30)),
    ("EditorStart", Duration::from_secs(5)),
    ("EditorEnd", Duration::from_secs(5)),
    ("DocumentOpen", Duration::from_secs(5)),
    ("DocumentClose", Duration::from_secs(5)),
    ("DocumentBeforeSave", Duration::from_secs(5)),
    ("DocumentAfterSave", Duration::from_secs(5)),
    ("Outline", Duration::from_secs(10)),
    ("SmartIndent", Duration::from_millis(500)),
    ("Complement", Duration::from_secs(1)),
    ("ComplementGlobal", Duration::from_secs(1)),
];

/// The limit of a jack that [`LIMITS`] does not name.
pub const DEFAULT_LIMIT: Duration = Duration::from_secs(5);

/// The time limit of a call from `jack`.
pub fn limit_of(jack: &str) -> Duration {
    LIMITS
        .iter()
        .find(|(name, _)| *name == jack)
        .map_or(DEFAULT_LIMIT, |(_, limit)| *limit)
}

/// How many meaningless messages [`PluginHost::poll`] drops in one call.
const MAX_DROPPED_IN_A_ROW: usize = 256;

/// How many events the host keeps for the window.
const EVENT_BACKLOG: usize = 4096;

/// How long a process that was told to end may take before it is killed.
const SHUTDOWN_GRACE: Duration = Duration::from_millis(1500);

/// The longest a call stays open after its answer while it waits for the lines that the handler
/// wrote to standard error before it (for the marker of the call, or, in a language that cannot
/// write one, for a quiet time), even if the process keeps writing or never writes the marker.
const SETTLE_CAP: Duration = Duration::from_millis(250);

/// In a language without a marker: how long a call stays open after its answer with nothing new
/// from the process.
const SETTLE_QUIET: Duration = Duration::from_millis(20);

/// What a host needs to know about its plugin.
#[derive(Debug, Clone)]
pub struct HostSettings {
    pub id: String,
    /// The folder of the plugin; the handlers are files in it.
    pub dir: PathBuf,
    pub language: Language,
    /// The program that serves the plugin: for the bundled languages `matsu-script`.
    pub interpreter: OsString,
    /// Keep the process for the next call (`UseCache=1`).
    pub resident: bool,
    /// Where the boot files of an external interpreter go (`None`: the temporary folder).
    pub temp_dir: Option<PathBuf>,
    /// The files the user agreed to (`plugins.ini`): the path from the folder, with `/`, and the
    /// SHA-256 in lower case hex. When set, the host hashes the bytes of every file it reads and
    /// refuses a file that is not in the list or does not match ([`BeginError::Changed`]); it then
    /// sends exactly those bytes to the plugin, and never reads them again.
    pub verified: Option<BTreeMap<String, String>>,
}

impl HostSettings {
    /// The usual interpreter of the language.
    pub fn new(id: &str, dir: &Path, language: Language, resident: bool) -> Self {
        Self {
            id: id.to_owned(),
            dir: dir.to_owned(),
            language,
            interpreter: language.default_interpreter(),
            resident,
            temp_dir: None,
            verified: None,
        }
    }

    /// The files the user agreed to ([`verified`](Self::verified)).
    #[must_use]
    pub fn with_verified(mut self, files: impl IntoIterator<Item = (String, String)>) -> Self {
        self.verified = Some(files.into_iter().collect());
        self
    }
}

/// How a call ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The handler ended.
    Ok,
    /// The handler threw: the text of the error, with the line of the script.
    Failed(String),
    /// The time limit passed; the process was killed.
    TimedOut,
    /// The user stopped it (not a failure).
    Cancelled,
    /// The process ended during the call (it crashed, or something killed it).
    Died(String),
}

impl Outcome {
    /// Whether this counts as a failure that adds up to a stop.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed(_) | Self::TimedOut | Self::Died(_))
    }
}

/// Something that happened during a call.
#[derive(Debug)]
pub enum Progress {
    /// A line the plugin printed.
    Output { stream: Stream, text: String },
    /// A request of the plugin (a line of JSON). Answer it with [`PluginHost::reply`].
    Request(String),
    /// The call is over.
    Done(Outcome),
}

/// Why a call could not begin.
#[derive(Debug)]
pub enum BeginError {
    /// Three failures in a row stopped the plugin.
    Stopped,
    /// A call is still running.
    Busy,
    /// The file of the handler cannot be read (it must be UTF-8).
    Handler { path: PathBuf, source: io::Error },
    /// The handler is not a file in the folder of the plugin (`..`, an absolute path, or a link
    /// that leads out). The agreement of the user covers the files of the folder and no others.
    Outside { handler: String },
    /// A file is not the one the user agreed to (it changed, or was added, after the agreement).
    Changed { path: String },
    /// The files that a Node.js plugin may import are too many or too large to hand over.
    Bundle { reason: String },
    /// The process did not start.
    Start(StartError),
}

impl std::fmt::Display for BeginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "失敗が続いたので止めています"),
            Self::Busy => write!(f, "前の呼び出しがまだ終わっていません"),
            Self::Outside { handler } => write!(
                f,
                "{handler} はプラグインのフォルダーの外にあります。フォルダーの中のファイルだけ動かせます"
            ),
            Self::Changed { path } => write!(
                f,
                "{path} が、同意したときのファイルと違います。もう一度「有効にする」を押してください"
            ),
            Self::Bundle { reason } => write!(f, "プラグインのファイルを渡せません: {reason}"),
            Self::Handler { path, source } => write!(
                f,
                "{} を読めませんでした (UTF-8 で保存してください): {source}",
                path.display()
            ),
            Self::Start(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for BeginError {}

struct Process {
    session: Session,
    events: Receiver<PluginEvent>,
    /// The module files of the plugin folder went to this Node.js process already (with its first
    /// call); it keeps them, so they are not sent again.
    files_sent: bool,
}

/// What the readers of a process send: what the session reports, and the marker of a call (the
/// readers cut it out of standard error, so nobody downstream shows it).
enum PluginEvent {
    Host(HostEvent),
    /// The plugin wrote the marker for the call with this id: all that the handler wrote to
    /// standard error before its answer has come.
    StderrDone(i64),
}

struct Current {
    id: i64,
    jack: String,
    deadline: Instant,
    /// The marker of this call has come.
    marked: bool,
}

/// How a call waits for the standard error of its handler after the answer.
#[derive(Clone, Copy)]
enum Wait {
    /// Until the marker of the call has come.
    Marker,
    /// Until nothing has come for [`SETTLE_QUIET`]: for a language that cannot write the marker.
    Quiet(Instant),
}

/// A call whose answer has come, kept open until the lines that the handler wrote to standard
/// error before the answer have come (see [`PluginHost::settle`]).
struct Settling {
    outcome: Outcome,
    id: i64,
    wait: Wait,
    /// The call ends at this time at the latest, however much comes.
    cap: Instant,
}

/// How a call from `jack` waits after its answer. `None`: it does not wait.
///
/// Every language that serves plugins writes the marker, except AppleScript: `osascript` has no
/// way to write to standard error and flush it (`do shell script` would start a process for each
/// call). There the call waits for a quiet time, except the jacks that run on a key (`SmartIndent`,
/// `Complement`, `ComplementGlobal`), which do not wait: a delay at each key is worse than a
/// line of standard error that comes late.
fn wait_after_answer(language: Language, jack: &str, now: Instant) -> Option<Wait> {
    if language != Language::AppleScript {
        return Some(Wait::Marker);
    }
    if matches!(jack, "SmartIndent" | "Complement" | "ComplementGlobal") {
        return None;
    }
    Some(Wait::Quiet(now + SETTLE_QUIET))
}

/// The sink of a process: the lines of standard error that are markers become [`PluginEvent::StderrDone`].
fn plugin_sink(tx: mpsc::SyncSender<PluginEvent>) -> Sink {
    Arc::new(move |event| {
        let event = match event {
            HostEvent::Output {
                stream: Stream::Stderr,
                text,
            } => match protocol::parse_stderr_done(&text) {
                Some(id) => PluginEvent::StderrDone(id),
                None => PluginEvent::Host(HostEvent::Output {
                    stream: Stream::Stderr,
                    text,
                }),
            },
            other => PluginEvent::Host(other),
        };
        let _ = tx.send(event);
    })
}

/// The host of one plugin in one language.
pub struct PluginHost {
    settings: HostSettings,
    process: Option<Process>,
    failures: u32,
    stopped: bool,
    next_id: i64,
    current: Option<Current>,
    /// The call in `current` has been answered and waits for the late lines of its output.
    settling: Option<Settling>,
    /// [`shutdown`](Self::shutdown) ended a call that was running: the next [`poll`](Self::poll)
    /// says so, so that whoever follows the call can clean up.
    interrupted: bool,
}

impl PluginHost {
    pub fn new(settings: HostSettings) -> Self {
        Self {
            settings,
            process: None,
            failures: 0,
            stopped: false,
            next_id: 1,
            current: None,
            settling: None,
            interrupted: false,
        }
    }

    pub fn settings(&self) -> &HostSettings {
        &self.settings
    }

    /// Whether three failures in a row stopped the plugin.
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// The failures in a row so far.
    pub fn failures(&self) -> u32 {
        self.failures
    }

    /// The user enabled the plugin again: the count starts over.
    pub fn reset(&mut self) {
        self.failures = 0;
        self.stopped = false;
    }

    /// Whether the process is alive now (a resident host between two calls, or a call that
    /// runs).
    pub fn is_running(&self) -> bool {
        self.process.is_some()
    }

    /// The id of the operating system's process, while there is one.
    pub fn process_id(&self) -> Option<u32> {
        self.process.as_ref().map(|p| p.session.process_id())
    }

    pub fn is_busy(&self) -> bool {
        self.current.is_some()
    }

    fn start_process(&self) -> Result<Process, StartError> {
        let identity = PluginIdentity {
            id: self.settings.id.clone(),
            dir: self.settings.dir.display().to_string(),
        };
        let mut config = MacroConfig::for_plugin(identity, self.settings.language, None);
        config.interpreter = self.settings.interpreter.clone();
        config.temp_dir = self.settings.temp_dir.clone();
        // Bounded: a plugin that prints without end waits here (on the reader thread of the
        // host) for the window to catch up, instead of piling up in memory.
        let (tx, events) = mpsc::sync_channel(EVENT_BACKLOG);
        let sink = plugin_sink(tx);
        let session = session::start(&config, sink)?;
        Ok(Process {
            session,
            events,
            files_sent: false,
        })
    }

    /// Starts a call: the process is started now if it is not running (the first call of a
    /// plugin is the first time its process is made), and the invoke is sent. `handler` is a
    /// file in the folder of the plugin.
    pub fn begin(
        &mut self,
        jack: &str,
        handler: &str,
        args: &Value,
        timeout: Duration,
    ) -> Result<(), BeginError> {
        if self.stopped {
            return Err(BeginError::Stopped);
        }
        if self.current.is_some() {
            return Err(BeginError::Busy);
        }
        // A new call replaces the end of the one that `shutdown` cut off: nobody follows that
        // one any more, and its end must not be taken for this call's.
        self.interrupted = false;
        let mut path = handler_path(&self.settings.dir, handler)?;
        // The spelling of the handler in the agreement (the one on the disk): a file system that
        // ignores case opens `Command.cjs` for `command.cjs`, and the record has the latter.
        let mut key = relative_name(handler);
        if let Some(agreed) = &self.settings.verified {
            key = agreed_key(agreed, &key, CASE_INSENSITIVE_FS)
                .ok_or(BeginError::Changed { path: key })?;
            path = handler_path(&self.settings.dir, &key)?;
        }
        // Node.js gets the verified text of every module file of the folder once, with the first
        // call of its process: its hooks serve imports from that text (`boot.cjs`), so no file is
        // read from the disk after the check.
        let node = matches!(
            self.settings.language,
            Language::JavaScript | Language::TypeScript
        );
        let send_files = node && !self.process.as_ref().is_some_and(|p| p.files_sent);
        let files = if send_files {
            Some(self.read_module_files()?)
        } else {
            None
        };
        // A module handler of Node.js (`.mjs`, `.ts`) is loaded once and called again: its text is
        // in the files. Every other handler is read, and checked, for each call.
        let source = if node && is_module_handler(handler) {
            None
        } else {
            Some(self.read_verified(&path, &key)?)
        };
        if self.process.is_none() {
            self.process = Some(self.start_process().map_err(BeginError::Start)?);
        }
        let id = self.next_id;
        self.next_id += 1;
        let mut params = json!({
            "jack": jack,
            "name": path.display().to_string(),
            "args": args,
        });
        if let Some(source) = source {
            params["source"] = json!(source);
        }
        if let Some(files) = files {
            params["files"] = files
                .into_iter()
                .map(|(path, source)| json!({"path": path.display().to_string(), "source": source}))
                .collect();
        }
        let line = protocol::ascii_json(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "plugin/invoke",
            "params": params,
        }));
        if let Some(process) = &mut self.process {
            process.files_sent |= node;
            process.session.send(line);
        }
        self.current = Some(Current {
            id,
            jack: jack.to_owned(),
            deadline: Instant::now() + timeout,
            marked: false,
        });
        Ok(())
    }

    /// The text of a handler, checked against the agreed files when there are any.
    fn read_verified(&self, path: &Path, key: &str) -> Result<String, BeginError> {
        let bytes = std::fs::read(path).map_err(|source| BeginError::Handler {
            path: path.to_owned(),
            source,
        })?;
        self.check_agreed(key, &bytes)?;
        decode_handler(&bytes).map_err(|source| BeginError::Handler {
            path: path.to_owned(),
            source,
        })
    }

    /// Whether `bytes` are what the user agreed to for the file `relative`. Without an agreement
    /// on record (a test, the command line) every file passes.
    fn check_agreed(&self, relative: &str, bytes: &[u8]) -> Result<(), BeginError> {
        let Some(agreed) = &self.settings.verified else {
            return Ok(());
        };
        if agreed.get(relative).map(String::as_str) == Some(sha256_hex(bytes).as_str()) {
            Ok(())
        } else {
            Err(BeginError::Changed {
                path: relative.to_owned(),
            })
        }
    }

    /// Every module file of the plugin folder (the files Node.js can import), as the editor reads
    /// and checks them now: the path, and the text.
    fn read_module_files(&self) -> Result<Vec<(PathBuf, String)>, BeginError> {
        let mut found = Vec::new();
        collect_module_files(&self.settings.dir, "", &mut found)?;
        found.sort();
        let mut total = 0_usize;
        let mut files = Vec::new();
        for (relative, path) in found {
            // At most what is left of the limit (and one byte more, to see it is over) is read.
            let left = MAX_MODULE_BYTES - total;
            let mut bytes = Vec::new();
            std::fs::File::open(&path)
                .and_then(|file| file.take(left as u64 + 1).read_to_end(&mut bytes))
                .map_err(|source| BeginError::Handler {
                    path: path.clone(),
                    source,
                })?;
            total += bytes.len();
            if total > MAX_MODULE_BYTES {
                return Err(BeginError::Bundle {
                    reason: format!("{} MiB を超えています", MAX_MODULE_BYTES >> 20),
                });
            }
            self.check_agreed(&relative, &bytes)?;
            let text = decode_handler(&bytes).map_err(|source| BeginError::Handler {
                path: path.clone(),
                source,
            })?;
            files.push((path, text));
        }
        Ok(files)
    }

    /// The jack of the call that runs.
    pub fn current_jack(&self) -> Option<&str> {
        self.current.as_ref().map(|c| c.jack.as_str())
    }

    /// Waits up to `wait` for the next thing that happens in the call. `None` when nothing did
    /// (or no call runs); ask again.
    pub fn poll(&mut self, wait_for: Duration) -> Option<Progress> {
        if std::mem::take(&mut self.interrupted) {
            return Some(Progress::Done(Outcome::Cancelled));
        }
        if self.settling.is_some() {
            return self.settle(wait_for);
        }
        let (id, deadline) = {
            let current = self.current.as_ref()?;
            (current.id, current.deadline)
        };
        let language = self.settings.language;
        let until = (Instant::now() + wait_for).min(deadline);
        // Messages that mean nothing are dropped, but only so many in a row: a plugin that sends
        // them without end must not keep the caller (the window) from its other work.
        let mut dropped = 0_usize;
        loop {
            // The limit holds even when the plugin keeps the queue full (a loop that prints).
            if Instant::now() >= deadline {
                return Some(self.finish(Outcome::TimedOut));
            }
            let left = until.saturating_duration_since(Instant::now());
            let received = self.process.as_ref()?.events.recv_timeout(left);
            match received {
                Ok(PluginEvent::Host(HostEvent::Output { stream, text })) => {
                    return Some(Progress::Output { stream, text });
                }
                // The marker of an earlier call, or one that the plugin wrote itself for another
                // call, is not this call's.
                Ok(PluginEvent::StderrDone(done)) => {
                    if let Some(current) = self.current.as_mut().filter(|c| c.id == done) {
                        current.marked = true;
                    }
                }
                Ok(PluginEvent::Host(HostEvent::Message(text))) => match classify(&text, id) {
                    Message::Request => return Some(Progress::Request(text)),
                    Message::Answer(outcome) => {
                        let current = self.current.as_ref()?;
                        let now = Instant::now();
                        let wait = if current.marked {
                            None
                        } else {
                            wait_after_answer(language, &current.jack, now)
                        };
                        let Some(wait) = wait else {
                            return Some(self.finish(outcome));
                        };
                        self.settling = Some(Settling {
                            outcome,
                            id,
                            wait,
                            cap: now + SETTLE_CAP,
                        });
                        return self.settle(wait_for);
                    }
                    Message::Other => {
                        dropped += 1;
                        if dropped >= MAX_DROPPED_IN_A_ROW {
                            return None;
                        }
                    }
                },
                Ok(PluginEvent::Host(HostEvent::Finished(finished))) => {
                    let text = crate::describe(&finished, "");
                    return Some(self.finish(Outcome::Died(text)));
                }
                Err(RecvTimeoutError::Timeout) => {
                    return (Instant::now() >= deadline).then(|| self.finish(Outcome::TimedOut));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Some(self.finish(Outcome::Died("the host has gone".into())));
                }
            }
        }
    }

    /// Waits for what the process wrote to standard error before its answer. Standard output and
    /// standard error are read by two threads, so a line that the handler wrote to standard error
    /// just before it answered can still be on its way when the answer is here. Every plugin
    /// language writes a marker line to standard error, after what the handler wrote and before
    /// the answer (`protocol::STDERR_DONE`); the call ends when the marker of the call has come,
    /// which puts every line of the handler before the end of the call, and no time is spent when
    /// the marker has come already. Each line that comes before is handed out as output of the
    /// call. A plugin that cannot write the marker (it crashed, its standard error is closed, or
    /// the language cannot, see [`wait_after_answer`]) cannot hold the call: it ends at
    /// [`SETTLE_CAP`] after the answer, and the bounded queue of events keeps the memory in check.
    fn settle(&mut self, wait_for: Duration) -> Option<Progress> {
        let until = Instant::now() + wait_for;
        loop {
            let settling = self.settling.as_ref()?;
            let now = Instant::now();
            let over = match settling.wait {
                Wait::Marker => false,
                Wait::Quiet(quiet_until) => now >= quiet_until,
            };
            if over || now >= settling.cap {
                return Some(self.end_settling());
            }
            let mut stop = settling.cap;
            if let Wait::Quiet(quiet_until) = settling.wait {
                stop = stop.min(quiet_until);
            }
            let left = until.min(stop).saturating_duration_since(now);
            let id = settling.id;
            let received = self.process.as_ref()?.events.recv_timeout(left);
            match received {
                Ok(PluginEvent::Host(HostEvent::Output { stream, text })) => {
                    if let Some(Settling {
                        wait: Wait::Quiet(quiet_until),
                        cap,
                        ..
                    }) = &mut self.settling
                    {
                        *quiet_until = (Instant::now() + SETTLE_QUIET).min(*cap);
                    }
                    return Some(Progress::Output { stream, text });
                }
                Ok(PluginEvent::StderrDone(done)) => {
                    if done == id {
                        return Some(self.end_settling());
                    }
                }
                // The call is answered: a message now is not part of it.
                Ok(PluginEvent::Host(HostEvent::Message(_))) => {}
                Ok(PluginEvent::Host(HostEvent::Finished(_)))
                | Err(RecvTimeoutError::Disconnected) => {
                    // Everything the process wrote has been read (`Finished` is the last event).
                    return Some(self.end_settling());
                }
                Err(RecvTimeoutError::Timeout) => {
                    if Instant::now() >= until {
                        return None;
                    }
                }
            }
        }
    }

    fn end_settling(&mut self) -> Progress {
        let outcome = self
            .settling
            .take()
            .map_or(Outcome::Cancelled, |settling| settling.outcome);
        self.finish(outcome)
    }

    /// Sends the answer to a request of the plugin.
    pub fn reply(&self, line: String) {
        if let Some(process) = &self.process {
            process.session.send(line);
        }
    }

    /// The user stopped the call: the process is killed (a handler that loops cannot be asked to
    /// stop). Not a failure.
    pub fn cancel(&mut self) -> Option<Outcome> {
        self.current.as_ref()?;
        Some(self.finish_progress(Outcome::Cancelled))
    }

    /// Ends the call with `outcome` and returns it as the last [`Progress`].
    fn finish(&mut self, outcome: Outcome) -> Progress {
        Progress::Done(self.finish_progress(outcome))
    }

    fn finish_progress(&mut self, outcome: Outcome) -> Outcome {
        self.current = None;
        self.settling = None;
        self.note(&outcome);
        let keep = self.settings.resident && outcome == Outcome::Ok;
        if !keep {
            self.close_process(outcome == Outcome::Ok);
        }
        outcome
    }

    /// Counts the outcome: a success starts the count over; a failure adds to it, and
    /// [`MAX_FAILURES`] in a row stop the plugin.
    pub fn note(&mut self, outcome: &Outcome) {
        if *outcome == Outcome::Ok {
            self.failures = 0;
        } else if outcome.is_failure() {
            self.failures += 1;
            if self.failures >= MAX_FAILURES {
                self.stopped = true;
            }
        }
    }

    /// Ends the process. After a call that went well it is asked to end and given a moment; in
    /// any other case it is killed.
    fn close_process(&mut self, graceful: bool) {
        let Some(process) = self.process.take() else {
            return;
        };
        if graceful {
            process.session.send(protocol::ascii_json(&json!({
                "jsonrpc": "2.0",
                "method": "plugin/shutdown",
            })));
            // The wait is on a thread of its own: the caller may be the window, which must not
            // stand still for a host that is slow to leave. The process is killed there if it has
            // not gone within the grace.
            std::thread::spawn(move || {
                let end = Instant::now() + SHUTDOWN_GRACE;
                while let Some(left) = end.checked_duration_since(Instant::now()) {
                    match process.events.recv_timeout(left) {
                        Ok(PluginEvent::Host(HostEvent::Finished(_)))
                        | Err(RecvTimeoutError::Disconnected) => return,
                        Ok(_) => {}
                        Err(RecvTimeoutError::Timeout) => break,
                    }
                }
                // Dropping the session kills a process that is still there.
                drop(process);
            });
            return;
        }
        // Dropping the session kills a process that is still there.
        drop(process);
    }

    /// Ends the process (the plugin was disabled). A process that is in a call is killed on the
    /// spot: it cannot read a request to end while its handler runs, and nobody should wait for it.
    /// An idle one is asked to end and left to do so; a thread kills it if it takes too long.
    /// This returns at once.
    pub fn shutdown(&mut self) {
        self.settling = None;
        let in_a_call = self.current.take().is_some();
        self.interrupted |= in_a_call;
        self.close_process(!in_a_call);
    }

    /// Kills the process on the spot, idle or not (the editor quits, or the host is dropped): no
    /// process is left behind, and nothing waits.
    pub fn shutdown_now(&mut self) {
        self.settling = None;
        self.interrupted |= self.current.take().is_some();
        self.close_process(false);
    }

    /// Runs a call to its end on this thread. `answer` gets each request of the plugin (a line)
    /// and returns the response line.
    pub fn run_blocking(
        &mut self,
        jack: &str,
        handler: &str,
        args: &Value,
        timeout: Duration,
        mut answer: impl FnMut(&str) -> Option<String>,
    ) -> Result<Report, BeginError> {
        self.begin(jack, handler, args, timeout)?;
        let mut output = Vec::new();
        loop {
            match self.poll(Duration::from_millis(50)) {
                None => {}
                Some(Progress::Output { stream, text }) => output.push((stream, text)),
                Some(Progress::Request(text)) => {
                    if let Some(line) = answer(&text) {
                        self.reply(line);
                    }
                }
                Some(Progress::Done(outcome)) => return Ok(Report { output, outcome }),
            }
        }
    }
}

impl Drop for PluginHost {
    fn drop(&mut self) {
        self.shutdown_now();
    }
}

/// What a call left behind.
#[derive(Debug)]
pub struct Report {
    pub output: Vec<(Stream, String)>,
    pub outcome: Outcome,
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

/// The file of a handler: a path under the folder of the plugin, after the links are followed
/// too. The files that the user agreed to are those of the folder (`consent::fingerprint`), so
/// nothing else may run.
fn handler_path(dir: &Path, handler: &str) -> Result<PathBuf, BeginError> {
    let outside = || BeginError::Outside {
        handler: handler.to_owned(),
    };
    // `plugin.def` files are written for Windows: the separator there is a backslash.
    let normalized = handler.replace('\\', "/");
    let relative = Path::new(&normalized);
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(outside());
    }
    let path = dir.join(relative);
    let unreadable = |source| BeginError::Handler {
        path: path.clone(),
        source,
    };
    let root = dir.canonicalize().map_err(unreadable)?;
    let real = path.canonicalize().map_err(unreadable)?;
    if !real.starts_with(&root) {
        return Err(outside());
    }
    Ok(path)
}

/// The text of a handler: UTF-8, without the BOM.
fn decode_handler(bytes: &[u8]) -> io::Result<String> {
    let text =
        std::str::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(text).to_owned())
}

/// The most bytes of module files that go to a Node.js plugin process.
const MAX_MODULE_BYTES: usize = 16 * 1024 * 1024;

/// The most module files that go to a Node.js plugin process.
const MAX_MODULE_FILES: usize = 5000;

/// The extensions of the files that Node.js can import (the host sends the text of these).
const MODULE_EXTENSIONS: &[&str] = &["mjs", "cjs", "js", "ts", "mts", "cts", "json"];

/// An ES module (`.mjs`) or TypeScript (`.ts`) handler: loaded once and called, not run again.
fn is_module_handler(handler: &str) -> bool {
    let ext = handler.rsplit_once('.').map(|(_, e)| e);
    ext.is_some_and(|e| e.eq_ignore_ascii_case("mjs") || e == "ts")
}

/// Whether the file system of this OS ignores case in names (Windows, and macOS by default).
const CASE_INSENSITIVE_FS: bool = cfg!(any(windows, target_os = "macos"));

/// The name of `relative` in the agreement. The exact spelling first. Where the file system ignores
/// case (`case_insensitive`), a name that differs only in case is the same file: the one agreed
/// name that equals it ignoring case, if there is exactly one (two that differ only in case are
/// refused). `None` when there is none.
fn agreed_key(
    agreed: &BTreeMap<String, String>,
    relative: &str,
    case_insensitive: bool,
) -> Option<String> {
    if agreed.contains_key(relative) {
        return Some(relative.to_owned());
    }
    if !case_insensitive {
        return None;
    }
    let lower = relative.to_lowercase();
    let mut same = agreed.keys().filter(|key| key.to_lowercase() == lower);
    let found = same.next()?;
    same.next().is_none().then(|| found.clone())
}

/// The path of a handler from the plugin folder, with `/` (the spelling of `plugins.ini`):
/// `\` becomes `/`, and empty and `.` components are dropped (`lib/./a.mjs`, `lib//a.mjs`,
/// `.\lib\a.mjs`). `..` stays: `handler_path` refuses it.
fn relative_name(handler: &str) -> String {
    handler
        .replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// The module files under `dir`: (the path from the plugin folder with `/`, the path on disk).
/// Links and files of other kinds are left out (a plugin that has a link cannot be enabled).
fn collect_module_files(
    dir: &Path,
    prefix: &str,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<(), BeginError> {
    let unreadable = |source| BeginError::Handler {
        path: dir.to_owned(),
        source,
    };
    for entry in std::fs::read_dir(dir).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let relative = format!("{prefix}{name}");
        let kind = entry.file_type().map_err(unreadable)?;
        if kind.is_dir() {
            collect_module_files(&path, &format!("{relative}/"), out)?;
        } else if kind.is_file()
            && path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| MODULE_EXTENSIONS.iter().any(|m| m.eq_ignore_ascii_case(e)))
        {
            if out.len() >= MAX_MODULE_FILES {
                return Err(BeginError::Bundle {
                    reason: "ファイルが多すぎます".into(),
                });
            }
            out.push((relative, path));
        }
    }
    Ok(())
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

enum Message {
    /// A request or notification of the plugin.
    Request,
    /// The answer to the call that runs.
    Answer(Outcome),
    /// Anything else (an answer to an older call).
    Other,
}

fn classify(text: &str, id: i64) -> Message {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Message::Other;
    };
    if value.get("method").is_some() {
        return Message::Request;
    }
    if value["id"] != json!(id) {
        return Message::Other;
    }
    match value.get("error") {
        Some(error) => Message::Answer(Outcome::Failed(
            error["message"]
                .as_str()
                .unwrap_or("the handler failed")
                .to_owned(),
        )),
        None => Message::Answer(Outcome::Ok),
    }
}

/// What the editor answers to the calls of the `Plugin` object (`Plugin.GetOption` and the
/// like): the definition file and the options of the plugin.
pub trait PluginApi {
    fn call(&mut self, name: &str, params: &[Value]) -> Result<Value, RpcError>;
}

/// The prefixes of the calls that [`PluginApi`] answers: the `Plugin` object, the `Complement`
/// object of the completion jacks, and the `Outline` object of the jack `Outline`.
pub const PLUGIN_PREFIXES: &[&str] = &["Plugin.", "Complement.", "Outline."];

/// Answers a request of a plugin: a call of the `Plugin` or `Outline` object goes to `api`, any other call
/// (`Editor.*`) to `editor`. Returns the response line, `None` for a notification.
pub fn respond(
    text: &str,
    api: &mut dyn PluginApi,
    editor: impl FnOnce(&str, &[Value]) -> Result<Value, RpcError>,
) -> Option<String> {
    api::respond(text, |method, params| {
        if PLUGIN_PREFIXES.iter().any(|p| method.starts_with(p)) {
            api.call(method, params)
        } else {
            editor(method, params)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agreed(names: &[&str]) -> BTreeMap<String, String> {
        names
            .iter()
            .map(|name| ((*name).to_owned(), sha256_hex(name.as_bytes())))
            .collect()
    }

    #[test]
    fn the_agreed_name_is_the_exact_one_and_case_differences_count_only_where_the_disk_ignores_case()
     {
        let files = agreed(&["command.cjs", "lib/Word.mjs"]);
        // The exact spelling, with and without the flag.
        for flag in [false, true] {
            assert_eq!(
                agreed_key(&files, "command.cjs", flag).as_deref(),
                Some("command.cjs")
            );
        }
        // A different case: only where the file system ignores case; the key is the agreed one.
        assert_eq!(
            agreed_key(&files, "Command.CJS", true).as_deref(),
            Some("command.cjs")
        );
        assert_eq!(
            agreed_key(&files, "LIB/word.mjs", true).as_deref(),
            Some("lib/Word.mjs")
        );
        assert_eq!(agreed_key(&files, "Command.cjs", false), None);
        assert_eq!(agreed_key(&files, "other.cjs", true), None);
        // Two agreed names that differ only in case: not decidable, refused; the exact one is fine.
        let two = agreed(&["a.cjs", "A.cjs"]);
        assert_eq!(agreed_key(&two, "A.CJS", true), None);
        assert_eq!(agreed_key(&two, "a.cjs", true).as_deref(), Some("a.cjs"));
    }

    #[test]
    fn a_handler_name_loses_redundant_components_before_the_lookup() {
        for spelled in [
            "lib/main.mjs",
            "lib/./main.mjs",
            "lib//main.mjs",
            "./lib/main.mjs",
            "lib\\main.mjs",
            ".\\lib\\main.mjs",
            "lib\\.\\main.mjs",
        ] {
            assert_eq!(relative_name(spelled), "lib/main.mjs", "{spelled}");
        }
        let files = agreed(&["lib/main.cjs"]);
        for spelled in ["lib/./main.cjs", "lib//main.cjs", ".\\lib\\main.cjs"] {
            assert_eq!(
                agreed_key(&files, &relative_name(spelled), false).as_deref(),
                Some("lib/main.cjs"),
                "{spelled}"
            );
        }
        assert_eq!(relative_name("a/../b.cjs"), "a/../b.cjs");
    }

    #[test]
    fn a_file_is_accepted_only_with_the_hash_of_its_agreed_name() {
        let dir = std::path::Path::new("/nowhere");
        let host = PluginHost::new(
            HostSettings::new("p", dir, Language::JavaScript, false)
                .with_verified(agreed(&["command.cjs"])),
        );
        assert!(host.check_agreed("command.cjs", b"command.cjs").is_ok());
        assert!(matches!(
            host.check_agreed("command.cjs", b"changed"),
            Err(BeginError::Changed { .. })
        ));
        // `check_agreed` takes the agreed spelling; another spelling is not looked up here.
        assert!(host.check_agreed("Command.cjs", b"command.cjs").is_err());
    }

    #[test]
    fn every_jack_has_a_limit_and_an_unknown_one_has_the_default() {
        assert_eq!(limit_of("Command"), Duration::from_secs(30));
        assert_eq!(limit_of("SmartIndent"), Duration::from_millis(500));
        assert_eq!(limit_of("Complement"), Duration::from_secs(1));
        assert_eq!(limit_of("Nothing"), DEFAULT_LIMIT);
        let names: Vec<_> = LIMITS.iter().map(|(n, _)| *n).collect();
        assert_eq!(names.len(), 11, "the eleven jacks of Sakura");
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 11, "each jack once");
    }

    fn host() -> PluginHost {
        PluginHost::new(HostSettings::new(
            "p",
            Path::new("/nowhere"),
            Language::Bundled(session::Engine::QuickJs),
            false,
        ))
    }

    #[test]
    fn three_failures_in_a_row_stop_the_plugin_and_a_success_starts_over() {
        let mut host = host();
        host.note(&Outcome::Failed("x".into()));
        host.note(&Outcome::TimedOut);
        assert_eq!(host.failures(), 2);
        assert!(!host.is_stopped());
        host.note(&Outcome::Ok);
        assert_eq!(host.failures(), 0);
        host.note(&Outcome::Died("gone".into()));
        host.note(&Outcome::Failed("x".into()));
        // The user stopping a call is not a failure.
        host.note(&Outcome::Cancelled);
        assert_eq!(host.failures(), 2);
        host.note(&Outcome::TimedOut);
        assert!(host.is_stopped());
        assert!(matches!(
            host.begin("Command", "a.js", &Value::Null, Duration::from_secs(1)),
            Err(BeginError::Stopped)
        ));
        host.reset();
        assert!(!host.is_stopped() && host.failures() == 0);
    }

    #[test]
    fn a_handler_outside_the_folder_is_refused_before_anything_starts() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = dir.path().join("plugin");
        std::fs::create_dir_all(plugin.join("sub")).unwrap();
        std::fs::write(plugin.join("sub").join("ok.js"), "1;").unwrap();
        std::fs::write(dir.path().join("outside.js"), "1;").unwrap();
        let mut host = PluginHost::new(HostSettings::new(
            "p",
            &plugin,
            Language::Bundled(session::Engine::QuickJs),
            false,
        ));
        let absolute = dir.path().join("outside.js").display().to_string();
        for handler in [
            "../outside.js",
            "sub/../../outside.js",
            absolute.as_str(),
            "./../outside.js",
        ] {
            let error = host
                .begin("Command", handler, &Value::Null, Duration::from_secs(1))
                .unwrap_err();
            assert!(
                matches!(error, BeginError::Outside { .. }),
                "{handler}: {error}"
            );
        }
        assert!(!host.is_running());
        // A file in a folder of the plugin is fine (the process is not started here: the path is
        // checked by `handler_path` alone).
        assert!(handler_path(&plugin, "sub/ok.js").is_ok());
        // The Windows spelling of a path in a folder works, and of one that leaves it does not.
        assert!(handler_path(&plugin, "sub\\ok.js").is_ok());
        assert!(matches!(
            handler_path(&plugin, "sub\\..\\..\\outside.js"),
            Err(BeginError::Outside { .. })
        ));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.path().join("outside.js"), plugin.join("link.js"))
                .unwrap();
            assert!(matches!(
                handler_path(&plugin, "link.js"),
                Err(BeginError::Outside { .. })
            ));
        }
    }

    #[test]
    fn a_handler_that_cannot_be_read_does_not_start_a_process() {
        let mut host = host();
        let error = host
            .begin(
                "Command",
                "missing.js",
                &Value::Null,
                Duration::from_secs(1),
            )
            .unwrap_err();
        assert!(matches!(error, BeginError::Handler { .. }), "{error}");
        assert!(
            !host.is_running(),
            "nothing starts for a file that is not there"
        );
    }

    #[test]
    fn a_call_of_the_plugin_object_goes_to_the_api_and_the_rest_to_the_editor() {
        struct Api;
        impl PluginApi for Api {
            fn call(&mut self, name: &str, params: &[Value]) -> Result<Value, RpcError> {
                Ok(json!([name, params]))
            }
        }
        let text = r#"{"jsonrpc":"2.0","id":3,"method":"Plugin.GetOption","params":["S","K"]}"#;
        let line = respond(text, &mut Api, |_, _| panic!("not the editor's")).unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["result"], json!(["Plugin.GetOption", ["S", "K"]]));
        let text = r#"{"jsonrpc":"2.0","id":5,"method":"Complement.AddList","params":["w"]}"#;
        let line = respond(text, &mut Api, |_, _| panic!("not the editor's")).unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["result"], json!(["Complement.AddList", ["w"]]));
        let text = r#"{"jsonrpc":"2.0","id":4,"method":"InsText","params":["a"]}"#;
        let line = respond(text, &mut Api, |m, p| Ok(json!([m, p]))).unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["result"], json!(["InsText", ["a"]]));
        // A notification gets no answer.
        let text = r#"{"jsonrpc":"2.0","method":"Plugin.GetOption","params":[]}"#;
        assert_eq!(respond(text, &mut Api, |_, _| Ok(Value::Null)), None);
    }

    #[test]
    fn only_applescript_waits_for_a_quiet_time_and_not_on_the_jacks_of_a_key() {
        let now = Instant::now();
        for language in [Language::JavaScript, Language::Python, Language::EmacsLisp] {
            let wait = wait_after_answer(language, "SmartIndent", now);
            assert!(matches!(wait, Some(Wait::Marker)), "{language:?}");
        }
        for jack in ["SmartIndent", "Complement", "ComplementGlobal"] {
            assert!(wait_after_answer(Language::AppleScript, jack, now).is_none());
        }
        assert!(matches!(
            wait_after_answer(Language::AppleScript, "Command", now),
            Some(Wait::Quiet(_))
        ));
    }

    #[test]
    fn a_marker_line_names_its_call_and_other_lines_are_not_markers() {
        let line = protocol::stderr_done_line(&json!(42));
        assert_eq!(protocol::parse_stderr_done(&line), Some(42));
        assert_eq!(protocol::parse_stderr_done("matsu-stderr-done 42"), None);
        assert_eq!(
            protocol::parse_stderr_done("\u{1f}matsu-stderr-done x"),
            None
        );
    }

    #[test]
    fn the_messages_of_a_plugin_are_told_apart() {
        assert!(matches!(
            classify(
                r#"{"jsonrpc":"2.0","id":1,"method":"InsText","params":[]}"#,
                5
            ),
            Message::Request
        ));
        assert!(matches!(
            classify(r#"{"jsonrpc":"2.0","id":5,"result":{"ok":true}}"#, 5),
            Message::Answer(Outcome::Ok)
        ));
        assert!(matches!(
            classify(r#"{"jsonrpc":"2.0","id":4,"result":{}}"#, 5),
            Message::Other
        ));
        match classify(
            r#"{"jsonrpc":"2.0","id":5,"error":{"code":-32000,"message":"boom"}}"#,
            5,
        ) {
            Message::Answer(Outcome::Failed(text)) => assert_eq!(text, "boom"),
            _ => panic!("an error answer"),
        }
        assert!(matches!(classify("garbage", 5), Message::Other));
    }
}
