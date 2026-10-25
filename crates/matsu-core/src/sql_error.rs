//! Finding the place of an error in the output of a database client (ADR-0023, step 3).
//!
//! The database functions run the command line client of the connection and put what it writes in
//! the output panel. A line of an error that names a line of the SQL can be clicked to jump there.
//! Each client writes it in its own way. The forms below are the ones the clients' documentation
//! and sources show; none was run against the real client by the author, so each is listed with
//! how sure it is:
//!
//! | style | form | line counts from | sure? |
//! |---|---|---|---|
//! | PostgreSQL (`psql -f`) | `psql:FILE:LINE: ERROR: ...` | the file | the prefix is written by `psql` when it reads a script (its `pg_log_*` calls with the script name and line); not run |
//! | MySQL, MariaDB | `ERROR 1064 (42000) at line 3: ...`, `... at line 3 in file: 'F'` | the script | the usual form of `mysql < file`; not run |
//! | SQL Server (`sqlcmd`) | `Msg 102, Level 15, State 1, Server S, Line 3` | the batch (the text since the last `GO`) | the form of the server's message; the batch start is not known here, so the line is exact only for the first batch; not run |
//! | SQLite (`sqlite3`) | `Parse error near line 3: ...`, `Runtime error near line 3: ...`, `Error: near line 3: ...` | the script | seen in `sqlite3` output; not run |
//! | BigQuery (`bq`) | `... at [3:5]` | the query text | the form of BigQuery's syntax errors; not run |
//! | Oracle SQL*Plus | `ERROR at line 3:`, the `LINE/COL` rows of `SHOW ERRORS` (`3/5 PLS-00103: ...`) | the statement or the object | SQL*Plus numbers the lines of the statement that failed, not of the file, so the jump is exact only for a script of one statement; not run |
//! | Oracle SQLcl | `Error starting at line : 3 in command -`, `Error at Command Line : 3 Column : 5` | the script | SQLcl's report; not run |
//! | anything | `FILE:LINE: message`, `FILE:LINE:COLUMN: message` | the file | the common form of compilers and many tools (the fake client of the tests writes it) |

/// The client whose way of writing an error is looked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorStyle {
    Oracle,
    MySql,
    PostgreSql,
    SqlServer,
    Sqlite,
    BigQuery,
    /// Only the common `FILE:LINE:` form.
    Generic,
}

/// Where an error is: a line (and a column) from 1, and the file when the line names one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorSpot {
    pub file: Option<String>,
    pub line: usize,
    pub column: Option<usize>,
}

fn spot(line: usize) -> Option<ErrorSpot> {
    (line > 0).then_some(ErrorSpot {
        file: None,
        line,
        column: None,
    })
}

/// The number at the start of `text` and what follows it.
fn number(text: &str) -> Option<(usize, &str)> {
    let end = text
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map_or(text.len(), |(i, _)| i);
    if end == 0 || end > 9 {
        return None;
    }
    Some((text[..end].parse().ok()?, &text[end..]))
}

/// The number that follows the first `marker` in `text`.
fn after<'a>(text: &'a str, marker: &str) -> Option<(usize, &'a str)> {
    let at = text.find(marker)?;
    number(&text[at + marker.len()..])
}

/// `FILE:LINE:` or `FILE:LINE:COLUMN:` at the start of `text`: the first colon that is followed by
/// a number and another colon ends the file name (a Windows drive letter's colon is followed by a
/// backslash or a slash, not by digits).
fn file_and_line(text: &str) -> Option<ErrorSpot> {
    for (i, c) in text.char_indices() {
        if c != ':' || i == 0 {
            continue;
        }
        let Some((line, rest)) = number(&text[i + 1..]) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let file = text[..i].trim();
        // a time (`12:30:45`) is not a file
        if file.is_empty()
            // an option, or a heading of the output panel (`#CmdLine  : ...`)
            || file.starts_with(['-', '#'])
            || line == 0
            || file.chars().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        let column = number(rest)
            .and_then(|(column, after)| (after.starts_with(':') && column > 0).then_some(column));
        return Some(ErrorSpot {
            file: Some(file.to_owned()),
            line,
            column,
        });
    }
    None
}

/// `psql:FILE:LINE: ERROR: ...`
fn postgresql(text: &str) -> Option<ErrorSpot> {
    let rest = text.strip_prefix("psql:")?;
    let mut found = file_and_line(rest)?;
    // reading the standard input has no file
    if found.file.as_deref() == Some("<stdin>") {
        found.file = None;
    }
    Some(found)
}

