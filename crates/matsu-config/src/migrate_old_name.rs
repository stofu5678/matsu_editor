//! Takes the settings of UME Editor (the old name of MATSU Editor, ADR-0021) over to the new places.
//!
//! TEMPORARY: keep this module until version 1.0, then delete it (ADR-0021, "利用者のデータの引き継ぎ").
//!
//! When `matsu.ini` is in none of its places and the old `ume.ini` is in one of UME Editor's places, the files of
//! the old folder are copied to the new folder, and `ume.ini` becomes `matsu.ini`:
//!
//! - Windows: `ume.ini` next to the executable (a portable copy) is copied to `matsu.ini` next to it; or the
//!   folder `%APPDATA%\ume-editor\` is copied to `%APPDATA%\matsu-editor\`.
//! - macOS: `~/Library/Application Support/ume-editor/` to `.../matsu-editor/`.
//! - Linux: `$XDG_CONFIG_HOME/ume-editor/` (or `~/.config/ume-editor/`) to `.../matsu-editor/`.
//!
//! The old files are never deleted or changed, so the user can go back. A file that is in the new folder already
//! is not overwritten. The `matsu.ini` files are written last, so a copy that stops half way leaves no
//! `matsu.ini` and is tried again at the next start. A file other than the settings file that cannot be copied is
//! skipped (and counted in the message), so that it does not keep the settings from coming over.
//!
//! The own section of the old name in the settings file (`[UME]`) is renamed `[MATSU]` in the copy, so that its
//! items (the macOS key layout, the save options) keep working. Nothing else in the file is touched.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::location::{self, Environment, Os};

/// The folder name UME Editor used for its settings.
const OLD_APP_DIR: &str = "ume-editor";

/// The name of UME Editor's settings file.
const OLD_FILE_NAME: &str = "ume.ini";

/// What [`migrate`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `matsu.ini` is in one of its places already (or the OS has no place): nothing to do.
    NotNeeded,
    /// There is no `ume.ini` in any of the old places.
    NothingToMigrate,
    /// The old files were copied from `from` to `to` (folders; for a portable `ume.ini`, the folder of the
    /// executable). `skipped` files other than the settings could not be copied.
    Migrated {
        from: PathBuf,
        to: PathBuf,
        skipped: usize,
    },
    /// The copy failed. The old files are as they were; `matsu.ini` was not written.
    Failed { from: PathBuf, error: String },
}

impl Outcome {
    /// The one-time message for the status bar, if the user should hear about it.
    pub fn notice(&self) -> Option<String> {
        match self {
            Outcome::NotNeeded | Outcome::NothingToMigrate => None,
            Outcome::Migrated { from, to, skipped } => Some(format!(
                "UME Editor の設定を引き継ぎました（{} から {} へコピー。古いファイルはそのまま残してあります）{}",
                from.display(),
                to.display(),
                if *skipped == 0 {
                    String::new()
                } else {
                    format!("。設定ファイル以外の {skipped} 個はコピーできませんでした")
                }
            )),
            Outcome::Failed { from, error } => Some(format!(
                "UME Editor の設定を引き継げませんでした: {error}（{} のファイルは変えていません）",
                from.display()
            )),
        }
    }
}

/// Copies UME Editor's settings to MATSU Editor's places when they are needed (see the module).
pub fn migrate(os: Os, env: &Environment) -> Outcome {
    let new = location::candidate_paths(os, env);
    if new.is_empty() || location::find_existing(&new).is_some() {
        return Outcome::NotNeeded;
    }
    let old = location::candidate_paths_named(os, env, OLD_APP_DIR, OLD_FILE_NAME);
    // The same places without the profile: the folders that are copied as a whole.
    let no_profile = Environment {
        profile_name: None,
        ..env.clone()
    };
    let old_roots = location::candidate_paths_named(os, &no_profile, OLD_APP_DIR, OLD_FILE_NAME);
    let new_roots = location::candidate_paths(os, &no_profile);
    for ((old_file, old_root), new_root) in old.iter().zip(&old_roots).zip(&new_roots) {
        if !old_file.is_file() {
            continue;
        }
        let (Some(from), Some(to)) = (old_root.parent(), new_root.parent()) else {
            continue;
        };
        // Next to the executable only the settings file is copied: the folder holds the program.
        let portable = env.exe_dir.as_deref() == Some(from);
        let result = if portable {
            let target = with_new_name(old_file, from, to);
            copy_settings_file(old_file, &target).map(|()| 0)
        } else {
            copy_folder(from, to)
        };
        return match result {
            Ok(skipped) => Outcome::Migrated {
                from: from.to_path_buf(),
                to: to.to_path_buf(),
                skipped,
            },
            Err(e) => Outcome::Failed {
                from: from.to_path_buf(),
                error: e.to_string(),
            },
        };
    }
    Outcome::NothingToMigrate
}

