//! Opening a command prompt or PowerShell in a folder (`F_OPEN_COMMAND_PROMPT`,
//! `F_OPEN_COMMAND_PROMPT_AS_ADMIN`, `F_OPEN_POWERSHELL`, `F_OPEN_POWERSHELL_AS_ADMIN`).
//!
//! Sakura: `Command_OPEN_COMMAND_PROMPT` and `Command_OPEN_POWERSHELL` in
//! `sakura_core/cmd/CViewCommander_File.cpp`. Both start the shell in the folder of the file with
//! `ShellExecute`; the "as administrator" variants use the verb `runas` instead of `open`.
//!
//! * Windows does what Sakura does ([`windows_launch`]).
//! * macOS and Linux have no such shells. The command prompt is the default terminal in the
//!   folder; PowerShell is `pwsh` in a terminal in the folder, if `pwsh` is installed; the
//!   administrator variants are unavailable (there is no `runas`, and a `sudo` terminal would
//!   need a password prompt that this program cannot own).
//!
//! The command lines and the paths are built by plain functions that every OS tests; only the
//! last call (`ShellExecuteW` on Windows, starting a process elsewhere) is not.

#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
mod unix;

use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

/// Which shell to open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    /// `cmd.exe` on Windows; the default terminal elsewhere.
    CommandPrompt,
    /// `powershell.exe` on Windows; `pwsh` in a terminal elsewhere.
    PowerShell,
}

/// Why no shell was opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellError {
    /// This OS (or this machine) cannot do it. The text says what.
    Unavailable(&'static str),
    /// The OS refused, or the program could not be started.
    Failed(String),
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(what) => write!(f, "{what} is not available"),
            Self::Failed(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for ShellError {}

/// Opens `shell` in `folder`; `admin` asks for an elevated one (Windows only). Returns when the
/// OS has started it, without waiting for it to end.
pub fn open_shell(shell: Shell, admin: bool, folder: &Path) -> Result<(), ShellError> {
    #[cfg(windows)]
    {
        let comspec = std::env::var_os("COMSPEC").map(PathBuf::from);
        windows::run(&windows_launch(shell, admin, folder, comspec.as_deref())?)
    }
    #[cfg(not(windows))]
    {
        unix::open(shell, admin, folder)
    }
}

// ---- Windows ----

/// What `ShellExecuteW` is called with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsLaunch {
    /// `open`, or `runas` to elevate.
    pub verb: &'static str,
    pub file: String,
    /// `None` for an interactive `cmd.exe`: Sakura passes no parameters.
    pub parameters: Option<String>,
    pub directory: String,
}

/// Whether `path` is a UNC path (`\\server\share`). `cmd.exe` cannot start in one.
/// Sakura: `PathIsUNCW`. `\\?\C:\...` is a drive path, `\\?\UNC\...` is a UNC path.
pub fn is_unc_path(path: &str) -> bool {
    if let Some(rest) = path.strip_prefix(r"\\?\") {
        return rest
            .get(..4)
            .is_some_and(|head| head.eq_ignore_ascii_case(r"UNC\"));
    }
    path.starts_with(r"\\")
}

/// Escapes `text` for a single-quoted PowerShell string: `'` becomes `''`. Sakura does this
/// against command injection through the folder name.
pub fn powershell_quote(text: &str) -> String {
    text.replace('\'', "''")
}

/// The parameters of `powershell.exe` that open it in `folder`: `-NoExit` keeps it open after
/// `Set-Location`. Sakura builds exactly this string.
pub fn powershell_parameters(folder: &str) -> String {
    format!(
        "-NoExit -Command \"Set-Location -Path '{}'\"",
        powershell_quote(folder)
    )
}

/// The `ShellExecute` call for `shell` in `folder`, as Sakura makes it. `comspec` is the
/// `COMSPEC` environment variable (the path of `cmd.exe`).
pub fn windows_launch(
    shell: Shell,
    admin: bool,
    folder: &Path,
    comspec: Option<&Path>,
) -> Result<WindowsLaunch, ShellError> {
    let verb = if admin { "runas" } else { "open" };
    let directory = folder.to_string_lossy().into_owned();
    match shell {
        Shell::CommandPrompt => {
            if is_unc_path(&directory) {
                return Err(ShellError::Unavailable(
                    "the command prompt in a network folder",
                ));
            }
            let comspec = comspec.ok_or_else(|| {
                ShellError::Failed("the environment variable COMSPEC is not set".to_owned())
            })?;
            Ok(WindowsLaunch {
                verb,
                file: comspec.to_string_lossy().into_owned(),
                parameters: None,
                directory,
            })
        }
        Shell::PowerShell => Ok(WindowsLaunch {
            verb,
            file: "powershell.exe".to_owned(),
            parameters: Some(powershell_parameters(&directory)),
            directory,
        }),
    }
}

// ---- macOS and Linux ----

/// A program to start with its arguments. It is started with the folder as its working directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spawn {
    pub program: String,
    pub args: Vec<String>,
}

impl Spawn {
    fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
        }
    }
}

