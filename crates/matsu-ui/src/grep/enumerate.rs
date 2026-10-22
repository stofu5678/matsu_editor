// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/grep/CGrepEnumFileBase.h
//   sakura_core/grep/CGrepEnumFiles.h
//   sakura_core/grep/CGrepEnumFolders.h
//   sakura_core/grep/CGrepEnumFilterFiles.h
//   sakura_core/grep/CGrepEnumFilterFolders.h
//   sakura_core/grep/CGrepExceptFileRegexps.h
//
// sakura_core/grep/CGrepEnumFileBase.h, CGrepEnumFiles.h, CGrepEnumFolders.h and
// CGrepEnumFilterFolders.h:
//   Copyright (C) 2008, wakura
//   Copyright (C) 2018-2022, Sakura Editor Organization
// sakura_core/grep/CGrepEnumFilterFiles.h:
//   Copyright (C) 2008, wakura
//   Copyright (C) 2018-2026, Sakura Editor Organization
// sakura_core/grep/CGrepExceptFileRegexps.h:
//   Copyright (C) 2026, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (the Win32 file search
// `FindFirstFile`/`PathMatchSpec`, the class hierarchy and UTF-16 buffers) was rewritten in Rust on
// `std::fs` and `&str`, the matching of a name against a wildcard is written here, and the tests
// are new.
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! Finding the files and folders that Grep goes through, in one folder at a time.
//!
//! Sakura: `CGrepEnumFilterFiles` and `CGrepEnumFilterFolders` (`sakura_core/grep/`). A search
//! pattern lists the entries of its folder that match it (`sub\*.h` goes into `sub`); a file is
//! left out when an excluded pattern lists it too, when a full path pattern excluded it, when a
//! pattern with a folder part found it before (it is not searched again when the search goes
//! down into that folder), or when the full path matches an excluded regular expression.
//!
//! Differences from Sakura: `PathMatchSpec` and the file system of Windows ignore the case of
//! names, so the wildcards do here, on every OS. The entries of a folder come in the order of
//! their names (the order of NTFS) and not in the order the OS gives. A backslash in a pattern
//! separates folders on every OS. The attributes of the options "隠しファイルを除く" and the like
//! (`CGrepEnumOptions`) are never set by Grep; the file tree of the outline sets them
//! ([`EnumOptions`], [`Walker::with_options`]). Where the file system has no hidden attribute, a
//! name that starts with a dot is hidden, and there is no system attribute.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use matsu_core::search::Matcher;

use super::keys::GrepKeys;

/// The separator that this program writes in the paths it builds.
pub const SEP: char = std::path::MAIN_SEPARATOR;

/// Whether `name` matches the wildcard pattern `spec` (`PathMatchSpec`): `*` is any run of
/// characters, `?` one character, and the case does not matter. `*.*` matches every name, also
/// one without a dot, as it does in the shell.
pub fn path_match_spec(name: &str, spec: &str) -> bool {
    if spec == "*.*" {
        return true;
    }
    let name: Vec<char> = name.chars().flat_map(char::to_lowercase).collect();
    let spec: Vec<char> = spec.chars().flat_map(char::to_lowercase).collect();
    let (mut n, mut s) = (0, 0);
    // Where the last `*` was in the pattern, and how much of the name it has taken.
    let mut star: Option<(usize, usize)> = None;
    while n < name.len() {
        if s < spec.len() && (spec[s] == '?' || spec[s] == name[n]) && spec[s] != '*' {
            n += 1;
            s += 1;
        } else if s < spec.len() && spec[s] == '*' {
            star = Some((s, n));
            s += 1;
        } else if let Some((star_s, star_n)) = star {
            s = star_s + 1;
            n = star_n + 1;
            star = Some((star_s, star_n + 1));
        } else {
            return false;
        }
    }
    spec[s..].iter().all(|c| *c == '*')
}

