//! What the user agreed to run: `plugins.ini`, next to the settings file (ADR-0026, chapters 4
//! and 5).
//!
//! A plugin runs code by itself, as the user, so it runs only when the user has enabled it. The
//! file keeps, for each enabled plugin, the date and the SHA-256 of every file in the folder of
//! the plugin at the time. When a file of an enabled plugin is not the same any more (changed,
//! added or removed), the plugin is *changed* and does not run until the user enables it again.
//!
//! ```text
//! [Plugin.jp.example.sample]
//! Enabled=1
//! Date=2026-10-26
//! Folder=Sample
//! File[001]=9f86d0... plugin.def
//! File[002]=60303a... local/plugin_ja_JP.def
//! ```
//!
//! The file is not in `matsu.ini` so that the agreement does not travel with a copy of that file
//! to another PC (ADR-0026, chapter 5). It is a [`Profile`], like `databases.ini`.
//!
//! Sakura has nothing like this: it enables a plugin as soon as it is added.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::profile::{Profile, ProfileError};

/// The name of the file.
pub const FILE_NAME: &str = "plugins.ini";
/// The start of the name of the section of a plugin; the ID of the plugin follows.
pub const SECTION_PREFIX: &str = "Plugin.";
/// The most files a plugin folder can have. A folder with more (a `node_modules`) is not a plugin
/// that can be checked, and is refused.
pub const MAX_FILES: usize = 5000;

/// The file that is next to the settings file `settings`.
pub fn path_beside(settings: &Path) -> PathBuf {
    settings.with_file_name(FILE_NAME)
}

/// A file of a plugin and its SHA-256.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileHash {
    /// The path from the folder of the plugin, with `/` between the names.
    pub path: String,
    /// 64 hexadecimal digits in lower case.
    pub sha256: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<FileHash>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)?;
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        if meta.is_dir() {
            walk(root, &path, out)?;
        } else if meta.is_file() {
            if out.len() >= MAX_FILES {
                return Err(io::Error::other("too many files"));
            }
            out.push(FileHash {
                path: relative,
                sha256: hash_file(&path)?,
            });
        } else if meta.file_type().is_symlink() {
            // A link would let the contents change without a change in the folder, so a plugin
            // with a link cannot be checked (and cannot be enabled).
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("symbolic link: {relative}"),
            ));
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}

/// The SHA-256 of every file under `dir`, in the order of their paths.
pub fn fingerprint(dir: &Path) -> io::Result<Vec<FileHash>> {
    let mut files = Vec::new();
    walk(dir, dir, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// The files as lines `<sha256> <path>` (for a place that holds only text).
pub fn manifest_text(files: &[FileHash]) -> String {
    files
        .iter()
        .map(|f| format!("{} {}", f.sha256, f.path))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The files of [`manifest_text`]; a line that is not one is skipped.
pub fn parse_manifest(text: &str) -> Vec<FileHash> {
    text.lines()
        .filter_map(|line| {
            let (hash, path) = line.split_once(' ')?;
            (hash.len() == 64 && !path.is_empty()).then(|| FileHash {
                path: path.to_owned(),
                sha256: hash.to_owned(),
            })
        })
        .collect()
}

/// What differs between the files that were agreed to and the files now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    pub changed: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.added.is_empty() && self.removed.is_empty()
    }
}

/// Compares the agreed files with the files now.
pub fn compare(agreed: &[FileHash], now: &[FileHash]) -> Changes {
    let old: BTreeMap<&str, &str> = agreed
        .iter()
        .map(|f| (f.path.as_str(), f.sha256.as_str()))
        .collect();
    let new: BTreeMap<&str, &str> = now
        .iter()
        .map(|f| (f.path.as_str(), f.sha256.as_str()))
        .collect();
    let mut changes = Changes::default();
    for (path, hash) in &new {
        match old.get(path) {
            None => changes.added.push((*path).to_owned()),
            Some(old_hash) if old_hash != hash => changes.changed.push((*path).to_owned()),
            Some(_) => {}
        }
    }
    for path in old.keys() {
        if !new.contains_key(path) {
            changes.removed.push((*path).to_owned());
        }
    }
    changes
}

/// What the file says about a plugin, set against its folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Consent {
    /// Nothing is recorded: the plugin does not run.
    Off,
    /// Enabled, and the files are the ones agreed to.
    On,
    /// Enabled once, but the files are not the same now. The plugin does not run.
    Changed(Changes),
}

/// What is recorded for one enabled plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub folder: String,
    pub date: String,
    pub files: Vec<FileHash>,
}

/// The file `plugins.ini`.
#[derive(Clone, Debug, Default)]
pub struct ConsentFile {
    profile: Profile,
}

fn section(id: &str) -> String {
    format!("{SECTION_PREFIX}{id}")
}

impl ConsentFile {
    /// Reads the file; one that is not there is an empty one.
    pub fn load(path: &Path) -> Self {
        ConsentFile {
            profile: Profile::load(path).unwrap_or_default(),
        }
    }

