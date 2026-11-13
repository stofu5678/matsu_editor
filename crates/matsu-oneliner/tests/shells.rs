//! One-liners run in real shells: `/bin/sh`, `zsh` and `bash` on macOS and Linux; Windows
//! PowerShell 5.1, PowerShell 7 and `cmd` on Windows. Every wait has a limit that says what was
//! seen.

use std::sync::Arc;
use std::time::{Duration, Instant};

use matsu_oneliner::{
    Event, Finished, Input, OutputKind, Request, Run, RunError, StartError, start,
};

const LIMIT: Duration = Duration::from_secs(120);

/// The text as an input, in chunks of 64 KiB the way a document gives it.
fn input_of(text: &str) -> Input {
    let text: Arc<str> = Arc::from(text);
    Arc::new(move |sink: &mut dyn FnMut(&str) -> bool| {
        let mut rest: &str = &text;
        while !rest.is_empty() {
            let mut end = rest.len().min(64 * 1024);
            while !rest.is_char_boundary(end) {
                end += 1;
            }
            let (chunk, tail) = rest.split_at(end);
            if !sink(chunk) {
                break;
            }
            rest = tail;
        }
        Ok(())
    })
}

fn request(shell: &str, line: &str, text: &str, output: OutputKind) -> Request {
    Request {
        shell: shell.to_owned(),
        line: line.to_owned(),
        folder: None,
        input: input_of(text),
        output,
        path: None,
    }
}

/// Waits for the end of a run, saying what it saw when it does not come.
fn wait(mut run: Run, what: &str) -> Event {
    let begun = Instant::now();
    loop {
        if let Some(event) = run.poll(Duration::from_millis(50)) {
            return event;
        }
        assert!(begun.elapsed() < LIMIT, "{what}: no result after {LIMIT:?}");
    }
}

fn finished(event: Event, what: &str) -> Finished {
    match event {
        Event::Finished(done) => done,
        Event::Failed(error) => panic!("{what}: failed: {error}"),
    }
}

/// Runs `line` in `shell` with `text` as the input; the output as one string.
fn run(shell: &str, line: &str, text: &str, output: OutputKind) -> (Finished, String) {
    let what = format!("{shell}: {line}");
    let run = start(request(shell, line, text, output)).unwrap_or_else(|e| panic!("{what}: {e}"));
    let done = finished(wait(run, &what), &what);
    let text = done.output.concat();
    (done, text)
}

// ---------------------------------------------------------------------------------------------
// The same on every OS
// ---------------------------------------------------------------------------------------------

#[test]
fn an_empty_line_does_not_start() {
    for line in ["", "   ", "\n\t"] {
        let result = start(request("", line, "x", OutputKind::Replace));
        assert!(matches!(result, Err(StartError::EmptyLine)), "{line:?}");
    }
}

#[test]
fn a_setting_with_an_unclosed_quote_does_not_start() {
    let result = start(request("\"C:\\no end", "echo", "", OutputKind::Replace));
    assert!(matches!(result, Err(StartError::Shell(_))));
}

#[test]
fn a_shell_that_is_not_there_does_not_start() {
    let shell = if cfg!(windows) {
        "C:\\no\\such\\shell.exe"
    } else {
        "/no/such/shell"
    };
    match start(request(shell, "echo", "", OutputKind::Replace)) {
        Err(StartError::Process { program, message }) => {
            assert_eq!(program, shell);
            assert!(!message.is_empty());
        }
        other => panic!("expected a start error: {:?}", other.map(|_| ())),
    }
}

