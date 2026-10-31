//! The lists of file types of the macro dialogs.
//!
//! * The dialog of `F_EXECEXTMACRO` ([`runnable`]): すべての対応マクロ, then one entry per
//!   language, then すべてのファイル.
//! * The dialogs of `F_LOADKEYMACRO` and `F_SAVEKEYMACRO` ([`key_macro`]): キーマクロ (`*.mac`)
//!   and すべてのファイル (Sakura: `Command_LOADKEYMACRO`, `Command_SAVEKEYMACRO` ask for
//!   `*.mac`).
//!
//! The languages and their extensions come from `matsu_plugin::session::Language::file_types`,
//! which reads the one table of extensions; a language this OS cannot run is left out.

use matsu_platform::dialog::FileFilter;
use matsu_plugin::session::Language;

/// The extension of key macro files.
const KEY_MACRO_EXTENSION: &str = "mac";

fn patterns(extensions: &[&str]) -> Vec<String> {
    extensions.iter().map(|ext| format!("*.{ext}")).collect()
}

/// An entry that shows its patterns after its name.
fn entry(name: &str, extensions: &[&str]) -> FileFilter {
    let patterns = patterns(extensions);
    FileFilter {
        name: format!("{name} ({})", patterns.join(";")),
        patterns,
    }
}

fn all_files() -> FileFilter {
    FileFilter {
        name: "すべてのファイル".to_owned(),
        patterns: vec!["*.*".to_owned()],
    }
}

/// The file types of the dialogs that choose a macro to run.
pub(crate) fn runnable() -> Vec<FileFilter> {
    let languages = Language::file_types();
    let mut every: Vec<&str> = vec![KEY_MACRO_EXTENSION];
    every.extend(
        languages
            .iter()
            .flat_map(|(_, extensions)| extensions.iter().copied()),
    );
    let mut list = vec![FileFilter {
        name: "すべての対応マクロ".to_owned(),
        patterns: patterns(&every),
    }];
    list.push(entry("キーマクロ", &[KEY_MACRO_EXTENSION]));
    for (language, extensions) in &languages {
        list.push(entry(language.display_name(), extensions));
    }
    list.push(all_files());
    list
}

/// The file types of the dialogs that load or save a key macro.
pub(crate) fn key_macro() -> Vec<FileFilter> {
    vec![entry("キーマクロ", &[KEY_MACRO_EXTENSION]), all_files()]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::macros::{FileKind, file_kind};

    #[test]
    fn the_run_dialog_lists_every_macro_then_the_key_macro_and_each_language_then_every_file() {
        let list = runnable();
        assert_eq!(list[0].name, "すべての対応マクロ");
        assert_eq!(list[1].name, "キーマクロ (*.mac)");
        let last = list.last().unwrap();
        assert_eq!(last.name, "すべてのファイル");
        assert!(last.matches_all());
        let per_language = &list[2..list.len() - 1];
        assert_eq!(per_language.len(), Language::file_types().len());
        for (filter, (language, _)) in per_language.iter().zip(Language::file_types()) {
            assert!(filter.name.starts_with(language.display_name()));
        }
    }

    #[test]
    fn the_first_entry_holds_every_extension_of_the_others() {
        let list = runnable();
        let mut each: Vec<&str> = list[1..list.len() - 1]
            .iter()
            .flat_map(FileFilter::extensions)
            .collect();
        let mut all = list[0].extensions();
        each.sort_unstable();
        all.sort_unstable();
        assert_eq!(all, each);
    }

    #[test]
    fn the_list_and_the_table_of_extensions_agree() {
        let list = runnable();
        let listed = list[0].extensions();
        // every extension of the table that this OS can run is listed, and no other
        for (ext, language) in Language::FILE_EXTENSIONS {
            assert_eq!(listed.contains(&ext), language.runs_here(), "{ext}");
        }
        assert_eq!(
            listed.len(),
            1 + Language::FILE_EXTENSIONS
                .iter()
                .filter(|(_, language)| language.runs_here())
                .count()
        );
        // and the macros accept every listed extension, with the language it is listed under
        for (language, extensions) in Language::file_types() {
            for ext in extensions {
                assert_eq!(
                    file_kind(&PathBuf::from(format!("a.{ext}"))),
                    FileKind::Script(language),
                    "{ext}"
                );
            }
        }
        assert_eq!(file_kind(&PathBuf::from("a.mac")), FileKind::KeyMacro);
    }

    #[test]
    fn applescript_is_listed_on_macos_only() {
        let listed = runnable()
            .iter()
            .any(|filter| filter.extensions().contains(&"applescript"));
        assert_eq!(listed, cfg!(target_os = "macos"));
    }

    #[test]
    fn the_key_macro_dialogs_list_mac_and_every_file() {
        let list = key_macro();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "キーマクロ (*.mac)");
        assert_eq!(list[0].patterns, ["*.mac"]);
        assert_eq!(list[1].name, "すべてのファイル");
        assert!(list[1].matches_all());
    }
}
