//! Tests with a real PTY (ConPTY on Windows). Each starts a non-interactive command as the shell (never the
//! user's own shell, so their configuration does not matter), waits for the result by events with a time
//! limit (no fixed sleeps), and reads the grid.

use std::time::{Duration, Instant};

use matsu_terminal::{EventReceiver, Options, ShellCommand, Terminal, TerminalEvent, TerminalSize};

/// Generous: ConPTY on a loaded Windows ARM64 runner is slow to start.
const LIMIT: Duration = Duration::from_secs(60);

/// `sh -c <script>` on Unix, `cmd.exe <args>` on Windows.
fn command(unix_script: &str, windows_args: &[&str]) -> ShellCommand {
    if cfg!(windows) {
        ShellCommand::new("cmd.exe", windows_args.iter().copied())
    } else {
        ShellCommand::new("/bin/sh", ["-c", unix_script])
    }
}

/// The terminal of a test. Dropping it shuts the terminal down and waits until its shell has ended, so that no
/// shell outlives the test binary: on Windows, a closing that the end of the process cuts short leaves the
/// shell behind.
struct Started(Option<Terminal>);

impl Started {
    /// The terminal itself, for a test of its own `Drop` (which then waits with `wait_for_closing`).
    fn into_inner(mut self) -> Terminal {
        self.0.take().expect("the terminal")
    }
}

impl std::ops::Deref for Started {
    type Target = Terminal;

    fn deref(&self) -> &Terminal {
        self.0.as_ref().expect("the terminal")
    }
}

impl std::ops::DerefMut for Started {
    fn deref_mut(&mut self) -> &mut Terminal {
        self.0.as_mut().expect("the terminal")
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        if let Some(terminal) = self.0.take() {
            drop(terminal);
            let closed = matsu_terminal::wait_for_closing(LIMIT);
            if !std::thread::panicking() {
                assert!(closed, "a shell was not closed in time");
            }
        }
    }
}

fn start(shell: ShellCommand) -> (Started, EventReceiver) {
    start_with(Options {
        shell: Some(shell),
        ..Options::default()
    })
}

fn start_with(options: Options) -> (Started, EventReceiver) {
    let (terminal, events) = Terminal::spawn(options).expect("spawn the terminal");
    (Started(Some(terminal)), events)
}

fn screen_text(terminal: &Terminal) -> String {
    let rows: Vec<String> = terminal
        .snapshot()
        .rows
        .iter()
        .map(matsu_terminal::Row::text)
        .collect();
    rows.join("\n")
}

/// Waits until the screen text satisfies `done`; returns the text. Panics with the text at the time limit.
fn wait_for_screen(
    terminal: &Terminal,
    events: &EventReceiver,
    what: &str,
    done: impl Fn(&str) -> bool,
) -> String {
    let deadline = Instant::now() + LIMIT;
    loop {
        let text = screen_text(terminal);
        if done(&text) {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}; the screen was:\n{text}"
        );
        // Redraw and the other events wake us; the timeout only covers a missed wake-up.
        let _ = events.recv_timeout(Duration::from_millis(100));
    }
}