/// A path as the sets of excluded paths store it: one separator, no case.
fn normalized(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

/// A pattern with the separator of the OS, so that `sub\*.h` works on every OS.
fn with_os_separator(key: &str) -> String {
    if cfg!(windows) {
        key.to_owned()
    } else {
        key.replace('\\', "/")
    }
}

/// What is listed: the files of a folder or its subfolders.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    File,
    Folder,
}

struct Entry {
    name: String,
    is_dir: bool,
    /// A regular file. A pipe, a socket or a device is neither this nor a folder: reading it
    /// could wait for ever.
    is_file: bool,
    /// `FILE_ATTRIBUTE_HIDDEN`. Where the file system has no such attribute, a name that starts
    /// with a dot (the convention of the shells).
    hidden: bool,
    /// `FILE_ATTRIBUTE_READONLY`.
    read_only: bool,
    /// `FILE_ATTRIBUTE_SYSTEM`; never set where the file system has no such attribute.
    system: bool,
}

/// What `CGrepEnumOptions` leaves out: entries with these attributes. Grep never sets them (see
/// the top of this file); the file tree of the outline does (`CFileTreeSetting` items).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnumOptions {
    /// `m_bIgnoreHidden`.
    pub ignore_hidden: bool,
    /// `m_bIgnoreReadOnly`.
    pub ignore_read_only: bool,
    /// `m_bIgnoreSystem`.
    pub ignore_system: bool,
}

impl EnumOptions {
    fn skips(self, entry: &Entry) -> bool {
        (self.ignore_hidden && entry.hidden)
            || (self.ignore_read_only && entry.read_only)
            || (self.ignore_system && entry.system)
    }
}

/// The attributes of an entry as `(hidden, read_only, system)`.
fn attributes(name: &str, metadata: &fs::Metadata) -> (bool, bool, bool) {
    let read_only = metadata.permissions().readonly();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
        let attrs = metadata.file_attributes();
        let _ = name;
        (
            attrs & FILE_ATTRIBUTE_HIDDEN != 0,
            read_only,
            attrs & FILE_ATTRIBUTE_SYSTEM != 0,
        )
    }
    #[cfg(not(windows))]
    {
        (name.starts_with('.'), read_only, false)
    }
}

/// The entries of a folder, sorted by name; nothing when it cannot be read (`FindFirstFile`
/// failing).
fn read_entries(dir: &Path) -> Vec<Entry> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            // A link is what it points to (nothing, when it points to nothing).
            let metadata = fs::metadata(entry.path())
                .or_else(|_| entry.metadata())
                .ok()?;
            let kind = metadata.file_type();
            let (hidden, read_only, system) = attributes(&name, &metadata);
            Some(Entry {
                is_dir: kind.is_dir(),
                is_file: kind.is_file(),
                hidden,
                read_only,
                system,
                name,
            })
        })
        .collect();
    entries.sort_by_cached_key(|entry| (entry.name.to_lowercase(), entry.name.clone()));
    entries
}

/// Joins the base folder and a name the way `Enumerates` builds `strFullPath`: no separator when
/// the base is empty.
pub fn join_path(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_owned()
    } else {
        format!("{base}{SEP}{name}")
    }
}

