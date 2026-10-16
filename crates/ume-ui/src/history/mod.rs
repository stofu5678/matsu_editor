//! The recent files and folders in the window (`ume_config::history`).
//!
//! * This file: the history of the application ([`HistoryState`], a global) and the three options of
//!   Sakura that say how it is used.
//! * [`menu`]: the list of recent files in the File menu (`CMRUFile::CreateMenu`).
//! * [`view`]: what the view does with it: remember a file when it is opened and closed, restore the
//!   caret, the view position and the bookmarks when it is opened again (`CMruListener`), and open a
//!   file of the menu.
//! * [`dialog`]: the window of `F_FAVORITE` (履歴の管理).
//!
//! Sakura keeps the history in shared memory and writes `sakura.ini` when the last window closes.
//! Here every change goes to the settings file at once ([`ume_config::history::HistoryStore`]), which
//! is safe for several windows and processes, so a crash loses nothing and another window sees the
//! change at its next menu. Without a store (the tests) the history lives in memory.
//!
//! # Options
//!
//! `[Common]` of the settings file, with Sakura's defaults: `m_bRestoreCurPosition` (1),
//! `m_bRestoreBookmarks` (1), `bQueryIfCodeChange` (1), and the number of files and folders shown,
//! `nMRUArrNum_MAX` and `nOPENFOLDERArrNum_MAX` (15 each, at most 36).

pub(crate) mod dialog;
pub(crate) mod menu;
pub(crate) mod view;

use gpui::{App, BorrowAppContext as _, Global};
use ume_config::Profile;
use ume_config::history::{FileEntry, History, HistoryStore};

use crate::settings::UserSettings;

/// How the history is used (`CommonSetting::m_sFile`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HistoryOptions {
    /// `m_bRestoreCurPosition`: the caret and the view go where they were when the file is opened.
    pub restore_cur_position: bool,
    /// `m_bRestoreBookmarks`: the bookmarks come back.
    pub restore_bookmarks: bool,
    /// `bQueryIfCodeChange`: ask when the encoding detected is not the one used last time.
    pub query_if_code_change: bool,
}

impl Default for HistoryOptions {
    fn default() -> Self {
        Self {
            restore_cur_position: true,
            restore_bookmarks: true,
            query_if_code_change: true,
        }
    }
}

impl HistoryOptions {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let get = |key: &str, default: bool| profile.get_bool("Common", key).unwrap_or(default);
        Self {
            restore_cur_position: get("m_bRestoreCurPosition", true),
            restore_bookmarks: get("m_bRestoreBookmarks", true),
            query_if_code_change: get("bQueryIfCodeChange", true),
        }
    }
}

/// Whether two paths that differ in case are the same file: on Windows and on macOS (whose file
/// systems are case-insensitive by default), as `_wcsicmp` makes them in Sakura.
pub(crate) const CASE_INSENSITIVE: bool = cfg!(any(windows, target_os = "macos"));

/// The history of the application.
pub(crate) struct HistoryState {
    store: Option<HistoryStore>,
    memory: History,
    pub(crate) options: HistoryOptions,
}

impl Global for HistoryState {}

impl HistoryState {
    /// The history in the settings file at `path`.
    pub(crate) fn on_disk(store: HistoryStore, options: HistoryOptions) -> Self {
        Self {
            memory: store.load(),
            store: Some(store),
            options,
        }
    }

    /// A history that is not saved: for tests, and for a system without a place for the file.
    pub(crate) fn in_memory(options: HistoryOptions) -> Self {
        Self {
            store: None,
            memory: History::new(15, 15, CASE_INSENSITIVE),
            options,
        }
    }

    /// The history as it is now (the file is read again, so what other windows added is in it).
    pub(crate) fn snapshot(&self) -> History {
        match &self.store {
            Some(store) => store.load(),
            None => self.memory.clone(),
        }
    }

    fn update(&mut self, change: impl FnOnce(&mut History)) -> Option<History> {
        match &self.store {
            Some(store) => store.update(change).ok(),
            None => {
                change(&mut self.memory);
                Some(self.memory.clone())
            }
        }
    }
}

/// Starts the history of the application from the settings that were read: the settings file's
/// `[MRU]` section. Call it before the menus are made.
pub(crate) fn install(cx: &mut App, user: &UserSettings) {
    let options = user
        .profile()
        .map_or_else(HistoryOptions::default, HistoryOptions::from_profile);
    let state = match crate::settings::history_file(user) {
        Some(path) => HistoryState::on_disk(HistoryStore::new(path, CASE_INSENSITIVE), options),
        None => HistoryState::in_memory(options),
    };
    cx.set_global(state);
}

/// The history, or `None` if the application keeps none.
pub(crate) fn current(cx: &App) -> Option<History> {
    cx.try_global::<HistoryState>().map(HistoryState::snapshot)
}

/// How the history is used. Sakura's defaults without a history.
pub(crate) fn options(cx: &App) -> HistoryOptions {
    cx.try_global::<HistoryState>()
        .map_or_else(HistoryOptions::default, |s| s.options)
}

/// Changes the history and brings the menus up to date. Without a history nothing happens.
pub(crate) fn update(cx: &mut App, change: impl FnOnce(&mut History)) -> Option<History> {
    if !cx.has_global::<HistoryState>() {
        return None;
    }
    let result = cx.update_global(|state: &mut HistoryState, _| state.update(change));
    crate::app::refresh_menus(cx);
    result
}

/// Adds a file (and its folder) at the front of the history.
pub(crate) fn add_file(cx: &mut App, entry: FileEntry) {
    update(cx, |h| h.add_file(entry));
}

#[cfg(test)]
mod dialog_tests;
#[cfg(test)]
mod menu_tests;
#[cfg(test)]
mod tests;