/// Waits for an event that `pick` accepts; other events are skipped.
fn wait_for_event<T>(
    events: &EventReceiver,
    what: &str,
    pick: impl Fn(&TerminalEvent) -> Option<T>,
) -> T {
    let deadline = Instant::now() + LIMIT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(!left.is_zero(), "timed out waiting for {what}");
        match events.recv_timeout(left.min(Duration::from_millis(500))) {
            Ok(event) => {
                if let Some(found) = pick(&event) {
                    return found;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(err) => panic!("the terminal's events ended while waiting for {what}: {err}"),
        }
    }
}

fn long_running() -> ShellCommand {
    command("sleep 60", &["/c", "ping", "-n", "60", "127.0.0.1"])
}

/// Starts [`long_running`]. On Windows it waits until `ping` has printed its first line (which names the
/// address in every language), so that the shell is attached to its console before the test closes it: a
/// ConPTY closed while its client is still starting can leave the client running when the test binary ends
/// right after. (`sleep` prints nothing; Unix needs no wait.)
fn start_long_running() -> (Started, EventReceiver) {
    let (terminal, events) = start(long_running());
    if cfg!(windows) {
        wait_for_screen(&terminal, &events, "ping's first line", |t| {
            t.contains("127.0.0.1")
        });
    }
    (terminal, events)
}

#[test]
fn output_reaches_the_grid() {
    let (terminal, events) = start(command(
        "printf 'hello from the pty\\n'",
        &["/c", "echo", "hello", "from", "the", "pty"],
    ));
    wait_for_screen(&terminal, &events, "the output", |t| {
        t.contains("hello from the pty")
    });
}

/// The shell of the setting `Shell`: a command line as the user writes it (ADR-0028, the fifth PR).
#[test]
fn a_command_line_of_the_setting_starts_the_shell() {
    let line = if cfg!(windows) {
        // /K keeps cmd.exe running (ConPTY loses what a program wrote right before it ended). cmd.exe reads its own rules: the quotes and the `&` reach it as they are written. (Through the C
        // runtime's quoting `"a  b"` would arrive as `\"a  b\"` and cmd.exe would print the backslashes.)
        r#"cmd.exe /Q /K echo "two  spaces" & echo second"#
    } else {
        r#"/bin/sh -c 'printf "two  spaces\nsecond\n"'"#
    };
    let shell = ShellCommand::from_command_line(line).expect("parse the command line");
    let (terminal, events) = start(shell);
    let text = wait_for_screen(&terminal, &events, "the two lines", |t| {
        t.contains("second") && t.contains("two  spaces")
    });
    // (A backslash can be in the prompt of cmd.exe: only the escaped quote would be the C runtime's.)
    assert!(
        !text.contains("\\\""),
        "the quotes must not be escaped:\n{text}"
    );
}

#[test]
fn the_scrollback_length_is_the_options() {
    let (terminal, events) = start_with(Options {
        shell: Some(command(
            "i=0; while [ $i -lt 100 ]; do echo line$i; i=$((i+1)); done; echo finished",
            &[
                "/c",
                "(for",
                "/L",
                "%i",
                "in",
                "(1,1,100)",
                "do",
                "@echo",
                "line%i)",
                "&",
                "echo",
                "finished",
            ],
        )),
        size: TerminalSize::new(40, 10),
        scrollback_lines: 7,
        ..Options::default()
    });
    wait_for_screen(&terminal, &events, "the end of the output", |t| {
        t.contains("finished")
    });
    let history = terminal.snapshot().info.history_size;
    assert!(
        history <= 7,
        "the scrollback has {history} lines, the option allows 7"
    );
    if cfg!(unix) {
        // 100 lines went by on a screen of 10: the scrollback is full.
        assert_eq!(history, 7);
    }
}

#[cfg(unix)]
#[test]
fn japanese_output_uses_wide_cells() {
    let (terminal, events) = start(command("printf '日本語ab\\n'", &[]));
    wait_for_screen(&terminal, &events, "the output", |t| t.contains("日本語ab"));
    let snapshot = terminal.snapshot();
    let row = snapshot
        .rows
        .iter()
        .find(|row| row.text() == "日本語ab")
        .expect("the row");
    let wide: Vec<bool> = row.cells.iter().take(8).map(|c| c.wide).collect();
    let spacer: Vec<bool> = row.cells.iter().take(8).map(|c| c.wide_spacer).collect();
    assert_eq!(wide, [true, false, true, false, true, false, false, false]);
    assert_eq!(
        spacer,
        [false, true, false, true, false, true, false, false]
    );
}

#[test]
fn exit_status_is_reported_after_the_output() {
    let (terminal, events) = start(command(
        "printf 'before exit\\n'; exit 3",
        &["/c", "exit", "3"],
    ));
    let code = wait_for_event(&events, "the child's exit", |event| match event {
        TerminalEvent::ChildExit { code } => Some(*code),
        _ => None,
    });
    assert_eq!(code, Some(3));
    if cfg!(unix) {
        // The output written before the exit is not lost.
        wait_for_screen(&terminal, &events, "the output", |t| {
            t.contains("before exit")
        });
    }
}

#[test]
fn exit_status_zero() {
    let (_terminal, events) = start(command("exit 0", &["/c", "exit", "0"]));
    let code = wait_for_event(&events, "the child's exit", |event| match event {
        TerminalEvent::ChildExit { code } => Some(*code),
        _ => None,
    });
    assert_eq!(code, Some(0));
}

#[test]
fn after_the_exit_nothing_can_be_written() {
    let (terminal, events) = start(command("exit 0", &["/c", "exit", "0"]));
    wait_for_event(&events, "the child's exit", |event| match event {
        TerminalEvent::ChildExit { .. } => Some(()),
        _ => None,
    });
    // The reader thread ends right after it reports the exit.
    let deadline = Instant::now() + LIMIT;
    while terminal.is_running() {
        assert!(Instant::now() < deadline, "the reader thread did not end");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(terminal.write(b"x").is_err());
    assert!(terminal.resize(TerminalSize::new(90, 20)).is_err());
    // The screen is still there, resized.
    assert_eq!(terminal.info().columns, 90);
}

#[test]
fn events_end_when_the_terminal_is_gone() {
    let (terminal, events) = start_long_running();
    drop(terminal);
    let deadline = Instant::now() + LIMIT;
    loop {
        match events.recv_timeout(Duration::from_millis(100)) {
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                assert!(Instant::now() < deadline, "the events never ended");
            }
        }
    }
}

#[test]
fn a_missing_program_is_an_error() {
    let result = Terminal::spawn(Options {
        shell: Some(ShellCommand::new(
            "matsu-no-such-program-4f9c2",
            Vec::<String>::new(),
        )),
        ..Options::default()
    });
    assert!(result.is_err());
}

#[test]
fn term_and_colorterm_reach_the_child() {
    let (terminal, events) = start(command(
        "printf '%s %s\\n' \"$TERM\" \"$COLORTERM\"",
        &["/c", "echo", "%TERM%", "%COLORTERM%"],
    ));
    let text = wait_for_screen(&terminal, &events, "the output", |t| {
        t.contains("xterm-256color")
    });
    assert!(text.contains("truecolor"), "{text}");
}

#[test]
fn the_working_directory_is_used() {
    let dir = tempfile::tempdir().expect("tempdir");
    let name = dir
        .path()
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    let (terminal, events) = start_with(Options {
        shell: Some(command("pwd", &["/c", "cd"])),
        working_directory: Some(dir.path().to_path_buf()),
        ..Options::default()
    });
    wait_for_screen(&terminal, &events, "the directory", |t| t.contains(&name));
}

#[test]
fn a_working_directory_that_is_gone_or_a_file_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("a file");
    std::fs::write(&file, "").expect("write the file");
    for (path, kind) in [
        (dir.path().join("gone"), std::io::ErrorKind::NotFound),
        (file, std::io::ErrorKind::NotADirectory),
    ] {
        let result = Terminal::spawn(Options {
            shell: Some(command("pwd", &["/c", "cd"])),
            working_directory: Some(path.clone()),
            ..Options::default()
        });
        let error = result.err().expect("an error");
        assert_eq!(error.kind(), kind, "{}: {error}", path.display());
    }
}

