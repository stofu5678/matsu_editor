//! The history of the search and replace strings that every window of the application shares
//! (Sakura's `m_sSearchKeywords.m_aSearchKeys` and `m_aReplaceKeys`, `CSearchKeywordManager`),
//! and the sequence numbers that tell a window whether the history has news for it
//! (`m_nSearchKeySequence`, `m_nReplaceKeySequence` in the shared data; `m_nCurSearchKeySequence`
//! in the view; `sakura_core/view/CEditView_Search.cpp`, `GetCurrentTextForSearchDlg`).
//!
//! The lists and the `[Keys]` section of the settings file are [`ume_config::settings::SearchKeys`].
//! This module keeps one copy in the application ([`SearchHistory`], a GPUI global) and writes
//! the file when a string is added. Sakura writes the file when the last window closes; the
//! history is small and the user may close the application with a signal, so it is written at once,
//! reading the file again first so that the other sections (written by other parts of the
//! application) are not overwritten. Without a place to write (the tests) nothing is written.

use std::path::{Path, PathBuf};

use gpui::{App, Global};
use ume_config::Profile;
use ume_config::history::update_profile;
use ume_config::settings::SearchKeys;

use crate::view::EditorView;

/// What the application remembers: the strings and where they are saved.
#[derive(Default)]
pub struct SearchHistory {
    keys: SearchKeys,
    /// `m_nSearchKeySequence`: counts the additions to the search strings.
    search_sequence: u64,
    /// `m_nReplaceKeySequence`.
    replace_sequence: u64,
    /// The settings file to write the strings to.
    store: Option<PathBuf>,
}

impl Global for SearchHistory {}

impl SearchHistory {
    /// The history in `settings` (the file the application read, if there was one), to be saved to
    /// `store`.
    pub fn load(settings: Option<&Path>, store: Option<PathBuf>) -> Self {
        let keys = settings
            .and_then(|path| Profile::load(path).ok())
            .map(|profile| SearchKeys::from_profile(&profile))
            .unwrap_or_default();
        Self {
            keys,
            store,
            ..Self::default()
        }
    }

    pub fn keys(&self) -> &SearchKeys {
        &self.keys
    }

    /// Adds a string to the history in the settings file, leaving the rest of the file alone.
    /// `add` is applied to the strings the file has *now*, under the lock that every writer of the
    /// file takes (`ume_config::history::update_profile`), so what another window or process
    /// saved at the same moment (a search string, a replacement, a recent file) is not lost; the
    /// merged strings replace this copy. A file that cannot be read is not touched.
    pub(super) fn persist(&mut self, add: impl FnOnce(&mut SearchKeys)) {
        let Some(path) = &self.store else { return };
        let mut merged = None;
        // A history that cannot be saved is not worth stopping a search for.
        let saved = update_profile(path, |profile| {
            let mut keys = SearchKeys::from_profile(profile);
            add(&mut keys);
            let written = keys.write_to(profile).is_ok();
            merged = Some(keys);
            Ok(written)
        });
        if saved.is_ok()
            && let Some(keys) = merged
        {
            self.keys = keys;
        }
    }
}

/// Makes the history of the application. Called once at startup.
pub fn install(cx: &mut App, history: SearchHistory) {
    cx.set_global(history);
}

/// The search strings, newest first.
pub fn search_keys(cx: &App) -> Vec<String> {
    cx.try_global::<SearchHistory>()
        .map(|h| h.keys.search.items().to_vec())
        .unwrap_or_default()
}

/// The replace strings, newest first.
pub fn replace_keys(cx: &App) -> Vec<String> {
    cx.try_global::<SearchHistory>()
        .map(|h| h.keys.replace.items().to_vec())
        .unwrap_or_default()
}

/// `AddToSearchKeyArr`. Returns the sequence number to put in the view that searched.
pub fn remember_search(cx: &mut App, text: &str) -> u64 {
    let history = cx.default_global::<SearchHistory>();
    let changed = history.keys.add_search(text);
    history.search_sequence += 1;
    if changed {
        history.persist(|keys| {
            keys.add_search(text);
        });
    }
    history.search_sequence
}

/// `AddToReplaceKeyArr`. Returns the sequence number to put in the view that replaced.
pub fn remember_replace(cx: &mut App, text: &str) -> u64 {
    let history = cx.default_global::<SearchHistory>();
    let changed = history.keys.add_replace(text);
    history.replace_sequence += 1;
    if changed {
        history.persist(|keys| {
            keys.add_replace(text);
        });
    }
    history.replace_sequence
}

/// The newest search string and the sequence number it came with, if a window that has seen
/// `seen` has not seen it: `0 < size() && m_nCurSearchKeySequence < m_nSearchKeySequence`.
pub fn newer_search_key(cx: &App, seen: Option<u64>) -> Option<(String, u64)> {
    let history = cx.try_global::<SearchHistory>()?;
    let newest = history.keys.search.newest()?;
    seen.is_none_or(|s| s < history.search_sequence)
        .then(|| (newest.to_owned(), history.search_sequence))
}

/// The same for the replace strings (`Command_REPLACE_DIALOG`).
pub fn newer_replace_key(cx: &App, seen: Option<u64>) -> Option<(String, u64)> {
    let history = cx.try_global::<SearchHistory>()?;
    let newest = history.keys.replace.newest()?;
    seen.is_none_or(|s| s < history.replace_sequence)
        .then(|| (newest.to_owned(), history.replace_sequence))
}

impl EditorView {
    /// Puts the pattern of the search in the history, as Sakura's dialogs, the search mark and the
    /// end of the incremental search do. A pattern that is empty or is a regular expression that
    /// does not compile is not kept (`CDlgFind::GetData` checks the syntax first).
    pub(crate) fn remember_search_key(&mut self, cx: &mut App) {
        let pattern = self.search.pattern().to_owned();
        if pattern.is_empty() || self.search.pattern_error().is_some() {
            return;
        }
        let sequence = remember_search(cx, &pattern);
        self.search.set_key_sequence(Some(sequence));
    }

    /// Puts the replacement in the history (`CDlgReplace::GetData`).
    pub(crate) fn remember_replace_key(&mut self, cx: &mut App) {
        let text = self.search.replace.text.clone();
        let sequence = remember_replace(cx, &text);
        self.search.set_replace_sequence(Some(sequence));
    }
}
