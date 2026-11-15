//! Which shell starts, and what its environment is (ADR-0028 points 3 and 6).
//!
//! The default shell (ADR-0028, option 3) is left to `alacritty_terminal` except on Windows:
//!
//! - macOS: the login shell (`$SHELL`, else the password database's) through `/usr/bin/login`, so the shell
//!   builds its own `PATH` (this is what makes a terminal started from Finder work).
//! - Linux and the other Unix systems: `$SHELL`, else the password database's.
//! - Windows: PowerShell 7 (`pwsh.exe`) when it is found on `PATH`, else Windows PowerShell.
//!
//! # The environment
//!
//! The child gets the parent's environment plus `TERM=xterm-256color` and `COLORTERM=truecolor`
//! ([`extra_env`]). MATSU's own environment is never changed: `alacritty_terminal::tty::setup_env` (which
//! calls `set_var`) is not called.
//!
//! **Variables whose names start with `MATSU_` reach the child too**, as do `ALACRITTY_WINDOW_ID` and
//! `WINDOWID`, which `alacritty_terminal` adds. It cannot leave a variable out on any OS: on Unix it only adds
//! variables to the `Command` it builds (`Options::env` has no way to remove one), and on Windows it builds
//! the environment block as `Options::env` plus the parent's whole environment. The user chose (2026-11-11)
//! to leave them there and to keep no secret in a `MATSU_` variable (ADR-0028, decision 6 as changed):
//! they are diagnostic switches (`MATSU_FONT_DIAGNOSTICS` and the like).

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

/// A shell to start instead of the default: the program and its arguments.
///
/// On Windows the arguments are quoted by the C runtime's rules, so that each one reaches the program whole.
/// cmd.exe reads its command line by its own rules instead, so an argument with a `"` in it reaches cmd.exe
/// changed. [`ShellCommand::from_command_line`] (what the setting `Shell` of `matsu.ini` uses) hands Windows the
/// text the user wrote as it is: see [`ShellCommand::raw_arguments`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellCommand {
    pub program: String,
    pub args: Vec<String>,
    /// The command line after the program, passed as it is, without quoting (Windows). `args` is then empty.
    /// Elsewhere there is no command line, only arguments, so this is one argument.
    pub raw_arguments: Option<String>,
}

/// Why a command line is not a shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandLineError {
    /// There is no program in it.
    Empty,
    /// A quote is not closed.
    UnclosedQuote,
}

impl std::fmt::Display for CommandLineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("the command line is empty"),
            Self::UnclosedQuote => f.write_str("a quote is not closed"),
        }
    }
}

impl std::error::Error for CommandLineError {}

impl ShellCommand {
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            raw_arguments: None,
        }
    }

    /// A program with the command line after it, which is passed as it is (see
    /// [`raw_arguments`](Self::raw_arguments)).
    pub fn with_raw_arguments(program: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            raw_arguments: Some(arguments.into()),
        }
    }

    /// The shell a command line of the setting names, in the form of this OS
    /// ([`from_windows_command_line`](Self::from_windows_command_line) or
    /// [`from_unix_command_line`](Self::from_unix_command_line)).
    pub fn from_command_line(line: &str) -> Result<Self, CommandLineError> {
        if cfg!(windows) {
            Self::from_windows_command_line(line)
        } else {
            Self::from_unix_command_line(line)
        }
    }

    /// The first word is the program (in double quotes when its path has a space; nothing is escaped inside
    /// them, a Windows path cannot hold a `"`), and the rest of the line is passed to it as it is. This is
    /// what Windows Terminal's `commandline` does, and what cmd.exe needs: `cmd.exe /K echo "a  b"` reaches
    /// cmd.exe with the two spaces.
    pub fn from_windows_command_line(line: &str) -> Result<Self, CommandLineError> {
        let line = line.trim();
        let (program, rest) = match line.strip_prefix('"') {
            Some(after) => {
                let end = after.find('"').ok_or(CommandLineError::UnclosedQuote)?;
                (&after[..end], &after[end + 1..])
            }
            None => match line.find([' ', '\t']) {
                Some(end) => (&line[..end], &line[end..]),
                None => (line, ""),
            },
        };
        if program.is_empty() {
            return Err(CommandLineError::Empty);
        }
        Ok(Self::with_raw_arguments(program, rest.trim_start()))
    }

    /// The words of the line, the first the program and the others its arguments. A word ends at a space or
    /// tab; `'...'` keeps everything; inside `"..."` a backslash in front of `"`, `\`, `$` or `` ` `` makes
    /// that character an ordinary one; outside quotes a backslash does so for any character. Nothing is
    /// expanded (`~`, `$HOME` and `*` are what they are): the program is started directly, without a shell in
    /// between.
    pub fn from_unix_command_line(line: &str) -> Result<Self, CommandLineError> {
        let mut words: Vec<String> = Vec::new();
        let mut word = String::new();
        let mut in_word = false;
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            match c {
                ' ' | '\t' | '\n' | '\r' => {
                    if in_word {
                        words.push(std::mem::take(&mut word));
                        in_word = false;
                    }
                }
                '\'' => {
                    in_word = true;
                    loop {
                        match chars.next() {
                            Some('\'') => break,
                            Some(c) => word.push(c),
                            None => return Err(CommandLineError::UnclosedQuote),
                        }
                    }
                }
                '"' => {
                    in_word = true;
                    loop {
                        match chars.next() {
                            Some('"') => break,
                            Some('\\') => match chars.next() {
                                Some(c @ ('"' | '\\' | '$' | '`')) => word.push(c),
                                Some(c) => {
                                    word.push('\\');
                                    word.push(c);
                                }
                                None => return Err(CommandLineError::UnclosedQuote),
                            },
                            Some(c) => word.push(c),
                            None => return Err(CommandLineError::UnclosedQuote),
                        }
                    }
                }
                '\\' => {
                    in_word = true;
                    // A backslash at the very end is a backslash.
                    word.push(chars.next().unwrap_or('\\'));
                }
                c => {
                    in_word = true;
                    word.push(c);
                }
            }
        }
        if in_word {
            words.push(word);
        }
        let mut words = words.into_iter();
        let program = words.next().filter(|p| !p.is_empty());
        let program = program.ok_or(CommandLineError::Empty)?;
        Ok(Self::new(program, words))
    }
}