/// Quotes `text` for a POSIX shell: in single quotes, with `'` as `'\''`.
pub fn sh_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Quotes `text` for an AppleScript string literal.
pub fn applescript_quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', r"\\").replace('"', r#"\""#))
}

/// The arguments of `pwsh` that open it in `folder`.
pub fn pwsh_arguments(folder: &str) -> Vec<String> {
    vec![
        "-NoExit".to_owned(),
        "-Command".to_owned(),
        format!("Set-Location -LiteralPath '{}'", powershell_quote(folder)),
    ]
}

/// macOS: Terminal in `folder`. With `pwsh` (its path) the shell is PowerShell: an AppleScript
/// opens a Terminal window that changes to the folder and runs it.
pub fn mac_launch(folder: &Path, pwsh: Option<&Path>) -> Spawn {
    let folder = folder.to_string_lossy();
    let Some(pwsh) = pwsh else {
        return Spawn::new("open", &["-a", "Terminal", &folder]);
    };
    let mut command = format!(
        "cd {} && exec {}",
        sh_quote(&folder),
        sh_quote(&pwsh.to_string_lossy())
    );
    for arg in pwsh_arguments(&folder) {
        command.push(' ');
        command.push_str(&sh_quote(&arg));
    }
    let script = format!(
        "tell application \"Terminal\"\nactivate\ndo script {}\nend tell",
        applescript_quote(&command)
    );
    Spawn::new("osascript", &["-e", &script])
}

/// Linux: the terminal emulators to try, in order, each for `folder`. With `inner` (a program and
/// its arguments) the terminal runs it instead of the user's shell.
///
/// The list starts with `x-terminal-emulator`, the Debian alternative that the user chose, then
/// the emulators of the large desktops, then `xterm`, which is nearly always there. The caller
/// also sets the working directory of the process, so an emulator that ignores its own option
/// still starts in the folder.
pub fn linux_terminals(folder: &Path, inner: Option<&[String]>) -> Vec<Spawn> {
    let folder = folder.to_string_lossy().into_owned();
    // (program, options that set the folder, the option that precedes the command)
    let table: [(&str, Vec<String>, &str); 5] = [
        ("x-terminal-emulator", vec![], "-e"),
        (
            "gnome-terminal",
            vec![format!("--working-directory={folder}")],
            "--",
        ),
        (
            "konsole",
            vec!["--workdir".to_owned(), folder.clone()],
            "-e",
        ),
        (
            "xfce4-terminal",
            vec![format!("--working-directory={folder}")],
            "-x",
        ),
        ("xterm", vec![], "-e"),
    ];
    table
        .into_iter()
        .map(|(program, mut args, run)| {
            if let Some(inner) = inner {
                args.push(run.to_owned());
                args.extend(inner.iter().cloned());
            }
            Spawn {
                program: program.to_owned(),
                args,
            }
        })
        .collect()
}

/// The first directory of `path_var` (a `PATH`) that has an executable file called `name`.
pub fn find_in_path(name: &str, path_var: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path_var)
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests;
