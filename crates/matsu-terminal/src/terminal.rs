//! The terminal: the shell in a PTY, the state behind it, and the events to the UI.

use std::borrow::Cow;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvError, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg, State};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::Line;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{ClipboardType, Config, Osc52, Term};
use alacritty_terminal::tty::{self, Pty};
use alacritty_terminal::vte::ansi::Rgb as VteRgb;

use crate::keys::{Key, Modifiers, encode_key};
use crate::paste::{PasteEncoding, encode_paste};
#[cfg(test)]
use crate::screen::Color;
use crate::screen::{ClipboardKind, Cursor, CursorShape, Modes, Rgb, Row, ScreenInfo};
use crate::shell::{self, ShellCommand};
use crate::title::clean_title;

/// The scrollback length when none is given (ADR-0028 point 11).
pub const DEFAULT_SCROLLBACK_LINES: usize = 5000;

/// The size of the terminal, in cells, and the size of a cell in pixels (told to programs that ask).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalSize {
    pub columns: u16,
    pub rows: u16,
    /// Width of a cell in pixels, 0 when unknown.
    pub cell_width: u16,
    /// Height of a cell in pixels, 0 when unknown.
    pub cell_height: u16,
}

impl TerminalSize {
    /// A size in cells; the pixel size of a cell is unknown (0).
    pub const fn new(columns: u16, rows: u16) -> Self {
        Self {
            columns,
            rows,
            cell_width: 0,
            cell_height: 0,
        }
    }

    /// At least 2 columns and 1 row, which the terminal state needs.
    fn clamped(self) -> Self {
        Self {
            columns: self.columns.max(2),
            rows: self.rows.max(1),
            ..self
        }
    }

    fn window_size(self) -> WindowSize {
        WindowSize {
            num_lines: self.rows,
            num_cols: self.columns,
            cell_width: self.cell_width,
            cell_height: self.cell_height,
        }
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        usize::from(self.rows)
    }

    fn screen_lines(&self) -> usize {
        usize::from(self.rows)
    }

    fn columns(&self) -> usize {
        usize::from(self.columns)
    }
}

/// How to start a terminal.
#[derive(Clone, Debug)]
pub struct Options {
    pub size: TerminalSize,
    /// The directory the shell starts in; the parent's directory when `None`. A path that is not a directory
    /// is an error, so that the caller chooses where to go instead (ADR-0028: the document's folder, else the
    /// home folder): Windows would refuse it, and Unix would start the shell elsewhere without a word. (On
    /// Unix, a directory the shell may not enter, or one removed in between, still starts it in the parent's
    /// directory: `alacritty_terminal` 0.26.0 ignores a failed `chdir`.)
    pub working_directory: Option<PathBuf>,
    /// The shell; `None` is the default shell (see [`shell`]).
    pub shell: Option<ShellCommand>,
    /// Lines kept above the screen (the scrollback).
    pub scrollback_lines: usize,
    /// The default foreground, told to programs that ask for it (OSC 10 and the like). The UI sets its
    /// theme's color.
    pub foreground: Rgb,
    /// The default background, told to programs that ask for it (OSC 11 and the like).
    pub background: Rgb,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            size: TerminalSize::default(),
            working_directory: None,
            shell: None,
            scrollback_lines: DEFAULT_SCROLLBACK_LINES,
            foreground: Rgb {
                r: 229,
                g: 229,
                b: 229,
            },
            background: Rgb { r: 0, g: 0, b: 0 },
        }
    }
}

/// What the terminal tells the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalEvent {
    /// The screen changed. Several changes are reported as one until the UI receives this event.
    Redraw,
    /// The program set the title; already cleaned by [`clean_title`].
    Title(String),
    /// The program reset the title: use the tab's own title again.
    ResetTitle,
    /// The bell rang.
    Bell,
    /// The program asked to store text in the clipboard (OSC 52 write). Reads are refused.
    ClipboardStore { kind: ClipboardKind, text: String },
    /// The shell (the child process) ended. `code` is its exit code, `None` when it is not known (on Unix
    /// also when a signal ended it). The screen stays readable.
    ChildExit { code: Option<i32> },
}

