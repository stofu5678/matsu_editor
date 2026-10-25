//! The connections of the database functions (ADR-0023): `databases.ini`, next to `matsu.ini`.
//!
//! Sakura has two database functions, both for Oracle (`F_PLSQL_COMPILE_ON_SQLPLUS`, `F_ACTIVATE_SQLPLUS`;
//! `sakura_core/cmd/CViewCommander_File.cpp`). MATSU runs the command line client of the database instead
//! (ADR-0023, the user's decision of 2026-10-24), and a connection says which command line that is:
//!
//! ```text
//! [Database]
//! Default=work
//!
//! [Connection.work]
//! Kind=postgresql
//! Command=psql -h db.example.com -d app -f $F
//! Encoding=utf-8
//! Folder=
//! ```
//!
//! * `Command` is a template. `$F` is the file of the document (MATSU quotes it for the OS, so it is
//!   written bare) and `$$` is a dollar sign. A template without `$F` gets the text on its standard
//!   input: the selection, or else the whole document.
//! * `Encoding` is the encoding of the output of the client (and of the input unless `InputEncoding`
//!   is given): `utf-8` (the default), `shift_jis` or `utf-16`.
//! * `Folder` is the folder to run in; the folder of the document when it is empty.
//! * A password is never written here and never accepted: [`password_in_command`] finds the usual ways
//!   to put one on a command line, and the connection is not run then. The clients keep the secrets
//!   (`~/.pgpass`, `mysql_config_editor`, Oracle Wallet, `gcloud auth`, integrated Windows
//!   authentication).
//!
//! The file is in the format of [`Profile`], so lines this program does not know are kept when it is
//! written back.

use std::io;
use std::path::{Path, PathBuf};

use crate::profile::{Profile, ProfileError};

/// The name of the file.
pub const FILE_NAME: &str = "databases.ini";

/// The section of the choices that are not about one connection.
pub const SECTION: &str = "Database";

/// The start of the name of the section of a connection.
pub const CONNECTION_PREFIX: &str = "Connection.";

/// The file that is next to the settings file `settings`.
pub fn path_beside(settings: &Path) -> PathBuf {
    settings.with_file_name(FILE_NAME)
}

/// Which database a connection is for. It only picks the example and the passwords that are looked for;
/// the command line is the user's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Oracle,
    MySql,
    PostgreSql,
    SqlServer,
    Sqlite,
    BigQuery,
    Other,
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::Oracle,
        Kind::MySql,
        Kind::PostgreSql,
        Kind::SqlServer,
        Kind::Sqlite,
        Kind::BigQuery,
        Kind::Other,
    ];

    /// The name in the file.
    pub fn ini_name(self) -> &'static str {
        match self {
            Kind::Oracle => "oracle",
            Kind::MySql => "mysql",
            Kind::PostgreSql => "postgresql",
            Kind::SqlServer => "sqlserver",
            Kind::Sqlite => "sqlite",
            Kind::BigQuery => "bigquery",
            Kind::Other => "other",
        }
    }

    /// The kind with this name in the file (case does not matter).
    pub fn from_ini_name(name: &str) -> Option<Kind> {
        let name = name.trim();
        Kind::ALL
            .into_iter()
            .find(|k| k.ini_name().eq_ignore_ascii_case(name))
    }
}

/// The encoding of what the client writes (and reads).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Encoding {
    #[default]
    Utf8,
    ShiftJis,
    Utf16,
}

impl Encoding {
    pub fn ini_name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "utf-8",
            Encoding::ShiftJis => "shift_jis",
            Encoding::Utf16 => "utf-16",
        }
    }

    pub fn from_ini_name(name: &str) -> Option<Encoding> {
        match name.trim().to_ascii_lowercase().as_str() {
            "utf-8" | "utf8" => Some(Encoding::Utf8),
            "shift_jis" | "shift-jis" | "sjis" | "windows-31j" | "cp932" => {
                Some(Encoding::ShiftJis)
            }
            "utf-16" | "utf16" | "utf-16le" => Some(Encoding::Utf16),
            _ => None,
        }
    }
}

/// One connection: a name and the command line that runs a file on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    pub name: String,
    pub kind: Kind,
    /// The template (see the module documentation).
    pub command: String,
    /// The folder to run in; `None` for the folder of the document.
    pub folder: Option<String>,
    /// The encoding of the output.
    pub encoding: Encoding,
    /// The encoding of the text sent to the standard input; the output's when `None`.
    pub input_encoding: Option<Encoding>,
}

