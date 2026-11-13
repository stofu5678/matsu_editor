//! Running a command on macOS and Linux: the program, or `/bin/sh -c`; a process group so that
//! the 中断 button ends the processes the command started too.

// `kill` is an `unsafe` call (ADR-0013: only the OS modules of this crate have unsafe code).
#![allow(unsafe_code)]

use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, ExitStatus};

use super::{Program, needs_shell, split_program};

/// A process group outlives its leader, so the processes a command left running can be ended.
pub(super) const KILLS_AFTER_EXIT: bool = true;

/// The command line as a program with its arguments, split at spaces; `None` when it needs a
/// shell (see [`needs_shell`]), so that the shell runs it.
pub(super) fn direct_command(line: &str) -> Option<Command> {
    if needs_shell(line) {
        return None;
    }
    let (program, rest) = split_program(line)?;
    let mut command = Command::new(program);
    command.args(rest.split_whitespace());
    Some(command)
}

/// `/bin/sh -c line` (Sakura: `cmd.exe /C line`).
pub(super) fn shell_command(line: &str, _capture: bool, _unicode: bool) -> Command {
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg(line);
    command
}

/// The program with its arguments, as they are (`raw_args` is for Windows).
pub(super) fn program_command(program: &Program) -> Command {
    let mut command = Command::new(&program.program);
    command.args(&program.args);
    command
}

/// The shell of the user for the one-liners, as the shell tab finds it: `SHELL` when it is an
/// absolute path, else the one the password database has for the user (what `alacritty_terminal`
/// does; its function is private, so this asks `getpwuid_r` itself). `None` when neither says.
pub fn login_shell(env_shell: Option<&std::ffi::OsStr>) -> Option<std::path::PathBuf> {
    use std::path::{Path, PathBuf};
    if let Some(shell) = env_shell.filter(|s| Path::new(s).is_absolute()) {
        return Some(PathBuf::from(shell));
    }
    passwd_shell().filter(|s| s.is_absolute())
}

fn passwd_shell() -> Option<std::path::PathBuf> {
    use std::ffi::{CStr, OsStr};
    use std::os::unix::ffi::OsStrExt;
    let mut entry = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut buffer = vec![0 as libc::c_char; 4096];
    let mut found: *mut libc::passwd = std::ptr::null_mut();
    // SAFETY: `getpwuid` and `getpwuid_r` take a user id and write into the buffers given, whose
    // sizes are passed. `entry` is read only when the call succeeded and `found` points to it; the
    // shell string it holds points into `buffer`, which lives until the copy below.
    unsafe {
        let uid = libc::getuid();
        let status = libc::getpwuid_r(
            uid,
            entry.as_mut_ptr(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut found,
        );
        if status != 0 || found.is_null() {
            return None;
        }
        let shell = (*found).pw_shell;
        if shell.is_null() {
            return None;
        }
        let bytes = CStr::from_ptr(shell).to_bytes();
        (!bytes.is_empty()).then(|| std::path::PathBuf::from(OsStr::from_bytes(bytes)))
    }
}

/// The process leads a group of its own, which [`kill_tree`] ends.
pub(super) fn prepare(command: &mut Command, _capture: bool) {
    command.process_group(0);
}

/// What a shell reports: the exit code, or 128 plus the signal that ended the process.
pub(super) fn exit_code(status: ExitStatus) -> Option<i32> {
    status.code().or_else(|| status.signal().map(|s| 128 + s))
}

/// Ends the process group of `pid` (the process leads it, see [`prepare`]).
pub(crate) fn kill_tree(pid: u32) {
    let Ok(pid) = i32::try_from(pid) else { return };
    if pid <= 1 {
        return;
    }
    // SAFETY: `kill` takes two integers and touches no memory of this program. The negative
    // number is the process group that `process_group(0)` made with the id of the process.
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
}
