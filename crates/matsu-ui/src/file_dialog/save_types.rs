//! The file types and the first name of the save dialog (`CDocFileOperation::SaveFileDialog`,
//! `sakura_core/doc/CDocFileOperation.cpp`; the list is that of
//! `CDlgOpenFile_CommonItemDialog::DoModalSaveDlgImpl1`).
//!
//! * Sakura's list has three entries (`STR_DLGOPNFL_EXTNAME1` to `3`): ユーザー指定 (the
//!   extensions of the type of the document, see `matsu_config::save_filter`), テキストファイル
//!   (`*.txt`) and すべてのファイル (`*.*`). MATSU Editor puts every other document type that
//!   has extensions (`name (*.c;*.h)`, in the order of the settings) between the last two. The
//!   first is the one shown. A type that has no extensions leaves the first entry out (Sakura
//!   gives the dialog an empty one).
//! * Every save dialog has the default extension `txt` (`SetDefaultExtension(L"txt")`, whatever
//!   the document is); the dialog changes it to the extension of the file type chosen.
//! * A document without a file is offered `無題_<date>_<time>` (`STR_NO_TITLE2` and
//!   `_%Y%m%d_%H%M%S`). Sakura puts the number of the window after 無題 (`m_nId`); MATSU
//!   Editor has no such number, and the name goes without it (Sakura does the same for a window
//!   that has none).
//!
//! The list is shown by the dialogs of the OS (`matsu_platform::dialog`); GPUI's dialog, the
//! fallback, has no list: it shows every file, and the default extension is added to the name it
//! answers (`matsu_platform::dialog::with_default_extension`), the one of the type shown first.

use std::path::Path;

use chrono::NaiveDateTime;
use gpui::App;
use matsu_config::save_filter::{SaveWildcard, types_ext_to_dialog_ext};
use matsu_platform::dialog::FileFilter;

/// The default extension of a document without a file (`SetDefaultExtension(L"txt")`).
const DEFAULT_EXTENSION: &str = "txt";

/// What a save dialog is given besides the name and the folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SaveTypes {
    pub filters: Vec<FileFilter>,
    pub default_extension: Option<String>,
}

/// The name offered to a document without a file (`STR_NO_TITLE2`, `_%Y%m%d_%H%M%S`).
pub(crate) fn untitled_name(now: NaiveDateTime) -> String {
    format!("無題{}", now.format("_%Y%m%d_%H%M%S"))
}

/// The list of file types for the wildcard of ユーザー指定 (`a;b` is `a` and `b`).
pub(crate) fn filters(wildcard: &str) -> Vec<FileFilter> {
    filters_with_types(wildcard, &[])
}

/// [`filters`] with the document types between テキストファイル and すべてのファイル. Each of
/// `types` is a type name and its extension list (`"c cpp;.h"`) in the order of the settings;
/// the dialog lists a type as `name (*.c;*.cpp;*.h)`. A type without extensions is left out, and
/// so is one whose patterns an earlier entry has already (the type of the document is the
/// ユーザー指定 entry, and `*.txt` is テキストファイル).
pub(crate) fn filters_with_types(wildcard: &str, types: &[(&str, &str)]) -> Vec<FileFilter> {
    let own: Vec<String> = wildcard
        .split(';')
        .filter(|pattern| !pattern.is_empty())
        .map(str::to_owned)
        .collect();
    let mut list = Vec::new();
    if !own.is_empty() {
        list.push(FileFilter {
            name: "ユーザー指定".to_owned(),
            patterns: own,
        });
    }
    list.push(FileFilter {
        name: "テキストファイル".to_owned(),
        patterns: vec!["*.txt".to_owned()],
    });
    for (name, exts) in types {
        let patterns: Vec<String> = types_ext_to_dialog_ext(exts, None)
            .split(';')
            .filter(|pattern| !pattern.is_empty())
            .map(str::to_owned)
            .collect();
        if patterns.is_empty() || list.iter().any(|f| same_patterns(&f.patterns, &patterns)) {
            continue;
        }
        list.push(FileFilter {
            name: (*name).to_owned(),
            patterns,
        });
    }
    list.push(FileFilter {
        name: "すべてのファイル".to_owned(),
        patterns: vec!["*.*".to_owned()],
    });
    list
}

/// Whether two pattern lists name the same files, whatever the order and the case.
fn same_patterns(a: &[String], b: &[String]) -> bool {
    let sorted = |list: &[String]| {
        let mut list: Vec<String> = list.iter().map(|p| p.to_lowercase()).collect();
        list.sort();
        list.dedup();
        list
    };
    sorted(a) == sorted(b)
}