/// The receiving end of the terminal's events.
pub struct EventReceiver {
    rx: Receiver<TerminalEvent>,
    /// Shared with the listener. The receiver must not own the `Sender` (through `Shared`): the channel
    /// disconnects when the terminal is gone, so `recv` returns an error instead of waiting for ever.
    redraw_pending: Arc<AtomicBool>,
}

impl EventReceiver {
    fn seen(&self, event: TerminalEvent) -> TerminalEvent {
        if event == TerminalEvent::Redraw {
            // Cleared before the UI reads the screen, so a change after the read sends a new event.
            self.redraw_pending.store(false, Ordering::Release);
        }
        event
    }

    /// The next event if there is one.
    pub fn try_recv(&self) -> Result<TerminalEvent, TryRecvError> {
        self.rx.try_recv().map(|event| self.seen(event))
    }

    /// Waits for the next event; an error when the terminal is gone.
    pub fn recv(&self) -> Result<TerminalEvent, RecvError> {
        self.rx.recv().map(|event| self.seen(event))
    }

    /// Waits for the next event for at most `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<TerminalEvent, RecvTimeoutError> {
        self.rx.recv_timeout(timeout).map(|event| self.seen(event))
    }
}

/// State shared by the UI side, the reader thread and the listener.
struct Shared {
    events: Sender<TerminalEvent>,
    redraw_pending: Arc<AtomicBool>,
    /// The way to the PTY, set once the event loop exists.
    writer: OnceLock<EventLoopSender>,
    size: Mutex<TerminalSize>,
    exit_status: Mutex<Option<ExitStatus>>,
    foreground: Rgb,
    background: Rgb,
    /// Everything written back to the PTY on behalf of the terminal (tests only).
    #[cfg(test)]
    written: Mutex<Vec<u8>>,
}

impl Shared {
    fn emit(&self, event: TerminalEvent) {
        // A closed receiver means the UI has dropped the terminal's events: nothing to tell.
        let _ = self.events.send(event);
    }

    /// Reports a redraw unless one is already waiting for the UI.
    fn request_redraw(&self) {
        if !self.redraw_pending.swap(true, Ordering::AcqRel) {
            self.emit(TerminalEvent::Redraw);
        }
    }

    fn write(&self, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }
        #[cfg(test)]
        self.written
            .lock()
            .expect("written")
            .extend_from_slice(&bytes);
        if let Some(writer) = self.writer.get() {
            let _ = writer.send(Msg::Input(Cow::Owned(bytes)));
        }
    }
}

/// Receives the events of `alacritty_terminal` (on the reader thread, with the terminal locked) and turns
/// them into [`TerminalEvent`]s and answers to the program.
#[derive(Clone)]
struct Listener {
    shared: Arc<Shared>,
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        let shared = &self.shared;
        match event {
            Event::Wakeup => shared.request_redraw(),
            Event::Title(title) => shared.emit(TerminalEvent::Title(clean_title(&title))),
            Event::ResetTitle => shared.emit(TerminalEvent::ResetTitle),
            Event::Bell => shared.emit(TerminalEvent::Bell),
            Event::ClipboardStore(kind, text) => shared.emit(TerminalEvent::ClipboardStore {
                kind: match kind {
                    ClipboardType::Clipboard => ClipboardKind::Clipboard,
                    ClipboardType::Selection => ClipboardKind::Selection,
                },
                text,
            }),
            // OSC 52 reads are refused (ADR-0028 point 7). The terminal does not raise this event with
            // `Osc52::OnlyCopy`; ignoring it here as well keeps a configuration slip from answering.
            Event::ClipboardLoad(..) => {}
            Event::ColorRequest(index, format) => {
                let rgb = palette_rgb(index, shared.foreground, shared.background);
                shared.write(
                    format(VteRgb {
                        r: rgb.r,
                        g: rgb.g,
                        b: rgb.b,
                    })
                    .into_bytes(),
                );
            }
            Event::PtyWrite(text) => shared.write(text.into_bytes()),
            Event::TextAreaSizeRequest(format) => {
                let size = *shared.size.lock().expect("size");
                shared.write(format(size.window_size()).into_bytes());
            }
            Event::ChildExit(status) => {
                *shared.exit_status.lock().expect("exit status") = Some(status);
            }
            Event::Exit => {
                let code = shared
                    .exit_status
                    .lock()
                    .expect("exit status")
                    .and_then(|status| status.code());
                shared.emit(TerminalEvent::ChildExit { code });
            }
            Event::MouseCursorDirty | Event::CursorBlinkingChange => {}
        }
    }
}

