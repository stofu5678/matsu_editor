//! Running a command on Windows as Sakura does: the command line as it is with `CreateProcess`,
//! and if that fails `cmd.exe /C` (`/K` when the output is not taken).

use std::os::windows::process::CommandExt;
use std::process::{Command, ExitStatus, Stdio};

use super::split_program;

/// `CREATE_NEW_CONSOLE`: Sakura's flag; a command whose output is not taken gets its own console.
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
/// `CREATE_NO_WINDOW`: a command whose output is taken shows nothing (Sakura: `SW_HIDE`).
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The program and the rest of the line as `CreateProcess` reads them. The rest is given to the
/// process as it is, without Rust's quoting.
pub(super) fn direct_command(line: &str) -> Option<Command> {
    let (program, rest) = split_program(line)?;
    let mut command = Command::new(program);
    if !rest.is_empty() {
        command.raw_arg(rest.trim_start_matches([' ', '\t']));
    }
    Some(command)
}

/// `cmd.exe /C line` (`/K` when the output is not taken, `/U` for Unicode output). Sakura gives
/// the full path in the system directory; `COMSPEC` is that file.
pub(super) fn shell_command(line: &str, capture: bool, unicode: bool) -> Command {
    let shell = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    let mut command = Command::new(shell);
    command.raw_arg(format!(
        "{}{} {line}",
        if unicode { "/U " } else { "" },
        if capture { "/C" } else { "/K" }
    ));
    command
}

pub(super) fn prepare(command: &mut Command, capture: bool) {
    command.creation_flags(if capture {
        CREATE_NO_WINDOW
    } else {
        CREATE_NEW_CONSOLE
    });
}

pub(super) fn exit_code(status: ExitStatus) -> Option<i32> {
    status.code()
}

/// Ends the process and the ones it started: `taskkill /T /F` (a job object would need `unsafe`).
pub(super) fn kill_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}
