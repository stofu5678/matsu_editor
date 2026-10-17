//! Tests of running a command. They run on every OS: the commands are chosen so that the same
//! line does the same thing on Windows (`cmd.exe` and the programs of the system) and on macOS and
//! Linux (`/bin/sh` and the programs of POSIX), or are picked per OS.

use std::time::{Duration, Instant};

use super::*;

const LIMIT: Duration = Duration::from_secs(60);

/// What a run gave: the output, the way it ended, and how many events the output came in.
struct Done {
    output: Vec<u8>,
    exit: Exit,
}

impl Done {
    /// The output with the line breaks of Windows as the ones of the others.
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.output).replace("\r\n", "\n")
    }
}

/// Waits for the end of a run.
fn finish(mut run: Run) -> Done {
    let start = Instant::now();
    let mut output = Vec::new();
    loop {
        assert!(start.elapsed() < LIMIT, "the command does not end");
        match run.poll(Duration::from_millis(50)) {
            Some(Event::Output(bytes)) => output.extend(bytes),
            Some(Event::Exited(exit)) => return Done { output, exit },
            None => {}
        }
    }
}

fn run(command: &str) -> Done {
    finish(start(Spec::new(command)).unwrap())
}

#[test]
fn the_output_is_taken_and_the_code_is_zero() {
    let done = run("echo hello");
    assert_eq!(done.text().trim(), "hello");
    assert_eq!(done.exit.code, Some(0));
    assert!(!done.exit.stopped);
}

#[test]
fn the_error_comes_with_the_output() {
    // `1>&2` needs the shell on every OS
    let done = run("echo err 1>&2");
    assert_eq!(done.text().trim(), "err");
}

#[test]
fn a_line_that_is_no_program_goes_through_the_shell_and_keeps_its_code() {
    // `exit` is no program on any OS: the shell runs it
    assert_eq!(run("exit 3").exit.code, Some(3));
    assert_eq!(run("exit 0").exit.code, Some(0));
}

#[test]
fn a_command_that_does_not_exist_fails_in_the_shell() {
    let done = run("ume-no-such-command-qzx");
    assert_ne!(done.exit.code, Some(0));
    // the shell says why (Sakura shows it in the output, too)
    assert!(!done.output.is_empty());
}

#[test]
fn an_empty_command_does_not_start() {
    assert!(start(Spec::new("")).is_err());
    assert!(start(Spec::new("  \t")).is_err());
}

#[test]
fn a_folder_that_is_not_there_does_not_start() {
    let mut spec = Spec::new("echo hi");
    spec.folder = Some(PathBuf::from("this-folder-does-not-exist-qzx"));
    let error = start(spec).err().expect("it cannot start");
    assert!(!error.message.is_empty());
}

#[test]
fn the_command_runs_in_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("marker-qzx.txt"), "x").unwrap();
    let mut spec = Spec::new(if cfg!(windows) { "dir /B" } else { "ls" });
    spec.folder = Some(dir.path().to_path_buf());
    let done = finish(start(spec).unwrap());
    assert_eq!(done.text().trim(), "marker-qzx.txt");
}

/// A program that copies its input to its output.
const COPY: &str = if cfg!(windows) {
    "findstr \"^\""
} else {
    "cat"
};

#[test]
fn the_input_reaches_the_command() {
    let (tx, rx) = stdin_channel();
    let mut spec = Spec::new(COPY);
    spec.stdin = Some(rx);
    let run = start(spec).unwrap();
    assert!(tx.send(b"one\ntwo\n".to_vec()));
    assert!(tx.send("三\n".as_bytes().to_vec()));
    drop(tx);
    let done = finish(run);
    assert_eq!(done.text(), "one\ntwo\n三\n");
    assert_eq!(done.exit.code, Some(0));
}

