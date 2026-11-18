//! The history of the search and replace strings that every window of the application shares
//! (Sakura's `m_sSearchKeywords.m_aSearchKeys` and `m_aReplaceKeys`, `CSearchKeywordManager`),
//! with the strings of the Grep boxes (`m_aGrepFiles` and the like) and the tag jump keywords
//! (`m_sTagJump.m_aTagJumpKeywords`) that are kept in the same way, and the sequence numbers that tell a window whether the history has news for it
//! (`m_nSearchKeySequence`, `m_nReplaceKeySequence` in the shared data; `m_nCurSearchKeySequence`
//! in the view; `sakura_core/view/CEditView_Search.cpp`, `GetCurrentTextForSearchDlg`).
//!
//! The lists and the sections of the settings file are [`matsu_config::settings::SearchKeys`].
//! This module keeps one copy in the application ([`SearchHistory`], a GPUI global) and writes
//! the file when a string is added. Sakura writes the file when the last window closes; the
//! history is small and the user may close the application with a signal, so it is written at once,
//! reading the file again first so that the other sections (written by other parts of the
//! application) are not overwritten. Without a place to write (the tests) nothing is written.

use std::path::{Path, PathBuf};

use gpui::{App, BorrowAppContext as _, Global};
use matsu_config::Profile;
use matsu_config::history::update_profile;
use matsu_config::settings::{KeyList, SearchKeys};

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

    /// Changes where the strings are saved (a test makes a save fail, then work).
    #[cfg(test)]
    pub(super) fn set_store(&mut self, store: Option<PathBuf>) {
        self.store = store;
    }

    #[cfg(test)]
    pub(super) fn search_sequence(&self) -> u64 {
        self.search_sequence
    }

    /// Adds a string to the history in the settings file, leaving the rest of the file alone.
    ///
    /// Under the lock that every writer of the file takes (`matsu_config::history::update_profile`)
    /// the strings the file has *now* are merged into ours ([`SearchKeys::merge_from`]) and `add`
    /// is applied, so what another window or process saved at the same moment is not lost. The
    /// file is not the whole local history: strings that only we have (a save that failed
    /// earlier, a string with a line break, which the file cannot hold) stay in memory, below the
    /// file's strings, and a later save writes the ones the file can hold. A list that the merge
    /// changed counts as news for the windows, so its sequence number advances. A file that
    /// cannot be read is not touched.
    pub(crate) fn persist(&mut self, add: impl FnOnce(&mut SearchKeys)) {
        let Some(path) = self.store.clone() else {
            return;
        };
        let local = self.keys.clone();
        let mut merged = None;
        // A history that cannot be saved is not worth stopping a search for.
        let saved = update_profile(&path, |profile| {
            let mut keys = local.clone();
            keys.merge_from(&SearchKeys::from_profile(profile));
            add(&mut keys);
            // Nothing is written when the file already has these strings.
            let before = profile.to_bytes();
            let written = keys.write_to(profile).is_ok() && profile.to_bytes() != before;
            merged = Some(keys);
            Ok(written)
        });
        if saved.is_ok()
            && let Some(keys) = merged
        {
            if keys.search != local.search {
                self.search_sequence += 1;
            }
            if keys.replace != local.replace {
                self.replace_sequence += 1;
            }
            self.keys = keys;
        }
    }
}

impl SearchHistory {
    /// Changes the lists by hand (the history management dialog: delete, add, edit), here and
    /// in the file.
    ///
    /// `change` is applied twice, each time to its own copy: to our lists (so strings that only
    /// we have, such as one with a line break that the file cannot hold, or one kept after a failed
    /// save, stay and can be edited and deleted), and, under the lock of the file, to the lists the
    /// file has *now*, which are then written. Nothing is merged, so a string deleted here is not
    /// brought back from the file, and one deleted elsewhere is not brought back from memory. The
    /// sequence numbers do not move: Sakura counts additions only (`AddToSearchKeyArr`).
    pub(crate) fn edit(&mut self, change: impl Fn(&mut SearchKeys)) {
        let before = self.keys.clone();
        change(&mut self.keys);
        let Some(path) = self.store.clone() else {
            return;
        };
        // A history that cannot be saved is not worth an error here.
        let _ = update_profile(&path, |profile| {
            let mut keys = SearchKeys::from_profile(profile);
            change(&mut keys);
            // An edit of a string only we have (one with a line break, or kept after a failed
            // save) cannot be replayed on the file's list: the string it replaced is not there.
            // What the user now sees and did not see before goes into the file as an addition in
            // front (newest first), so the file has the result.
            add_new(&before, &self.keys, &mut keys);
            let bytes = profile.to_bytes();
            let written = keys.write_to(profile).is_ok() && profile.to_bytes() != bytes;
            Ok(written)
        });
    }
}