impl Connection {
    /// Whether the template names the file (`$F`). If not, the text goes to the standard input.
    pub fn uses_file(&self) -> bool {
        template_parts(&self.command)
            .iter()
            .any(|p| matches!(p, Part::File))
    }

    pub fn input_encoding(&self) -> Encoding {
        self.input_encoding.unwrap_or(self.encoding)
    }
}

/// A piece of a template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Text(String),
    /// `$F`
    File,
}

/// Splits a template: `$F` is the file, `$$` is a dollar sign, any other `$` stays as it is.
pub fn template_parts(template: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            match chars.peek() {
                Some('F') => {
                    chars.next();
                    if !text.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut text)));
                    }
                    parts.push(Part::File);
                    continue;
                }
                Some('$') => {
                    chars.next();
                    text.push('$');
                    continue;
                }
                _ => {}
            }
        }
        text.push(c);
    }
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    parts
}

/// What a connection or the file got wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The name is empty, or has a character that cannot be in a section name.
    BadName(String),
    /// `Kind=` is not one of the names.
    UnknownKind { connection: String, value: String },
    /// `Encoding=` or `InputEncoding=` is not one of the names.
    UnknownEncoding { connection: String, value: String },
    /// The connection has no `Command=`.
    NoCommand(String),
    /// `Default=` names a connection that is not in the file.
    NoSuchDefault(String),
}

/// `databases.ini` as it is understood. Connections with a mistake are left out and listed in
/// `problems`, so that one wrong section does not stop the others.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Databases {
    pub default: Option<String>,
    pub connections: Vec<Connection>,
    pub problems: Vec<Problem>,
}

impl Databases {
    pub fn from_profile(profile: &Profile) -> Self {
        let mut out = Databases {
            default: profile
                .get(SECTION, "Default")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            ..Databases::default()
        };
        let mut seen = Vec::new();
        for section in profile.section_names() {
            let Some(name) = section.strip_prefix(CONNECTION_PREFIX) else {
                continue;
            };
            if seen.contains(&name) {
                continue; // the first section of a name counts, as for every key
            }
            seen.push(name);
            match read_connection(profile, section, name) {
                Ok(connection) => out.connections.push(connection),
                Err(problem) => out.problems.push(problem),
            }
        }
        if let Some(default) = &out.default
            && !out.connections.iter().any(|c| &c.name == default)
            && !out.problems.iter().any(|p| problem_names(p, default))
        {
            out.problems.push(Problem::NoSuchDefault(default.clone()));
        }
        out
    }

    /// Reads the file. An error when it cannot be read (`NotFound` when there is none).
    pub fn load(path: &Path) -> io::Result<Self> {
        Ok(Self::from_profile(&Profile::load(path)?))
    }

    pub fn get(&self, name: &str) -> Option<&Connection> {
        self.connections.iter().find(|c| c.name == name)
    }

    /// The connection `Default=` names.
    pub fn default_connection(&self) -> Option<&Connection> {
        self.default.as_deref().and_then(|name| self.get(name))
    }

    /// Writes the connections and the default into `profile`. Other sections and lines stay.
    pub fn write_to(&self, profile: &mut Profile) -> Result<(), ProfileError> {
        match &self.default {
            Some(name) => profile.set(SECTION, "Default", name)?,
            None => {
                profile.remove(SECTION, "Default");
            }
        }
        for c in &self.connections {
            check_name(&c.name).map_err(|_| ProfileError::InvalidSection(c.name.clone()))?;
            let section = format!("{CONNECTION_PREFIX}{}", c.name);
            profile.set(&section, "Kind", c.kind.ini_name())?;
            profile.set(&section, "Command", &c.command)?;
            profile.set(&section, "Encoding", c.encoding.ini_name())?;
            match c.input_encoding {
                Some(e) => profile.set(&section, "InputEncoding", e.ini_name())?,
                None => {
                    profile.remove(&section, "InputEncoding");
                }
            }
            profile.set(&section, "Folder", c.folder.as_deref().unwrap_or(""))?;
        }
        Ok(())
    }

    /// Saves to `path` (the other lines of the file are kept).
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut profile = match Profile::load(path) {
            Ok(profile) => profile,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Profile::new(),
            Err(e) => return Err(e),
        };
        self.write_to(&mut profile)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        profile.save(path)
    }
}