/// `CGrepEnumFileBase::Enumerates`: the names, with the folder part of their pattern, that the
/// patterns list in `base` and that `accept` takes (`accept(name, full_path)`).
///
/// `already` is the set of full paths that must not be listed; when a pattern has a folder part,
/// the paths it lists are added to it (`pExceptItems`).
fn enumerate(
    base: &str,
    keys: &[String],
    kind: Kind,
    options: EnumOptions,
    mut accept: impl FnMut(&str, &str) -> bool,
    mut already: Option<&mut HashSet<String>>,
) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for key in keys {
        let key = with_os_separator(key);
        let dir_len = key.rfind(['\\', '/']).map_or(0, |i| i + 1);
        let (key_dir, pattern) = key.split_at(dir_len);
        let dir = join_path(base, key_dir);
        let dir = if dir.is_empty() { ".".to_owned() } else { dir };
        for entry in read_entries(Path::new(&dir)) {
            if !path_match_spec(&entry.name, pattern) {
                continue;
            }
            let wanted = match kind {
                Kind::File => entry.is_file,
                Kind::Folder => entry.is_dir && entry.name != "." && entry.name != "..",
            };
            if !wanted || options.skips(&entry) {
                continue;
            }
            let name = format!("{key_dir}{}", entry.name);
            let full = join_path(base, &name);
            if found.contains(&name) || !accept(&name, &full) {
                continue;
            }
            if already
                .as_deref()
                .is_some_and(|set| set.contains(&normalized(&full)))
            {
                continue;
            }
            if dir_len > 0
                && let Some(set) = already.as_deref_mut()
            {
                // searched here; the search of the folder below does not take it again
                set.insert(normalized(&full));
            }
            found.push(name);
        }
    }
    found
}

/// Goes through folders for Grep: the files to search in a folder and the folders to go into
/// (`CGrepEnumFilterFiles::Enumerates`, `CGrepEnumFilterFolders::Enumerates`). One walker is used
/// for the whole search, since it remembers which files were searched by a pattern with a folder
/// part.
pub struct Walker {
    keys: GrepKeys,
    /// The full paths that the excluded full path patterns list (`cGrepExceptAbsFiles`); also the
    /// files already searched.
    skip_files: HashSet<String>,
    /// `cGrepExceptAbsFolders`.
    skip_folders: HashSet<String>,
    /// The excluded regular expressions, matched against the full path (`m_fnIsExceptFilePath`).
    regexes: Vec<Arc<dyn Matcher>>,
    /// Entries with these attributes are left out (`CGrepEnumOptions`).
    options: EnumOptions,
}

impl Walker {
    /// `regexes` are the compiled `keys.except_file_regexes`.
    pub fn new(keys: GrepKeys, regexes: Vec<Arc<dyn Matcher>>) -> Walker {
        Self::with_options(keys, regexes, EnumOptions::default())
    }

    /// A walker that leaves out the entries with the attributes of `options`.
    pub fn with_options(
        keys: GrepKeys,
        regexes: Vec<Arc<dyn Matcher>>,
        options: EnumOptions,
    ) -> Walker {
        let skip_files = enumerate(
            "",
            &keys.except_abs_files,
            Kind::File,
            options,
            |_, _| true,
            None,
        )
        .into_iter()
        .map(|path| normalized(&path))
        .collect();
        let skip_folders = enumerate(
            "",
            &keys.except_abs_folders,
            Kind::Folder,
            options,
            |_, _| true,
            None,
        )
        .into_iter()
        .map(|path| normalized(&path))
        .collect();
        Walker {
            keys,
            skip_files,
            skip_folders,
            regexes,
            options,
        }
    }

    /// The files of `dir` to search, as names relative to `dir` (a name has the folder part of
    /// its pattern, such as `sub\a.h`).
    pub fn files_in(&mut self, dir: &str) -> Vec<String> {
        // `m_cGrepEnumExceptFiles`: the names the excluded patterns list in this folder.
        let excluded = enumerate(
            dir,
            &self.keys.except_files,
            Kind::File,
            self.options,
            |_, _| true,
            None,
        );
        let regexes = &self.regexes;
        enumerate(
            dir,
            &self.keys.search_files,
            Kind::File,
            self.options,
            |name, full| {
                !excluded.iter().any(|e| e == name)
                    && !regexes
                        .iter()
                        .any(|regex| matches!(regex.find_at(full, 0), Ok(Some(_))))
            },
            Some(&mut self.skip_files),
        )
    }

    /// The subfolders of `dir` to go into, as names.
    pub fn folders_in(&mut self, dir: &str) -> Vec<String> {
        let excluded = enumerate(
            dir,
            &self.keys.except_folders,
            Kind::Folder,
            self.options,
            |_, _| true,
            None,
        );
        enumerate(
            dir,
            &self.keys.search_folders,
            Kind::Folder,
            self.options,
            |name, _| !excluded.iter().any(|e| e == name),
            Some(&mut self.skip_folders),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in files {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"x").unwrap();
        }
        dir
    }

