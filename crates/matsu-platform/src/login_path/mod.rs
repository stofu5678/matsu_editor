//! The `PATH` of the login shell, for macOS.
//!
//! A program started from Finder, the Dock or Launchpad (launchd) does not pass through a shell:
//! its `PATH` is the short one of the system (`/usr/bin:/bin:/usr/sbin:/sbin`), and `~/.zshrc`
//! is never read. Homebrew (`/opt/homebrew/bin`) and the places a user added to the `PATH`
//! are not searched, so the program of a database client, a macro language or ctags is
//! "not found" although it is installed. This module asks the login shell for its `PATH` once,
//! at the start, in the background (the way VS Code resolves the shell environment).
//!
//! * [`start`] begins the reading and returns at once; it does nothing but on macOS, and when
//!   the program was started from a terminal (which already has the `PATH` of its shell).
//! * [`get`] is the `PATH` when it has been read, and never waits.
//! * [`wait`] waits for the reading (at most [`TIMEOUT`] after [`start`]); call it from a thread
//!   that may block, not from the one of the window.
//! * Nothing here sets the environment of this process (`std::env::set_var` is not safe with
//!   other threads running): the callers hand the `PATH` to the programs with
//!   `Command::env("PATH", ..)`.
//!
//! No `unsafe` code; the shell is ended with the `exec` module's `kill_tree`.

use std::ffi::{OsStr, OsString};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(unix)]
mod unix;

#[cfg(test)]
mod tests;

/// How long the login shell may take, counted from [`start`]. A `~/.zshrc` that waits for a
/// network or a prompt must not hold back the programs for ever. VS Code gives its shell 10
/// seconds as well.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// How much of the output of the shell is looked at. A configuration file that prints a lot must
/// not use up the memory.
#[cfg(any(unix, test))]
const OUTPUT_LIMIT: usize = 1024 * 1024;

/// The variable the shell is started with, so that a heavy `~/.zshrc` can skip what a `PATH` does
/// not need (`[ -n "$MATSU_RESOLVING_ENVIRONMENT" ] && return`). VS Code does the same with
/// `VSCODE_RESOLVING_ENVIRONMENT`.
pub const RESOLVING_VARIABLE: &str = "MATSU_RESOLVING_ENVIRONMENT";

/// The shell of macOS, when `SHELL` is not an absolute path.
const DEFAULT_SHELL: &str = "/bin/zsh";

struct Slot {
    /// `None` while the shell runs; then the `PATH`, or `None` when it could not be read.
    result: Mutex<Option<Option<OsString>>>,
    done: Condvar,
    deadline: Instant,
}

static SLOT: OnceLock<Slot> = OnceLock::new();

/// Whether the shell should be asked: on macOS, and when the program is not run from a terminal.
fn should_read(macos: bool, from_terminal: bool) -> bool {
    macos && !from_terminal
}

/// The shell to ask: `SHELL` when it is an absolute path, else the one of macOS. `getpwuid` is
/// not used (it would need `unsafe`).
fn choose_shell(shell: Option<&OsStr>) -> PathBuf {
    match shell {
        Some(shell) if Path::new(shell).is_absolute() => PathBuf::from(shell),
        _ => PathBuf::from(DEFAULT_SHELL),
    }
}

/// Starts reading the `PATH` of the login shell in the background and returns at once. Call it
/// once at the start; the later calls do nothing.
pub fn start() {
    let from_terminal = std::io::stdin().is_terminal()
        || std::io::stdout().is_terminal()
        || std::io::stderr().is_terminal();
    if !should_read(cfg!(target_os = "macos"), from_terminal) {
        return;
    }
    start_with(
        choose_shell(std::env::var_os("SHELL").as_deref()),
        TIMEOUT,
        std::env::var_os("PATH"),
    );
}

#[cfg(unix)]
fn start_with(shell: PathBuf, timeout: Duration, startup: Option<OsString>) {
    let slot = Slot {
        result: Mutex::new(None),
        done: Condvar::new(),
        deadline: Instant::now() + timeout,
    };
    if SLOT.set(slot).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("login-path".to_owned())
        .spawn(move || {
            let deadline = SLOT.get().map_or_else(Instant::now, |s| s.deadline);
            let path = match unix::read(&shell, deadline, startup.as_deref()) {
                Ok(path) => Some(path),
                Err(why) => {
                    eprintln!(
                        "matsu: could not read the PATH of the login shell ({}): {why}; \
                         programs get the PATH MATSU started with",
                        shell.display()
                    );
                    None
                }
            };
            publish(path);
        });
    if spawned.is_err() {
        publish(None);
    }
}

