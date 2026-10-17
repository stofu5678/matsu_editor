//! Running a command on macOS and Linux: the program, or `/bin/sh -c`; a process group so that
//! the 中断 button ends the processes the command started too.

// `kill` is an `unsafe` call (ADR-0013: only the OS modules of this crate have unsafe code).
#![allow(unsafe_code)]

use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, ExitStatus};

use super::{needs_shell, split_program};

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

/// The process leads a group of its own, which [`kill_tree`] ends.
pub(super) fn prepare(command: &mut Command, _capture: bool) {
    command.process_group(0);
}

/// What a shell reports: the exit code, or 128 plus the signal that ended the process.
pub(super) fn exit_code(status: ExitStatus) -> Option<i32> {
    status.code().or_else(|| status.signal().map(|s| 128 + s))
}

/// Ends the process group of `pid` (the process leads it, see [`prepare`]).
pub(super) fn kill_tree(pid: u32) {
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
