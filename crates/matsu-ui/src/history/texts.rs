//! The tabs of 履歴の管理 whose items are text: 検索, 置換, GREPファイル, GREPフォルダー, コマンド and
//! カレントディレクトリ (`m_cRecentSearch`, `m_cRecentReplace`, `m_cRecentGrepFile`,
//! `m_cRecentGrepFolder`, `m_cRecentCmd`, `m_cRecentCurDir` of `CDlgFavorite`).
//!
//! The lists live elsewhere: the first four in [`crate::search::history`] (`[Keys]` and `[Grep]`
//! of the settings file), the last two in [`crate::exec`] (`[Cmd]`). This module reads them as
//! one value ([`TextLists`]) for the dialog and changes them through [`EditableList`], which is
//! what `CRecentImp` gives Sakura's dialog.

use std::collections::BTreeSet;

use gpui::App;
use matsu_config::settings::{EditableList, ExecHistory, SearchKeys};

use super::dialog::Tab;

/// The lists of the six tabs, as they are now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextLists {
    pub keys: SearchKeys,
    pub exec: ExecHistory,
}

impl TextLists {
    /// The lists of the application.
    pub(crate) fn current(cx: &App) -> Self {
        Self {
            keys: crate::search::history::snapshot(cx),
            exec: crate::exec::history(cx),
        }
    }

    /// The strings of a text tab, newest first (empty for the other tabs).
    pub(crate) fn texts(&self, tab: Tab) -> &[String] {
        match tab {
            Tab::Search => self.keys.search.texts(),
            Tab::Replace => self.keys.replace.texts(),
            Tab::GrepFile => self.keys.grep_file.texts(),
            Tab::GrepFolder => self.keys.grep_folder.texts(),
            Tab::Command => self.exec.commands.texts(),
            Tab::CurDir => self.exec.cur_dirs.texts(),
            Tab::Files | Tab::Folders | Tab::Except => &[],
        }
    }
}

/// Runs `change` on the list of a text tab, in the application and in the settings file. Does
/// nothing for another tab.
pub(crate) fn change(
    cx: &mut App,
    tab: Tab,
    removes: bool,
    change: impl Fn(&mut dyn EditableList),
) {
    match tab {
        Tab::Search => {
            crate::search::history::edit_keys_with(cx, removes, |k| change(&mut k.search))
        }
        Tab::Replace => {
            crate::search::history::edit_keys_with(cx, removes, |k| change(&mut k.replace))
        }
        Tab::GrepFile => {
            crate::search::history::edit_keys_with(cx, removes, |k| change(&mut k.grep_file))
        }
        Tab::GrepFolder => {
            crate::search::history::edit_keys_with(cx, removes, |k| change(&mut k.grep_folder))
        }
        Tab::Command => crate::exec::edit_history_with(cx, removes, |h| change(&mut h.commands)),
        Tab::CurDir => crate::exec::edit_history_with(cx, removes, |h| change(&mut h.cur_dirs)),
        Tab::Files | Tab::Folders | Tab::Except => {}
    }
}

/// Whether an empty string may be in the list of the tab. Sakura's dialog adds one anywhere
/// (`CDlgInput1` takes an empty text). The replace strings keep an empty one because replacing with
/// nothing is a replacement (`AddToReplaceKeyArr`); the other lists have none in MATSU
/// (`SearchKeys::from_profile` leaves a blank line out of them), so an empty string is refused
/// there, and on the tab of the excluded paths, where it would keep every file out.
pub(crate) fn allows_empty(tab: Tab) -> bool {
    tab == Tab::Replace
}

/// The place of each of `texts` in the list now. Rows are told by their text, not by their number:
/// another window may have changed the order since the dialog showed it.
pub(crate) fn indices_of(list: &dyn EditableList, texts: &[String]) -> BTreeSet<usize> {
    texts
        .iter()
        .filter_map(|text| list.texts().iter().position(|t| t == text))
        .collect()
}

/// "選択項目" (`DeleteSelected`): removes the strings, the last first so that the numbers of the others
/// stay right.
pub(crate) fn delete_texts(list: &mut dyn EditableList, texts: &[String]) {
    for i in indices_of(list, texts).into_iter().rev() {
        list.delete_at(i);
    }
}

/// "存在しない項目" (`IDC_BUTTON_DELETE_NOTFOUND`): removes the strings that name a folder that is
/// gone. `exists` says whether a path is there.
pub(crate) fn delete_missing(
    list: &mut dyn EditableList,
    exists: impl Fn(&str) -> bool,
    folders: bool,
) {
    for i in (0..list.texts().len()).rev() {
        if !is_found(&list.texts()[i], &exists, folders) {
            list.delete_at(i);
        }
    }
}

/// Whether the string names something that is there. Sakura checks the whole string
/// (`CutLastYenFromDirectoryPath`, `IsFileExists(path, false)`), which counts a GREPフォルダー
/// string of several folders (`C:\a;C:\b`, which the Grep dialog takes) as gone even when they
/// all are there. Here such a string stays when any of its folders is there, split as the Grep
/// engine does (`split_folders`: `;` inside quotation marks does not split, and the marks go). That
/// is for the GREPフォルダー tab (`folders`); a current directory is one literal path.
fn is_found(text: &str, exists: &impl Fn(&str) -> bool, folders: bool) -> bool {
    exists(text)
        || (folders
            && text.contains([';', '"'])
            && crate::grep::keys::split_folders(text)
                .iter()
                .any(|part| exists(part)))
}

/// `AddItem` (the 追加 button): puts a new string at the front. `false` when the string is refused
/// (see [`EditableList::append_text`] and [`allows_empty`]).
pub(crate) fn add_text(list: &mut dyn EditableList, tab: Tab, text: &str) -> bool {
    (allows_empty(tab) || !text.is_empty()) && list.append_text(text)
}

/// `EditItem`: replaces the string `from` with `to` where it is. `false` when it is gone, or `to` is
/// refused.
pub(crate) fn edit_text(list: &mut dyn EditableList, tab: Tab, from: &str, to: &str) -> bool {
    let Some(index) = list.texts().iter().position(|t| t == from) else {
        return false;
    };
    (allows_empty(tab) || !to.is_empty()) && list.edit_text(index, to)
}
