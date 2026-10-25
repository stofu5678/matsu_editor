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
    /// The template has an escape character (`\` or `^`) right before `$F`.
    EscapeBeforeFile,
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
            BuildError::EscapeBeforeFile => f.write_str(
                "接続のコマンドで、$F の直前に \\ か ^ があります。ファイルの名前を安全に渡せないので、書き直してください。",
            ),
            BuildError::TooLong => write!(
                f,
                "コマンドが長すぎます（{MAX_COMMAND_UNITS} 文字まで）。ファイルの場所を短くするか、接続のコマンドを短くしてください。"
            ),
        }
    }
}

/// Where in the quotes of the shell the template is when it reaches `$F`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Quote {
    /// Outside any quote.
    Bare,
    /// Between single quotes (`sh` only).
    Single,
    /// Between double quotes.
    Double,
}

/// Follows the quotes of `text` as the shell reads them, from the state `quote`. `escaped` is true when
/// the text ends with an escape character (`\` for `sh`, `^` for `cmd.exe`) that has not been given its
/// character yet.
fn follow(shell: Shell, text: &str, quote: &mut Quote, escaped: &mut bool) {
    for c in text.chars() {
        if std::mem::take(escaped) {
            continue;
        }
        match (shell, *quote, c) {
            // `sh`: a backslash escapes the next character outside quotes and between double quotes
            (Shell::Posix, Quote::Bare | Quote::Double, '\\') => *escaped = true,
            (Shell::Posix, Quote::Bare, '\'') => *quote = Quote::Single,
            (Shell::Posix, Quote::Single, '\'') => *quote = Quote::Bare,
            (Shell::Posix, Quote::Bare, '"') => *quote = Quote::Double,
            (Shell::Posix, Quote::Double, '"') => *quote = Quote::Bare,
            // `cmd.exe`: a caret escapes the next character outside quotes
            (Shell::Cmd, Quote::Bare, '^') => *escaped = true,
            (Shell::Cmd, Quote::Bare, '"') => *quote = Quote::Double,
            (Shell::Cmd, Quote::Double, '"') => *quote = Quote::Bare,
            _ => {}
        }
    }
}

/// The characters that `powershell.exe` and `pwsh.exe` read as code or as separators in the text of
/// `-Command`: a file name with one of them is refused when the client is PowerShell.
const POWERSHELL_SPECIAL: &[char] = &[
    '`', '$', ';', '\'', ',', '(', ')', '{', '}', '&', '|', '<', '>', '@', '#',
];

/// The name of the program of the line (its first word, without the folder and `.exe`, in lower case).
fn program_of(text: &str) -> String {
    let text = text.trim_start();
    // a program in quotes may have blanks in its path
    let word = match text.chars().next() {
        Some(q @ ('"' | '\'')) => text[1..].split(q).next().unwrap_or(""),
        _ => text.split_whitespace().next().unwrap_or(""),
    };
    let name = word.rsplit(['/', '\\']).next().unwrap_or(word);
    let name = name.to_ascii_lowercase();
    name.strip_suffix(".exe").unwrap_or(&name).to_owned()
}

/// Whether the program is PowerShell.
fn is_powershell(program: &str) -> bool {
    matches!(program, "powershell" | "pwsh")
}

/// Whether the program reads one of its arguments as another command line (`sh -c "..."`, `ssh host
/// "..."`, `cmd /c "..."`). A name put inside such an argument is read by two shells, and the quoting for the
/// first does not protect it from the second.
fn runs_a_command_line(program: &str) -> bool {
    matches!(
        program,
        "sh" | "bash"
            | "zsh"
            | "dash"
            | "ksh"
            | "fish"
            | "csh"
            | "tcsh"
            | "ash"
            | "busybox"
            | "ssh"
            | "su"
            | "sudo"
            | "doas"
            | "env"
            | "xargs"
            | "nohup"
            | "cmd"
            | "wsl"
            | "start"
    )
}