/// Puts the strings that `after` has and `before` did not into the lists of `file`, in front, when
/// the file does not have them (the lists the dialog edits).
fn add_new(before: &SearchKeys, after: &SearchKeys, file: &mut SearchKeys) {
    fn one(before: &KeyList, after: &KeyList, file: &mut KeyList) {
        for text in after.items().iter().rev() {
            if !before.items().contains(text) && !file.items().contains(text) {
                file.add(text);
            }
        }
    }
    one(&before.search, &after.search, &mut file.search);
    one(&before.replace, &after.replace, &mut file.replace);
    one(&before.grep_file, &after.grep_file, &mut file.grep_file);
    one(
        &before.grep_folder,
        &after.grep_folder,
        &mut file.grep_folder,
    );
}

/// The lists of the search, replace and Grep boxes as they are now.
pub(crate) fn snapshot(cx: &App) -> SearchKeys {
    cx.try_global::<SearchHistory>()
        .map(|h| h.keys.clone())
        .unwrap_or_default()
}

/// Changes the lists by hand ([`SearchHistory::edit`]) and tells the observers.
pub(crate) fn edit_keys(cx: &mut App, change: impl Fn(&mut SearchKeys)) {
    cx.update_default_global(|history: &mut SearchHistory, _| history.edit(change));
}

/// Makes the history of the application. Called once at startup.
pub fn install(cx: &mut App, history: SearchHistory) {
    cx.set_global(history);
}

/// Runs `f` on the strings of a history list, newest first, without copying the list (`&[]` when
/// the application has no history).
fn with_keys<R>(
    cx: &App,
    list: impl Fn(&SearchKeys) -> &KeyList,
    f: impl FnOnce(&[String]) -> R,
) -> R {
    match cx.try_global::<SearchHistory>() {
        Some(h) => f(list(&h.keys).items()),
        None => f(&[]),
    }
}

/// [`search_keys`], read in place.
pub fn with_search_keys<R>(cx: &App, f: impl FnOnce(&[String]) -> R) -> R {
    with_keys(cx, |k| &k.search, f)
}

/// [`replace_keys`], read in place.
pub fn with_replace_keys<R>(cx: &App, f: impl FnOnce(&[String]) -> R) -> R {
    with_keys(cx, |k| &k.replace, f)
}

/// [`grep_files`], read in place.
pub fn with_grep_files<R>(cx: &App, f: impl FnOnce(&[String]) -> R) -> R {
    with_keys(cx, |k| &k.grep_file, f)
}

/// [`grep_folders`], read in place.
pub fn with_grep_folders<R>(cx: &App, f: impl FnOnce(&[String]) -> R) -> R {
    with_keys(cx, |k| &k.grep_folder, f)
}

/// [`exclude_files`], read in place.
pub fn with_exclude_files<R>(cx: &App, f: impl FnOnce(&[String]) -> R) -> R {
    with_keys(cx, |k| &k.exclude_file, f)
}

/// [`exclude_folders`], read in place.
pub fn with_exclude_folders<R>(cx: &App, f: impl FnOnce(&[String]) -> R) -> R {
    with_keys(cx, |k| &k.exclude_folder, f)
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

/// The ファイル strings of the Grep dialog, newest first.
pub fn grep_files(cx: &App) -> Vec<String> {
    keys_of(cx, |k| &k.grep_file)
}

/// The フォルダー strings of the Grep dialog.
pub fn grep_folders(cx: &App) -> Vec<String> {
    keys_of(cx, |k| &k.grep_folder)
}

/// The 除外ファイル strings of the Grep dialog.
pub fn exclude_files(cx: &App) -> Vec<String> {
    keys_of(cx, |k| &k.exclude_file)
}

/// The 除外フォルダー strings of the Grep dialog.
pub fn exclude_folders(cx: &App) -> Vec<String> {
    keys_of(cx, |k| &k.exclude_folder)
}

/// The keywords of the tag jump dialog.
pub fn tag_keywords(cx: &App) -> Vec<String> {
    keys_of(cx, |k| &k.tag_keyword)
}

fn keys_of(cx: &App, list: impl Fn(&SearchKeys) -> &KeyList) -> Vec<String> {
    cx.try_global::<SearchHistory>()
        .map(|h| list(&h.keys).items().to_vec())
        .unwrap_or_default()
}

/// What the Grep dialog keeps when it starts a search (`CDlgGrep::GetData`): the file, the
/// folder, and the excluded file and folder.
pub struct GrepBoxes<'a> {
    pub file: &'a str,
    pub folder: &'a str,
    pub exclude_file: &'a str,
    pub exclude_folder: &'a str,
}

/// `AddToGrepFileArr`, `AddToGrepFolderArr`, `AddToExcludeFileArr` and `AddToExcludeFolderArr`.
pub fn remember_grep_boxes(cx: &mut App, boxes: &GrepBoxes<'_>) {
    remember_other(cx, |keys| {
        keys.add_grep_file(boxes.file);
        keys.add_grep_folder(boxes.folder);
        keys.add_exclude_file(boxes.exclude_file);
        keys.add_exclude_folder(boxes.exclude_folder);
    });
}