fn problem_names(problem: &Problem, name: &str) -> bool {
    match problem {
        Problem::BadName(n) | Problem::NoCommand(n) => n == name,
        Problem::UnknownKind { connection, .. } | Problem::UnknownEncoding { connection, .. } => {
            connection == name
        }
        Problem::NoSuchDefault(_) => false,
    }
}

fn check_name(name: &str) -> Result<(), Problem> {
    if name.trim().is_empty()
        || name != name.trim()
        || name.contains([']', '=', '\r', '\n', '\0'])
        || name.chars().any(char::is_control)
    {
        return Err(Problem::BadName(name.to_owned()));
    }
    Ok(())
}

fn read_connection(profile: &Profile, section: &str, name: &str) -> Result<Connection, Problem> {
    check_name(name)?;
    let get = |key: &str| profile.get(section, key).map(str::trim);
    let kind = match get("Kind").filter(|v| !v.is_empty()) {
        None => Kind::Other,
        Some(value) => Kind::from_ini_name(value).ok_or_else(|| Problem::UnknownKind {
            connection: name.to_owned(),
            value: value.to_owned(),
        })?,
    };
    let encoding_of =
        |key: &str| -> Result<Option<Encoding>, Problem> {
            match get(key).filter(|v| !v.is_empty()) {
                None => Ok(None),
                Some(value) => Encoding::from_ini_name(value).map(Some).ok_or_else(|| {
                    Problem::UnknownEncoding {
                        connection: name.to_owned(),
                        value: value.to_owned(),
                    }
                }),
            }
        };
    let encoding = encoding_of("Encoding")?.unwrap_or_default();
    let input_encoding = encoding_of("InputEncoding")?;
    let command = get("Command")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| Problem::NoCommand(name.to_owned()))?
        .to_owned();
    Ok(Connection {
        name: name.to_owned(),
        kind,
        command,
        folder: get("Folder").filter(|v| !v.is_empty()).map(str::to_owned),
        encoding,
        input_encoding,
    })
}

/// The first way of putting a password on the command line `command` that is found, named for the
/// message ("-p", "--password=", ...), or `None`. MATSU does not run such a command: the password
/// would be in the list of processes and in the history of commands.
///
/// This looks for the usual forms of the clients of ADR-0023; it cannot know every client.
pub fn password_in_command(kind: Kind, command: &str) -> Option<&'static str> {
    // a shell removes a backslash before a letter (`--passw\ord=x` is `--password=x`)
    let unescaped = remove_backslashes(command);
    // a connection string is read with `%XX` decoded (`?pass%77ord=x` is `?password=x`)
    let decoded = percent_decode(command);
    // Each command of the line (`a && b`, `a | b`) is looked at by itself, with the rules of its own
    // client: the client the command names says more than the `Kind=` of the file (`mysql -psecret`
    // is MySQL's when the connection says `other`, also as `env mysql -psecret`), and the rules of one
    // client must not be applied to the command of another (`psql -p5432 && mysql ...`: psql's `-p` is a
    // port).
    [command, unescaped.as_str(), decoded.as_str()]
        .into_iter()
        .flat_map(|text| {
            // the commands by themselves, and the whole line for the rules that every client shares
            // (`password=`, a URI with a password): a quote that the cutting does not understand cannot
            // hide those
            let mut parts: Vec<(Kind, &str, bool)> = Vec::new();
            for segment in split_commands(text) {
                let kinds = program_kinds(segment);
                let glued = glued_to_next(text, segment);
                if kinds.is_empty() {
                    parts.push((kind, segment, glued));
                }
                parts.extend(kinds.into_iter().map(|k| (k, segment, glued)));
            }
            parts.push((Kind::Other, text, false));
            parts
        })
        .find_map(|(k, segment, glued)| {
            // `mysql -p(secret)`: a program that is run directly (Windows) gets `(secret)` as the rest
            // of the argument, but the cutting leaves `-p` at the end of the command and `secret` in
            // the next one
            if glued && ends_with_open_password(k, segment) {
                return Some(if k == Kind::SqlServer { "-P" } else { "-p" });
            }
            password_in_text(k, segment)
        })
}

/// Whether a cut character follows `segment` in `text` (`segment` is a slice of `text`), at once: the command
/// ends with something and then a `;` `|` `(` `)` `&`, as in `-p(secret)`, `-p&&secret` or `-p;`.
fn glued_to_next(text: &str, segment: &str) -> bool {
    let start = segment.as_ptr() as usize - text.as_ptr() as usize;
    text[start + segment.len()..].starts_with([';', '|', '(', ')', '&'])
}

