//! MATSU Editor's own keyword set for CSS: the CSS-wide keywords (CSS Cascading and Inheritance,
//! "Explicit Defaulting"), `important` (of the `!important` flag) and the at-rules whose names are
//! one word (CSS Syntax and the modules that define them). Names with a hyphen (`@font-face`,
//! `@counter-style`) are not here: the coloring reads a hyphen as the end of a word. Written from
//! the specifications, not from any editor's syntax file.

/// The words.
pub const WORDS: &[&str] = &[
    "@charset",
    "@container",
    "@import",
    "@keyframes",
    "@layer",
    "@media",
    "@namespace",
    "@page",
    "@property",
    "@scope",
    "@supports",
    "important",
    "inherit",
    "initial",
    "revert",
    "unset",
];