/// Whether `c` is a character that two shells read the same way: letters and digits (of any language) and
/// `. _ - / + , : = @ ~`, and the backslash of a Windows path for `cmd.exe`.
fn is_plain(shell: Shell, c: char) -> bool {
    c.is_alphanumeric()
        || matches!(c, '.' | '_' | '-' | '/' | '+' | ',' | ':' | '=' | '@' | '~')
        || (shell == Shell::Cmd && c == '\\')
}

/// `path` as it goes into the line where the quote state is `quote`:
///
/// * outside quotes: one word, between single quotes (`sh`) or double quotes (`cmd.exe`);
/// * between single quotes: the path with each `'` as `'\''` (`sh`);
/// * between double quotes: for `sh`, with `\`, `"`, `$` and the backquote escaped by a backslash (inside
///   double quotes `sh` still expands `$(...)` and the backquote); for `cmd.exe` as it is.
fn quote_in(shell: Shell, quote: Quote, path: &str) -> Result<String, BuildError> {
    if let Some(c) = path.chars().find(|c| c.is_control()) {
        // a line break inside quotes is harmless to `sh`, but a name with one is never meant
        return Err(BuildError::Unquotable(c));
    }
    match shell {
        Shell::Posix => Ok(match quote {
            Quote::Bare => format!("'{}'", path.replace('\'', "'\\''")),
            Quote::Single => path.replace('\'', "'\\''"),
            Quote::Double => {
                let mut out = String::with_capacity(path.len());
                for c in path.chars() {
                    if matches!(c, '\\' | '"' | '$' | '`') {
                        out.push('\\');
                    }
                    out.push(c);
                }
                out
            }
        }),
        Shell::Cmd => {
            if let Some(c) = path.chars().find(|c| matches!(c, '"' | '%' | '!')) {
                return Err(BuildError::Unquotable(c));
            }
            Ok(match quote {
                Quote::Double => path.to_owned(),
                Quote::Bare | Quote::Single => format!("\"{path}\""),
            })
        }
    }
}

/// `path` as one word of the line for `shell`, where the template is outside any quote.
#[cfg(test)]
pub(crate) fn quote_path(shell: Shell, path: &str) -> Result<String, BuildError> {
    quote_in(shell, Quote::Bare, path)
}

/// The command line of `connection` for `file` (`None` when the text goes to the standard input):
/// the template with `$F` replaced by the file, quoted for the place of `$F` in the quotes of the
/// shell (a template that quotes `$F` itself, `-f '$F'` or `-c "psql -f $F"`, is followed, so that
/// the file name still cannot end the quote), and `$$` by `$`.
pub(crate) fn build(
    shell: Shell,
    connection: &Connection,
    file: Option<&str>,
) -> Result<String, BuildError> {
    let parts = template_parts(&connection.command);
    let program = match parts.first() {
        Some(Part::Text(text)) => program_of(text),
        _ => String::new(),
    };
    let powershell = is_powershell(&program);
    let nested = runs_a_command_line(&program);
    let mut line = String::new();
    let mut quote = Quote::Bare;
    let mut escaped = false;
    for part in parts {
        match part {
            Part::Text(text) => {
                follow(shell, &text, &mut quote, &mut escaped);
                line.push_str(&text);
            }
            Part::File => {
                let file = file.ok_or(BuildError::NoFile)?;
                if escaped {
                    // `\$F`: the escape would take the first character of the name
                    return Err(BuildError::EscapeBeforeFile);
                }
                if powershell && let Some(c) = file.chars().find(|c| POWERSHELL_SPECIAL.contains(c))
                {
                    return Err(BuildError::Unquotable(c));
                }
                // inside the quotes of an argument that another shell reads again, only a plain name is
                // safe: the quoting for the first shell does not reach the second
                if nested
                    && quote != Quote::Bare
                    && let Some(c) = file.chars().find(|&c| !is_plain(shell, c))
                {
                    return Err(BuildError::Unquotable(c));
                }
                line.push_str(&quote_in(shell, quote, file)?);
            }
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
