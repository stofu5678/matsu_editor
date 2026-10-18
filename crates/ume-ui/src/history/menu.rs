//! The recent files in the File menu.
//!
//! Sakura: `CMRUFile::CreateMenu` (`sakura_core/recent/CMRUFile.cpp`) adds one item for each of the
//! first "ファイルの履歴MAX" files; `CFileNameManager::GetMenuFullLabel` makes the label
//! (`sakura_core/env/CFileNameManager.cpp`): the access key `1` to `9`, `A` to `Z`, `0`, a star
//! for a favorite, the path, and the encoding in brackets unless it is Shift_JIS.
//!
//! The recent folders (`CMRUFolder::CreateMenu`, `sakura_core/recent/CMRUFolder.cpp`) are listed the
//! same way. Choosing a folder shows the open dialog in it (`IDM_SELOPENFOLDER`,
//! `CEditWnd::OnCommand` in `sakura_core/window/CEditWnd.cpp`): [`OpenRecentFolder`] runs
//! [`EditorView::open_recent_folder`].

use ume_config::history::{FileEntry, FolderEntry, History};
use ume_core::encoding::Encoding;

use crate::file_names::FileNames;
use crate::menu::{Item, Lists, MenuAction};
use crate::window_ops::list::access_key;

/// `F_FILE_USED_RECENTLY`, the row of the File menu that stands for the recent files.
pub(crate) const F_FILE_USED_RECENTLY: u16 = 29002;

/// `F_FOLDER_USED_RECENTLY`, the row that stands for the recent folders.
pub(crate) const F_FOLDER_USED_RECENTLY: u16 = 29003;

/// The text of a recent file in the menu (`GetMenuFullLabel` with a path). Sakura puts `&` before
/// the access key and doubles `&` in the path; a GPUI menu has no mnemonics, so neither is done.
///
/// `shown` is the path as 「ファイル名表示」 makes it ([`FileNames::transform`]).
pub(crate) fn label(index: usize, entry: &FileEntry, shown: &str) -> String {
    let charset = match Encoding::from_sakura_code(entry.encoding_code) {
        Some(Encoding::ShiftJis) | None => String::new(),
        Some(encoding) => encoding.bracket_name(),
    };
    format!(
        "{} {}{}{}",
        access_key(index),
        if entry.favorite { "★ " } else { "" },
        shown,
        charset
    )
}

/// The text of a recent folder in the menu (`GetMenuFullLabel` with a folder: no encoding).
pub(crate) fn folder_label(index: usize, entry: &FolderEntry, shown: &str) -> String {
    format!(
        "{} {}{}",
        access_key(index),
        if entry.favorite { "★ " } else { "" },
        shown
    )
}

/// The run-time lists of the main menu that the history fills in.
pub(crate) struct RecentLists {
    history: Option<History>,
    /// How the paths are shown (`GetMenuFullLabel_MRU` shortens them with 「ファイル名表示」).
    names: FileNames,
}

impl RecentLists {
    /// The lists with the paths as they are.
    pub(crate) fn new(history: Option<History>) -> Self {
        Self {
            history,
            names: FileNames::plain(),
        }
    }

    /// The lists with the paths as `names` shows them.
    pub(crate) fn with_names(mut self, names: FileNames) -> Self {
        self.names = names;
        self
    }
}

impl Lists for RecentLists {
    fn items(&self, code: u16) -> Vec<Item> {
        let Some(history) = &self.history else {
            return Vec::new();
        };
        match code {
            F_FILE_USED_RECENTLY => history
                .files
                .visible()
                .iter()
                .enumerate()
                .map(|(i, entry)| Item {
                    label: label(i, entry, &self.names.transform(&entry.path, true)),
                    action: MenuAction::RecentFile(entry.path.clone()),
                    shortcut: None,
                    enabled: true,
                })
                .collect(),
            F_FOLDER_USED_RECENTLY => history
                .folders
                .visible()
                .iter()
                .enumerate()
                .map(|(i, entry)| Item {
                    label: folder_label(i, entry, &self.names.transform(&entry.path, true)),
                    action: MenuAction::RecentFolder(entry.path.clone()),
                    shortcut: None,
                    enabled: true,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}
