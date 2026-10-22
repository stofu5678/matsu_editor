//! Starting a terminal on macOS and Linux. The command lines are built in `super`.

use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Stdio};

use super::{Shell, ShellError, Spawn, find_in_path, linux_terminals, mac_launch, pwsh_arguments};

/// `pwsh` on the `PATH`.
fn pwsh() -> Option<std::path::PathBuf> {
    find_in_path("pwsh", &std::env::var_os("PATH")?)
}

pub(super) fn open(shell: Shell, admin: bool, folder: &Path) -> Result<(), ShellError> {
    if admin {
        return Err(ShellError::Unavailable("a shell as administrator"));
    }
    if !folder.is_dir() {
        return Err(ShellError::Failed(format!(
            "{} is not a folder",
            folder.display()
        )));
    }
    let pwsh = match shell {
        Shell::CommandPrompt => None,
        Shell::PowerShell => Some(pwsh().ok_or(ShellError::Unavailable("PowerShell (pwsh)"))?),
    };
    if cfg!(target_os = "macos") {
        return start(&mac_launch(folder, pwsh.as_deref()), folder);
    }
    let inner = pwsh.map(|pwsh| {
        let mut inner = vec![pwsh.to_string_lossy().into_owned()];
        inner.extend(pwsh_arguments(&folder.to_string_lossy()));
        inner
    });
    // The first terminal that is installed.
    for terminal in linux_terminals(folder, inner.as_deref()) {
        match start(&terminal, folder) {
            Err(ShellError::Unavailable(_)) => {}
            result => return result,
        }
    }
    Err(ShellError::Failed("no terminal emulator found".to_owned()))
}

/// Starts the program in `folder` and lets it run. `Unavailable` if the program is not there.
fn start(spawn: &Spawn, folder: &Path) -> Result<(), ShellError> {
    let child = Command::new(&spawn.program)
        .args(&spawn.args)
        .current_dir(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(mut child) => {
            // Reaps the process when it ends, so that it does not stay a zombie.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            Ok(())
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            Err(ShellError::Unavailable("the program to start"))
        }
        Err(e) => Err(ShellError::Failed(format!("{}: {e}", spawn.program))),
    }
}