#[test]
fn a_text_that_cannot_be_read_stops_the_run() {
    let (line, shell) = if cfg!(windows) {
        ("$input | Out-Null", "powershell.exe")
    } else {
        ("cat", "/bin/sh")
    };
    let input: Input = Arc::new(|sink: &mut dyn FnMut(&str) -> bool| {
        sink("first\n");
        Err("the document went away".to_owned())
    });
    let mut r = request(shell, line, "", OutputKind::Replace);
    r.input = input;
    let run = start(r).unwrap();
    match wait(run, "an unreadable text") {
        Event::Failed(RunError::Input(message)) => {
            assert_eq!(message, "the document went away");
        }
        other => panic!("expected an input error: {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// macOS and Linux
// ---------------------------------------------------------------------------------------------

#[cfg(unix)]
mod unix {
    use std::path::PathBuf;

    use super::*;

    const SH: &str = "/bin/sh";

    /// 200 000 lines of 100 bytes: 20 MB.
    fn twenty_megabytes() -> String {
        format!("{}\n", "x".repeat(99)).repeat(200_000)
    }

    #[test]
    fn tr_changes_the_text_and_replace_drops_the_break_it_did_not_add() {
        let (done, output) = run(SH, "tr a-z A-Z", "hello\nworld", OutputKind::Replace);
        assert_eq!(output, "HELLO\nWORLD");
        assert_eq!(done.exit.code, Some(0));
        assert!(!done.exit.stopped);
        assert!(!done.input_ended_with_break);
        assert!(done.error.is_empty());
    }

    #[test]
    fn sort_sorts_and_the_break_that_sort_adds_is_dropped() {
        let (_, output) = run(SH, "sort", "b\nc\na", OutputKind::Replace);
        assert_eq!(output, "a\nb\nc");
        // the input had its own break: the output keeps its break
        let (done, output) = run(SH, "sort", "b\nc\na\n", OutputKind::Replace);
        assert_eq!(output, "a\nb\nc\n");
        assert!(done.input_ended_with_break);
    }

    #[test]
    fn next_line_ends_with_a_break_and_the_window_is_as_it_is() {
        let (_, output) = run(SH, "printf 'x\\ny'", "in", OutputKind::NextLine);
        assert_eq!(output, "x\ny\n");
        let (_, output) = run(SH, "printf 'x\\ny'", "in", OutputKind::Window);
        assert_eq!(output, "x\ny");
        let (_, output) = run(SH, "printf 'x\\n'", "in", OutputKind::NextLine);
        assert_eq!(output, "x\n");
    }

    #[test]
    fn the_exit_code_is_kept_and_the_output_with_it() {
        let (done, output) = run(SH, "echo partial; exit 3", "x", OutputKind::Replace);
        assert_eq!(done.exit.code, Some(3));
        assert_eq!(output, "partial");
    }

    #[test]
    fn the_error_comes_apart_from_the_output() {
        let (done, output) = run(
            SH,
            "echo out; echo err 1>&2; echo out2",
            "",
            OutputKind::Window,
        );
        assert_eq!(output, "out\nout2\n");
        assert_eq!(done.error, "err\n");
        assert!(!done.error_truncated);
    }

    #[test]
    fn a_flood_of_error_keeps_its_first_part() {
        let (done, output) = run(
            SH,
            "yes err | head -c 1000000 1>&2; echo done",
            "",
            OutputKind::Window,
        );
        assert_eq!(output, "done\n");
        assert!(done.error_truncated);
        assert_eq!(done.error.len(), matsu_core::oneliner::ERROR_CAP_BYTES);
        assert!(done.error.starts_with("err\nerr\n"));
    }

    #[test]
    fn japanese_and_emoji_go_there_and_back() {
        let text = "日本語のテキスト\n😀 emoji 👨‍👩‍👧 ｱｲｳ\n\n終わり";
        let (_, output) = run(SH, "cat", text, OutputKind::Replace);
        assert_eq!(output, text);
    }

    #[test]
    fn crlf_in_the_output_becomes_lf() {
        let (_, output) = run(SH, "printf 'a\\r\\nb\\rc\\n'", "", OutputKind::Window);
        assert_eq!(output, "a\nb\nc\n");
    }

    #[test]
    fn bytes_that_are_not_utf8_become_replacement_characters() {
        let (_, output) = run(SH, "printf 'a\\377b'", "", OutputKind::Window);
        assert_eq!(output, "a\u{FFFD}b");
    }

    #[test]
    fn twenty_megabytes_go_through_cat_without_a_deadlock() {
        let text = twenty_megabytes();
        let (done, output) = run(SH, "cat", &text, OutputKind::Window);
        assert_eq!(done.exit.code, Some(0));
        assert_eq!(output.len(), text.len());
        assert!(output == text, "the output is not the input");
    }

    #[test]
    fn a_command_that_does_not_read_its_input_is_given_a_large_one() {
        let text = twenty_megabytes();
        let (done, output) = run(SH, "echo hi", &text, OutputKind::Replace);
        // the text ends with a line break, so the break of `echo` stays
        assert_eq!(output, "hi\n");
        assert_eq!(done.exit.code, Some(0));
        // the text is read to its end even though nobody reads it
        assert!(done.input_ended_with_break);
    }

    #[test]
    fn a_descendant_that_keeps_the_input_open_does_not_hold_the_run_back() {
        // the shell ends at once; a background `sleep` still holds the input pipe, unread, and
        // the text is far larger than the pipe: sending it must be given up
        let line = "exec 3<&0; sleep 5 <&3 >/dev/null 2>&1 &";
        let begun = Instant::now();
        let (done, output) = run(SH, line, &twenty_megabytes(), OutputKind::Replace);
        assert_eq!(done.exit.code, Some(0));
        assert!(output.is_empty());
        assert!(done.input_ended_with_break);
        assert!(
            begun.elapsed() < Duration::from_secs(4),
            "the run waited for the descendant: {:?}",
            begun.elapsed()
        );
    }

    #[test]
    fn a_command_that_writes_a_lot_before_it_reads_does_not_lock_up() {
        // 30 MB out, and only then the input is read
        let (done, output) = run(
            SH,
            "head -c 30000000 /dev/zero | tr '\\0' x; cat >/dev/null",
            &twenty_megabytes(),
            OutputKind::Window,
        );
        assert_eq!(done.exit.code, Some(0));
        assert_eq!(output.len(), 30_000_000);
    }

    #[test]
    fn a_command_that_never_ends_is_stopped() {
        let mut run = start(request(SH, "sleep 1000", "", OutputKind::Replace)).unwrap();
        let stopper = run.stopper();
        // it must not end by itself
        assert!(run.poll(Duration::from_millis(300)).is_none());
        stopper.stop();
        let begun = Instant::now();
        loop {
            if let Some(event) = run.poll(Duration::from_millis(50)) {
                let done = finished(event, "sleep");
                assert!(done.exit.stopped);
                break;
            }
            assert!(begun.elapsed() < LIMIT, "a stopped command did not end");
        }
    }

    #[test]
    fn the_processes_it_started_are_stopped_too() {
        let mut run = start(request(SH, "sleep 1000 & wait", "", OutputKind::Replace)).unwrap();
        let stopper = run.stopper();
        assert!(run.poll(Duration::from_millis(300)).is_none());
        stopper.stop();
        let begun = Instant::now();
        loop {
            if run.poll(Duration::from_millis(50)).is_some() {
                break;
            }
            assert!(begun.elapsed() < LIMIT, "the group was not stopped");
        }
    }

    #[test]
    fn output_over_the_cap_stops_the_command_and_changes_nothing() {
        let run = start(request(SH, "yes", "", OutputKind::Replace)).unwrap();
        match wait(run, "yes") {
            Event::Failed(RunError::OutputTooLarge) => {}
            other => panic!("expected the cap to fail the run: {other:?}"),
        }
    }

    #[test]
    fn the_command_runs_in_the_folder_it_is_given() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("marker.txt"), "x").unwrap();
        let mut r = request(SH, "ls", "", OutputKind::Window);
        r.folder = Some(PathBuf::from(dir.path()));
        let done = finished(wait(start(r).unwrap(), "ls"), "ls");
        assert_eq!(done.output.concat(), "marker.txt\n");
    }

    #[test]
    fn only_the_program_of_the_setting_is_used() {
        // the arguments of the shell tab must not reach the one-liner's shell
        let (done, output) = run("/bin/sh -x -c false", "echo ok", "", OutputKind::Window);
        assert_eq!(output, "ok\n");
        assert_eq!(done.exit.code, Some(0));
        assert!(done.error.is_empty(), "{}", done.error);
    }

    #[test]
    fn an_empty_setting_runs_in_the_default_shell() {
        let (done, output) = run("", "echo ok", "", OutputKind::Window);
        assert_eq!(output, "ok\n");
        assert_eq!(done.exit.code, Some(0));
    }

    #[test]
    fn the_environment_is_inherited_and_nothing_is_added() {
        let (_, output) = run(SH, "env", "", OutputKind::Window);
        assert!(output.lines().any(|l| l.starts_with("PATH=")), "{output}");
        // what the shell itself sets is not ours; the rest of what the child has is ours
        let ours: Vec<String> = std::env::vars_os()
            .map(|(k, _)| k.to_string_lossy().into_owned())
            .collect();
        for line in output.lines() {
            let Some((key, _)) = line.split_once('=') else {
                continue; // the continuation of a value with a line break
            };
            if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                let shell_own = ["PWD", "OLDPWD", "SHLVL", "_"].contains(&key);
                assert!(shell_own || ours.iter().any(|k| k == key), "added: {key}");
            }
        }
    }

    #[test]
    fn the_input_is_the_standard_input_only() {
        let (_, output) = run(SH, "echo \"[$1][$#]\"", "secret", OutputKind::Window);
        assert_eq!(output, "[][0]\n");
        let (_, output) = run(SH, "env", "secret-text", OutputKind::Window);
        assert!(!output.contains("secret-text"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn zsh_runs_a_one_liner() {
        let (done, output) = run("/bin/zsh", "tr a-z A-Z | sort", "b\na", OutputKind::Replace);
        assert_eq!(output, "A\nB");
        assert_eq!(done.exit.code, Some(0));
        let (done, output) = run("/bin/zsh", "echo err 1>&2; exit 4", "", OutputKind::Window);
        assert_eq!(done.exit.code, Some(4));
        assert!(output.is_empty());
        assert_eq!(done.error, "err\n");
        let text = "日本語 😀\n";
        let (_, output) = run("/bin/zsh", "cat", text, OutputKind::Replace);
        assert_eq!(output, text);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn bash_runs_a_one_liner() {
        let bash = ["/bin/bash", "/usr/bin/bash"]
            .into_iter()
            .find(|p| std::path::Path::new(p).exists())
            .expect("bash is not installed");
        let (done, output) = run(bash, "tr a-z A-Z | sort", "b\na", OutputKind::Replace);
        assert_eq!(output, "A\nB");
        assert_eq!(done.exit.code, Some(0));
        // bash-only syntax
        let (_, output) = run(
            bash,
            "while read -r l; do echo \"<${l^^}>\"; done",
            "ab\ncd\n",
            OutputKind::Window,
        );
        assert_eq!(output, "<AB>\n<CD>\n");
        let (done, output) = run(bash, "echo err 1>&2; exit 4", "", OutputKind::Window);
        assert_eq!(done.exit.code, Some(4));
        assert!(output.is_empty());
        assert_eq!(done.error, "err\n");
        let text = "日本語 😀\n";
        let (_, output) = run(bash, "cat", text, OutputKind::Replace);
        assert_eq!(output, text);
    }
}

// ---------------------------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------------------------

// PowerShell: Windows PowerShell 5.1 and 7 on Windows, 7 elsewhere ------------------------------

mod powershell {
    use super::*;

    /// Windows: Windows PowerShell 5.1 and, when it is installed, PowerShell 7. Elsewhere:
    /// PowerShell 7 when it is installed (the runners of GitHub have it).
    fn powershells() -> Vec<String> {
        let path = std::env::var_os("PATH").unwrap_or_default();
        let pwsh = if cfg!(windows) { "pwsh.exe" } else { "pwsh" };
        let mut shells = Vec::new();
        if cfg!(windows) {
            shells.push("powershell.exe".to_owned());
        }
        if matsu_terminal::shell::find_on_path(pwsh, &path).is_some() {
            shells.push(pwsh.to_owned());
        } else {
            eprintln!("{pwsh} is not on the PATH: PowerShell 7 is not tested");
        }
        shells
    }

    fn ps(line: &str, text: &str, output: OutputKind) -> Vec<(String, Finished, String)> {
        powershells()
            .into_iter()
            .map(|shell| {
                let (done, output) = run(&shell, line, text, output);
                (shell, done, output)
            })
            .collect()
    }

    /// A native program that exits with `code`.
    fn native_exit(code: i32) -> String {
        if cfg!(windows) {
            format!("cmd /c exit {code}")
        } else {
            format!("/bin/sh -c 'exit {code}'")
        }
    }

    #[test]
    fn powershell_sorts_its_input() {
        for (shell, done, output) in ps("$input | Sort-Object", "b\nc\na", OutputKind::Replace) {
            assert_eq!(output, "a\nb\nc", "{shell}");
            assert_eq!(done.exit.code, Some(0), "{shell}");
            assert!(done.error.is_empty(), "{shell}: {}", done.error);
        }
        for (shell, _, output) in ps("$input | Sort-Object", "b\nc\na\n", OutputKind::Replace) {
            assert_eq!(output, "a\nb\nc\n", "{shell}");
        }
    }

    #[test]
    fn powershell_changes_each_line() {
        for (shell, _, output) in ps(
            "$input | % { $_.ToUpper() }",
            "abc\r\ndef",
            OutputKind::Replace,
        ) {
            assert_eq!(output, "ABC\nDEF", "{shell}");
        }
    }

    #[test]
    fn powershell_keeps_empty_lines_and_adds_no_last_one() {
        for (shell, _, output) in ps("$input", "a\n\nb\n\n\nc\n", OutputKind::Window) {
            assert_eq!(output, "a\n\nb\n\n\nc\n", "{shell}");
        }
        // a text that is one empty line
        for (shell, _, output) in ps("@($input).Count", "\n", OutputKind::Window) {
            assert_eq!(output, "1\n", "{shell}");
        }
        // no text, no line
        for (shell, _, output) in ps("@($input).Count", "", OutputKind::Window) {
            assert_eq!(output, "0\n", "{shell}");
        }
    }

    #[test]
    fn powershell_round_trips_japanese_and_emoji() {
        let text = "日本語のテキスト\n😀 emoji 👨‍👩‍👧 ｱｲｳ\n\n終わり";
        for (shell, _, output) in ps("$input", text, OutputKind::Replace) {
            assert_eq!(output, text, "{shell}");
        }
    }

    #[test]
    fn powershell_exit_codes() {
        for (line, code) in [
            ("exit 3", 3),
            (&native_exit(5), 5),
            ("Get-Item C:\\no\\such\\path-qzx", 1),
            ("throw 'boom'", 1),
            ("Write-Error 'x'; 'after'", 1),
            ("'fine'", 0),
            ("return 'early'", 0),
            ("if ($true) { return }", 0),
        ] {
            for (shell, done, _) in ps(line, "x", OutputKind::Window) {
                assert_eq!(done.exit.code, Some(code), "{shell}: {line}");
            }
        }
    }

    #[test]
    fn a_native_program_that_writes_to_its_error_is_not_a_failure() {
        let line = if cfg!(windows) {
            "cmd /c \"echo native-err 1>&2 & echo native-out\""
        } else {
            "/bin/sh -c 'echo native-err 1>&2; echo native-out'"
        };
        for (shell, done, output) in ps(line, "", OutputKind::Window) {
            assert_eq!(done.exit.code, Some(0), "{shell}");
            assert_eq!(output.trim(), "native-out", "{shell}");
            assert_eq!(done.error.trim(), "native-err", "{shell}");
        }
    }

    #[test]
    fn powershell_error_comes_apart_as_plain_text() {
        for (shell, done, output) in ps(
            "[Console]::Error.WriteLine('エラー'); Write-Error 'oops'; 'out'",
            "",
            OutputKind::Window,
        ) {
            assert_eq!(output, "out\n", "{shell}");
            assert!(done.error.contains("エラー"), "{shell}: {:?}", done.error);
            assert!(done.error.contains("oops"), "{shell}: {:?}", done.error);
            assert!(!done.error.contains("CLIXML"), "{shell}: {:?}", done.error);
            assert!(!done.error.contains("<Objs"), "{shell}: {:?}", done.error);
        }
    }

    #[test]
    fn the_other_streams_come_apart_as_plain_text_too() {
        for (shell, done, output) in ps(
            "Write-Warning 'warn'; Write-Host 'hosted'; Write-Verbose 'v' -Verbose; 'ok'",
            "",
            OutputKind::Window,
        ) {
            assert_eq!(output, "ok\n", "{shell}");
            assert_eq!(done.exit.code, Some(0), "{shell}");
            assert!(
                done.error.contains("WARNING: warn"),
                "{shell}: {:?}",
                done.error
            );
            assert!(done.error.contains("hosted"), "{shell}: {:?}", done.error);
            assert!(
                done.error.contains("VERBOSE: v"),
                "{shell}: {:?}",
                done.error
            );
            assert!(!done.error.contains("CLIXML"), "{shell}: {:?}", done.error);
        }
    }

    #[test]
    fn objects_are_formatted_whole_before_the_end_and_without_colors() {
        for (shell, done, output) in ps(
            "[pscustomobject]@{Name='abc';Value=3}",
            "",
            OutputKind::Window,
        ) {
            assert_eq!(done.exit.code, Some(0), "{shell}");
            assert!(output.contains("abc"), "{shell}: {output:?}");
            assert!(output.contains("Value"), "{shell}: {output:?}");
            assert!(!output.contains('\u{1b}'), "{shell}: {output:?}");
        }
        for (shell, _, output) in ps(
            "$input | Measure-Object -Line",
            "a\nb\n",
            OutputKind::Window,
        ) {
            assert!(output.contains("Lines"), "{shell}: {output:?}");
        }
    }

    #[test]
    fn a_large_input_goes_through_console_out() {
        let text = format!("{}\n", "x".repeat(99)).repeat(100_000);
        for (shell, done, output) in ps(
            "[Console]::Out.Write((@($input) -join \"`n\"))",
            &text,
            OutputKind::Window,
        ) {
            assert_eq!(done.exit.code, Some(0), "{shell}");
            assert_eq!(output.len(), text.len() - 1, "{shell}");
            let want = &text[..text.len() - 1];
            if output != want {
                let at = output.bytes().zip(want.bytes()).position(|(a, b)| a != b);
                panic!(
                    "{shell}: not the input, first difference at {at:?}: {:?} / {:?}",
                    &output[at.unwrap_or(0).saturating_sub(5)
                        ..(at.unwrap_or(0) + 20).min(output.len())],
                    &want
                        [at.unwrap_or(0).saturating_sub(5)..(at.unwrap_or(0) + 20).min(want.len())]
                );
            }
        }
    }

    #[test]
    fn the_command_that_ignores_its_input_is_given_a_large_one() {
        let line = "0123456789".repeat(9);
        let text = format!("{line}\n").repeat(100_000);
        for (shell, done, output) in ps("'hi'", &text, OutputKind::Replace) {
            // the text ends with a line break, so the break of the output stays
            assert_eq!(output, "hi\n", "{shell}");
            assert_eq!(done.exit.code, Some(0), "{shell}");
        }
    }

    #[test]
    fn a_command_that_never_ends_is_stopped() {
        let Some(shell) = powershells().into_iter().next() else {
            return;
        };
        let mut run = start(request(&shell, "Start-Sleep 1000", "", OutputKind::Replace)).unwrap();
        let stopper = run.stopper();
        assert!(run.poll(Duration::from_millis(500)).is_none());
        stopper.stop();
        let begun = Instant::now();
        loop {
            if let Some(event) = run.poll(Duration::from_millis(50)) {
                assert!(finished(event, "Start-Sleep").exit.stopped);
                break;
            }
            assert!(begun.elapsed() < LIMIT, "a stopped command did not end");
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    // cmd -------------------------------------------------------------------------------------

    /// Text that the OEM code page can hold: ASCII, and the letters of the code page when it has
    /// some (CP932 has Japanese, CP437 has `é`). The runners of GitHub are not Japanese. It ends
    /// with a line break: `findstr` drops a last line that has none.
    fn oem_text() -> (String, String) {
        let extra = ["あいう", "é"]
            .into_iter()
            .find(|sample| matsu_platform::exec::oem::encode(sample).is_some())
            .unwrap_or("");
        (format!("b{extra}"), format!("b{extra}\nc\na\n"))
    }

    #[test]
    fn cmd_numbers_lines_with_findstr() {
        let (first, text) = oem_text();
        let (done, output) = run("cmd.exe", "findstr /n .", &text, OutputKind::Replace);
        assert_eq!(output, format!("1:{first}\n2:c\n3:a\n"));
        assert_eq!(done.exit.code, Some(0));
        assert!(done.error.is_empty(), "{}", done.error);
    }

    #[test]
    fn cmd_sorts_in_the_oem_code_page() {
        // `sort` adds a line break at the end; the input had none
        let (done, output) = run("cmd.exe", "sort", "b\nc\na", OutputKind::Replace);
        assert_eq!(output, "a\nb\nc");
        assert!(!done.input_ended_with_break);
        // a line of the code page comes back as it went in
        let (_, text) = oem_text();
        let (_, output) = run("cmd.exe", "findstr .", &text, OutputKind::Window);
        assert_eq!(output, text);
    }

    #[test]
    fn cmd_keeps_the_exit_code_and_the_error_apart() {
        let (done, output) = run(
            "cmd.exe",
            "echo out& echo err 1>&2& exit /b 7",
            "",
            OutputKind::Window,
        );
        assert_eq!(done.exit.code, Some(7));
        assert_eq!(output.trim(), "out");
        assert_eq!(done.error.trim(), "err");
    }

    #[test]
    fn cmd_gets_its_command_line_verbatim() {
        let (_, output) = run(
            "cmd.exe",
            "echo \"a  b\" & echo \"c|d\"",
            "",
            OutputKind::Window,
        );
        assert_eq!(output, "\"a  b\" \n\"c|d\"\n");
    }

    #[test]
    fn an_emoji_is_refused_for_cmd_before_anything_starts() {
        if matsu_platform::exec::oem::code_page() == 65001 {
            eprintln!("the OEM code page is UTF-8: an emoji can be written");
            return;
        }
        let result = start(request(
            "cmd.exe",
            "findstr .",
            "ok\n😀\n",
            OutputKind::Replace,
        ));
        assert!(
            matches!(result, Err(StartError::CannotEncode)),
            "{:?}",
            result.map(|_| ())
        );
    }
}