/// Whether the command `segment` ends with the option that takes the password and has no value after it:
/// `-p` of MySQL, `-P` of sqlcmd, as a word of its own.
fn ends_with_open_password(kind: Kind, segment: &str) -> bool {
    let option = match kind {
        Kind::MySql => "-p",
        Kind::SqlServer => "-P",
        _ => return false,
    };
    segment
        .strip_suffix(option)
        .is_some_and(|before| before.is_empty() || before.ends_with(|c: char| c.is_whitespace()))
}

/// The commands of a line: it is cut at `;`, `|` and parentheses, and at `&` when it is `&&` or has blanks
/// around it (`a & b`), outside quotes. A `&` inside a word is a character of that word
/// (`?sslmode=require&password=x` of a connection string, `password=&secret`), and so is a separator
/// inside quotes (`--password=';secret'`).
fn split_commands(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, &(at, c)) in chars.iter().enumerate() {
        if std::mem::take(&mut escaped) {
            continue;
        }
        match (quote, c) {
            (None, '\\') | (Some('"'), '\\') => {
                escaped = true;
                continue;
            }
            (None, '"' | '\'') => {
                quote = Some(c);
                continue;
            }
            (Some(q), _) if q == c => {
                quote = None;
                continue;
            }
            (Some(_), _) => continue,
            (None, _) => {}
        }
        let cut = match c {
            ';' | '|' | '(' | ')' => true,
            '&' => {
                let before = i.checked_sub(1).map(|j| chars[j].1);
                let after = chars.get(i + 1).map(|&(_, c)| c);
                before.is_none_or(|c| c.is_whitespace() || c == '&')
                    || after.is_none_or(|c| c.is_whitespace() || c == '&')
            }
            _ => false,
        };
        if cut {
            parts.push(&text[start..at]);
            start = at + c.len_utf8();
        }
    }
    parts.push(&text[start..]);
    parts
}

/// The kinds of the clients that the command `segment` runs: its program (the first word), not any word, so
/// that a host called `mysql` in `psql -h mysql` is not taken for the client. After `env`, `sudo` and the like
/// the program is somewhere after their options and operands (`sudo -u me mysql`), and a word of those can be
/// a name of a client too (`sudo -u mysql sqlcmd`), so every client in the segment is a kind then. A client that has
/// no password rules of its own (`psql`) gives `Other`, so that the connection's `Kind=` is not applied to
/// it. Empty for a program that is not known.
fn program_kinds(segment: &str) -> Vec<Kind> {
    const WRAPPERS: &[&str] = &[
        "env", "sudo", "doas", "nohup", "time", "exec", "command", "xargs",
    ];
    let words = segment
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '\''))
        .filter(|w| !w.is_empty());
    let mut kinds = Vec::new();
    let mut wrapped = false;
    for word in words {
        let word = word.to_ascii_lowercase();
        let name = word.rsplit(['/', '\\']).next().unwrap_or("");
        let name = name.strip_suffix(".exe").unwrap_or(name);
        if WRAPPERS.contains(&name) {
            wrapped = true;
            continue;
        }
        let known = match name {
            "mysql" | "mariadb" | "mysqlsh" => Some(Kind::MySql),
            "sqlcmd" => Some(Kind::SqlServer),
            "sqlplus" | "sql" | "sqlcl" | "sqlldr" => Some(Kind::Oracle),
            "psql" | "sqlite3" | "bq" | "duckdb" | "usql" | "clickhouse-client" | "snowsql" => {
                Some(Kind::Other)
            }
            _ => None,
        };
        match known {
            Some(kind) => {
                kinds.push(kind);
                if !wrapped {
                    break;
                }
            }
            None if !wrapped => break,
            None => {}
        }
    }
    kinds
}