/// The color a program is told for a palette index (OSC 4, 10, 11, 12 queries): xterm's default palette,
/// and the options' foreground and background.
fn palette_rgb(index: usize, foreground: Rgb, background: Rgb) -> Rgb {
    const ANSI: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];
    match index {
        0..=15 => {
            let (r, g, b) = ANSI[index];
            Rgb { r, g, b }
        }
        16..=231 => {
            let n = index - 16;
            Rgb {
                r: CUBE[n / 36],
                g: CUBE[n / 6 % 6],
                b: CUBE[n % 6],
            }
        }
        232..=255 => {
            let level = u8::try_from(8 + 10 * (index - 232)).unwrap_or(238);
            Rgb {
                r: level,
                g: level,
                b: level,
            }
        }
        257 => background,
        // 256 is the foreground; the cursor and anything else follow it.
        _ => foreground,
    }
}

/// The settings of the terminal state. OSC 52 is `OnlyCopy`: writes are allowed, reads are refused. The
/// Kitty keyboard protocol is off (not encoded yet).
fn term_config(scrollback_lines: usize) -> Config {
    Config {
        scrolling_history: scrollback_lines,
        kitty_keyboard: false,
        osc52: Osc52::OnlyCopy,
        ..Config::default()
    }
}

/// A copy of the screen, taken under one short lock.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub info: ScreenInfo,
    /// The lines of the view: with `info.display_offset` 0 the live screen, otherwise scrolled back.
    pub rows: Vec<Row>,
    /// Colors a program set with OSC 4, 10, 11 and 12, as `(index, color)`: 0 to 255 the palette, 256 the
    /// foreground, 257 the background, 258 the cursor. The UI draws these instead of its theme's colors for
    /// the same [`crate::Color::Indexed`] and [`crate::Color::Default`] cells.
    ///
    /// Queries (OSC 4, 10, 11 and 12 with `?`) are answered from the options' colors and the xterm
    /// palette, not from these: the answer is written while the reader thread holds the terminal's lock.
    pub color_overrides: Vec<(usize, Rgb)>,
}

/// A terminal with a shell running in it.
///
/// Dropping it (or [`Terminal::shutdown`]) stops the reader thread and closes the PTY on another thread, so
/// the UI thread never waits for the child or for ConPTY. Before the process ends, [`wait_for_closing`] waits
/// for those threads.
pub struct Terminal {
    term: Arc<FairMutex<Term<Listener>>>,
    sender: EventLoopSender,
    shared: Arc<Shared>,
    reader: Option<JoinHandle<(EventLoop<Pty, Listener>, State)>>,
}

