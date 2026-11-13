//! Which shell runs a one-liner, and the arguments that make it run one line (ADR-0029,
//! decision 3). Everything here is pure, except [`program`] and [`default_program`], which read
//! the setting's OS defaults; the rest is tested on every OS.
//!
//! * The program is the program part of the shell tab's setting (`[Terminal]` `Shell=`), never
//!   its arguments (`-NoExit` and `/K` are for the tab). An empty setting is the shell tab's
//!   default.
//! * The kind of the shell comes from the file name of the program ([`ShellKind::of`]), and the
//!   arguments from the kind ([`invocation`]).

use matsu_terminal::shell::{CommandLineError, ShellCommand};

use crate::powershell;

/// The family of a shell, which decides how it is asked to run one line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellKind {
    /// `pwsh` or `powershell`.
    PowerShell,
    /// `cmd`.
    Cmd,
    /// `sh`, `bash`, `zsh`, `fish` and every unknown one.
    Posix,
}

impl ShellKind {
    /// The kind by the file name of `program`: case-insensitive, without `.exe`, and a directory
    /// in front of it (either separator) does not count.
    pub fn of(program: &str) -> Self {
        let name = program.rsplit(['/', '\\']).next().unwrap_or(program);
        let name = name.to_ascii_lowercase();
        let name = name.strip_suffix(".exe").unwrap_or(&name);
        match name {
            "pwsh" | "powershell" => Self::PowerShell,
            "cmd" => Self::Cmd,
            _ => Self::Posix,
        }
    }

    /// How the text goes to the shell and the output comes back: UTF-8 for PowerShell (its script
    /// makes it so) and for the POSIX-like shells, the OEM code page for `cmd` (its `sort` and
    /// `more` read that, whatever `chcp` says).
    pub fn text_encoding(self) -> TextEncoding {
        match self {
            Self::Cmd => TextEncoding::Oem,
            Self::PowerShell | Self::Posix => TextEncoding::Utf8,
        }
    }
}

/// The encoding of the standard input, the standard output and the standard error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEncoding {
    Utf8,
    /// The OEM code page of Windows (`GetOEMCP`): CP932 on Japanese Windows. A character it
    /// cannot hold is an error before anything starts. Off Windows it is UTF-8.
    Oem,
}

/// A shell and the arguments that make it run one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub program: String,
    pub kind: ShellKind,
    /// The arguments; each reaches the program as one.
    pub args: Vec<String>,
    /// `cmd`: its command line after the program, as it is (Windows reads `/D /S /C "line"` by
    /// its own rules, which the quoting of the other arguments would break).
    pub raw_args: Option<String>,
}

/// The program and the arguments that run `line`:
///
/// * PowerShell: `-NoProfile -NonInteractive -EncodedCommand <script>` ([`powershell`]).
/// * `cmd`: `/D /S /C "<line>"`, verbatim. `/D` skips the AutoRun commands of the registry; `/S`
///   makes the first and the last quote of the command line the ones that are taken off.
/// * Otherwise: `-c <line>`. No `-l` and no `-i`: the startup files are not read beyond what the
///   shell always reads (zsh reads `.zshenv`).
pub fn invocation(program: &str, line: &str) -> Invocation {
    let kind = ShellKind::of(program);
    let (args, raw_args) = match kind {
        ShellKind::PowerShell => (
            vec![
                "-NoProfile".to_owned(),
                "-NonInteractive".to_owned(),
                "-EncodedCommand".to_owned(),
                powershell::encoded_command(line),
            ],
            None,
        ),
        ShellKind::Cmd => (Vec::new(), Some(format!("/D /S /C \"{line}\""))),
        ShellKind::Posix => (vec!["-c".to_owned(), line.to_owned()], None),
    };
    Invocation {
        program: program.to_owned(),
        kind,
        args,
        raw_args,
    }
}

/// Why no shell could be chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellError {
    /// The setting is not a command line (an unclosed quote).
    BadSetting(CommandLineError),
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSetting(error) => write!(f, "the shell setting is not valid: {error}"),
        }
    }
}

impl std::error::Error for ShellError {}

/// The program part of the shell tab's setting, read the way `windows` or Unix reads a command
/// line: on Windows as `CreateProcess` reads it (a program in double quotes ends at the closing
/// quote), elsewhere the first word as [`ShellCommand::from_unix_command_line`] splits it.
/// `None` for an empty setting (the shell tab's default).
pub fn program_of_setting(setting: &str, windows: bool) -> Result<Option<String>, ShellError> {
    if setting.trim().is_empty() {
        return Ok(None);
    }
    let parsed = if windows {
        ShellCommand::from_windows_command_line(setting)
    } else {
        ShellCommand::from_unix_command_line(setting)
    };
    match parsed {
        Ok(shell) => Ok(Some(shell.program)),
        Err(CommandLineError::Empty) => Ok(None),
        Err(error) => Err(ShellError::BadSetting(error)),
    }
}

/// The program for `setting`, and `default` when the setting is empty. Not tied to the OS, so
/// that the choice can be tested everywhere.
pub fn choose_program(
    setting: &str,
    windows: bool,
    default: impl FnOnce() -> String,
) -> Result<String, ShellError> {
    Ok(program_of_setting(setting, windows)?.unwrap_or_else(default))
}

/// The shell of the shell tab when the setting is empty (ADR-0028, option 3):
///
/// * Windows: `matsu_terminal::shell::default_shell`, PowerShell 7 when `pwsh.exe` is on the `PATH`,
///   else Windows PowerShell.
/// * macOS and Linux: `$SHELL` when it is an absolute path, else the shell of the password
///   database (what `alacritty_terminal` does; its function is private, so
///   `matsu_platform::exec::login_shell` asks again), else `/bin/sh`.
pub fn default_program() -> String {
    #[cfg(windows)]
    {
        matsu_terminal::shell::default_shell()
            .map_or_else(|| "powershell.exe".to_owned(), |shell| shell.program)
    }
    #[cfg(unix)]
    {
        matsu_platform::exec::login_shell(std::env::var_os("SHELL").as_deref()).map_or_else(
            || "/bin/sh".to_owned(),
            |p| p.to_string_lossy().into_owned(),
        )
    }
}

/// The program for the shell tab's setting `setting` on this OS.
pub fn program(setting: &str) -> Result<String, ShellError> {
    choose_program(setting, cfg!(windows), default_program)
}

#[cfg(test)]
mod tests;