#[test]
fn a_big_input_goes_through_in_pieces() {
    // 2 MB through a small queue: the sender waits for the command, and the output is read
    // meanwhile, so neither side blocks the other
    let (tx, rx) = stdin_channel();
    let mut spec = Spec::new(COPY);
    spec.stdin = Some(rx);
    let run = start(spec).unwrap();
    let line = format!("{}\n", "x".repeat(99));
    let sender = std::thread::spawn(move || {
        for _ in 0..20 {
            assert!(tx.send(line.repeat(1000).into_bytes()));
        }
    });
    let done = finish(run);
    sender.join().unwrap();
    assert_eq!(done.text().lines().count(), 20_000);
}

#[test]
fn a_command_that_does_not_read_ends_the_sending() {
    let (tx, rx) = stdin_channel();
    let mut spec = Spec::new("exit 0");
    spec.stdin = Some(rx);
    let done = finish(start(spec).unwrap());
    assert_eq!(done.exit.code, Some(0));
    // the receiver went with the writer thread: sending ends in a failure, not in a wait
    let start = Instant::now();
    let mut refused = false;
    while start.elapsed() < Duration::from_secs(10) {
        if !tx.send(vec![b'x'; 1024]) {
            refused = true;
            break;
        }
    }
    assert!(refused, "the sender is told that nobody reads");
}

/// A command that runs for half a minute.
const LONG: &str = if cfg!(windows) {
    "ping -n 30 127.0.0.1"
} else {
    "sleep 30"
};

#[test]
fn stopping_ends_a_long_command() {
    let mut run = start(Spec::new(LONG)).unwrap();
    // it is running
    assert_eq!(run.poll(Duration::from_millis(300)), None);
    let stopper = run.stopper();
    let begun = Instant::now();
    stopper.stop();
    let done = finish(run);
    assert!(done.exit.stopped);
    assert!(
        begun.elapsed() < Duration::from_secs(20),
        "it ends at once, not after the half minute"
    );
}

#[test]
fn stopping_after_the_end_does_nothing() {
    let run = start(Spec::new("exit 4")).unwrap();
    let stopper = run.stopper();
    let done = finish(run);
    stopper.stop();
    assert_eq!(done.exit.code, Some(4));
    assert!(!done.exit.stopped);
}

#[test]
fn a_command_that_is_not_taken_ends_without_output() {
    let mut spec = Spec::new("exit 0");
    spec.capture = false;
    let done = finish(start(spec).unwrap());
    assert!(done.output.is_empty());
    assert_eq!(done.exit.code, Some(0));
}

#[test]
fn the_run_is_over_after_its_exit() {
    let mut run = start(Spec::new("exit 0")).unwrap();
    let start = Instant::now();
    loop {
        assert!(start.elapsed() < LIMIT);
        if let Some(Event::Exited(_)) = run.poll(Duration::from_millis(50)) {
            break;
        }
    }
    assert_eq!(run.poll(Duration::from_millis(10)), None);
}

#[cfg(unix)]
mod unix_only {
    use super::*;

    #[test]
    fn the_output_comes_while_the_command_runs() {
        let mut run = start(Spec::new("echo first; sleep 2; echo second")).unwrap();
        let begun = Instant::now();
        let mut first = None;
        let mut seen = Vec::new();
        let mut exited_at = None;
        while begun.elapsed() < LIMIT {
            match run.poll(Duration::from_millis(20)) {
                Some(Event::Output(bytes)) => {
                    if first.is_none() {
                        first = Some(begun.elapsed());
                    }
                    seen.extend(bytes);
                }
                Some(Event::Exited(_)) => {
                    exited_at = Some(begun.elapsed());
                    break;
                }
                None => {}
            }
        }
        assert_eq!(String::from_utf8_lossy(&seen), "first\nsecond\n");
        assert!(first.unwrap() < Duration::from_millis(1500), "{first:?}");
        assert!(exited_at.unwrap() >= Duration::from_secs(2));
    }

    #[test]
    fn a_process_left_running_does_not_hold_the_end_back() {
        // the shell ends at once and leaves `sleep` with the pipe
        let begun = Instant::now();
        let done = run("sleep 20 &");
        assert_eq!(done.exit.code, Some(0));
        assert!(
            begun.elapsed() < Duration::from_secs(10),
            "{:?}",
            begun.elapsed()
        );
    }

