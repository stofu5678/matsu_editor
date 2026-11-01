//! A guard against leftovers of the old product name (ADR-0021).
//!
//! The product was called "UME Editor" until 0.1.0. Since the rename to MATSU Editor, the old name must not
//! come back by accident: this test fails when the old name is in a tracked file outside the allow-list below.
//! It looks for the word (an `ume` that is not inside a longer word such as `document` or `resume`, in lower,
//! upper and title case) and for the katakana form, in every text file of the repository.
//!
//! To allow a new place, add it to [`ALLOWED`] with the reason. Keep the list short.

use std::fs;
use std::path::{Path, PathBuf};

/// Places where the old name stays on purpose: (path prefix from the repository root, why).
const ALLOWED: &[(&str, &str)] = &[
    (
        "CHANGELOG.md",
        "history: written by release-please for the releases that were made under the old name",
    ),
    (
        "docs/adr/0021-rename-to-matsu.md",
        "history: the decision to rename names the old product, its files and its folders",
    ),
    (
        "docs/adr/0025-script-languages.md",
        "says which old names ADR-0007 was first written with",
    ),
    (
        "docs/poc/",
        "history: the records of the prototypes, written when the old name was in use",
    ),
    (
        "poc/",
        "history: throwaway prototypes outside the workspace and CI; not renamed",
    ),
    (
        "crates/matsu-config/src/migrate_old_name.rs",
        "the migration of the old settings (until 1.0) must name the old file, folder and product",
    ),
    (
        "crates/matsu-config/src/migrate_old_name/",
        "the tests of that migration",
    ),
    (
        "crates/matsu/tests/leftover_names.rs",
        "this guard names the old word in its own explanation",
    ),
];

/// Folders that are not part of the repository's own text.
const SKIPPED_DIRS: &[&str] = &[".git", "target", ".claude", "vendor", "node_modules"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        let name = entry.file_name();
        if SKIPPED_DIRS.iter().any(|s| name == *s) {
            // (`.git` is a file in a worktree.)
            continue;
        }
        if kind.is_dir() {
            files_under(&path, out);
        } else if kind.is_file() {
            out.push(path);
        }
    }
}

/// The places in `text` where the old name is, as (line number, line).
fn hits(text: &str) -> Vec<(usize, String)> {
    let word = ["ume", "UME", "Ume"];
    let mut found = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut bad = line.contains("ウメ");
        for w in word {
            for (at, _) in line.match_indices(w) {
                let before = at.checked_sub(1).map(|b| bytes[b]);
                let after = bytes.get(at + w.len()).copied();
                let inside_word = before.is_some_and(|b| b.is_ascii_alphabetic())
                    || after.is_some_and(|a| a.is_ascii_lowercase())
                    // A longer word in capitals (UMEDIA); `Ume` is the head of a name like `UmeBoot`.
                    || (w == "UME" && after.is_some_and(|a| a.is_ascii_uppercase()));
                if !inside_word {
                    bad = true;
                }
            }
        }
        if bad {
            found.push((i + 1, line.trim().chars().take(120).collect()));
        }
    }
    found
}

#[test]
fn the_old_name_is_only_where_the_allow_list_says() {
    let root = repository_root();
    // A source archive without the repository still has the files; the guard does not need git.
    let mut files = Vec::new();
    files_under(&root, &mut files);
    assert!(
        files.len() > 100,
        "the repository was not found at {root:?}"
    );
    let mut report = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if ALLOWED
            .iter()
            .any(|(prefix, _)| relative.starts_with(prefix))
        {
            continue;
        }
        // Binary files (pictures, fonts, archives) are not text.
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        for (line, content) in hits(&text) {
            report.push(format!("{relative}:{line}: {content}"));
        }
    }
    assert!(
        report.is_empty(),
        "the old product name is in files outside the allow-list of tests/leftover_names.rs (ADR-0021):\n{}",
        report.join("\n")
    );
}

#[test]
fn the_guard_sees_the_old_name_and_not_longer_words() {
    assert_eq!(hits("a UME Editor b").len(), 1);
    assert_eq!(hits("crates/ume-core").len(), 1);
    assert_eq!(hits("ume_config::x").len(), 1);
    assert_eq!(hits("UmeBoot").len(), 1);
    assert_eq!(hits("ウメエディタ").len(), 1);
    assert_eq!(hits("document resume assume numeric UMEDIA").len(), 0);
    assert_eq!(
        hits("see https://github.com/stofu5678/ume_editor/blob/main/x").len(),
        1
    );
}

#[test]
fn every_allowed_place_exists() {
    let root = repository_root();
    for (prefix, why) in ALLOWED {
        assert!(!why.is_empty());
        assert!(
            root.join(prefix).exists(),
            "{prefix} is in the allow-list but not in the repository"
        );
    }
}