/// `text` with each `%XX` (two hexadecimal digits that make an ASCII character) replaced by that character,
/// except the characters that a URI is made of (`: @ / ? & = # %`): `alice%3Aadmin@host` is a user called
/// `alice:admin`, not a user and a password.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = text.get(i + 1..i + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
            && byte.is_ascii()
            && !matches!(byte, b':' | b'@' | b'/' | b'?' | b'&' | b'=' | b'#' | b'%')
        {
            out.push(char::from(byte));
            i += 3;
        } else {
            let c = text[i..].chars().next().unwrap_or('\u{fffd}');
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

/// `text` without the backslashes that a shell would remove (a backslash and the character after it become
/// that character). A Windows path loses its separators too, which the scan does not need.
fn remove_backslashes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `-psecret`, and the same after short options that take no value (`-vpsecret` is `-v -psecret`): MySQL
/// reads a word of short options one letter at a time. A bare `-p` asks for the password and is fine.
fn mysql_cluster_password(word: &str) -> bool {
    const FLAGS: &str = "vsBNtEfwCAGHqrnTUVWZ";
    let Some(rest) = word.strip_prefix('-').filter(|r| !r.starts_with('-')) else {
        return false;
    };
    let after_flags = rest.trim_start_matches(|c| FLAGS.contains(c));
    after_flags.starts_with('p') && after_flags.len() > 1
}

fn password_in_text(kind: Kind, command: &str) -> Option<&'static str> {
    // both as written (`-p` after an empty `--opt=`) and with `key = value` tightened; and with
    // the words read by quotes and by blanks alone (a shell's escapes, `\"`, can make a quote
    // mean nothing, so what the quotes hide is looked at again)
    let tight = tighten_equals(command);
    [command, tight.as_str()].into_iter().find_map(|text| {
        scan_for_password(kind, text, true).or_else(|| scan_for_password(kind, text, false))
    })
}