    fn walker(keys: &str) -> Walker {
        Walker::new(GrepKeys::parse(keys, false).unwrap(), Vec::new())
    }

    fn base(dir: &tempfile::TempDir) -> String {
        dir.path().to_string_lossy().into_owned()
    }

    #[test]
    fn wildcards_ignore_the_case_and_match_japanese() {
        assert!(path_match_spec("Readme.TXT", "*.txt"));
        assert!(path_match_spec("a.txt", "?.txt"));
        assert!(!path_match_spec("ab.txt", "?.txt"));
        assert!(path_match_spec("議事録2026.txt", "議事録*.txt"));
        assert!(path_match_spec("議事録.txt", "議事?.txt"));
        assert!(!path_match_spec("議事.txt", "議事?.txt"));
        // `*.*` is everything, also a name without a dot
        assert!(path_match_spec("Makefile", "*.*"));
        assert!(path_match_spec(".gitignore", "*.*"));
        assert!(path_match_spec("anything", "*"));
        assert!(!path_match_spec("Makefile", "*.mk"));
        // the stars backtrack
        assert!(path_match_spec("aXbXc.txt", "a*X*c.txt"));
        assert!(path_match_spec("abc", "a*b*c*"));
        assert!(!path_match_spec("abx", "a*b*c"));
        assert!(path_match_spec("", "*"));
        assert!(!path_match_spec("", "?"));
    }

    #[test]
    fn the_files_of_a_folder_come_in_the_order_of_their_names() {
        let dir = tree(&["b.txt", "A.txt", "c.md", "sub/d.txt"]);
        let mut w = walker("*.txt");
        assert_eq!(w.files_in(&base(&dir)), vec!["A.txt", "b.txt"]);
        // a folder is not a file, and the subfolders are listed by `folders_in`
        let mut w = walker("*.*");
        assert_eq!(w.files_in(&base(&dir)), vec!["A.txt", "b.txt", "c.md"]);
        assert_eq!(w.folders_in(&base(&dir)), vec!["sub"]);
    }

    #[test]
    fn several_patterns_list_in_the_order_of_the_patterns_without_repeats() {
        let dir = tree(&["a.txt", "b.md", "ab.txt"]);
        let mut w = walker("*.md;a*;*.txt");
        assert_eq!(w.files_in(&base(&dir)), vec!["b.md", "a.txt", "ab.txt"]);
    }

    #[test]
    fn excluded_files_are_left_out_by_name() {
        let dir = tree(&["a.txt", "a.bak", "b.exe", "c.txt"]);
        let mut w = walker("*.*;!*.bak;!*.exe");
        assert_eq!(w.files_in(&base(&dir)), vec!["a.txt", "c.txt"]);
        let mut w = walker("*.txt;!c.*");
        assert_eq!(w.files_in(&base(&dir)), vec!["a.txt"]);
    }

    #[test]
    fn excluded_folders_are_not_gone_into() {
        let dir = tree(&[".git/x", "src/y", "target/z", "目次/w"]);
        let mut w = walker("*.*;#.git;#target");
        assert_eq!(w.folders_in(&base(&dir)), vec!["src", "目次"]);
        // names are exact matches of the pattern: `tar*` matches `target`
        let mut w = walker("#tar*");
        assert_eq!(w.folders_in(&base(&dir)), vec![".git", "src", "目次"]);
    }