#[test]
fn resize_changes_the_screen_and_the_child_sees_it() {
    let (terminal, events) = start(if cfg!(windows) {
        long_running()
    } else {
        // Prints the size at start and again at every SIGWINCH (the trap runs between the short sleeps).
        command(
            "stty size; trap 'stty size' WINCH; while :; do sleep 0.2; done",
            &[],
        )
    });
    let info = terminal.info();
    assert_eq!((info.columns, info.rows), (80, 24));
    if cfg!(unix) {
        wait_for_screen(&terminal, &events, "the first size", |t| {
            t.contains("24 80")
        });
    } else {
        // As in `start_long_running`: the shell is attached before the test goes on (and closes it).
        wait_for_screen(&terminal, &events, "ping's first line", |t| {
            t.contains("127.0.0.1")
        });
    }

    terminal.resize(TerminalSize::new(100, 30)).expect("resize");
    let info = terminal.info();
    assert_eq!((info.columns, info.rows), (100, 30));
    assert_eq!(terminal.snapshot().rows.len(), 30);
    assert!(
        terminal
            .snapshot()
            .rows
            .iter()
            .all(|r| r.cells.len() == 100)
    );
    if cfg!(unix) {
        wait_for_screen(&terminal, &events, "the new size in the child", |t| {
            t.contains("30 100")
        });
    }
}

