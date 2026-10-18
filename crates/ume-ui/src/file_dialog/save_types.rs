//! The file types and the first name of the save dialog (`CDocFileOperation::SaveFileDialog`,
//! `sakura_core/doc/CDocFileOperation.cpp`; the list is that of
//! `CDlgOpenFile_CommonItemDialog::DoModalSaveDlgImpl1`).
//!
//! * The list has three entries, as Sakura's (`STR_DLGOPNFL_EXTNAME1` to `3`): ユーザー指定 (the
//!   extensions of the type of the document, see `ume_config::save_filter`), テキストファイル
//!   (`*.txt`) and すべてのファイル (`*.*`). The first is the one shown. A type that has no
//!   extensions leaves the first entry out (Sakura gives the dialog an empty one).
//! * A document without a file is offered `無題_<date>_<time>` (`STR_NO_TITLE2` and
//!   `_%Y%m%d_%H%M%S`) and the extension `txt` is added to a name typed without one
//!   (`SetDefaultExtension`). Sakura puts the number of the window after 無題 (`m_nId`); UME
//!   Editor has no such number, and the name goes without it (Sakura does the same for a window
//!   that has none).
//!
//! The list is shown by the dialogs of the OS (`ume_platform::dialog`); GPUI's dialog, the
//! fallback, has no list: it shows every file, and the default extension is added to the name it
//! answers (`ume_platform::dialog::with_default_extension`).

use std::path::Path;

use chrono::NaiveDateTime;
use gpui::App;
use ume_config::save_filter::SaveWildcard;
use ume_platform::dialog::FileFilter;

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
    list.push(FileFilter {
        name: "すべてのファイル".to_owned(),
        patterns: vec!["*.*".to_owned()],
    });
    list
}

/// The file types and the default extension for a document of type `type_index` whose file is
/// `path` (`None`: it has none).
pub(crate) fn for_document(cx: &App, type_index: usize, path: Option<&Path>) -> SaveTypes {
    let prefs = crate::file_ops::prefs::prefs(cx);
    let type_exts = crate::doc_types::with_user(cx, |user| {
        user.document_types()
            .get(type_index)
            .map(|t| t.type_exts.clone())
            .unwrap_or_default()
    });
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
        filters: filters(&wildcard),
        default_extension: path.is_none().then(|| DEFAULT_EXTENSION.to_owned()),
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
