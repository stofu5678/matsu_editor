//! Making the command line of a connection.
//!
//! The template is the user's (they wrote it, and it is run as 外部コマンド実行 runs a command
//! line), but the file name is not: it comes from the file system, and a name such as
//! `x'; rm -rf ~; '.sql` or `a & b.sql` must stay one argument. So the file is quoted for the shell
//! that will read the line, and what cannot be quoted for that shell is refused.
//!
//! * macOS and Linux: `/bin/sh -c` (`matsu_platform::exec`). The name goes between single quotes,
//!   and each `'` in it becomes `'\''`. Nothing else is special between single quotes, not even a
//!   line break.
//! * Windows: the line is read by `CreateProcess` (the C runtime rules for the arguments) and, if
//!   that fails, by `cmd.exe /C`. The name goes between double quotes. A Windows file name cannot
//!   have a `"`, and between double quotes `& | < > ^ ( )` mean nothing to `cmd.exe`, but `%` still
//!   starts a variable (`"%PATH%"`) and `!` does with delayed expansion (`cmd /V:ON`, also
//!   inside quotes: `"!PATH!"`), so a name with a `%` or a `!` is refused. So is a name with a control
//!   character.
//!
//! The text of the line then goes through `ExpandParameter` (`$F` and the like) in the exec flow,
//! as every command line does; [`escape_for_expand`] doubles each `$` so that a `$` in a file name
//! or in the template comes out unchanged.

use std::fmt;

use matsu_config::databases::{Connection, Part, template_parts};
use matsu_core::exec::{MAX_COMMAND_UNITS, limit_command};

/// The shell that reads the command line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shell {
    /// `/bin/sh -c`
    Posix,
    /// `cmd.exe /C` (and `CreateProcess`)
    Cmd,
}

impl Shell {
    pub(crate) fn current() -> Self {
        if cfg!(windows) {
            Shell::Cmd
        } else {
            Shell::Posix
        }
    }
}

/// Why a command line was not made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BuildError {
    /// The template has `$F` and there is no file.
    NoFile,
    /// The file name has a character that cannot be quoted for the shell.
    Unquotable(char),
    /// The line is longer than 外部コマンド実行 keeps (1024 UTF-16 units); a cut line could end in
    /// the middle of a quote.
    TooLong,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::NoFile => f.write_str("実行するファイルがありません。"),
            BuildError::Unquotable(c) => write!(
                f,
                "ファイルの名前に、コマンドに安全に渡せない文字（{}）があります。名前を変えてから実行してください。",
                c.escape_default()
            ),
            BuildError::TooLong => write!(
                f,
                "コマンドが長すぎます（{MAX_COMMAND_UNITS} 文字まで）。ファイルの場所を短くするか、接続のコマンドを短くしてください。"
            ),
        }
    }
}

/// `path` as one word of the line for `shell`.
pub(crate) fn quote_path(shell: Shell, path: &str) -> Result<String, BuildError> {
    if let Some(c) = path.chars().find(|c| c.is_control()) {
        // a line break inside quotes is harmless to `sh`, but a name with one is never meant
        return Err(BuildError::Unquotable(c));
    }
    match shell {
        Shell::Posix => Ok(format!("'{}'", path.replace('\'', "'\\''"))),
        Shell::Cmd => {
            if let Some(c) = path.chars().find(|c| matches!(c, '"' | '%' | '!')) {
                return Err(BuildError::Unquotable(c));
            }
            Ok(format!("\"{path}\""))
        }
    }
}

/// The command line of `connection` for `file` (`None` when the text goes to the standard input):
/// the template with `$F` replaced by the quoted file and `$$` by `$`.
pub(crate) fn build(
    shell: Shell,
    connection: &Connection,
    file: Option<&str>,
) -> Result<String, BuildError> {
    let mut line = String::new();
    for part in template_parts(&connection.command) {
        match part {
            Part::Text(text) => line.push_str(&text),
            Part::File => line.push_str(&quote_path(shell, file.ok_or(BuildError::NoFile)?)?),
        }
    }
    if limit_command(&line).len() != line.len() {
        return Err(BuildError::TooLong);
    }
    Ok(line)
}

/// Doubles every `$`, so that `ExpandParameter` gives `line` back unchanged.
pub(crate) fn escape_for_expand(line: &str) -> String {
    line.replace('$', "$$")
}