#[cfg(unix)]
#[test]
fn title_bell_and_clipboard_store_arrive_as_events() {
    let (_terminal, events) = start(command(
        "printf '\\033]0;my title\\007\\007\\033]52;c;aGVsbG8=\\007'",
        &[],
    ));
    let mut seen_title = false;
    let mut seen_bell = false;
    let mut seen_clipboard = false;
    let deadline = Instant::now() + LIMIT;
    while !(seen_title && seen_bell && seen_clipboard) {
        assert!(
            Instant::now() < deadline,
            "title {seen_title}, bell {seen_bell}, clipboard {seen_clipboard}"
        );
        match events.recv_timeout(Duration::from_millis(500)) {
            Ok(TerminalEvent::Title(title)) => {
                assert_eq!(title, "my title");
                seen_title = true;
            }
            Ok(TerminalEvent::Bell) => seen_bell = true,
            Ok(TerminalEvent::ClipboardStore { kind, text }) => {
                assert_eq!(kind, matsu_terminal::ClipboardKind::Clipboard);
                assert_eq!(text, "hello");
                seen_clipboard = true;
            }
            Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(err) => panic!("events ended: {err}"),
        }
    }
}

#[cfg(unix)]
#[test]
fn input_reaches_the_child() {
    // `cat` echoes what it reads; the terminal's own echo shows the typed text too, so wait for two copies.
    let (terminal, events) = start(command("cat", &[]));
    terminal.write(b"typed text\r").expect("write");
    wait_for_screen(&terminal, &events, "the echo", |t| {
        t.matches("typed text").count() >= 2
    });
}

#[test]
fn dropping_the_terminal_does_not_block() {
    let (terminal, _events) = start_long_running();
    // The child has started; drop while it still runs.
    let terminal = terminal.into_inner();
    let started = Instant::now();
    drop(terminal);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "dropping took {:?}",
        started.elapsed()
    );
    // The closing goes on elsewhere, and can be waited for.
    assert!(
        matsu_terminal::wait_for_closing(LIMIT),
        "the shell was not closed in time"
    );
}

#[test]
fn writing_after_shutdown_is_an_error() {
    let (mut terminal, _events) = start_long_running();
    terminal.shutdown();
    assert!(terminal.write(b"x").is_err());
    assert!(terminal.resize(TerminalSize::new(90, 20)).is_err());
    // A second shutdown does nothing.
    terminal.shutdown();
}

#[test]
fn encodings_follow_the_terminal_modes() {
    use matsu_terminal::{Key, Modifiers};
    let (terminal, _events) = start_long_running();
    assert_eq!(
        terminal.encode_key(Key::Up, Modifiers::NONE).expect("key"),
        b"\x1b[A"
    );
    let paste = terminal.encode_paste("a\nb");
    assert!(paste.needs_confirmation);
    assert_eq!(paste.bytes, b"a\rb");
}

#[cfg(unix)]
#[test]
fn links_are_found_by_osc_8_and_in_plain_text() {
    let (terminal, events) = start(command(
        "printf '\\033]8;;https://osc.example/a\\033\\\\label\\033]8;;\\033\\\\ and http://plain.example/b. done\\n'; exec cat",
        &[],
    ));
    wait_for_screen(&terminal, &events, "the line", |text| text.contains("done"));
    // "label" is columns 0..=4: the address of the OSC 8 link, which need not be shown
    let osc = terminal.link_at(0, 2).expect("the OSC 8 link");
    assert_eq!(osc.uri, "https://osc.example/a");
    assert!(osc.contains(0, 0) && osc.contains(0, 4) && !osc.contains(0, 5));
    // the address in the text, without the full stop of the sentence
    let plain = terminal.link_at(0, 12).expect("the plain address");
    assert_eq!(plain.uri, "http://plain.example/b");
    // the blank between them, and rows below the text
    assert_eq!(terminal.link_at(0, 5), None);
    assert_eq!(terminal.link_at(5, 0), None);
    assert_eq!(terminal.link_at(1000, 0), None);
}

#[cfg(unix)]
#[test]
fn the_mouse_modes_the_program_sets_are_seen() {
    use matsu_terminal::{Modifiers, MouseAction, MouseButton};
    let (terminal, events) = start(command(
        "printf '\\033[?1002h\\033[?1006h ready\\n'; exec cat",
        &[],
    ));
    wait_for_screen(&terminal, &events, "the line", |text| {
        text.contains("ready")
    });
    let modes = terminal.modes();
    assert!(modes.mouse_drag && modes.sgr_mouse && !modes.mouse_motion);
    let press = terminal.encode_mouse(MouseAction::Press(MouseButton::Left), Modifiers::NONE, 2, 3);
    assert_eq!(press.expect("reported"), b"\x1b[<0;4;3M");
    // the drag mode does not report motion with no button
    assert_eq!(
        terminal.encode_mouse(MouseAction::Move(None), Modifiers::NONE, 0, 0),
        None
    );
}
