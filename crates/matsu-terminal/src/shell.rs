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
//! **Variables whose names start with `MATSU_` are not removed from the child's environment.** ADR-0028
//! point 6 wants them left out, but `alacritty_terminal` 0.26.0 cannot do that on any OS: on Unix it only
//! adds variables to the `Command` it builds (`Options::env` has no way to remove one), and on Windows it
//! builds the environment block as `Options::env` plus the parent's whole environment. `MATSU_` variables are
//! diagnostic switches (`MATSU_FONT_DIAGNOSTICS` and the like), not secrets. The PR description lists the
//! options for the user.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

/// A shell to start instead of the default: the program and its arguments.
///
/// On Windows the arguments are quoted by the C runtime's rules, so that each one reaches the program whole.
/// cmd.exe reads its command line by its own rules instead, so an argument with a `"` in it reaches cmd.exe
/// changed. How the setting hands cmd.exe its command line is decided with the setting (ADR-0028, the fifth PR).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl ShellCommand {
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
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

    #[test]
    fn default_shell_is_left_to_alacritty_off_windows() {
        assert_eq!(default_shell().is_some(), cfg!(windows));
    }
}