    #[test]
    fn stopping_ends_what_the_command_started() {
        let dir = tempfile::tempdir().unwrap();
        let mark = dir.path().join("late");
        // `sleep` is a child of the shell; if only the shell died, `touch` would still run
        let command = format!("sleep 3 && touch {}", mark.display());
        let mut run = start(Spec::new(command)).unwrap();
        assert_eq!(run.poll(Duration::from_millis(300)), None);
        run.stopper().stop();
        let done = finish(run);
        assert!(done.exit.stopped);
        assert_eq!(done.exit.code, Some(128 + 9));
        std::thread::sleep(Duration::from_secs(4));
        assert!(!mark.exists(), "the command went on after it was stopped");
    }

    #[test]
    fn plain_words_run_as_a_program_and_the_rest_in_the_shell() {
        // quotes keep a space in an argument: only the shell does that
        let done = run("printf '%s|' 'a b' c");
        assert_eq!(done.text(), "a b|c|");
        // plain words are split at spaces
        assert_eq!(run("printf %s_%s_  x  y").text(), "x_y_");
    }

    #[test]
    fn the_bytes_are_not_changed() {
        // Shift_JIS "あ" and a lone 0xFF: the platform hands the bytes over as they are
        let done = run("printf '\\202\\240\\377'");
        assert_eq!(done.output, [0x82, 0xA0, 0xFF]);
    }

    #[test]
    fn a_slow_reader_loses_nothing_after_the_exit() {
        // 3 MB is more than the queue and the pipe hold: the command waits for us, and ends
        // before we read; the grace period after its end does not cut off what is queued
        let mut run = start(Spec::new("head -c 3000000 /dev/zero")).unwrap();
        std::thread::sleep(Duration::from_millis(1200));
        let mut total = 0;
        let begun = Instant::now();
        loop {
            assert!(begun.elapsed() < LIMIT);
            match run.poll(Duration::from_millis(50)) {
                Some(Event::Output(bytes)) => {
                    total += bytes.len();
                    // slower than the grace period over the whole output
                    if total % (1 << 20) < 8192 {
                        std::thread::sleep(Duration::from_millis(400));
                    }
                }
                Some(Event::Exited(exit)) => {
                    assert_eq!(exit.code, Some(0));
                    break;
                }
                None => {}
            }
        }
        assert_eq!(total, 3_000_000);
    }

    #[test]
    fn a_big_output_arrives_whole() {
        let done = run("head -c 3000000 /dev/zero");
        assert_eq!(done.output.len(), 3_000_000);
    }
}

#[test]
fn a_program_is_cut_off_the_line_as_createprocess_does() {
    assert_eq!(split_program("dir /B"), Some(("dir", " /B")));
    assert_eq!(split_program("  dir\t/B x"), Some(("dir", "\t/B x")));
    assert_eq!(
        split_program("\"C:\\Program Files\\x.exe\" a \"b c\""),
        Some(("C:\\Program Files\\x.exe", " a \"b c\""))
    );
    assert_eq!(split_program("one"), Some(("one", "")));
    assert_eq!(split_program(""), None);
    assert_eq!(split_program("   "), None);
    // an open quote has no program
    assert_eq!(split_program("\"abc"), None);
    assert_eq!(split_program("\"\" x"), None);
}

#[test]
fn a_line_with_shell_characters_needs_the_shell() {
    for line in [
        "echo $HOME",
        "a | b",
        "a > b",
        "a && b",
        "a; b",
        "echo 'x'",
        "echo \"x\"",
        "ls *.rs",
        "a\\b",
        "echo `x`",
        "a\nb",
    ] {
        assert!(needs_shell(line), "{line:?}");
    }
    for line in ["ls", "ls -l /tmp", "sort /R", "git status --short"] {
        assert!(!needs_shell(line), "{line:?}");
    }
}