    #[test]
    fn a_pattern_with_a_folder_part_lists_that_folder_once() {
        let dir = tree(&["sub/a.h", "sub/b.c", "top.h"]);
        let mut w = walker(r"*.h;sub\*.h");
        let b = base(&dir);
        let files = w.files_in(&b);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0], "top.h");
        assert_eq!(files[1].replace('\\', "/"), "sub/a.h");
        // the search goes down into `sub` and its `a.h` was done: it is not searched again
        let sub = join_path(&b, "sub");
        assert!(w.files_in(&sub).is_empty());
    }

    #[test]
    fn full_path_exclusions_skip_that_file_in_any_folder() {
        let dir = tree(&["a.log", "b.log", "sub/c.log"]);
        let b = base(&dir);
        let skip = join_path(&b, "a.log");
        let mut w = walker(&format!("*.log;!{skip}"));
        assert_eq!(w.files_in(&b), vec!["b.log"]);
        assert_eq!(w.files_in(&join_path(&b, "sub")), vec!["c.log"]);
    }

    #[test]
    fn full_path_folder_exclusions_skip_that_folder() {
        let dir = tree(&["keep/a", "skip/b"]);
        let b = base(&dir);
        let skip = join_path(&b, "skip");
        let mut w = walker(&format!("*.*;#{skip}"));
        assert_eq!(w.folders_in(&b), vec!["keep"]);
    }

    #[test]
    fn excluded_regular_expressions_match_the_full_path() {
        let dir = tree(&["a.rs", "gen/b.rs", "gen/c.rs"]);
        let keys = GrepKeys::parse(r"*.rs;!gen", true).unwrap();
        let regex = matsu_regex::RegexMatcher::new(
            "gen",
            matsu_regex::RegexSearchOptions { match_case: false },
        )
        .unwrap();
        let mut w = Walker::new(keys, vec![Arc::new(regex)]);
        let b = base(&dir);
        assert_eq!(w.files_in(&b), vec!["a.rs"]);
        assert!(w.files_in(&join_path(&b, "gen")).is_empty());
    }

    #[test]
    fn hidden_and_read_only_entries_are_left_out_when_asked() {
        let dir = tree(&["a.txt", ".hidden.txt", "ro.txt", ".git/x", "src/y"]);
        let b = base(&dir);
        let ro = dir.path().join("ro.txt");
        let mut permissions = fs::metadata(&ro).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&ro, permissions).unwrap();
        let keys = || GrepKeys::parse("*.*", false).unwrap();
        // nothing is left out by default
        let mut w = Walker::new(keys(), Vec::new());
        assert_eq!(
            w.files_in(&b),
            vec![".hidden.txt", "a.txt", "ro.txt"],
            "no option, no filter"
        );
        let hidden = EnumOptions {
            ignore_hidden: true,
            ..EnumOptions::default()
        };
        let mut w = Walker::with_options(keys(), Vec::new(), hidden);
        // a dot-file is hidden where the file system has no attribute (not on Windows)
        let expected: Vec<&str> = if cfg!(windows) {
            vec![".hidden.txt", "a.txt", "ro.txt"]
        } else {
            vec!["a.txt", "ro.txt"]
        };
        assert_eq!(w.files_in(&b), expected);
        let folders: Vec<&str> = if cfg!(windows) {
            vec![".git", "src"]
        } else {
            vec!["src"]
        };
        assert_eq!(w.folders_in(&b), folders);
        let read_only = EnumOptions {
            ignore_read_only: true,
            ..EnumOptions::default()
        };
        let mut w = Walker::with_options(keys(), Vec::new(), read_only);
        assert_eq!(w.files_in(&b), vec![".hidden.txt", "a.txt"]);
        // the system attribute exists on Windows only
        let system = EnumOptions {
            ignore_system: true,
            ..EnumOptions::default()
        };
        let mut w = Walker::with_options(keys(), Vec::new(), system);
        assert_eq!(w.files_in(&b), vec![".hidden.txt", "a.txt", "ro.txt"]);
    }

    #[test]
    fn a_missing_folder_has_nothing() {
        let mut w = walker("*.*");
        assert!(w.files_in("/no/such/folder/for/matsu").is_empty());
        assert!(w.folders_in("/no/such/folder/for/matsu").is_empty());
    }
}
