//! A resident host for one plugin (ADR-0026, chapter 2).
//!
//! A macro is a process that runs once. A plugin is called again and again, by the events it
//! declared, so its process can stay: the editor sends `plugin/invoke` (the jack, the text of
//! the handler, the arguments), the plugin runs the handler, asks the editor for what it needs
//! while it runs (`Editor.*`, `Plugin.*`), and answers the invoke. The messages are those of the
//! macro protocol (`docs/poc/json-rpc.md`); the host is `matsu-script --engine quickjs` and the
//! first language is `.js`.
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

use std::ffi::OsString;
use std::io;
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

/// How many events the host keeps for the window.
const EVENT_BACKLOG: usize = 4096;

/// How long a process that was told to end may take before it is killed.
const SHUTDOWN_GRACE: Duration = Duration::from_millis(1500);

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
        }
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
    /// The process did not start.
    Start(StartError),
}

impl std::fmt::Display for BeginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "失敗が続いたので止めています"),
            Self::Busy => write!(f, "前の呼び出しがまだ終わっていません"),
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
    events: Receiver<HostEvent>,
}

struct Current {
    id: i64,
    jack: String,
    deadline: Instant,
}

/// The host of one plugin in one language.
pub struct PluginHost {
    settings: HostSettings,
    process: Option<Process>,
    failures: u32,
    stopped: bool,
    next_id: i64,
    current: Option<Current>,
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
        let sink: Sink = Arc::new(move |event| {
            let _ = tx.send(event);
        });
        let session = session::start(config, sink)?;
        Ok(Process { session, events })
    }

    /// Starts a call: the process is started now if it is not running (the first call of a
    /// plugin is the first time its process is made), and the invoke is sent. `handler` is a
    /// file in the folder of the plugin.
    pub fn begin(
        &mut self,
        jack: &str,
        handler: &str,
        args: Value,
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
        let path = self.settings.dir.join(handler);
        let source = read_handler(&path).map_err(|source| BeginError::Handler {
            path: path.clone(),
            source,
        })?;
        if self.process.is_none() {
            self.process = Some(self.start_process().map_err(BeginError::Start)?);
        }
        let id = self.next_id;
        self.next_id += 1;
        let line = protocol::ascii_json(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "plugin/invoke",
            "params": {
                "jack": jack,
                "name": path.display().to_string(),
                "source": source,
                "args": args,
            },
        }));
        if let Some(process) = &self.process {
            process.session.send(line);
        }
        self.current = Some(Current {
            id,
            jack: jack.to_owned(),
            deadline: Instant::now() + timeout,
        });
        Ok(())
    }

    /// The jack of the call that runs.
    pub fn current_jack(&self) -> Option<&str> {
        self.current.as_ref().map(|c| c.jack.as_str())
    }

    /// Waits up to `wait` for the next thing that happens in the call. `None` when nothing did
    /// (or no call runs); ask again.
    pub fn poll(&mut self, wait: Duration) -> Option<Progress> {
        if std::mem::take(&mut self.interrupted) {
            return Some(Progress::Done(Outcome::Cancelled));
        }
        let (id, deadline) = {
            let current = self.current.as_ref()?;
            (current.id, current.deadline)
        };
        let until = (Instant::now() + wait).min(deadline);
        loop {
            // The limit holds even when the plugin keeps the queue full (a loop that prints).
            if Instant::now() >= deadline {
                return Some(self.finish(Outcome::TimedOut));
            }
            let left = until.saturating_duration_since(Instant::now());
            let received = self.process.as_ref()?.events.recv_timeout(left);
            match received {
                Ok(HostEvent::Output { stream, text }) => {
                    return Some(Progress::Output { stream, text });
                }
                Ok(HostEvent::Message(text)) => match classify(&text, id) {
                    Message::Request => return Some(Progress::Request(text)),
                    Message::Answer(outcome) => return Some(self.finish(outcome)),
                    Message::Other => {}
                },
                Ok(HostEvent::Finished(finished)) => {
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
            let end = Instant::now() + SHUTDOWN_GRACE;
            while let Some(left) = end.checked_duration_since(Instant::now()) {
                match process.events.recv_timeout(left) {
                    Ok(HostEvent::Finished(_)) | Err(RecvTimeoutError::Disconnected) => return,
                    Ok(_) => {}
                    Err(RecvTimeoutError::Timeout) => break,
                }
            }
        }
        // Dropping the session kills a process that is still there.
        drop(process);
    }

    /// Ends the process at once (the plugin was disabled, or the editor closes).
    pub fn shutdown(&mut self) {
        self.interrupted |= self.current.take().is_some();
        self.close_process(true);
    }

    /// Runs a call to its end on this thread. `answer` gets each request of the plugin (a line)
    /// and returns the response line.
    pub fn run_blocking(
        &mut self,
        jack: &str,
        handler: &str,
        args: Value,
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
        self.shutdown();
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

/// The text of a handler: UTF-8, without the BOM.
fn read_handler(path: &Path) -> io::Result<String> {
    let bytes = std::fs::read(path)?;
    let text =
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned())
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

/// The prefix of the calls that [`PluginApi`] answers.
pub const PLUGIN_PREFIX: &str = "Plugin.";

/// Answers a request of a plugin: a call of the `Plugin` object goes to `api`, any other call
/// (`Editor.*`) to `editor`. Returns the response line, `None` for a notification.
pub fn respond(
    text: &str,
    api: &mut dyn PluginApi,
    editor: impl FnOnce(&str, &[Value]) -> Result<Value, RpcError>,
) -> Option<String> {
    api::respond(text, |method, params| {
        if method.starts_with(PLUGIN_PREFIX) {
            api.call(method, params)
        } else {
            editor(method, params)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
            host.begin("Command", "a.js", Value::Null, Duration::from_secs(1)),
            Err(BeginError::Stopped)
        ));
        host.reset();
        assert!(!host.is_stopped() && host.failures() == 0);
    }

    #[test]
    fn a_handler_that_cannot_be_read_does_not_start_a_process() {
        let mut host = host();
        let error = host
            .begin("Command", "missing.js", Value::Null, Duration::from_secs(1))
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
        let text = r#"{"jsonrpc":"2.0","id":4,"method":"InsText","params":["a"]}"#;
        let line = respond(text, &mut Api, |m, p| Ok(json!([m, p]))).unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["result"], json!(["InsText", ["a"]]));
        // A notification gets no answer.
        let text = r#"{"jsonrpc":"2.0","method":"Plugin.GetOption","params":[]}"#;
        assert_eq!(respond(text, &mut Api, |_, _| Ok(Value::Null)), None);
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
