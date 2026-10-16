//! The recent files in the File menu.
//!
//! Sakura: `CMRUFile::CreateMenu` (`sakura_core/recent/CMRUFile.cpp`) adds one item for each of the
//! first "ファイルの履歴MAX" files; `CFileNameManager::GetMenuFullLabel` makes the label
//! (`sakura_core/env/CFileNameManager.cpp`): the access key `1` to `9`, `A` to `Z`, `0`, a star
//! for a favorite, the path, and the encoding in brackets unless it is Shift_JIS.
//!
//! The recent folders are not in the menu: choosing a folder opens the open dialog in it
//! (`SelectOpenFileFromFolder`, `sakura_core/_main/CControlTray.cpp`), and the dialog of the OS
//! that GPUI shows cannot be told where to start.

use ume_config::history::{FileEntry, History};
use ume_core::encoding::Encoding;

use crate::menu::{Item, Lists, MenuAction};
use crate::window_ops::list::access_key;

/// `F_FILE_USED_RECENTLY`, the row of the File menu that stands for the recent files.
pub(crate) const F_FILE_USED_RECENTLY: u16 = 29002;

/// The text of a recent file in the menu (`GetMenuFullLabel` with a path). Sakura puts `&` before
/// the access key and doubles `&` in the path; a GPUI menu has no mnemonics, so neither is done.
pub(crate) fn label(index: usize, entry: &FileEntry) -> String {
    let charset = match Encoding::from_sakura_code(entry.encoding_code) {
        Some(Encoding::ShiftJis) | None => String::new(),
        Some(encoding) => encoding.bracket_name(),
    };
    format!(
        "{} {}{}{}",
        access_key(index),
        if entry.favorite { "★ " } else { "" },
        entry.path,
        charset
    )
}

/// The run-time lists of the main menu that the history fills in.
pub(crate) struct RecentLists {
    history: Option<History>,
}

impl RecentLists {
    pub(crate) fn new(history: Option<History>) -> Self {
        Self { history }
    }
}

impl Lists for RecentLists {
    fn items(&self, code: u16) -> Vec<Item> {
        let Some(history) = &self.history else {
            return Vec::new();
        };
        if code != F_FILE_USED_RECENTLY {
            return Vec::new();
        }
        history
            .files
            .visible()
            .iter()
            .enumerate()
            .map(|(i, entry)| Item {
                label: label(i, entry),
                action: MenuAction::RecentFile(entry.path.clone()),
                shortcut: None,
                enabled: true,
            })
            .collect()
    }
}