/// `CRecentTagjumpKeyword::AppendItem`: the keyword of the tag jump dialog.
pub fn remember_tag_keyword(cx: &mut App, keyword: &str) {
    remember_other(cx, |keys| {
        keys.add_tag_keyword(keyword);
    });
}

/// Applies `add` to the strings here and in the file. These lists have no sequence numbers: no
/// other dialog waits for their news.
fn remember_other(cx: &mut App, add: impl Fn(&mut SearchKeys)) {
    cx.update_default_global(|history: &mut SearchHistory, _| {
        add(&mut history.keys);
        history.persist(add);
    });
}

/// `AddToSearchKeyArr`. Returns the sequence number to put in the view that searched.
pub fn remember_search(cx: &mut App, text: &str) -> u64 {
    // `update_global` tells the observers (the search boxes of the toolbars) about the news.
    cx.update_default_global(|history: &mut SearchHistory, _| {
        history.keys.add_search(text);
        history.search_sequence += 1;
        // Also when the string is already first here: another process may have put another one on
        // top since, and this search is the latest use.
        history.persist(|keys| {
            keys.add_search(text);
        });
        history.search_sequence
    })
}

/// `AddToReplaceKeyArr`. Returns the sequence number to put in the view that replaced.
pub fn remember_replace(cx: &mut App, text: &str) -> u64 {
    let history = cx.default_global::<SearchHistory>();
    history.keys.add_replace(text);
    history.replace_sequence += 1;
    history.persist(|keys| {
        keys.add_replace(text);
    });
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

/// The sequence number of the newest search in the shared history (`m_nSearchKeySequence`).
fn current_search_sequence(cx: &App) -> u64 {
    cx.try_global::<SearchHistory>()
        .map_or(0, |history| history.search_sequence)
}

impl EditorView {
    /// This view's search string was set without going into the history (a macro with the flag
    /// 0x800, or a string too long for it): the view has seen every search so far, so the next
    /// `ChangeCurRegexp` keeps this string instead of taking an older one from the history.
    pub(crate) fn mark_search_key_seen(&mut self, cx: &App) {
        self.search
            .set_key_sequence(Some(current_search_sequence(cx)));
    }

    /// The start of a next or previous search without a dialog (`CEditView::ChangeCurRegexp`): the
    /// view takes the newest search string and the options of the last search in any window,
    /// when it has not seen that search and 他のビューの検索条件を引き継ぐ is on
    /// (`m_bInheritKeyOtherView`), or when it has no search string at all (which counts whether
    /// the switch is on or off). Otherwise it keeps its own.
    pub(crate) fn change_cur_regexp(&mut self, cx: &App) {
        let inherit = super::prefs::prefs(cx).inherit_key_other_view;
        let news = inherit
            .then(|| newer_search_key(cx, self.search.key_sequence()))
            .flatten()
            // `0 == m_strCurSearchKey.size()`: nothing seen counts as news.
            .or_else(|| {
                self.search
                    .pattern()
                    .is_empty()
                    .then(|| newer_search_key(cx, None))
                    .flatten()
            });
        let Some((newest, sequence)) = news else {
            return;
        };
        let options = crate::saved_state::search::saved(cx).options(self.search.options());
        self.search.set_options(options);
        self.search.set_pattern(&newest);
        self.search.set_key_sequence(Some(sequence));
    }

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

    /// [`remember_search_key`](Self::remember_search_key), and the regular expression, case and
    /// word options of the view become the shared ones (`m_sSearchOption`) when the key was
    /// kept: what a macro search (`CMacro.cpp` 804), `ISearchExit` and `Command_SEARCH_CLEARMARK`
    /// do. Another view that inherits the search (`ChangeCurRegexp`) then searches as this one
    /// did. The Grep window and `-GREP` do not (`CDlgGrep` and `CNormalProcess` write them on
    /// their own terms).
    pub(crate) fn publish_search_key(&mut self, cx: &mut App) {
        let before = self.search.key_sequence();
        self.remember_search_key(cx);
        if self.search.key_sequence() == before {
            // Not kept (empty, or a pattern that does not compile).
            return;
        }
        let options = self.search.options();
        crate::saved_state::search::remember_pattern_options(
            cx,
            options.regex,
            options.match_case,
            options.whole_word,
        );
    }

    /// Puts the replacement in the history (`CDlgReplace::GetData`).
    pub(crate) fn remember_replace_key(&mut self, cx: &mut App) {
        let text = self.search.replace.text.clone();
        let sequence = remember_replace(cx, &text);
        self.search.set_replace_sequence(Some(sequence));
    }
}