/// Where `old_file` (under `from`) goes under `to`, with `ume.ini` named `matsu.ini`.
fn with_new_name(old_file: &Path, from: &Path, to: &Path) -> PathBuf {
    let relative = old_file.strip_prefix(from).unwrap_or(old_file);
    let mut target = to.join(relative);
    if relative.file_name().is_some_and(|n| n == OLD_FILE_NAME) {
        target.set_file_name(location::FILE_NAME);
    }
    target
}

/// Copies the regular files of `from` (with its sub folders) into `to`. Files that are in `to` already stay.
/// The settings files are copied last. Returns how many other files could not be copied.
fn copy_folder(from: &Path, to: &Path) -> io::Result<usize> {
    let mut others = Vec::new();
    let mut settings = Vec::new();
    collect(from, &mut others, &mut settings)?;
    let mut skipped = 0;
    for file in others {
        let target = to.join(file.strip_prefix(from).unwrap_or(&file));
        if copy_new_file(&file, &target).is_err() {
            skipped += 1;
        }
    }
    for file in settings {
        let target = with_new_name(&file, from, to);
        copy_settings_file(&file, &target)?;
    }
    Ok(skipped)
}

/// Lists the regular files under `dir` (symbolic links are skipped), the `ume.ini` files in `settings`.
fn collect(dir: &Path, others: &mut Vec<PathBuf>, settings: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect(&path, others, settings)?;
        } else if kind.is_file() {
            if entry.file_name() == OLD_FILE_NAME {
                settings.push(path);
            } else {
                others.push(path);
            }
        }
    }
    Ok(())
}

/// Copies `from` to `to` unless `to` is there. Makes the folder.
fn copy_new_file(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        return Ok(());
    }
    if let Some(dir) = to.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::copy(from, to).map(drop)
}

/// Like [`copy_new_file`], but the target appears whole or not at all (it is the file whose presence says that the
/// migration is done).
fn copy_settings_file(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        return Ok(());
    }
    if let Some(dir) = to.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut temp = to.as_os_str().to_owned();
    temp.push(".migrating");
    let temp = PathBuf::from(temp);
    let copied = fs::read(from)
        .and_then(|bytes| fs::write(&temp, rename_own_section(&bytes)))
        .and_then(|()| fs::rename(&temp, to));
    if copied.is_err() {
        fs::remove_file(&temp).ok();
    }
    copied
}

/// The settings file with the line `[UME]` named `[MATSU]`. The file is UTF-8 or Shift_JIS; in both a line that is
/// exactly `[UME]` is ASCII, and every other byte is passed on as it is (line endings, BOM, other sections).
fn rename_own_section(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 8);
    for (i, line) in bytes.split_inclusive(|&b| b == b'\n').enumerate() {
        let (bom, rest) = match line.strip_prefix(b"\xEF\xBB\xBF") {
            Some(rest) if i == 0 => (&line[..3], rest),
            _ => (&line[..0], line),
        };
        let body_len = rest
            .iter()
            .rposition(|&b| b != b'\r' && b != b'\n')
            .map_or(0, |p| p + 1);
        let (body, ending) = rest.split_at(body_len);
        out.extend_from_slice(bom);
        if body.eq_ignore_ascii_case(b"[UME]") {
            out.extend_from_slice(b"[MATSU]");
        } else {
            out.extend_from_slice(body);
        }
        out.extend_from_slice(ending);
    }
    out
}

#[cfg(test)]
mod tests;