/// The variables added to the parent's environment for the child.
pub fn extra_env() -> HashMap<String, String> {
    HashMap::from([
        ("TERM".to_owned(), "xterm-256color".to_owned()),
        ("COLORTERM".to_owned(), "truecolor".to_owned()),
    ])
}

/// The shell to start when none is given: `None` lets `alacritty_terminal` choose (macOS, Linux), or the
/// PowerShell found on Windows.
pub fn default_shell() -> Option<ShellCommand> {
    if cfg!(windows) {
        let path = std::env::var_os("PATH").unwrap_or_default();
        Some(windows_default_shell(&path))
    } else {
        None
    }
}

/// PowerShell 7 when `pwsh.exe` is in a directory of `path`, else Windows PowerShell. Not tied to the OS so
/// the choice can be tested everywhere.
pub fn windows_default_shell(path: &OsStr) -> ShellCommand {
    match find_on_path("pwsh.exe", path) {
        Some(pwsh) => ShellCommand::new(pwsh.to_string_lossy().into_owned(), ["-NoLogo"]),
        None => ShellCommand::new("powershell.exe", ["-NoLogo"]),
    }
}

/// The program as `alacritty_terminal` must be given it: on Windows it puts the program into the command
/// line as it is, so a path with spaces (`C:\Program Files\PowerShell\7\pwsh.exe`) must be quoted or
/// `CreateProcessW` may start `C:\Program.exe` instead. Elsewhere the program is used as it is.
pub fn program_for_launch(program: &str) -> String {
    if cfg!(windows) {
        quote_for_command_line(program)
    } else {
        program.to_owned()
    }
}

/// `program` in double quotes when it has a space or tab and is not quoted yet. A Windows path cannot
/// contain a double quote, so no escaping is needed.
fn quote_for_command_line(program: &str) -> String {
    let needs_quotes = program.contains([' ', '\t']) && !program.starts_with('"');
    if needs_quotes {
        format!("\"{program}\"")
    } else {
        program.to_owned()
    }
}