impl Terminal {
    /// Starts the shell in a new PTY.
    ///
    /// # Errors
    ///
    /// When the working directory is not a directory (the error of reading it, or `NotADirectory`), and when
    /// the PTY or the shell cannot be started.
    pub fn spawn(options: Options) -> io::Result<(Self, EventReceiver)> {
        if let Some(dir) = &options.working_directory
            && !std::fs::metadata(dir)?.is_dir()
        {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!("not a directory: {}", dir.display()),
            ));
        }
        let size = options.size.clamped();
        let (events, receiver) = mpsc::channel();
        let shared = Arc::new(Shared {
            events,
            redraw_pending: Arc::new(AtomicBool::new(false)),
            writer: OnceLock::new(),
            size: Mutex::new(size),
            exit_status: Mutex::new(None),
            foreground: options.foreground,
            background: options.background,
            #[cfg(test)]
            written: Mutex::new(Vec::new()),
        });
        let listener = Listener {
            shared: Arc::clone(&shared),
        };

        let term = Arc::new(FairMutex::new(Term::new(
            term_config(options.scrollback_lines),
            &size,
            listener.clone(),
        )));

        let shell = options.shell.or_else(shell::default_shell);
        let pty_options = tty::Options {
            shell: shell.map(|s| tty::Shell::new(shell::program_for_launch(&s.program), s.args)),
            working_directory: options.working_directory,
            // Read what the child wrote before it ended, so a short command's output is not lost. (One read of
            // at most about 64 KiB: `alacritty_terminal` 0.26.0 stops draining there, so the tail of a huge
            // burst right at the exit can still be lost.)
            drain_on_exit: true,
            env: shell::extra_env(),
            // Escape arguments by the C runtime's rules: an argument with spaces stays one argument.
            // (Not compiled on Linux and macOS: Windows only.)
            #[cfg(windows)]
            escape_args: true,
        };
        let pty = tty::new(&pty_options, size.window_size(), 0)?;

        let event_loop = EventLoop::new(Arc::clone(&term), listener, pty, true, false)?;
        let sender = event_loop.channel();
        let _ = shared.writer.set(sender.clone());
        let reader = event_loop.spawn();

        let terminal = Self {
            term,
            sender,
            shared: Arc::clone(&shared),
            reader: Some(reader),
        };
        let redraw_pending = Arc::clone(&shared.redraw_pending);
        Ok((
            terminal,
            EventReceiver {
                rx: receiver,
                redraw_pending,
            },
        ))
    }

    /// Whether the reader thread is still running: false after [`Terminal::shutdown`] and once the shell
    /// has ended (the screen stays readable, but nothing can be written or resized any more).
    pub fn is_running(&self) -> bool {
        self.reader
            .as_ref()
            .is_some_and(|reader| !reader.is_finished())
    }

    /// Writes bytes to the program (keys from [`encode_key`], pastes from [`encode_paste`]).
    ///
    /// An error means the terminal is not running (see [`Terminal::is_running`]).
    pub fn write(&self, bytes: &[u8]) -> io::Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        if !self.is_running() {
            return Err(io::Error::from(io::ErrorKind::BrokenPipe));
        }
        self.sender
            .send(Msg::Input(Cow::Owned(bytes.to_vec())))
            .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))
    }

    /// Encodes a key with the terminal's current modes ([`encode_key`]).
    pub fn encode_key(&self, key: Key, mods: Modifiers) -> Option<Vec<u8>> {
        encode_key(key, mods, &self.modes())
    }

    /// Encodes pasted text with the terminal's current modes ([`encode_paste`]); write the bytes with
    /// [`Terminal::write`] once the user has agreed, if `needs_confirmation` is set.
    pub fn encode_paste(&self, text: &str) -> PasteEncoding {
        encode_paste(text, &self.modes())
    }

    /// Changes the size of the screen and of the PTY (the program gets SIGWINCH or the ConPTY resize). The
    /// screen is resized even when the terminal is not running; the error then tells the PTY was not.
    pub fn resize(&self, size: TerminalSize) -> io::Result<()> {
        let size = size.clamped();
        *self.shared.size.lock().expect("size") = size;
        self.with_term(|term| term.resize(size));
        self.shared.request_redraw();
        if !self.is_running() {
            return Err(io::Error::from(io::ErrorKind::BrokenPipe));
        }
        self.sender
            .send(Msg::Resize(size.window_size()))
            .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))
    }

    /// The terminal modes.
    pub fn modes(&self) -> Modes {
        self.with_term(|term| Modes::from_term(*term.mode()))
    }

    /// The size, scrollback, cursor and modes of the screen.
    pub fn info(&self) -> ScreenInfo {
        self.with_term(|term| screen_info(term))
    }

    /// Copies lines of the grid. Line 0 is the first line of the live screen, lines above it (negative) are
    /// the scrollback, down to `-history_size`; the last is `rows - 1`. Lines outside that range are left
    /// out, so fewer than `count` rows may come back.
    pub fn read_lines(&self, first: i32, count: usize) -> Vec<Row> {
        self.with_term(|term| {
            let top = -i32::try_from(term.history_size()).unwrap_or(i32::MAX);
            let bottom = i32::try_from(term.screen_lines()).unwrap_or(i32::MAX) - 1;
            let last = i64::from(first) + i64::try_from(count).unwrap_or(i64::MAX) - 1;
            let start = first.max(top);
            let end = i32::try_from(last.min(i64::from(bottom))).unwrap_or(bottom);
            (start..=end)
                .map(|line| Row::from_grid(&term.grid()[Line(line)]))
                .collect()
        })
    }

    /// Copies the view: the live screen, or the scrollback when the view is scrolled back, with the screen
    /// state, all under one lock.
    pub fn snapshot(&self) -> Snapshot {
        self.with_term(|term| snapshot_of(term))
    }

    /// Scrolls the view back (positive) or forward (negative) by lines. A redraw is reported, as for
    /// [`Terminal::scroll_to_bottom`] and [`Terminal::resize`].
    pub fn scroll_display(&self, lines: i32) {
        self.with_term(|term| term.scroll_display(Scroll::Delta(lines)));
        self.shared.request_redraw();
    }

    /// Scrolls the view to the live screen.
    pub fn scroll_to_bottom(&self) {
        self.with_term(|term| term.scroll_display(Scroll::Bottom));
        self.shared.request_redraw();
    }

    /// Runs `f` with the terminal state locked for the UI, holding the reader's lease as well.
    ///
    /// The reader thread takes the lease, reads the PTY, and then only tries the lock; when the lock is taken,
    /// it reads again, and `alacritty_terminal` 0.26.0 drops the bytes it has read so far when that read fails
    /// (`EIO` on Linux, once the child has ended). Holding the lease keeps the reader out while the UI has the
    /// state, so it never meets a taken lock, and the last output of a child is not lost.
    fn with_term<R>(&self, f: impl FnOnce(&mut Term<Listener>) -> R) -> R {
        let _lease = self.term.lease();
        f(&mut self.term.lock_unfair())
    }

    /// Stops the reader thread and closes the PTY, which ends the shell if it is still running. The work
    /// is done on another thread; this returns at once. Called by `Drop`; calling it again does nothing.
    pub fn shutdown(&mut self) {
        let Some(reader) = self.reader.take() else {
            return;
        };
        let _ = self.sender.send(Msg::Shutdown);
        // Joining the reader returns the event loop with the PTY; dropping that waits for the child on
        // Unix and may wait for ConPTY's output to be read on Windows. Neither may block the UI thread.
        CLOSING.started();
        let closer = std::thread::Builder::new()
            .name("matsu-terminal-close".to_owned())
            .spawn(move || {
                drop(reader.join());
                CLOSING.finished();
            });
        if closer.is_err() {
            // The handle went with the closure: the reader thread ends by itself and closes the PTY on its
            // own thread, which cannot be waited for.
            CLOSING.finished();
        }
    }
}