    /// Writes the file (the folder is made if it is not there).
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        self.profile.save(path)
    }

    /// What is recorded for the plugin `id`.
    pub fn record(&self, id: &str) -> Option<Record> {
        let section = section(id);
        if self.profile.get_bool(&section, "Enabled") != Some(true) {
            return None;
        }
        let mut files: Vec<FileHash> = Vec::new();
        for (key, value) in self.profile.entries(&section) {
            if !key.starts_with("File[") {
                continue;
            }
            if let Some((hash, path)) = value.split_once(' ')
                && hash.len() == 64
                && !path.is_empty()
            {
                files.push(FileHash {
                    path: path.to_owned(),
                    sha256: hash.to_owned(),
                });
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Some(Record {
            folder: self
                .profile
                .get(&section, "Folder")
                .unwrap_or_default()
                .to_owned(),
            date: self
                .profile
                .get(&section, "Date")
                .unwrap_or_default()
                .to_owned(),
            files,
        })
    }

    /// Records that the user enabled the plugin `id` in `folder` with these files, on `date`.
    pub fn enable(
        &mut self,
        id: &str,
        folder: &str,
        files: &[FileHash],
        date: &str,
    ) -> Result<(), ProfileError> {
        self.disable(id);
        let section = section(id);
        self.profile.set(&section, "Enabled", "1")?;
        self.profile.set(&section, "Date", date)?;
        self.profile.set(&section, "Folder", folder)?;
        for (index, file) in files.iter().enumerate() {
            self.profile.set(
                &section,
                &format!("File[{:03}]", index + 1),
                &format!("{} {}", file.sha256, file.path),
            )?;
        }
        Ok(())
    }

    /// Forgets the plugin `id`. Returns whether anything was recorded.
    pub fn disable(&mut self, id: &str) -> bool {
        self.profile.remove_section(&section(id))
    }

    /// Whether the plugin may run: enabled, and its files are the agreed ones.
    pub fn consent(&self, id: &str, folder_now: &[FileHash]) -> Consent {
        let Some(record) = self.record(id) else {
            return Consent::Off;
        };
        let changes = compare(&record.files, folder_now);
        if changes.is_empty() {
            Consent::On
        } else {
            Consent::Changed(changes)
        }
    }

    /// [`consent`](Self::consent) for the folder `dir`. A folder that cannot be read (it is gone)
    /// is a change of every file.
    pub fn consent_for_dir(&self, id: &str, dir: Option<&Path>) -> Consent {
        // Nothing is read for a plugin that was never enabled.
        let Some(record) = self.record(id) else {
            return Consent::Off;
        };
        match dir.and_then(|dir| fingerprint(dir).ok()) {
            None => Consent::Changed(compare(&record.files, &[])),
            Some(now) => self.consent(id, &now),
        }
    }
}

/// Today's date in UTC as `YYYY-MM-DD`.
pub fn today() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    date_of_days(i64::try_from(seconds / 86_400).unwrap_or(0))
}

