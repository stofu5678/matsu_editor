//! The folders of the home directory on macOS and Linux.

use std::path::{Path, PathBuf};

/// The XDG name of a folder (`user-dirs.dirs`) and the name in the home directory, for each name of
/// Sakura's registry key.
fn folder_of(name: &str) -> Option<(&'static str, &'static str)> {
    const MOVIES: &str = if cfg!(target_os = "macos") {
        "Movies"
    } else {
        "Videos"
    };
    let table = [
        ("Personal", "DOCUMENTS", "Documents"),
        ("Desktop", "DESKTOP", "Desktop"),
        ("My Pictures", "PICTURES", "Pictures"),
        ("My Music", "MUSIC", "Music"),
        ("Video", "VIDEOS", MOVIES),
    ];
    table
        .iter()
        .find(|(key, ..)| key.eq_ignore_ascii_case(name))
        .map(|&(_, xdg, home)| (xdg, home))
}

pub(super) fn known_folder(name: &str) -> Option<PathBuf> {
    let (xdg, in_home) = folder_of(name)?;
    let home = PathBuf::from(std::env::var_os("HOME").filter(|home| !home.is_empty())?);
    if cfg!(target_os = "macos") {
        return Some(home.join(in_home));
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map_or_else(|| home.join(".config"), PathBuf::from);
    let listed = std::fs::read_to_string(config.join("user-dirs.dirs"))
        .ok()
        .and_then(|text| user_dir(&text, xdg, &home));
    Some(listed.unwrap_or_else(|| home.join(in_home)))
}

/// `XDG_<name>_DIR="$HOME/..."` of a `user-dirs.dirs` file. Only values under `$HOME` or an absolute
/// path are taken (the file is a shell script; nothing else is run).
fn user_dir(text: &str, name: &str, home: &Path) -> Option<PathBuf> {
    let key = format!("XDG_{name}_DIR");
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix(&key)?.trim_start();
        let value = value.strip_prefix('=')?.trim();
        let value = value.strip_prefix('"')?.strip_suffix('"')?;
        if let Some(rest) = value.strip_prefix("$HOME") {
            let rest = rest.trim_start_matches('/');
            Some(if rest.is_empty() {
                home.to_owned()
            } else {
                home.join(rest)
            })
        } else {
            value.starts_with('/').then(|| PathBuf::from(value))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRS: &str = "# comment\nXDG_DESKTOP_DIR=\"$HOME/デスクトップ\"\nXDG_DOCUMENTS_DIR=\"$HOME/文書\"\nXDG_MUSIC_DIR=\"/mnt/music\"\nXDG_VIDEOS_DIR=\"$HOME/\"\n";

    #[test]
    fn user_dirs_under_home_or_absolute_are_read() {
        let home = Path::new("/home/a");
        assert_eq!(
            user_dir(DIRS, "DOCUMENTS", home),
            Some(PathBuf::from("/home/a/文書"))
        );
        assert_eq!(
            user_dir(DIRS, "DESKTOP", home),
            Some(PathBuf::from("/home/a/デスクトップ"))
        );
        assert_eq!(
            user_dir(DIRS, "MUSIC", home),
            Some(PathBuf::from("/mnt/music"))
        );
        assert_eq!(user_dir(DIRS, "VIDEOS", home), Some(home.to_owned()));
        assert_eq!(user_dir(DIRS, "PICTURES", home), None);
    }

    #[test]
    fn a_name_is_matched_without_case_and_others_do_not_exist() {
        assert_eq!(folder_of("personal").map(|f| f.0), Some("DOCUMENTS"));
        assert_eq!(folder_of("MY PICTURES").map(|f| f.0), Some("PICTURES"));
        assert_eq!(folder_of("Common Desktop"), None);
        assert_eq!(folder_of("AppData"), None);
    }
}
