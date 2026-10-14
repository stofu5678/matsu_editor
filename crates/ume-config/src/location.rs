//! Where `sakura.ini` is.
//!
//! Sakura looks for the file like this (`sakura_core/_main/CControlProcess.cpp`, `GetIniFileName` and
//! `GetPrivateIniFileName`; `sakura_core/env/CShareData_IO.cpp`, `GetIniFileNameForIO`):
//!
//! 1. Normally it is `sakura.ini` next to `sakura.exe` (in a sub folder with the profile name, if the
//!    command line has `-PROF`).
//! 2. If `sakura.exe.ini` next to the exe has `[Settings] MultiUser=1`, it is
//!    `<root>\<UserSubFolder>\sakura.ini`, where `UserSubFolder` defaults to `sakura` and the root is
//!    `%APPDATA%` (`UserRootFolder` 0, the default), the user's folder (1), Documents (2) or the Desktop
//!    folder in the user's folder (3).
//! 3. Reading prefers the multi-user file when it exists (`GetIniFileNameForIO`).
//!
//! The list below is UME Editor's proposal. It is plain data, so it is easy to change.

use std::path::{Path, PathBuf};

/// The name of the settings file.
pub const FILE_NAME: &str = "sakura.ini";

/// The folder name UME Editor uses under the user's configuration folder on macOS and Linux.
pub const APP_DIR: &str = "ume-editor";

/// The folder name Sakura uses under `%APPDATA%`.
pub const SAKURA_DIR: &str = "sakura";

/// The operating system, as far as the location is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Linux,
}

impl Os {
    /// The OS this build runs on. Other Unix systems are treated like Linux.
    pub fn current() -> Self {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }
}

/// The folders the candidates are made from. Tests fill it by hand; [`Environment::from_process`] fills it
/// from the running process.
#[derive(Debug, Clone, Default)]
pub struct Environment {
    /// The folder of the executable.
    pub exe_dir: Option<PathBuf>,
    /// The user's home folder (`%USERPROFILE%`, `$HOME`).
    pub home: Option<PathBuf>,
    /// `%APPDATA%` (Windows).
    pub app_data: Option<PathBuf>,
    /// `$XDG_CONFIG_HOME` (Linux).
    pub xdg_config_home: Option<PathBuf>,
    /// The profile name of `-PROF`; Sakura uses a sub folder with this name.
    pub profile_name: Option<String>,
}

impl Environment {
    /// Reads the environment of the running process.
    pub fn from_process() -> Self {
        let var = |name: &str| {
            std::env::var_os(name)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let home = var(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
        Environment {
            exe_dir: std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf)),
            home,
            app_data: var("APPDATA"),
            xdg_config_home: var("XDG_CONFIG_HOME").filter(|p| p.is_absolute()),
            profile_name: None,
        }
    }
}

fn with_profile(mut dir: PathBuf, env: &Environment) -> PathBuf {
    if let Some(name) = env.profile_name.as_deref().filter(|n| !n.is_empty()) {
        dir.push(name);
    }
    dir.join(FILE_NAME)
}

/// The places to look for the settings file on `os`, best first.
///
/// - Windows: `<exe folder>\sakura.ini` (Sakura's default, so that a portable copy keeps working), then
///   `%APPDATA%\sakura\sakura.ini` (Sakura's multi-user place).
/// - macOS: `~/Library/Application Support/ume-editor/sakura.ini`.
/// - Linux: `$XDG_CONFIG_HOME/ume-editor/sakura.ini`, or `~/.config/ume-editor/sakura.ini`.
pub fn candidate_paths(os: Os, env: &Environment) -> Vec<PathBuf> {
    let mut out = Vec::new();
    match os {
        Os::Windows => {
            if let Some(dir) = &env.exe_dir {
                out.push(with_profile(dir.clone(), env));
            }
            if let Some(dir) = &env.app_data {
                out.push(with_profile(dir.join(SAKURA_DIR), env));
            }
        }
        Os::MacOs => {
            if let Some(home) = &env.home {
                let dir = home
                    .join("Library")
                    .join("Application Support")
                    .join(APP_DIR);
                out.push(with_profile(dir, env));
            }
        }
        Os::Linux => {
            let base = env
                .xdg_config_home
                .clone()
                .or_else(|| env.home.as_ref().map(|h| h.join(".config")));
            if let Some(base) = base {
                out.push(with_profile(base.join(APP_DIR), env));
            }
        }
    }
    out
}

/// The file to read: the first candidate that exists.
pub fn find_existing(candidates: &[PathBuf]) -> Option<&Path> {
    candidates
        .iter()
        .map(PathBuf::as_path)
        .find(|p| p.is_file())
}

/// The file to write: the one that exists, or else the last candidate, which is the user's own folder
/// (a file next to the executable may be in a folder the user cannot write to).
pub fn write_path(candidates: &[PathBuf]) -> Option<&Path> {
    find_existing(candidates).or_else(|| candidates.last().map(PathBuf::as_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Environment {
        Environment {
            exe_dir: Some(PathBuf::from("/app")),
            home: Some(PathBuf::from("/home/u")),
            app_data: Some(PathBuf::from("/appdata")),
            xdg_config_home: None,
            profile_name: None,
        }
    }

    #[test]
    fn windows_looks_next_to_the_exe_then_in_appdata() {
        assert_eq!(
            candidate_paths(Os::Windows, &env()),
            [
                PathBuf::from("/app/sakura.ini"),
                PathBuf::from("/appdata/sakura/sakura.ini")
            ]
        );
    }

    #[test]
    fn profile_name_is_a_sub_folder() {
        let mut e = env();
        e.profile_name = Some("work".to_owned());
        assert_eq!(
            candidate_paths(Os::Windows, &e),
            [
                PathBuf::from("/app/work/sakura.ini"),
                PathBuf::from("/appdata/sakura/work/sakura.ini")
            ]
        );
    }

    #[test]
    fn macos_and_linux_use_the_user_folder() {
        assert_eq!(
            candidate_paths(Os::MacOs, &env()),
            [PathBuf::from(
                "/home/u/Library/Application Support/ume-editor/sakura.ini"
            )]
        );
        assert_eq!(
            candidate_paths(Os::Linux, &env()),
            [PathBuf::from("/home/u/.config/ume-editor/sakura.ini")]
        );
        let mut e = env();
        e.xdg_config_home = Some(PathBuf::from("/xdg"));
        assert_eq!(
            candidate_paths(Os::Linux, &e),
            [PathBuf::from("/xdg/ume-editor/sakura.ini")]
        );
    }

    #[test]
    fn no_folders_no_candidates() {
        assert!(candidate_paths(Os::Linux, &Environment::default()).is_empty());
        assert_eq!(write_path(&[]), None);
    }

    #[test]
    fn existing_file_wins_and_new_files_go_to_the_user_folder() {
        let dir = std::env::temp_dir().join(format!("ume-config-loc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.ini");
        let b = dir.join("b.ini");
        let list = [a.clone(), b.clone()];
        assert_eq!(find_existing(&list), None);
        assert_eq!(write_path(&list), Some(b.as_path()));
        std::fs::write(&a, "").unwrap();
        assert_eq!(find_existing(&list), Some(a.as_path()));
        assert_eq!(write_path(&list), Some(a.as_path()));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