#[cfg(not(unix))]
fn start_with(_shell: PathBuf, _timeout: Duration, _startup: Option<OsString>) {}

#[cfg(unix)]
fn publish(path: Option<OsString>) {
    if let Some(slot) = SLOT.get() {
        *slot.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(path);
        slot.done.notify_all();
    }
}

/// The `PATH` of the login shell, mixed with the one this program started with (see [`mix`]),
/// when it has been read; `None` while it is read, when it could not be, and when [`start`] did
/// nothing. Never waits.
pub fn get() -> Option<OsString> {
    let slot = SLOT.get()?;
    slot.result
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .flatten()
}

/// Whether the shell is still being asked, so that [`wait`] would wait. `false` when [`start`]
/// did nothing: then there is no need to go to another thread to call it.
pub fn is_pending() -> bool {
    SLOT.get().is_some_and(|slot| {
        slot.result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_none()
    })
}

/// Waits until the `PATH` has been read or [`TIMEOUT`] after [`start`] has passed, then gives
/// [`get`]. Returns at once when [`start`] did nothing or the reading is over. Blocks the thread:
/// not for the one of the window.
pub fn wait() -> Option<OsString> {
    let slot = SLOT.get()?;
    let mut result = slot.result.lock().unwrap_or_else(|e| e.into_inner());
    while result.is_none() {
        let left = slot.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        result = slot
            .done
            .wait_timeout(result, left)
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
    result.clone().flatten()
}

/// The command line that the login shell runs: one program, so that the syntax of the shell
/// (zsh, bash, fish) does not matter. Everything is inside single quotes, which every shell
/// passes on as they are, and `mark` is letters, digits and `_` only.
#[cfg(any(unix, test))]
fn shell_script(mark: &str) -> String {
    format!("/bin/sh -c 'printf \"%s%s%s\" {mark} \"$PATH\" {mark}'")
}

/// A mark that is different at every start (the process id and the time).
#[cfg(any(unix, test))]
fn make_mark() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("MATSU_PATH_{}_{nanos}_", std::process::id())
}

/// The bytes between the first mark and the next one; `None` when there are fewer than two.
#[cfg(any(unix, test))]
fn between_marks<'a>(output: &'a [u8], mark: &str) -> Option<&'a [u8]> {
    let mark = mark.as_bytes();
    let start = find(output, mark)? + mark.len();
    let end = start + find(&output[start..], mark)?;
    Some(&output[start..end])
}

#[cfg(any(unix, test))]
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// The `PATH` in the output of the shell: what is between the marks (the configuration files of
/// the shell may print anything around them). Why not, in words.
#[cfg(any(unix, test))]
fn parse_output(output: &[u8], mark: &str) -> Result<Vec<u8>, &'static str> {
    let path = between_marks(output, mark).ok_or("the shell did not print the PATH")?;
    if path.is_empty() {
        return Err("the PATH of the shell is empty");
    }
    if path.contains(&0) {
        return Err("the PATH of the shell has a NUL");
    }
    Ok(path.to_vec())
}

/// The `PATH` for the programs: the absolute folders of the `PATH` of the shell in its order
/// (the first of a repeated one), then the absolute folders of the `PATH` this program started
/// with that are not in it yet, in their order.
///
/// * An empty or relative folder (`.`) is dropped: the programs run in the folder of a document,
///   and a program of that folder must not be run by its name.
/// * The `PATH` of the start follows, so that nothing that was found before is lost (what a test
///   or a launcher added stays).
///
/// `None` when the shell has no absolute folder: it could not be read.
#[cfg(any(unix, test))]
fn mix(shell: &OsStr, startup: Option<&OsStr>) -> Option<OsString> {
    fn absolute_new(path: &OsStr, folders: &mut Vec<PathBuf>) {
        for folder in std::env::split_paths(path) {
            if folder.is_absolute() && !folders.contains(&folder) {
                folders.push(folder);
            }
        }
    }
    let mut folders: Vec<PathBuf> = Vec::new();
    absolute_new(shell, &mut folders);
    if folders.is_empty() {
        return None;
    }
    if let Some(startup) = startup {
        absolute_new(startup, &mut folders);
    }
    std::env::join_paths(folders).ok()
}
