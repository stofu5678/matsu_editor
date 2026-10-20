//! The menu of the drop-down arrow of the open button (`F_FILEOPEN_DROPDOWN`).
//!
//! Sakura: `TBN_DROPDOWN` (`CEditWnd::DispatchEvent`, `sakura_core/window/CEditWnd.cpp`) shows
//! `CEditWnd::CreateFileDropDownMenu` under the button: the recent files (`CMRUFile::CreateMenu`),
//! a separator when there are any, the recent folders as a submenu (grayed when there are none),
//! a separator, 履歴の管理 (`F_FAVORITE`), a separator, 新規作成, 新規ウインドウを開く and 開く. The
//! recent files and folders are the items of the File menu ([`crate::history::menu`]): choosing a
//! file opens it, and choosing a folder shows the open dialog in it (`IDM_SELOPENFOLDER`).

use ume_commands::keymap::KeyMap;
use ume_commands::sakura::FUNCTIONS;

use crate::history::menu::{F_FILE_USED_RECENTLY, F_FOLDER_USED_RECENTLY};
use crate::menu::{Available, Entry, Item, Lists, MenuAction, menu_label_with};

/// `F_FILENEW`, `F_FILENEW_NEWWINDOW`, `F_FILEOPEN` and `F_FAVORITE`.
const FILENEW: u16 = 30101;
const FILENEW_NEWWINDOW: u16 = 30110;
const FILEOPEN: u16 = 30102;
const FAVORITE: u16 = 31113;

/// The name of function `code` in the table (`LS( code )`).
fn name_of(code: u16) -> &'static str {
    FUNCTIONS
        .iter()
        .find(|f| f.code == code)
        .and_then(|f| f.label)
        .unwrap_or_default()
}

/// The rows of the menu. `lists` has the recent files and folders, `keymap` the keys shown next to
/// the last four items, and `available` says which functions have a command.
pub(crate) fn entries(
    lists: &dyn Lists,
    keymap: &KeyMap,
    available: &dyn Available,
    parentheses: bool,
) -> Vec<Entry> {
    let function = |code: u16, key: char| {
        Entry::Item(Item {
            label: menu_label_with(name_of(code), Some(key), parentheses),
            action: MenuAction::Function(code),
            shortcut: keymap.shortcut_for(code),
            enabled: available.is_available(code),
        })
    };
    let mut rows = Vec::new();
    let files = lists.items(F_FILE_USED_RECENTLY);
    if !files.is_empty() {
        rows.extend(files.into_iter().map(Entry::Item));
        rows.push(Entry::Separator);
    }
    let label = menu_label_with(name_of(F_FOLDER_USED_RECENTLY), None, parentheses);
    let folders = lists.items(F_FOLDER_USED_RECENTLY);
    rows.push(if folders.is_empty() {
        // `MF_GRAYED`.
        Entry::Item(Item {
            label,
            action: MenuAction::Placeholder,
            shortcut: None,
            enabled: false,
        })
    } else {
        Entry::Submenu {
            label,
            entries: folders.into_iter().map(Entry::Item).collect(),
        }
    });
    rows.push(Entry::Separator);
    rows.push(function(FAVORITE, 'M'));
    rows.push(Entry::Separator);
    rows.push(function(FILENEW, 'N'));
    rows.push(function(FILENEW_NEWWINDOW, 'M'));
    rows.push(function(FILEOPEN, 'O'));
    rows
}

impl crate::view::EditorView {
    /// `CreateFileDropDownMenu`: shows the menu with its top left corner at `at`.
    pub(crate) fn show_file_dropdown(
        &mut self,
        at: gpui::Point<gpui::Pixels>,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let recent = crate::history::menu::RecentLists::new(crate::history::current(cx))
            .with_names(crate::file_names::current(cx));
        let rows = entries(
            &recent,
            &self.keymap,
            &crate::menu::Registered,
            crate::doc_types::with_user(cx, |user| user.key_parentheses()),
        );
        self.open_popup_menu(rows, at, window, cx);
    }
}