/// The file types and the default extension for a document of type `type_index` whose file is
/// `path` (`None`: it has none).
pub(crate) fn for_document(cx: &App, type_index: usize, path: Option<&Path>) -> SaveTypes {
    let prefs = crate::file_ops::prefs::prefs(cx);
    // (name, extensions) of every type; the type of the document is the ユーザー指定 entry, so
    // it is not listed again
    let (type_exts, others) = crate::doc_types::with_user(cx, |user| {
        let types = user.document_types();
        let own = types
            .get(type_index)
            .map(|t| t.type_exts.clone())
            .unwrap_or_default();
        let others: Vec<(String, String)> = types
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != type_index)
            .map(|(_, t)| (t.type_name.clone(), t.type_exts.clone()))
            .collect();
        (own, others)
    });
    let others: Vec<(&str, &str)> = others
        .iter()
        .map(|(name, exts)| (name.as_str(), exts.as_str()))
        .collect();
    let path_text = path.map(|p| p.to_string_lossy().into_owned());
    let wildcard = SaveWildcard {
        basic_type: type_index == 0,
        type_exts: &type_exts,
        path: path_text.as_deref(),
        no_filter_save_new: prefs.no_filter_save_new,
        no_filter_save_file: prefs.no_filter_save_file,
    }
    .wildcard();
    SaveTypes {
        filters: filters_with_types(&wildcard, &others),
        default_extension: Some(DEFAULT_EXTENSION.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn names(list: &[FileFilter]) -> Vec<&str> {
        list.iter().map(|f| f.name.as_str()).collect()
    }

    #[test]
    fn the_name_of_a_new_document_has_the_date_and_the_time() {
        let now = NaiveDate::from_ymd_opt(2026, 10, 9)
            .unwrap()
            .and_hms_opt(3, 4, 5)
            .unwrap();
        assert_eq!(untitled_name(now), "無題_20261009_030405");
    }

    #[test]
    fn the_list_has_sakuras_three_entries() {
        let list = filters("*.cpp;*.h;*.*");
        assert_eq!(
            names(&list),
            ["ユーザー指定", "テキストファイル", "すべてのファイル"]
        );
        assert_eq!(list[0].patterns, ["*.cpp", "*.h", "*.*"]);
        assert_eq!(list[1].patterns, ["*.txt"]);
        assert_eq!(list[2].patterns, ["*.*"]);
    }

    fn types(list: &[FileFilter]) -> Vec<(&str, Vec<&str>)> {
        list.iter()
            .map(|f| {
                (
                    f.name.as_str(),
                    f.patterns.iter().map(String::as_str).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn the_document_types_go_between_the_text_file_and_all_files() {
        let list = filters_with_types(
            "*.c;*.cpp",
            &[
                ("基本", ""),
                ("C/C++", "c cpp;.h"),
                ("HTML", "html,htm"),
                ("Markdown", "md"),
            ],
        );
        assert_eq!(
            types(&list),
            [
                ("ユーザー指定", vec!["*.c", "*.cpp"]),
                ("テキストファイル", vec!["*.txt"]),
                ("C/C++", vec!["*.c", "*.cpp", "*.h"]),
                ("HTML", vec!["*.html", "*.htm"]),
                ("Markdown", vec!["*.md"]),
                ("すべてのファイル", vec!["*.*"]),
            ]
        );
    }

    #[test]
    fn a_type_without_extensions_or_with_known_patterns_is_left_out() {
        let list = filters_with_types(
            "*.md;*.*",
            &[
                ("基本", ""),
                ("空白", " ; , "),
                ("テキスト", "TXT"),
                ("Markdown", "md"),
                ("別の名前", "MD"),
                ("Notes", "txt,log"),
            ],
        );
        // Markdown is ユーザー指定 plus *.* (not the same patterns), so it is listed once
        assert_eq!(
            names(&list),
            [
                "ユーザー指定",
                "テキストファイル",
                "Markdown",
                "Notes",
                "すべてのファイル"
            ]
        );
    }

    #[test]
    fn types_of_the_same_name_are_both_listed_when_their_extensions_differ() {
        let list = filters_with_types("", &[("Web", "html"), ("Web", "css"), ("Web", "css")]);
        assert_eq!(
            types(&list),
            [
                ("テキストファイル", vec!["*.txt"]),
                ("Web", vec!["*.html"]),
                ("Web", vec!["*.css"]),
                ("すべてのファイル", vec!["*.*"]),
            ]
        );
    }

    #[test]
    fn a_type_lists_its_extensions_as_wildcards() {
        let list = filters_with_types("", &[("C", "c;h cpp,.hpp,m*")]);
        assert_eq!(list[1].patterns, ["*.c", "*.h", "*.cpp", "*.hpp", "m*"]);
    }

    #[test]
    fn an_empty_wildcard_leaves_the_first_entry_out() {
        assert_eq!(
            names(&filters("")),
            ["テキストファイル", "すべてのファイル"]
        );
        assert_eq!(
            names(&filters(";")),
            ["テキストファイル", "すべてのファイル"]
        );
    }
}