/// `ERROR 1064 (42000) at line 3: ...` and `... at line 3 in file: 'F'`
fn mysql(text: &str) -> Option<ErrorSpot> {
    if !text.starts_with("ERROR ") {
        return None;
    }
    let (line, rest) = after(text, " at line ")?;
    let file = rest
        .strip_prefix(" in file: '")
        .and_then(|r| r.split_once('\'').map(|(f, _)| f.to_owned()));
    let mut found = spot(line)?;
    found.file = file;
    Some(found)
}

/// `Msg 102, Level 15, State 1, Server S, Line 3`
fn sql_server(text: &str) -> Option<ErrorSpot> {
    if !text.starts_with("Msg ") {
        return None;
    }
    spot(after(text, ", Line ")?.0)
}

/// `Parse error near line 3: ...`, `Runtime error near line 3: ...`, `Error: near line 3: ...`
fn sqlite(text: &str) -> Option<ErrorSpot> {
    if !(text.contains("error") || text.contains("Error")) {
        return None;
    }
    spot(after(text, " near line ")?.0)
}

/// `... at [3:5]`
fn big_query(text: &str) -> Option<ErrorSpot> {
    let (line, rest) = after(text, " at [")?;
    let (column, after) = number(rest.strip_prefix(':')?)?;
    if !after.starts_with(']') || column == 0 {
        return None;
    }
    let mut found = spot(line)?;
    found.column = Some(column);
    Some(found)
}

/// SQL*Plus and SQLcl.
fn oracle(text: &str) -> Option<ErrorSpot> {
    let trimmed = text.trim_start();
    // SQL*Plus: `ERROR at line 3:`
    if let Some(rest) = trimmed.strip_prefix("ERROR at line ") {
        return spot(number(rest)?.0);
    }
    // SQLcl: `Error starting at line : 3 in command -` and `Error at Command Line : 3 Column : 5`
    if let Some(rest) = trimmed.strip_prefix("Error starting at line : ") {
        return spot(number(rest)?.0);
    }
    if let Some(rest) = trimmed.strip_prefix("Error at Command Line : ") {
        let (line, rest) = number(rest)?;
        let mut found = spot(line)?;
        found.column = rest
            .strip_prefix(" Column : ")
            .and_then(number)
            .map(|(column, _)| column)
            .filter(|c| *c > 0);
        return Some(found);
    }
    // `SHOW ERRORS`: `3/5     PLS-00103: Encountered ...`
    let (line, rest) = number(trimmed)?;
    let (column, rest) = number(rest.strip_prefix('/')?)?;
    let message = rest.trim_start();
    if ["PLS-", "ORA-", "PL/SQL:"]
        .iter()
        .any(|p| message.starts_with(p))
    {
        let mut found = spot(line)?;
        found.column = (column > 0).then_some(column);
        return Some(found);
    }
    None
}

/// Whether a jump may open the file `path` that a line of the output names (a security rule, ADR-0023).
///
/// The output of a client holds data that other people wrote (the rows of a table, a message), so a line
/// such as `\\\\evil\\share\\x.sql:1: boom` can be made to appear, and a click on it must not reach out to
/// another machine (opening a path on a network share sends the user's credentials to it) or open a
/// device. So a path that starts with two slashes or backslashes (a network path, `\\?\\`, `\\.\\`), has a
/// NUL, or ends in the name of a Windows device (`CON`, `NUL`, `COM1`, ..., with or without an
/// extension) is not a place. The caller also checks that the path is a regular file.
pub fn is_safe_jump_path(path: &str) -> bool {
    if path.is_empty() || path.contains('\0') {
        return false;
    }
    if path.starts_with("\\\\")
        || path.starts_with("//")
        || path.starts_with("\\/")
        || path.starts_with("/\\")
    {
        return false;
    }
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim_end()
        .to_ascii_uppercase();
    let device = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|p| {
        stem.strip_prefix(p)
            .is_some_and(|n| n.len() == 1 && n.chars().all(|c| c.is_ascii_digit() && c != '0'))
    });
    !device
}

/// The place that `text` (a line of the output) names, in the way of `style`, or in the common
/// `FILE:LINE:` form.
pub fn parse(style: ErrorStyle, text: &str) -> Option<ErrorSpot> {
    let by_style = match style {
        ErrorStyle::Oracle => oracle(text),
        ErrorStyle::MySql => mysql(text),
        ErrorStyle::PostgreSql => postgresql(text),
        ErrorStyle::SqlServer => sql_server(text),
        ErrorStyle::Sqlite => sqlite(text),
        ErrorStyle::BigQuery => big_query(text),
        ErrorStyle::Generic => None,
    };
    by_style.or_else(|| file_and_line(text))
}

#[cfg(test)]
mod tests;
