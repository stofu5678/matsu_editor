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
    // both as written (`-p` after an empty `--opt=`) and with `key = value` tightened
    scan_for_password(kind, command).or_else(|| scan_for_password(kind, &tighten_equals(command)))
}

fn scan_for_password(kind: Kind, command: &str) -> Option<&'static str> {
    for (index, word) in command.split_whitespace().enumerate() {
        let word = word.trim_matches(['"', '\'']);
        let lower = word.to_ascii_lowercase();
        // `--password=x`, and the parameter of a connection string: `password=x` (libpq's
        // "host=h user=u password=x", ODBC's ";Pwd=x"). The key must start a word or follow a
        // separator, so `PGPASSWORD=` (below) and names such as `nopassword=` are not this.
        for key in [
            "password=",
            "passwd=",
            "pwd=",
            "pass=",
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
                    matches!(c, '-' | ';' | ',' | '"' | '\'' | '(' | '{' | '?' | '&')
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
            Kind::MySql if word.starts_with("-p") && !word.starts_with("--") && word.len() > 2 => {
                return Some("-p");
            }
            // `-P secret` and `-Psecret` (sqlcmd; its other options use a lower case p)
            Kind::SqlServer if word.starts_with("-P") => return Some("-P"),
            // `user/password@service` and `user/password` (the local database). Not a first word
            // (the program), not an option and not a script (`@file`); `/` and `/@service` have
            // no user and ask for no password.
            Kind::Oracle if index > 0 && !word.starts_with(['-', '@']) => {
                if let Some((user, rest)) = word.split_once('/')
                    && !user.is_empty()
                    && !user.contains(['@', '\\', ':', '$'])
                {
                    let password = rest.split_once('@').map_or(rest, |(password, _)| password);
                    if !password.is_empty() && !password.contains('/') {
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
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
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
