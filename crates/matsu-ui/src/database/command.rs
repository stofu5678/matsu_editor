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
//! The review of ADR-0023 (step 6) tried payloads on this, and the rules below are what was left:
//!
//! * The quotes of the template are followed up to `$F`. Written bare, the name is one word, quoted as
//!   above. Written inside a quote of the template (`-f '$F'`, `-c "\i $F"`), the name is an argument that
//!   the client or another shell reads itself (`psql -c '\i $F'` runs a backquote), and no escaping is known
//!   to be right for that, so only a plain name goes in (letters, digits and `. _ - / + , : = @ ~`, and `\`
//!   for `cmd.exe`), as it is.
//! * Only a known client (`psql`, `mysql`, `sqlcmd`, ...; `is_database_client`) takes a name that is not
//!   plain, and only when the template is one command with its arguments (no `;`, `&`, `|`, `<`, `>`,
//!   backquote, `$(`). Any other program may read the name as code (`sh -c`, `eval`, `pwsh -Command`, a
//!   script of the user's): the quoting for the first shell does not reach the second, so such a template
//!   gets a plain name only.
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
                "ファイルの名前に、コマンドに安全に渡せない文字（{}）があります。名前を変えてから実行してください。接続のコマンドが psql や mysql などのクライアントで始まる 1 つのコマンドなら、空白などの入った名前も使えます。",
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

/// The command line clients of the databases of ADR-0023 (and a few like them). Only these are known to take
/// a file name as a file name: any other program may read an argument as code (`sh -c`, `eval`, `env pwsh
/// -Command`, `python -c`, a script of the user's), and then the quoting for the first shell is not enough.
fn is_database_client(program: &str) -> bool {
    matches!(
        program,
        "sqlplus"
            | "sql"
            | "sqlcl"
            | "mysql"
            | "mariadb"
            | "mysqlsh"
            | "psql"
            | "sqlcmd"
            | "sqlite3"
            | "bq"
            | "duckdb"
            | "usql"
            | "clickhouse-client"
            | "snowsql"
    )
}

/// Characters of the template (outside the name) with which it is more than one command or reads a
/// substitution, so that it is not just a client with its arguments.
const COMPOUND: &[char] = &[';', '&', '|', '(', ')', '<', '>', '`', '\n', '\r'];

/// Whether `c` is a character that two shells read the same way: letters and digits (of any language) and
/// `. _ - / + , : = @ ~`, and the backslash of a Windows path for `cmd.exe`.
fn is_plain(shell: Shell, c: char) -> bool {
    c.is_alphanumeric()
        || matches!(c, '.' | '_' | '-' | '/' | '+' | ',' | ':' | '=' | '@' | '~')
        || (shell == Shell::Cmd && c == '\\')
}

/// `path` as it goes into the line where the quote state is `quote`:
///
/// * outside quotes: one word, between single quotes (`sh`) or double quotes (`cmd.exe`), with each `'` of
///   the name as `'\''` for `sh`;
/// * inside a quote of the template: as it is. [`build`] lets only a plain name get here (the client or the
///   other shell reads that argument itself, and no escaping is known to be right for it).
fn quote_in(shell: Shell, quote: Quote, path: &str) -> Result<String, BuildError> {
    if let Some(c) = path.chars().find(|c| c.is_control()) {
        // a line break inside quotes is harmless to `sh`, but a name with one is never meant
        return Err(BuildError::Unquotable(c));
    }
    match (shell, quote) {
        (Shell::Posix, Quote::Bare) => Ok(format!("'{}'", path.replace('\'', "'\\''"))),
        (Shell::Cmd, Quote::Bare) => {
            if let Some(c) = path.chars().find(|c| matches!(c, '"' | '%' | '!')) {
                return Err(BuildError::Unquotable(c));
            }
            Ok(format!("\"{path}\""))
        }
        // the name was checked to be plain
        (_, Quote::Single | Quote::Double) => Ok(path.to_owned()),
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
    // A name goes in with the full quoting only when the template is a known client with its arguments. Any
    // other program, or a template that has more than one command, may have the name read again as code (a
    // shell's `-c`, `eval`, `env pwsh -Command`, `$(...)`), and the quoting for the first shell does not
    // reach the second: then only a plain name is safe.
    let text: String = parts
        .iter()
        .filter_map(|p| match p {
            Part::Text(text) => Some(text.as_str()),
            Part::File => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let client = is_database_client(&program_of(&text))
        && !text.contains(COMPOUND)
        && !text.contains("$(")
        && !text.contains("${");
    // `psql -c '\\i $F'`: inside a quote of the template the name is part of an argument that the client
    // reads itself (a backquote runs a command in psql), whatever the shell does with it
    let strict = !client;
    let mut line = String::new();
    let mut quote = Quote::Bare;
    let mut escaped = false;
    for (index, part) in parts.iter().enumerate() {
        match part {
            Part::Text(text) => {
                follow(shell, text, &mut quote, &mut escaped);
                line.push_str(text);
            }
            Part::File => {
                let file = file.ok_or(BuildError::NoFile)?;
                if escaped {
                    // `\$F`: the escape would take the first character of the name
                    return Err(BuildError::EscapeBeforeFile);
                }
                // read by more than one shell, only a plain name is safe: the quoting for the first
                // shell does not reach the second
                // `psql -c "\\i '"$F"'"`: a name that touches a quote of the template is spliced into
                // an argument that the client reads itself, as if it were inside the quote
                let touches_quote = line.ends_with(['"', '\''])
                    || matches!(parts.get(index + 1), Some(Part::Text(next)) if next.starts_with(['"', '\'']));
                if (strict || touches_quote || quote != Quote::Bare)
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
