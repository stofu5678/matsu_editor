//! MATSU Editor's own keyword set for BigQuery GoogleSQL (ADR-0023): words of "Lexical structure and
//! syntax" (reserved keywords), of its data types and of the functions and statements used all the time
//! that the common SQL set (`sql.rs`) does not have. The set is not case sensitive, so the words are in
//! lower case. Written from the documentation, not from any editor's syntax file.

/// The words.
pub const WORDS: &[&str] = &[
    "any_value",
    "approx_count_distinct",
    "array_agg",
    "assert",
    "bignumeric",
    "bytes",
    "cluster",
    "countif",
    "date_trunc",
    "export",
    "float64",
    "format_date",
    "geography",
    "immediate",
    "iterate",
    "leave",
    "model",
    "options",
    "parse_date",
    "predict",
    "repeat",
    "safe_divide",
    "safe_cast",
    "string_agg",
    "timestamp_trunc",
];