/// The words of a command line, as a shell reads them: blanks separate, a quote (`'` or `"`) keeps
/// the blanks inside it in the word and is not part of it (so `"/opt/Oracle Client/sqlplus"` is
/// one word). The flag says that a quote was in the word. A line with a quote that never closes
/// is cut at the blanks, so that nothing after the quote hides from the scan.
fn split_words(command: &str) -> Vec<(String, bool)> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut quoted = false;
    let mut open: Option<char> = None;
    for c in command.chars() {
        match open {
            Some(q) if c == q => open = None,
            Some(_) => word.push(c),
            None if c == '"' || c == '\'' => {
                open = Some(c);
                in_word = true;
                quoted = true;
            }
            None if c.is_whitespace() => {
                if in_word {
                    words.push((std::mem::take(&mut word), quoted));
                    in_word = false;
                    quoted = false;
                }
            }
            None => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if open.is_some() {
        return command
            .split_whitespace()
            .map(|w| {
                (
                    w.trim_matches(['"', '\'']).to_owned(),
                    w.starts_with(['"', '\'']),
                )
            })
            .collect();
    }
    if in_word {
        words.push((word, quoted));
    }
    words
}

/// `quote_aware`: the words are read with their quotes ([`split_words`]); else by blanks alone, and
/// the Oracle login is not looked for (a quoted path with blanks would be taken for one).
fn scan_for_password(kind: Kind, command: &str, quote_aware: bool) -> Option<&'static str> {
    let words = if quote_aware {
        split_words(command)
    } else {
        command
            .split_whitespace()
            .map(|w| {
                (
                    w.trim_matches(['"', '\'']).to_owned(),
                    w.starts_with(['"', '\'']),
                )
            })
            .collect()
    };
    for (index, (word, quoted)) in words.into_iter().enumerate() {
        let word = word.as_str();
        let lower = word.to_ascii_lowercase();
        // `--password=x`, and the parameter of a connection string: `password=x` (libpq's
        // "host=h user=u password=x", ODBC's ";Pwd=x"). The key must start a word or follow a
        // separator, so `PGPASSWORD=` (below) and names such as `nopassword=` are not this.
        for key in [
            "password=",
            "passwd=",
            "pwd=",
            "pass=",
            "password1=",
            "password2=",
            "password3=",
            "sslpassword=",
            "passphrase=",
        ] {
            let mut from = 0;
            while let Some(found) = lower[from..].find(key) {
                let at = from + found;
                from = at + key.len();
                let before = lower[..at].chars().next_back();
                let value = lower[from..].chars().next();
                if before.is_none_or(|c| {
                    c.is_whitespace()
                        || matches!(c, '-' | ';' | ',' | '"' | '\'' | '(' | '{' | '?' | '&')
                }) && value.is_some_and(|c| {
                    // `&` ends an empty value only in the query of a URI
                    c != ';' && !(c == '&' && matches!(before, Some('?' | '&')))
                }) {
                    return Some(if before == Some('-') {
                        "--password="
                    } else {
                        "password="
                    });
                }
            }
        }
        for var in [
            "pgpassword=",
            "mysql_pwd=",
            "sqlcmdpassword=",
            "oracle_pwd=",
        ] {
            if lower.starts_with(var) && word.len() > var.len() {
                return Some("PGPASSWORD=");
            }
        }
        // `scheme://user:password@host`
        if let Some((_, rest)) = word.split_once("://")
            && let Some((userinfo, _)) = rest.split_once('@')
            && userinfo.contains(':')
        {
            return Some("://user:password@");
        }
        match kind {
            // `-psecret` (a bare `-p` asks for it; psql's `-p5432` is a port, so only MySQL)
            Kind::MySql if mysql_cluster_password(word) => return Some("-p"),
            // `-P secret` and `-Psecret` (sqlcmd; its other options use a lower case p)
            Kind::SqlServer if word.starts_with("-P") => return Some("-P"),
            // `user/password@service` and `user/password` (the local database). Not a first word
            // (the program), not an option and not a script (`@file`); `/` and `/@service` have
            // no user and ask for no password.
            Kind::Oracle if quote_aware && index > 0 && !word.starts_with(['-', '@']) => {
                if let Some((user, rest)) = word.split_once('/')
                    && !user.is_empty()
                    && !user.contains(['@', '\\', ':'])
                {
                    let password = rest.split_once('@').map_or(rest, |(password, _)| password);
                    // a quoted word is a login, whatever it holds; a bare one with a `/` after it is a path
                    if !password.is_empty() && (quoted || !password.contains('/')) {
                        return Some("user/password");
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// `command` with the blanks around each `=` taken out (libpq allows `password = secret`), so that
/// the words that follow are `key=value`.
fn tighten_equals(command: &str) -> String {
    let mut out = String::with_capacity(command.len());
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            let mut blanks = String::from(c);
            while let Some(&next) = chars.peek()
                && next.is_whitespace()
            {
                blanks.push(next);
                chars.next();
            }
            if chars.peek() == Some(&'=') || out.ends_with('=') {
                continue;
            }
            out.push_str(&blanks);
        } else {
            out.push(c);
        }
    }
    out
}

/// The connections the example file has, one for each database. None of them is tried against the real
/// client (ADR-0023: an example is not a claim that it works); the names after `-h`, `-d` and so on are
/// placeholders.
pub fn examples() -> Vec<Connection> {
    let make = |name: &str, kind, command: &str| Connection {
        name: name.to_owned(),
        kind,
        command: command.to_owned(),
        folder: None,
        encoding: Encoding::Utf8,
        input_encoding: None,
    };
    vec![
        make("oracle", Kind::Oracle, "sqlplus -L /@MYDB @$F"),
        make(
            "mysql",
            Kind::MySql,
            "mysql --login-path=mylogin --database=mydb",
        ),
        make(
            "postgresql",
            Kind::PostgreSql,
            "psql -h localhost -d mydb -v ON_ERROR_STOP=1 -f $F",
        ),
        make(
            "sqlserver",
            Kind::SqlServer,
            "sqlcmd -S localhost -d mydb -i $F",
        ),
        make("sqlite", Kind::Sqlite, "sqlite3 mydb.db"),
        make(
            "bigquery",
            Kind::BigQuery,
            "bq query --use_legacy_sql=false",
        ),
    ]
}

/// The text of the file that is made for a user who has none: the examples, and no `Default=`.
pub fn example_file_bytes() -> Vec<u8> {
    let mut profile = Profile::from_bytes(
        "\u{feff}; MATSU Editor: connections of the database functions (ADR-0023)\n\
         ; Set Default= to the name of a connection below. These are examples that were not\n\
         ; tried against the real clients: change the names, hosts and databases.\n\
         ; $F is the file of the document; without $F the text goes to the standard input.\n\
         ; Never write a password here: use the client's own way (~/.pgpass, mysql_config_editor,\n\
         ; Oracle Wallet, gcloud auth, integrated Windows authentication).\n\
         \n\
         [Database]\n\
         Default=\n"
            .as_bytes(),
    );
    let examples = Databases {
        default: None,
        connections: examples(),
        problems: Vec::new(),
    };
    examples
        .write_to(&mut profile)
        .expect("the example names are valid");
    profile.to_bytes()
}

/// Makes the example file at `path` when there is none. `Ok(false)` when a file is there already (it is
/// never overwritten).
pub fn create_example_file(path: &Path) -> io::Result<bool> {
    use std::io::Write;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut options = std::fs::OpenOptions::new();
    // `create_new` does not follow a link that is already there; the file names hosts and users, so
    // it is private to the user (like the settings file's temporary copy, `Profile::save`)
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(&example_file_bytes())?;
            file.sync_all()?;
            Ok(true)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests;