/// The date of the day `days` after 1970-01-01 (the civil-from-days algorithm).
fn date_of_days(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, path: &str, text: &str) {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }

    fn sample(dir: &Path) {
        write(dir, "plugin.def", "[Plugin]\nId=x\n");
        write(dir, "main.js", "Editor.InsText('a');");
        write(dir, "local/plugin_ja_JP.def", "[Plugin]\nName=名前\n");
    }

    #[test]
    fn the_hash_is_sha256_and_the_files_are_in_path_order() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "b.txt", "abc");
        write(dir.path(), "a/z.txt", "");
        let files = fingerprint(dir.path()).unwrap();
        let paths: Vec<_> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["a/z.txt", "b.txt"]);
        // SHA-256 of the empty string and of "abc".
        assert_eq!(
            files[0].sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            files[1].sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn enabling_records_the_files_and_reading_gives_them_back() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = dir.path().join("Sample");
        sample(&plugin);
        let files = fingerprint(&plugin).unwrap();
        let mut file = ConsentFile::default();
        assert_eq!(file.consent("id.a", &files), Consent::Off);
        file.enable("id.a", "Sample", &files, "2026-10-26").unwrap();
        let path = dir.path().join("plugins.ini");
        file.save(&path).unwrap();
        let back = ConsentFile::load(&path);
        let record = back.record("id.a").unwrap();
        assert_eq!(record.folder, "Sample");
        assert_eq!(record.date, "2026-10-26");
        assert_eq!(record.files, files);
        assert_eq!(back.consent("id.a", &files), Consent::On);
        assert_eq!(back.record("other"), None);
    }

    #[test]
    fn a_changed_added_or_removed_file_stops_the_plugin() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let files = fingerprint(dir.path()).unwrap();
        let mut file = ConsentFile::default();
        file.enable("p", "F", &files, "2026-10-26").unwrap();

        write(dir.path(), "main.js", "Editor.InsText('b');");
        let Consent::Changed(c) = file.consent_for_dir("p", Some(dir.path())) else {
            panic!("a changed file");
        };
        assert_eq!(c.changed, ["main.js"]);
        assert!(c.added.is_empty() && c.removed.is_empty());

        write(dir.path(), "main.js", "Editor.InsText('a');");
        assert_eq!(file.consent_for_dir("p", Some(dir.path())), Consent::On);

        write(dir.path(), "extra/new.js", "x");
        let Consent::Changed(c) = file.consent_for_dir("p", Some(dir.path())) else {
            panic!("an added file");
        };
        assert_eq!(c.added, ["extra/new.js"]);
        std::fs::remove_dir_all(dir.path().join("extra")).unwrap();

        std::fs::remove_file(dir.path().join("local/plugin_ja_JP.def")).unwrap();
        let Consent::Changed(c) = file.consent_for_dir("p", Some(dir.path())) else {
            panic!("a removed file");
        };
        assert_eq!(c.removed, ["local/plugin_ja_JP.def"]);
    }

    #[test]
    fn enabling_again_takes_the_new_files_and_disabling_forgets() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let mut file = ConsentFile::default();
        file.enable("p", "F", &fingerprint(dir.path()).unwrap(), "2026-01-01")
            .unwrap();
        write(dir.path(), "main.js", "changed");
        assert!(matches!(
            file.consent_for_dir("p", Some(dir.path())),
            Consent::Changed(_)
        ));
        file.enable("p", "F", &fingerprint(dir.path()).unwrap(), "2026-02-02")
            .unwrap();
        assert_eq!(file.consent_for_dir("p", Some(dir.path())), Consent::On);
        assert_eq!(file.record("p").unwrap().date, "2026-02-02");
        assert!(file.disable("p"));
        assert!(!file.disable("p"));
        assert_eq!(file.consent_for_dir("p", Some(dir.path())), Consent::Off);
    }

    #[test]
    fn a_folder_that_is_gone_is_a_change_of_every_file() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let mut file = ConsentFile::default();
        file.enable("p", "F", &fingerprint(dir.path()).unwrap(), "d")
            .unwrap();
        let Consent::Changed(c) = file.consent_for_dir("p", None) else {
            panic!("a folder that is gone");
        };
        assert_eq!(c.removed.len(), 3);
    }

    #[test]
    fn the_records_of_two_plugins_do_not_mix_and_other_lines_stay() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugins.ini");
        std::fs::write(&path, "\u{feff}; mine\r\n[Other]\r\nx=1\r\n").unwrap();
        let mut file = ConsentFile::load(&path);
        let a = [FileHash {
            path: "a".into(),
            sha256: "0".repeat(64),
        }];
        file.enable("one", "A", &a, "d").unwrap();
        file.enable("two", "B", &[], "d").unwrap();
        file.disable("one");
        file.save(&path).unwrap();
        let back = ConsentFile::load(&path);
        assert!(back.record("one").is_none());
        assert_eq!(back.record("two").unwrap().folder, "B");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("; mine") && text.contains("[Other]"));
    }

    #[test]
    fn japanese_names_are_handled() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "日本語/スクリプト.js", "あ");
        let files = fingerprint(dir.path()).unwrap();
        assert_eq!(files[0].path, "日本語/スクリプト.js");
        let mut file = ConsentFile::default();
        file.enable("日本語のID", "フォルダー", &files, "d")
            .unwrap();
        assert_eq!(file.consent("日本語のID", &files), Consent::On);
    }

    #[cfg(unix)]
    #[test]
    fn a_plugin_with_a_link_cannot_be_checked() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.js", "x");
        std::os::unix::fs::symlink("a.js", dir.path().join("link.js")).unwrap();
        assert!(fingerprint(dir.path()).is_err());
        // An enabled plugin that gets a link is stopped.
        std::fs::remove_file(dir.path().join("link.js")).unwrap();
        let mut file = ConsentFile::default();
        file.enable("p", "F", &fingerprint(dir.path()).unwrap(), "d")
            .unwrap();
        std::os::unix::fs::symlink("a.js", dir.path().join("link.js")).unwrap();
        assert!(matches!(
            file.consent_for_dir("p", Some(dir.path())),
            Consent::Changed(_)
        ));
    }

    #[test]
    fn the_manifest_text_round_trips() {
        let files = vec![
            FileHash {
                path: "a b/c.js".into(),
                sha256: "1".repeat(64),
            },
            FileHash {
                path: "d".into(),
                sha256: "2".repeat(64),
            },
        ];
        assert_eq!(parse_manifest(&manifest_text(&files)), files);
        assert!(parse_manifest("garbage\nshort x").is_empty());
    }

    #[test]
    fn the_date_is_the_civil_date() {
        assert_eq!(date_of_days(0), "1970-01-01");
        assert_eq!(date_of_days(20_752), "2026-10-26");
        assert_eq!(date_of_days(11_016), "2000-02-29");
        assert_eq!(today().len(), 10);
    }
}