/// How many terminals are still closing their PTY on a closer thread ([`Terminal::shutdown`]).
static CLOSING: Closing = Closing::new();

struct Closing {
    count: Mutex<usize>,
    done: Condvar,
}

impl Closing {
    const fn new() -> Self {
        Self {
            count: Mutex::new(0),
            done: Condvar::new(),
        }
    }

    fn started(&self) {
        *self.count.lock().unwrap_or_else(PoisonError::into_inner) += 1;
    }

    fn finished(&self) {
        let mut count = self.count.lock().unwrap_or_else(PoisonError::into_inner);
        *count = count.saturating_sub(1);
        self.done.notify_all();
    }
}

/// Waits until every terminal shut down so far has closed its PTY, which ends its shell, or until `timeout`
/// has passed. Returns whether they all have.
///
/// For the end of the editor: the closing runs on other threads, and a process that ends before them leaves
/// the shells behind on Windows. A shell whose ConPTY was never closed keeps running, and one whose closing
/// was cut short can hang for good (ADR-0028, 10: no child outlives its window).
pub fn wait_for_closing(timeout: Duration) -> bool {
    let count = CLOSING.count.lock().unwrap_or_else(PoisonError::into_inner);
    let (count, _) = CLOSING
        .done
        .wait_timeout_while(count, timeout, |count| *count > 0)
        .unwrap_or_else(PoisonError::into_inner);
    *count == 0
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Copies the view of `term`: see [`Terminal::snapshot`].
fn snapshot_of(term: &Term<Listener>) -> Snapshot {
    let info = screen_info(term);
    let offset = i32::try_from(info.display_offset).unwrap_or(0);
    let rows = (0..term.screen_lines())
        .map(|row| {
            let line = i32::try_from(row).unwrap_or(0) - offset;
            Row::from_grid(&term.grid()[Line(line)])
        })
        .collect();
    let colors = term.colors();
    let color_overrides = (0..=258)
        .filter_map(|index| {
            colors[index].map(|rgb| {
                (
                    index,
                    Rgb {
                        r: rgb.r,
                        g: rgb.g,
                        b: rgb.b,
                    },
                )
            })
        })
        .collect();
    Snapshot {
        info,
        rows,
        color_overrides,
    }
}

fn screen_info(term: &Term<Listener>) -> ScreenInfo {
    let cursor = term.renderable_content().cursor;
    ScreenInfo {
        columns: term.columns(),
        rows: term.screen_lines(),
        history_size: term.history_size(),
        display_offset: term.grid().display_offset(),
        cursor: Cursor {
            row: cursor.point.line.0,
            column: cursor.point.column.0,
            shape: CursorShape::from(cursor.shape),
        },
        modes: Modes::from_term(*term.mode()),
    }
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::vte::ansi::Processor;

    use super::*;

    /// A terminal state fed by hand, without a PTY.
    struct Harness {
        term: Term<Listener>,
        processor: Processor,
        events: EventReceiver,
        shared: Arc<Shared>,
    }

    impl Harness {
        fn new(size: TerminalSize) -> Self {
            let (events, receiver) = mpsc::channel();
            let shared = Arc::new(Shared {
                events,
                redraw_pending: Arc::new(AtomicBool::new(false)),
                writer: OnceLock::new(),
                size: Mutex::new(size),
                exit_status: Mutex::new(None),
                foreground: Rgb {
                    r: 10,
                    g: 20,
                    b: 30,
                },
                background: Rgb {
                    r: 40,
                    g: 50,
                    b: 60,
                },
                written: Mutex::new(Vec::new()),
            });
            let listener = Listener {
                shared: Arc::clone(&shared),
            };
            Self {
                term: Term::new(term_config(100), &size, listener),
                processor: Processor::new(),
                events: EventReceiver {
                    rx: receiver,
                    redraw_pending: Arc::clone(&shared.redraw_pending),
                },
                shared,
            }
        }

        fn feed(&mut self, bytes: &[u8]) {
            self.processor.advance(&mut self.term, bytes);
        }

        fn drain(&self) -> Vec<TerminalEvent> {
            std::iter::from_fn(|| self.events.try_recv().ok()).collect()
        }

        fn written(&self) -> Vec<u8> {
            self.shared.written.lock().expect("written").clone()
        }

        fn line(&self, line: i32) -> Row {
            Row::from_grid(&self.term.grid()[Line(line)])
        }
    }

    fn harness() -> Harness {
        Harness::new(TerminalSize::new(20, 5))
    }

    #[test]
    fn osc52_write_is_reported() {
        let mut h = harness();
        h.feed(b"\x1b]52;c;aGVsbG8=\x07");
        assert_eq!(
            h.drain(),
            [TerminalEvent::ClipboardStore {
                kind: ClipboardKind::Clipboard,
                text: "hello".to_owned()
            }]
        );
        h.feed("\x1b]52;p;5pel5pys\x1b\\".as_bytes());
        assert_eq!(
            h.drain(),
            [TerminalEvent::ClipboardStore {
                kind: ClipboardKind::Selection,
                text: "日本".to_owned()
            }]
        );
    }

    #[test]
    fn osc52_read_is_refused() {
        let mut h = harness();
        h.feed(b"\x1b]52;c;?\x07");
        h.feed(b"\x1b]52;p;?\x1b\\");
        h.feed(b"\x1b]52;;?\x07");
        assert!(h.drain().is_empty());
        assert!(h.written().is_empty(), "nothing is answered");
    }

    #[test]
    fn osc52_read_event_is_ignored_by_the_listener() {
        let h = harness();
        let listener = Listener {
            shared: Arc::clone(&h.shared),
        };
        listener.send_event(Event::ClipboardLoad(
            ClipboardType::Clipboard,
            Arc::new(|text| format!("answer {text}")),
        ));
        assert!(h.drain().is_empty());
        assert!(h.written().is_empty());
    }

    #[test]
    fn titles_are_reported() {
        let mut h = harness();
        h.feed(b"\x1b]0;one\x07\x1b]2;two\x07");
        assert_eq!(
            h.drain(),
            [
                TerminalEvent::Title("one".to_owned()),
                TerminalEvent::Title("two".to_owned())
            ]
        );
        let long = format!("\x1b]0;{}\x07", "a".repeat(300));
        h.feed(long.as_bytes());
        assert_eq!(
            h.drain(),
            [TerminalEvent::Title("a".repeat(crate::MAX_TITLE_CHARS))]
        );
    }

    #[test]
    fn titles_from_the_terminal_are_cleaned_by_the_listener() {
        let h = harness();
        let listener = Listener {
            shared: Arc::clone(&h.shared),
        };
        listener.send_event(Event::Title("a\x1b[31mb\u{202E}c\x07".to_owned()));
        listener.send_event(Event::ResetTitle);
        assert_eq!(
            h.drain(),
            [
                TerminalEvent::Title("a[31mbc".to_owned()),
                TerminalEvent::ResetTitle
            ]
        );
    }

    #[test]
    fn bell_is_reported() {
        let mut h = harness();
        h.feed(b"\x07");
        assert_eq!(h.drain(), [TerminalEvent::Bell]);
    }

    #[test]
    fn redraw_events_are_coalesced_until_received() {
        let h = harness();
        let listener = Listener {
            shared: Arc::clone(&h.shared),
        };
        listener.send_event(Event::Wakeup);
        listener.send_event(Event::Wakeup);
        listener.send_event(Event::Wakeup);
        assert_eq!(h.drain(), [TerminalEvent::Redraw]);
        listener.send_event(Event::Wakeup);
        assert_eq!(h.drain(), [TerminalEvent::Redraw]);
    }

    #[test]
    fn exit_carries_the_status() {
        let h = harness();
        let listener = Listener {
            shared: Arc::clone(&h.shared),
        };
        listener.send_event(Event::Exit);
        assert_eq!(h.drain(), [TerminalEvent::ChildExit { code: None }]);
    }

    #[test]
    fn plain_text_and_wide_characters_fill_the_grid() {
        let mut h = harness();
        h.feed("ab日本c\r\nline2".as_bytes());
        let row = h.line(0);
        assert_eq!(row.text(), "ab日本c");
        let chars: Vec<char> = row.cells.iter().take(7).map(|cell| cell.ch).collect();
        assert_eq!(chars, ['a', 'b', '日', ' ', '本', ' ', 'c']);
        assert!(row.cells[2].wide && !row.cells[2].wide_spacer);
        assert!(row.cells[3].wide_spacer && !row.cells[3].wide);
        assert!(row.cells[4].wide);
        assert!(row.cells[5].wide_spacer);
        assert!(!row.cells[6].wide);
        assert_eq!(h.line(1).text(), "line2");
        assert_eq!(row.cells.len(), 20);
    }

    #[test]
    fn colors_and_attributes_are_copied() {
        let mut h = harness();
        h.feed(b"\x1b[1;3;4;7;31;48;2;1;2;3mX\x1b[0m\x1b[38;5;200;9mY\x1b[0mZ");
        let row = h.line(0);
        let x = &row.cells[0];
        assert!(x.bold && x.italic && x.underline && x.inverse && !x.strikeout);
        assert_eq!(x.fg, Color::Indexed(1));
        assert_eq!(x.bg, Color::Rgb(Rgb { r: 1, g: 2, b: 3 }));
        let y = &row.cells[1];
        assert_eq!(y.fg, Color::Indexed(200));
        assert!(y.strikeout && !y.bold);
        let z = &row.cells[2];
        assert_eq!((z.fg, z.bg), (Color::Default, Color::Default));
        assert!(!z.bold && !z.underline);
    }

    #[test]
    fn combining_characters_stay_with_their_base() {
        let mut h = harness();
        h.feed("e\u{301}x".as_bytes());
        let row = h.line(0);
        assert_eq!(row.cells[0].ch, 'e');
        assert_eq!(row.cells[0].zerowidth, ['\u{301}']);
        assert_eq!(row.text(), "e\u{301}x");
    }

    #[test]
    fn hyperlinks_are_kept_in_the_cells() {
        let mut h = harness();
        h.feed(b"\x1b]8;;https://example.com/a\x1b\\ab\x1b]8;;\x1b\\c");
        let row = h.line(0);
        assert_eq!(
            row.cells[0].hyperlink.as_deref(),
            Some("https://example.com/a")
        );
        assert_eq!(
            row.cells[1].hyperlink.as_deref(),
            Some("https://example.com/a")
        );
        assert!(Arc::ptr_eq(
            row.cells[0].hyperlink.as_ref().expect("link"),
            row.cells[1].hyperlink.as_ref().expect("link")
        ));
        assert_eq!(row.cells[2].hyperlink, None);
    }

    #[test]
    fn modes_follow_the_program() {
        let mut h = harness();
        let modes = |h: &Harness| Modes::from_term(*h.term.mode());
        assert_eq!(modes(&h), Modes::DEFAULT);
        h.feed(b"\x1b[?2004h\x1b[?1h\x1b[?1000h\x1b[?1006h\x1b[?1049h");
        let on = modes(&h);
        assert!(on.bracketed_paste && on.app_cursor && on.mouse_click && on.sgr_mouse);
        assert!(on.alt_screen && on.mouse_reporting());
        h.feed(b"\x1b[?2004l\x1b[?1l\x1b[?1000l\x1b[?1006l\x1b[?1049l");
        assert_eq!(modes(&h), Modes::DEFAULT);
    }

    #[test]
    fn cursor_shape_and_position() {
        let mut h = harness();
        h.feed(b"abc\r\nde");
        let cursor = screen_info(&h.term).cursor;
        assert_eq!((cursor.row, cursor.column), (1, 2));
        assert_ne!(cursor.shape, CursorShape::Hidden);
        h.feed(b"\x1b[?25l");
        assert_eq!(screen_info(&h.term).cursor.shape, CursorShape::Hidden);
    }

    #[test]
    fn scrollback_is_readable_and_bounded() {
        let mut h = harness();
        for i in 0..12 {
            h.feed(format!("line{i}\r\n").as_bytes());
        }
        let info = screen_info(&h.term);
        assert_eq!(info.rows, 5);
        assert_eq!(info.history_size, 8);
        assert_eq!(h.line(-8).text(), "line0");
        assert_eq!(h.line(-1).text(), "line7");
        assert_eq!(h.line(0).text(), "line8");
    }

    #[test]
    fn queries_are_answered_without_the_clipboard() {
        let mut h = harness();
        // Cursor position report, primary device attributes, foreground color query, text area size.
        h.feed(b"ab\x1b[6n");
        assert_eq!(h.written(), b"\x1b[1;3R");
        h.feed(b"\x1b]10;?\x07");
        let written = String::from_utf8(h.written()).expect("utf-8");
        assert!(written.contains("rgb:0a0a/1414/1e1e"), "{written}");
        h.feed(b"\x1b[18t");
        let written = String::from_utf8(h.written()).expect("utf-8");
        assert!(written.ends_with("\x1b[8;5;20t"), "{written}");
    }

    #[test]
    fn palette_answers() {
        let fg = Rgb { r: 1, g: 2, b: 3 };
        let bg = Rgb { r: 4, g: 5, b: 6 };
        assert_eq!(palette_rgb(1, fg, bg), Rgb { r: 205, g: 0, b: 0 });
        assert_eq!(palette_rgb(16, fg, bg), Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(
            palette_rgb(231, fg, bg),
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(palette_rgb(196, fg, bg), Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(palette_rgb(232, fg, bg), Rgb { r: 8, g: 8, b: 8 });
        assert_eq!(
            palette_rgb(255, fg, bg),
            Rgb {
                r: 238,
                g: 238,
                b: 238
            }
        );
        assert_eq!(palette_rgb(256, fg, bg), fg);
        assert_eq!(palette_rgb(257, fg, bg), bg);
        assert_eq!(palette_rgb(258, fg, bg), fg);
    }

    #[test]
    fn size_is_clamped() {
        let size = TerminalSize::new(0, 0).clamped();
        assert_eq!((size.columns, size.rows), (2, 1));
    }
}
