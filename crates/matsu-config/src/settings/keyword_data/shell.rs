//! MATSU Editor's own keyword set for shell scripts: the reserved words and the built-in utilities of
//! POSIX (Shell Command Language, "Reserved Words" and "Special Built-In Utilities"; also the
//! regular utilities that shells usually build in, such as `cd`, `echo`, `printf`, `test`) and the
//! common additions of bash (`local`, `declare`, `source`, `function`, `select`, `let`, `shopt`).
//! Written from the standard and the bash manual, not from any editor's syntax file.

/// The words.
pub const WORDS: &[&str] = &[
    "alias", "bg", "break", "case", "cd", "command", "continue", "declare", "do", "done", "echo",
    "elif", "else", "esac", "eval", "exec", "exit", "export", "false", "fc", "fg", "fi", "for",
    "function", "getopts", "if", "in", "jobs", "kill", "let", "local", "printf", "pwd", "read",
    "readonly", "return", "select", "set", "shift", "shopt", "source", "test", "then", "time",
    "times", "trap", "true", "typeset", "umask", "unalias", "unset", "until", "wait", "while",
];