/// The first file named `name` in the directories of `path` (a `PATH` value).
pub fn find_on_path(name: &str, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_env_has_term_and_colorterm_only() {
        let env = extra_env();
        assert_eq!(env.len(), 2);
        assert_eq!(env["TERM"], "xterm-256color");
        assert_eq!(env["COLORTERM"], "truecolor");
        assert!(env.keys().all(|name| !name.starts_with("MATSU_")));
    }

    #[test]
    fn pwsh_on_path_is_preferred() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pwsh = dir.path().join("pwsh.exe");
        std::fs::write(&pwsh, b"").expect("write");
        let path =
            std::env::join_paths([PathBuf::from("/nonexistent-matsu-dir"), dir.path().into()])
                .expect("join");

        let shell = windows_default_shell(&path);
        assert_eq!(PathBuf::from(&shell.program), pwsh);
        assert_eq!(shell.args, ["-NoLogo"]);
    }

    #[test]
    fn windows_powershell_when_pwsh_is_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = std::env::join_paths([dir.path()]).expect("join");

        let shell = windows_default_shell(&path);
        assert_eq!(shell.program, "powershell.exe");
    }

    #[test]
    fn programs_with_spaces_are_quoted_for_the_command_line() {
        assert_eq!(
            quote_for_command_line(r"C:\Program Files\PowerShell\7\pwsh.exe"),
            r#""C:\Program Files\PowerShell\7\pwsh.exe""#
        );
        assert_eq!(quote_for_command_line("cmd.exe"), "cmd.exe");
        assert_eq!(
            quote_for_command_line(r#""C:\a b\x.exe""#),
            r#""C:\a b\x.exe""#
        );
        assert_eq!(quote_for_command_line("a\tb"), "\"a\tb\"");
    }

    #[test]
    fn a_directory_named_like_the_program_does_not_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("pwsh.exe")).expect("mkdir");
        let path = std::env::join_paths([dir.path()]).expect("join");
        assert_eq!(find_on_path("pwsh.exe", &path), None);
    }

    #[test]
    fn empty_path_finds_nothing() {
        assert_eq!(find_on_path("pwsh.exe", OsStr::new("")), None);
    }

    fn unix(line: &str) -> Vec<String> {
        let shell = ShellCommand::from_unix_command_line(line).expect("parse");
        assert_eq!(shell.raw_arguments, None);
        std::iter::once(shell.program).chain(shell.args).collect()
    }

    #[test]
    fn unix_words_split_at_spaces() {
        assert_eq!(unix("/bin/zsh"), ["/bin/zsh"]);
        assert_eq!(unix("  /bin/zsh   -l \t -i  "), ["/bin/zsh", "-l", "-i"]);
    }

    #[test]
    fn unix_quotes_keep_what_is_inside() {
        assert_eq!(
            unix(r#"sh -c 'echo "a  b" $HOME'"#),
            ["sh", "-c", r#"echo "a  b" $HOME"#]
        );
        assert_eq!(
            unix(r#"sh -c "echo \"a b\" \$x \\ \n""#),
            ["sh", "-c", r#"echo "a b" $x \ \n"#]
        );
        assert_eq!(unix(r"a\ b c"), ["a b", "c"]);
        assert_eq!(unix(r#"a"b c"d"#), ["ab cd"]);
        assert_eq!(unix(r#"x "" y"#), ["x", "", "y"]);
        assert_eq!(unix(r"x\"), ["x\\"]);
    }

    #[test]
    fn unix_bad_lines_are_errors() {
        for line in ["sh -c 'x", "sh -c \"x", "sh \"x\\"] {
            assert_eq!(
                ShellCommand::from_unix_command_line(line),
                Err(CommandLineError::UnclosedQuote),
                "{line:?}"
            );
        }
        for line in ["", "   ", "\"\"", "'' x"] {
            assert_eq!(
                ShellCommand::from_unix_command_line(line),
                Err(CommandLineError::Empty),
                "{line:?}"
            );
        }
    }

    #[test]
    fn windows_command_line_is_passed_as_it_is() {
        let shell = ShellCommand::from_windows_command_line(r#"cmd.exe /K echo "a  b" & echo c"#)
            .expect("parse");
        assert_eq!(shell.program, "cmd.exe");
        assert!(shell.args.is_empty());
        assert_eq!(
            shell.raw_arguments.as_deref(),
            Some(r#"/K echo "a  b" & echo c"#)
        );

        let shell =
            ShellCommand::from_windows_command_line(r#"  "C:\Program Files\x y\a.exe"   -x "q" "#)
                .expect("parse");
        assert_eq!(shell.program, r"C:\Program Files\x y\a.exe");
        assert_eq!(shell.raw_arguments.as_deref(), Some(r#"-x "q""#));

        let shell = ShellCommand::from_windows_command_line("powershell.exe").expect("parse");
        assert_eq!(shell.raw_arguments.as_deref(), Some(""));
        // The quoted program touching the arguments
        let shell = ShellCommand::from_windows_command_line(r#""a b"/c"#).expect("parse");
        assert_eq!(shell.program, "a b");
        assert_eq!(shell.raw_arguments.as_deref(), Some("/c"));
    }

    #[test]
    fn windows_bad_lines_are_errors() {
        assert_eq!(
            ShellCommand::from_windows_command_line(r#""C:\a b\x.exe -x"#),
            Err(CommandLineError::UnclosedQuote)
        );
        for line in ["", "  ", "\"\" x"] {
            assert_eq!(
                ShellCommand::from_windows_command_line(line),
                Err(CommandLineError::Empty),
                "{line:?}"
            );
        }
    }

    #[test]
    fn the_command_line_of_this_os_is_used() {
        let shell = ShellCommand::from_command_line("sh -c 'echo hi'").expect("parse");
        assert_eq!(shell.raw_arguments.is_some(), cfg!(windows));
        assert_eq!(shell.program, "sh");
    }

    #[test]
    fn default_shell_is_left_to_alacritty_off_windows() {
        assert_eq!(default_shell().is_some(), cfg!(windows));
    }
}
