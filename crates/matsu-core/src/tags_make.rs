//! Making a tags file with `ctags` (`F_TAGS_MAKE`, タグファイルの作成).
//!
//! Sakura: `CViewCommander::Command_TagsMake` (`sakura_core/cmd/CViewCommander_TagJump.cpp`) and
//! `CDlgTagsMake` (`sakura_core/dlg/CDlgTagsMake.cpp`); the help is `help/sakura/res/HLP000064.html`.
//!
//! Sakura runs `ctags.exe` from its own folder with `--excmd=n`, `-R` if "サブフォルダーも対象に
//! する" is checked, the options the user typed, and `*` (everything in the folder), in the chosen
//! folder, and treats any text that the command writes as an error message.
//!
//! This program does not carry ctags (Universal Ctags and Exuberant Ctags are GPL). It runs one
//! that the user has installed: next to the program (as Sakura's `ctags.exe` is next to
//! `sakura.exe`) or in a folder of `PATH`. When there is none, [`Outcome::NotFound`] says so.
//!
//! Differences from Sakura: the command line is split into words here (Sakura gives it to
//! `cmd.exe`) and `*` is made by listing the folder (without names that begin with a dot, as a
//! shell does), so no shell is involved; a ctags that fails without writing anything is reported
//! too (Sakura says it was finished).

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// The name of the program to look for.
pub const CTAGS_COMMAND: &str = if cfg!(windows) { "ctags.exe" } else { "ctags" };

/// How long the output of the command is kept for the message (`char work[1024]` in Sakura).
const MAX_OUTPUT: usize = 1000;

/// How many bytes of the output are kept while the rest is read and thrown away (a character
/// takes up to 4 bytes, and a Shift_JIS or UTF-8 character must not be cut).
const MAX_OUTPUT_BYTES: usize = MAX_OUTPUT * 4;

/// The choices of the dialog (`m_nTagsOpt`, `m_szTagsCmdLine`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TagsMakeOptions {
    /// サブフォルダーも対象にする (`m_nTagsOpt & 0x0001`): `-R`.
    pub recursive: bool,
    /// コマンドラインオプション: more options for ctags.
    pub command_line: String,
}

/// What came of running ctags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No ctags was found.
    NotFound,
    /// The command could not be started (the message of the OS).
    CannotStart(String),
    /// ctags wrote nothing and succeeded (`STR_ERR_CEDITVIEW_CMD07`).
    Finished,
    /// ctags wrote something: Sakura takes it as an error message (`STR_ERR_CEDITVIEW_CMD06`).
    Output(String),
    /// The user stopped it.
    Cancelled,
}

/// The folders to look for ctags in: the folder of the program, then those of `PATH`.
pub fn ctags_dirs(program_dir: Option<&Path>, path_var: Option<OsString>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = program_dir.map(Path::to_path_buf).into_iter().collect();
    if let Some(path) = path_var {
        dirs.extend(std::env::split_paths(&path).filter(|p| !p.as_os_str().is_empty()));
    }
    dirs
}

/// The first ctags that is a file in `dirs`.
pub fn find_ctags(dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(CTAGS_COMMAND))
        .find(|path| path.is_file())
        // the program is started in another folder: a relative path would not find it
        .map(|path| std::path::absolute(&path).unwrap_or(path))
}

/// Splits the line of options into words: white space separates them and quotes (`"` or `'`)
/// keep white space inside a word.
pub fn split_command_line(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            (None, c) => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    words
}

/// The arguments of ctags: `--excmd=n`, `-R`, the words of the command line, and the names in
/// the folder (`*`).
pub fn ctags_arguments(options: &TagsMakeOptions, folder: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["--excmd=n".into()];
    if options.recursive {
        args.push("-R".into());
    }
    args.extend(
        split_command_line(&options.command_line)
            .into_iter()
            .map(OsString::from),
    );
    args.extend(folder_entries(folder));
    args
}

/// `*`: the names in the folder, in order, without those that begin with a dot (hidden files
/// and folders such as `.git`).
fn folder_entries(folder: &Path) -> Vec<OsString> {
    let mut names: Vec<OsString> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name())
        .filter(|name| !name.to_string_lossy().starts_with('.'))
        .collect();
    names.sort();
    // a name that begins with `-` would be taken for an option
    for name in &mut names {
        if name.to_string_lossy().starts_with('-') {
            let mut safe = OsString::from("./");
            safe.push(&*name);
            *name = safe;
        }
    }
    names
}

