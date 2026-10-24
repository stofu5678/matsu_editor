//! MATSU Editor's own keyword set for SQLite (ADR-0023): words of the keywords list of sqlite.org
//! ("SQLite Keywords") and of its functions and pragmas that the common SQL set (`sql.rs`) does not have
//! and that statements, clauses and types use all the time. The set is not case sensitive, so the words are
//! in lower case. Written from the documentation, not from any editor's syntax file.

/// The words.
pub const WORDS: &[&str] = &[
    "attach",
    "conflict",
    "detach",
    "fail",
    "filter",
    "fts5",
    "glob",
    "iif",
    "indexed",
    "instead",
    "julianday",
    "match",
    "nulls",
    "plan",
    "printf",
    "query",
    "randomblob",
    "reindex",
    "release",
    "rowid",
    "rtree",
    "strftime",
    "strict",
    "typeof",
    "virtual",
    "zeroblob",
];
