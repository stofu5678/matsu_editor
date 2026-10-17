//! UME Editor's own keyword set for YAML: the null and boolean values of the Core Schema of YAML
//! 1.2.2 (section 10.3.2). The null value `~` is not a word for the coloring, so it is not here.

/// The words.
pub const WORDS: &[&str] = &[
    "false", "False", "FALSE", "null", "Null", "NULL", "true", "True", "TRUE",
];