/// Runs `ctags` in `folder` and waits for it. `cancel` stops it when it becomes true. The
/// waiting polls, so run this off the thread of the window.
pub fn run_ctags(
    ctags: &Path,
    folder: &Path,
    options: &TagsMakeOptions,
    cancel: &AtomicBool,
) -> Outcome {
    let mut child = match Command::new(ctags)
        .args(ctags_arguments(options, folder))
        .current_dir(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Outcome::CannotStart(error.to_string()),
    };
    let output = Arc::new(Mutex::new(Vec::<u8>::new()));
    let readers: Vec<_> = [
        child.stdout.take().map(|p| spawn_reader(p, &output)),
        child.stderr.take().map(|p| spawn_reader(p, &output)),
    ]
    .into_iter()
    .flatten()
    .collect();
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            // The readers are not waited for: a child of ctags may still hold the pipes.
            return Outcome::Cancelled;
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => return Outcome::CannotStart(error.to_string()),
        }
    };
    // A child of ctags may keep the pipes open; the wait can still be cancelled.
    while readers.iter().any(|r| !r.is_finished()) {
        if cancel.load(Ordering::Relaxed) {
            return Outcome::Cancelled;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let bytes = output.lock().map(|b| b.clone()).unwrap_or_default();
    let text = crate::tag_file::decode_text(&bytes);
    let text = text.trim();
    if !text.is_empty() {
        return Outcome::Output(text.chars().take(MAX_OUTPUT).collect());
    }
    if status.success() {
        Outcome::Finished
    } else {
        Outcome::Output(format!("{status}"))
    }
}

fn spawn_reader<R: Read + Send + 'static>(
    mut pipe: R,
    into: &Arc<Mutex<Vec<u8>>>,
) -> thread::JoinHandle<()> {
    let into = Arc::clone(into);
    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        while let Ok(n) = pipe.read(&mut buf) {
            if n == 0 {
                break;
            }
            if let Ok(mut all) = into.lock() {
                // the pipe is read to the end so that ctags is never blocked, but only the start
                // of the output is kept
                let room = MAX_OUTPUT_BYTES.saturating_sub(all.len());
                all.extend_from_slice(&buf[..n.min(room)]);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_line_is_split_into_words() {
        assert_eq!(split_command_line(""), Vec::<String>::new());
        assert_eq!(split_command_line("  --a  -b=1 "), ["--a", "-b=1"]);
        assert_eq!(
            split_command_line(r#"--exclude="my dir" 'x y' "" z"#),
            ["--exclude=my dir", "x y", "", "z"]
        );
    }

    #[test]
    fn the_arguments_are_sakuras() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.c"), "").unwrap();
        std::fs::write(dir.path().join("a.c"), "").unwrap();
        std::fs::write(dir.path().join(".hidden"), "").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let text = |o: &TagsMakeOptions| -> Vec<String> {
            ctags_arguments(o, dir.path())
                .into_iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect()
        };
        assert_eq!(
            text(&TagsMakeOptions::default()),
            ["--excmd=n", "a.c", "b.c", "sub"]
        );
        let options = TagsMakeOptions {
            recursive: true,
            command_line: "--languages=C,C++".into(),
        };
        assert_eq!(
            text(&options),
            ["--excmd=n", "-R", "--languages=C,C++", "a.c", "b.c", "sub"]
        );
    }

    #[test]
    fn a_name_that_looks_like_an_option_is_made_a_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("--languages=C"), "").unwrap();
        std::fs::write(dir.path().join("a.c"), "").unwrap();
        let args: Vec<String> = ctags_arguments(&TagsMakeOptions::default(), dir.path())
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["--excmd=n", "./--languages=C", "a.c"]);
    }

    #[test]
    fn ctags_is_looked_for_in_the_folders_in_order() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let dirs = [first.path().to_path_buf(), second.path().to_path_buf()];
        assert_eq!(find_ctags(&dirs), None);
        std::fs::write(second.path().join(CTAGS_COMMAND), "").unwrap();
        assert_eq!(find_ctags(&dirs), Some(second.path().join(CTAGS_COMMAND)));
        std::fs::write(first.path().join(CTAGS_COMMAND), "").unwrap();
        assert_eq!(find_ctags(&dirs), Some(first.path().join(CTAGS_COMMAND)));
        // a folder with that name is not a program
        let third = tempfile::tempdir().unwrap();
        std::fs::create_dir(third.path().join(CTAGS_COMMAND)).unwrap();
        assert_eq!(find_ctags(&[third.path().to_path_buf()]), None);
    }

    #[test]
    fn the_folders_are_the_program_folder_then_path() {
        let path = std::env::join_paths(["/x/bin", "/y/bin"]).unwrap();
        let dirs = ctags_dirs(Some(Path::new("/app")), Some(path));
        assert_eq!(
            dirs,
            [
                PathBuf::from("/app"),
                PathBuf::from("/x/bin"),
                PathBuf::from("/y/bin")
            ]
        );
        assert!(ctags_dirs(None, None).is_empty());
    }

    #[cfg(unix)]
    mod with_a_fake_ctags {
        use std::os::unix::fs::PermissionsExt;

        use super::*;

        /// A script that stands for ctags.
        fn fake(dir: &Path, body: &str) -> PathBuf {
            let path = dir.join("fake-ctags");
            std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }

        #[test]
        fn a_quiet_success_is_a_finished_tags_file() {
            let bin = tempfile::tempdir().unwrap();
            let work = tempfile::tempdir().unwrap();
            std::fs::write(work.path().join("a.c"), "").unwrap();
            // writes the arguments it got into the file `tags`, in the folder it runs in
            let ctags = fake(bin.path(), "echo \"$@\" > tags");
            let options = TagsMakeOptions {
                recursive: true,
                command_line: "-x".into(),
            };
            let outcome = run_ctags(&ctags, work.path(), &options, &AtomicBool::new(false));
            assert_eq!(outcome, Outcome::Finished);
            let tags = std::fs::read_to_string(work.path().join("tags")).unwrap();
            assert_eq!(tags.trim(), "--excmd=n -R -x a.c");
        }

        #[test]
        fn any_output_is_an_error_message() {
            let bin = tempfile::tempdir().unwrap();
            let work = tempfile::tempdir().unwrap();
            let ctags = fake(bin.path(), "echo 'bad option' >&2");
            let outcome = run_ctags(
                &ctags,
                work.path(),
                &TagsMakeOptions::default(),
                &AtomicBool::new(false),
            );
            assert_eq!(outcome, Outcome::Output("bad option".into()));
            // also on standard output, also when it exits with success
            let ctags = fake(bin.path(), "echo warning; exit 0");
            let outcome = run_ctags(
                &ctags,
                work.path(),
                &TagsMakeOptions::default(),
                &AtomicBool::new(false),
            );
            assert_eq!(outcome, Outcome::Output("warning".into()));
        }

        #[test]
        fn a_flood_of_output_is_drained_and_cut() {
            let bin = tempfile::tempdir().unwrap();
            let work = tempfile::tempdir().unwrap();
            // about 2 MB, more than a pipe holds
            let ctags = fake(bin.path(), "yes 0123456789 | head -c 2000000");
            let outcome = run_ctags(
                &ctags,
                work.path(),
                &TagsMakeOptions::default(),
                &AtomicBool::new(false),
            );
            assert!(matches!(outcome, Outcome::Output(m) if m.chars().count() == MAX_OUTPUT));
        }

        #[test]
        fn cancelling_works_while_a_child_holds_the_pipes() {
            let bin = tempfile::tempdir().unwrap();
            let work = tempfile::tempdir().unwrap();
            let ctags = fake(bin.path(), "sleep 30 &\nexit 0");
            let cancel = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&cancel);
            let stopper = thread::spawn(move || {
                thread::sleep(Duration::from_millis(300));
                flag.store(true, Ordering::Relaxed);
            });
            let started = std::time::Instant::now();
            let outcome = run_ctags(&ctags, work.path(), &TagsMakeOptions::default(), &cancel);
            stopper.join().unwrap();
            assert_eq!(outcome, Outcome::Cancelled);
            assert!(started.elapsed() < Duration::from_secs(10));
        }

        #[test]
        fn a_silent_failure_is_not_called_finished() {
            let bin = tempfile::tempdir().unwrap();
            let work = tempfile::tempdir().unwrap();
            let ctags = fake(bin.path(), "exit 3");
            let outcome = run_ctags(
                &ctags,
                work.path(),
                &TagsMakeOptions::default(),
                &AtomicBool::new(false),
            );
            assert!(matches!(outcome, Outcome::Output(m) if m.contains('3')));
        }

        #[test]
        fn a_running_ctags_can_be_cancelled() {
            let bin = tempfile::tempdir().unwrap();
            let work = tempfile::tempdir().unwrap();
            let ctags = fake(bin.path(), "sleep 30");
            let cancel = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&cancel);
            let stopper = thread::spawn(move || {
                thread::sleep(Duration::from_millis(200));
                flag.store(true, Ordering::Relaxed);
            });
            let started = std::time::Instant::now();
            let outcome = run_ctags(&ctags, work.path(), &TagsMakeOptions::default(), &cancel);
            stopper.join().unwrap();
            assert_eq!(outcome, Outcome::Cancelled);
            assert!(started.elapsed() < Duration::from_secs(10));
        }

        #[test]
        fn a_program_that_cannot_start_says_why() {
            let work = tempfile::tempdir().unwrap();
            let outcome = run_ctags(
                &work.path().join("missing"),
                work.path(),
                &TagsMakeOptions::default(),
                &AtomicBool::new(false),
            );
            assert!(matches!(outcome, Outcome::CannotStart(_)));
        }
    }
}
