//! `F_FAVORITE`: the window that manages the history (履歴の管理).
//!
//! Sakura: `CDlgFavorite` (`sakura_core/dlg/CDlgFavorite.cpp`) is a dialog with a tab for each
//! history. Each has a list with a check box (お気に入り, favorite) for each item and the buttons
//! "すべて", "お気に入り以外", "存在しない項目", "選択項目" (delete), "追加" and "閉じる". An item
//! beyond the number that is shown in the menu is marked "(非表示)".
//!
//! The tabs are those of Sakura, in its order: ファイル, フォルダー, ファイル・フォルダー除外
//! (`ExceptMRU`), 検索, 置換, GREPファイル, GREPフォルダー, コマンド and カレントディレクトリ (the
//! last six are in [`super::texts`]). What a tab can do is what `m_aFavoriteInfo` says of it:
//!
//! | tab | check boxes | 存在しない項目 | 除外リストに追加 | 追加・編集 |
//! |---|---|---|---|---|
//! | ファイル, フォルダー | yes | yes | yes | no |
//! | ファイル・フォルダー除外 | no | no | no | yes |
//! | 検索, 置換, GREPファイル, コマンド | no | no | no | yes |
//! | GREPフォルダー, カレントディレクトリ | no | yes | no | no |
//!
//! 追加 and 編集 (a double click, or F2) ask for the text in a panel over the dialog (`CDlgInput1`).
//! A change is written to the history at once; Sakura keeps the check boxes in the dialog and
//! applies them when it is closed or before a delete, which comes to the same. Closing the dialog
//! with the button or with the close box is the same (Sakura dropped the cancel in 2010,
//! `IDCANCEL` of `OnBnClicked`).
//!
//! The dialog is a window of its own, as in Sakura, with the window of the editor behind it. Its
//! place and size are kept when it is closed and used by the next one (`rcFavoriteDialog`,
//! [`crate::saved_state::dialog_rect`]).

use std::cell::Cell;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle,
    Focusable, Global, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, PromptLevel,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowOptions,
    div, px, rgb, size,
};
use gpui_component::Root;
use gpui_component::input::{Input, InputEvent, InputState};
use matsu_config::history::{History, Recent};
use matsu_config::settings::EditableList;

use super::texts::{self, TextLists};
use crate::access_keys::{Entry, Found, OnAccessKey as _, pick};
use crate::file_ops::windows::editors;
use crate::input_select::{focus_selecting, input_field};
use crate::saved_state::dialog_rect::{
    Rect, favorite as saved_rect, remember_favorite, work_areas,
};
use crate::support::help::OnHelpKey as _;

/// The histories the dialog manages, in the order of Sakura's tabs (`m_aFavoriteInfo`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tab {
    Files,
    Folders,
    /// `STR_DLGFAV_FF_EXCLUDE`: `ExceptMRU`, the texts that keep a path out of the history.
    Except,
    /// `STR_DLGFAV_SEARCH`: the search strings.
    Search,
    /// `STR_DLGFAV_REPLACE`.
    Replace,
    /// `STR_DLGFAV_GREP_FILE`: ファイル of the Grep dialog.
    GrepFile,
    /// `STR_DLGFAV_GREP_FOLDER`: フォルダー of the Grep dialog.
    GrepFolder,
    /// `STR_DLGFAV_EXT_COMMAND`: the commands of 外部コマンド実行.
    Command,
    /// `STR_DLGFAV_CURRENT_DIR`: the folders those commands ran in.
    CurDir,
}

impl Tab {
    /// The tabs in the order of the dialog.
    pub(crate) const ALL: [Tab; 9] = [
        Tab::Files,
        Tab::Folders,
        Tab::Except,
        Tab::Search,
        Tab::Replace,
        Tab::GrepFile,
        Tab::GrepFolder,
        Tab::Command,
        Tab::CurDir,
    ];

    /// The next tab, going round (Ctrl+PageDown in Sakura; here Tab).
    pub(crate) fn next(self) -> Tab {
        let at = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }

    /// Whether the items can be made favorites (`m_bHaveFavorite`).
    pub(crate) fn has_favorites(self) -> bool {
        matches!(self, Tab::Files | Tab::Folders)
    }

    /// Whether the items are paths (`m_bFilePath`): only those can be searched for what is gone.
    pub(crate) fn is_path_list(self) -> bool {
        matches!(
            self,
            Tab::Files | Tab::Folders | Tab::GrepFolder | Tab::CurDir
        )
    }

    /// Whether the selected items can be put in the list of excluded paths (`m_bAddExcept`).
    pub(crate) fn can_add_except(self) -> bool {
        matches!(self, Tab::Files | Tab::Folders)
    }

    /// Whether the user can add and edit the items (`m_bEditable`).
    pub(crate) fn is_editable(self) -> bool {
        matches!(
            self,
            Tab::Except | Tab::Search | Tab::Replace | Tab::GrepFile | Tab::Command
        )
    }

    /// Whether the items are plain text kept in a list of [`super::texts`].
    pub(crate) fn is_text_list(self) -> bool {
        matches!(
            self,
            Tab::Search
                | Tab::Replace
                | Tab::GrepFile
                | Tab::GrepFolder
                | Tab::Command
                | Tab::CurDir
        )
    }

    /// The caption of the tab (`STR_DLGFAV_FILE` and the like).
    pub(crate) fn caption(self) -> &'static str {
        match self {
            Tab::Files => "ファイル",
            Tab::Folders => "フォルダー",
            Tab::Except => "ファイル・フォルダー除外",
            Tab::Search => "検索",
            Tab::Replace => "置換",
            Tab::GrepFile => "GREPファイル",
            Tab::GrepFolder => "GREPフォルダー",
            Tab::Command => "コマンド",
            Tab::CurDir => "カレントディレクトリ",
        }
    }

    /// The number of UTF-16 units an item of the tab must stay under (`GetTextMaxLength`).
    fn max_units(self, texts: &TextLists) -> usize {
        match self {
            Tab::Search => texts.keys.search.max_units(),
            Tab::Replace => texts.keys.replace.max_units(),
            Tab::GrepFile => texts.keys.grep_file.max_units(),
            Tab::GrepFolder => texts.keys.grep_folder.max_units(),
            Tab::Command => texts.exec.commands.max_units(),
            Tab::CurDir => texts.exec.cur_dirs.max_units(),
            // `CMetaPath` is as long as a path.
            Tab::Files | Tab::Folders | Tab::Except => EXCEPT_MAX_UNITS,
        }
    }
}

/// `_MAX_PATH`: the size of a string of `ExceptMRU` (`CMetaPath`).
const EXCEPT_MAX_UNITS: usize = 260;

/// An item of a history as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    /// The access key and the "(非表示)" mark (`FormatFavoriteColumn`).
    pub key: String,
    pub favorite: bool,
    pub text: String,
}

/// The access key of the item `index` (0 to 9, A to Z, and again) and "(非表示)" when the menu does
/// not show it (`FormatFavoriteColumn`).
pub(crate) fn key_column(index: usize, shown: bool) -> String {
    let n = index % 36;
    let key = char::from_digit(n as u32, 36)
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('0');
    format!("{key} {}", if shown { "  " } else { "(非表示)" })
}

/// The rows of a tab.
pub(crate) fn rows(history: &History, tab: Tab) -> Vec<Row> {
    fn make<T: Recent>(list: &matsu_config::history::RecentList<T>) -> Vec<Row> {
        list.items()
            .iter()
            .enumerate()
            .map(|(i, item)| Row {
                key: key_column(i, i < list.view_count()),
                favorite: item.is_favorite(),
                text: item.text().to_owned(),
            })
            .collect()
    }
    match tab {
        Tab::Files => make(&history.files),
        Tab::Folders => make(&history.folders),
        Tab::Except => plain_rows(&history.except),
        // The text lists are not in `History`; see `text_rows`.
        _ => Vec::new(),
    }
}

/// The rows of a list without favorites.
fn plain_rows(texts: &[String]) -> Vec<Row> {
    texts
        .iter()
        .enumerate()
        .map(|(i, text)| Row {
            // These lists have no part that a menu shows: every item is shown
            // (`CRecentImp::Create` is given no view count, so it counts as the whole list).
            key: key_column(i, true),
            favorite: false,
            text: text.clone(),
        })
        .collect()
}

/// The rows of a tab of the text lists (検索, 置換, GREPファイル, GREPフォルダー, コマンド,
/// カレントディレクトリ).
pub(crate) fn text_rows(texts: &TextLists, tab: Tab) -> Vec<Row> {
    plain_rows(texts.texts(tab))
}

/// Makes the item a favorite or not.
pub(crate) fn set_favorite(history: &mut History, tab: Tab, index: usize, favorite: bool) {
    match tab {
        Tab::Files => history.files.set_favorite(index, favorite),
        Tab::Folders => history.folders.set_favorite(index, favorite),
        // No favorites in the other lists.
        _ => false,
    };
    // A favorite comes into the part the menu shows (`UpdateView`).
    let (files, folders) = (history.files.view_count(), history.folders.view_count());
    history.files.change_view_count(files);
    history.folders.change_view_count(folders);
}

/// The index of the item with this path in the latest history: another window may have changed the
/// order since the dialog showed it, so rows are told by their path.
fn index_of(history: &History, tab: Tab, path: &str) -> Option<usize> {
    match tab {
        Tab::Files => history.files.find(path),
        Tab::Folders => history.folders.find(path),
        Tab::Except => history.except.iter().position(|text| text == path),
        _ => None,
    }
}

/// "すべて": removes every item, favorites too (`DeleteAllItem`).
pub(crate) fn delete_all(history: &mut History, tab: Tab) {
    match tab {
        Tab::Files => history.files.clear(),
        Tab::Folders => history.folders.clear(),
        Tab::Except => history.except.clear(),
        _ => {}
    }
}

/// "お気に入り以外" (`DeleteItemsNoFavorite`).
pub(crate) fn delete_non_favorites(history: &mut History, tab: Tab) {
    match tab {
        Tab::Files => history.files.delete_non_favorites(),
        Tab::Folders => history.folders.delete_non_favorites(),
        // Nothing is a favorite there, and the button is not there either (`m_bHaveFavorite`).
        _ => false,
    };
}

/// Whether Sakura would find the path (`CutLastYenFromDirectoryPath`, `IsFileExists(path, false)`:
/// a file or a folder).
pub(crate) fn path_exists(path: &str) -> bool {
    let trimmed = path
        .strip_suffix(['\\', '/'])
        .filter(|rest| !rest.is_empty() && !rest.ends_with(':'))
        .unwrap_or(path);
    Path::new(trimmed).exists()
}

/// "存在しない項目" (`IDC_BUTTON_DELETE_NOTFOUND`): removes the items whose path is gone.
pub(crate) fn delete_missing(history: &mut History, tab: Tab, exists: impl Fn(&str) -> bool) {
    match tab {
        Tab::Files => history.files.delete_where(|f| !exists(&f.path)),
        Tab::Folders => history.folders.delete_where(|f| !exists(&f.path)),
        // Not paths (`m_bFilePath` is off): `ExceptMRU` is text that a path has in it. The
        // text lists have their own function ([`texts::delete_missing`]).
        _ => false,
    };
}

/// "選択項目" (`DeleteSelected`): removes the items at `indices`, the last first so that the numbers
/// of the others stay right.
pub(crate) fn delete_selected(history: &mut History, tab: Tab, indices: &BTreeSet<usize>) {
    for &i in indices.iter().rev() {
        match tab {
            Tab::Files => history.files.delete(i),
            Tab::Folders => history.folders.delete(i),
            Tab::Except => {
                let found = i < history.except.len();
                if found {
                    history.except.remove(i);
                }
                found
            }
            _ => false,
        };
    }
}

/// Whether two texts of `ExceptMRU` are the same (`CRecentExceptMRU::CompareItem`: `_wcsicmp`).
fn same_except(a: &str, b: &str) -> bool {
    a == b || a.to_lowercase() == b.to_lowercase()
}

/// `AppendItemText` of `ExceptMRU`: puts the text at the front. Refused when it is empty (see
/// [`texts::allows_empty`]), when it is too long, or when the list has it already in any case (the
/// old one does not move). A full list drops its oldest text (`AppendItem`; nothing is a
/// favorite there).
pub(crate) fn append_except(history: &mut History, text: impl AsRef<str> + Into<String>) -> bool {
    // Checked by borrowing; an accepted text moves into the list (a `String` is not copied).
    let shown = text.as_ref();
    if shown.is_empty()
        || shown.encode_utf16().count() >= EXCEPT_MAX_UNITS
        || history.except.iter().any(|t| same_except(t, shown))
    {
        return false;
    }
    if history.except.len() >= matsu_config::history::MAX_EXCEPT_MRU {
        history.except.pop();
    }
    history.except.insert(0, text.into());
    true
}

/// `EditItemText` of `ExceptMRU`: replaces the text `from` with `to` where it is. Refused when
/// `from` is gone, `to` is empty or too long, or another text of the list is the same as `to`
/// (the same text in another case is fine: it may be a change of case).
pub(crate) fn edit_except(history: &mut History, from: &str, to: &str) -> bool {
    let Some(index) = history.except.iter().position(|t| t == from) else {
        return false;
    };
    if to.is_empty()
        || to.encode_utf16().count() >= EXCEPT_MAX_UNITS
        || history
            .except
            .iter()
            .enumerate()
            .any(|(i, t)| i != index && same_except(t, to))
    {
        return false;
    }
    history.except[index] = to.to_owned();
    true
}

/// A path of 260 UTF-16 units or more is left out without a word, as in Sakura: `MENU_ADD_EXCEPT`
/// calls `AppendItemText`, whose `ValidateReceiveType` refuses a text that does not fit the
/// `_MAX_PATH` buffer of `CMetaPath` (`wcsnlen(p, max) < max`, `CRecentImp.h`), and the caller
/// does not look at the result. The same limit holds for what is typed in the panel.
/// 除外リストに追加 (`MENU_ADD_EXCEPT`): puts the paths in front of `ExceptMRU`, one by one with
/// [`append_except`] (a text that is there already stays where it is). The list takes
/// `MAX_EXCEPT_MRU` texts; once it is full the rest is left out, as Sakura does, and the dialog
/// says so. The texts move into the list. Returns whether it did not run full.
pub(crate) fn add_to_except(history: &mut History, paths: Vec<String>) -> bool {
    let mut all = true;
    for path in paths {
        if history.except.len() >= matsu_config::history::MAX_EXCEPT_MRU {
            all = false;
        } else {
            append_except(history, path);
        }
    }
    all
}

/// The message before "すべて" (`STR_DLGFAV_CONF_DEL_FAV`).
fn confirm_all(tab: Tab) -> String {
    format!(
        "最近使った{}の履歴を削除します。\nよろしいですか？\n",
        tab.caption()
    )
}

/// `STR_DLGFAV_LIST_LIMIT_OVER`.
const EXCEPT_FULL: &str = "除外リストがいっぱいで追加できませんでした。";

/// The message before "お気に入り以外" (`STR_DLGFAV_CONF_DEL_NOTFAV`).
fn confirm_non_favorites(tab: Tab) -> String {
    format!(
        "最近使った{}の履歴のお気に入り以外を削除します。\nよろしいですか？",
        tab.caption()
    )
}

/// The message before "存在しない項目" (`STR_DLGFAV_CONF_DEL_PATH`).
fn confirm_missing(tab: Tab) -> String {
    format!(
        "最近使った{}の存在しないパスを削除します。\nよろしいですか？",
        tab.caption()
    )
}

/// The labels of the buttons that show an access key (`IDD_FAVORITE`: `sakura_core/sakura_rc.rc`,
/// `追加(&I)`, `すべて(&A)...`, `お気に入り以外(&F)...`, `存在しない項目(&N)...`, `選択項目(&D)`,
/// `閉じる(&C)`).
mod labels {
    pub(super) const ADD: &str = "追加(I)";
    pub(super) const ALL: &str = "すべて(A)";
    pub(super) const NON_FAVORITES: &str = "お気に入り以外(F)";
    pub(super) const MISSING: &str = "存在しない項目(N)";
    pub(super) const SELECTED: &str = "選択項目(D)";
    pub(super) const ADD_EXCEPT: &str = "除外リストに追加(X)";
    pub(super) const CLOSE: &str = "閉じる(C)";
    pub(super) const HELP: &str = crate::support::help::BUTTON_LABEL;
}

/// What an access key does in the dialog.
#[derive(Clone, Copy)]
enum Use {
    Add,
    All,
    NonFavorites,
    Missing,
    Selected,
    AddExcept,
    Close,
    /// ヘルプ(H): the page of the dialog.
    Help,
}

/// `IDC_BUTTON_HELP`: `CDlgFavorite` opens the page of `F_FAVORITE`. The dialog stays.
fn open_help(cx: &mut App) {
    crate::support::help::open_function_page(cx, "F_FAVORITE");
}

/// A change the dialog makes to the list of the tab that is shown. The rows it names are told by
/// their text, not by their number: another window may have changed the order since the dialog
/// showed them.
#[derive(Clone, Debug)]
enum Op {
    /// "すべて".
    DeleteAll,
    /// "お気に入り以外".
    DeleteNonFavorites,
    /// "存在しない項目".
    DeleteMissing,
    /// "選択項目".
    DeleteRows(Vec<String>),
    /// The check box of a row.
    SetFavorite { text: String, on: bool },
    /// 追加.
    Add(String),
    /// 編集: `from` becomes `to`.
    Replace { from: String, to: String },
}

/// Applies `op` to a tab of the history of files and folders. Returns whether it was taken whole:
/// `false` for an addition that is refused.
fn apply_to_history(history: &mut History, tab: Tab, op: &Op) -> bool {
    match op {
        Op::DeleteAll => delete_all(history, tab),
        Op::DeleteNonFavorites => delete_non_favorites(history, tab),
        Op::DeleteMissing => delete_missing(history, tab, path_exists),
        Op::DeleteRows(texts) => {
            let indices = texts
                .iter()
                .filter_map(|text| index_of(history, tab, text))
                .collect();
            delete_selected(history, tab, &indices);
        }
        Op::SetFavorite { text, on } => {
            if let Some(index) = index_of(history, tab, text) {
                set_favorite(history, tab, index, *on);
            }
        }
        Op::Add(text) => return tab == Tab::Except && append_except(history, text),
        Op::Replace { from, to } => return tab == Tab::Except && edit_except(history, from, to),
    }
    true
}

/// Applies `op` to a tab of the text lists ([`super::texts`]). Returns whether it was taken.
fn apply_to_list(list: &mut dyn EditableList, tab: Tab, op: &Op) -> bool {
    match op {
        Op::DeleteAll => list.clear_all(),
        Op::DeleteMissing => texts::delete_missing(list, path_exists, tab == Tab::GrepFolder),
        Op::DeleteRows(rows) => texts::delete_texts(list, rows),
        Op::Add(text) => return texts::add_text(list, tab, text),
        Op::Replace { from, to } => return texts::edit_text(list, tab, from, to),
        // No favorites.
        Op::DeleteNonFavorites | Op::SetFavorite { .. } => return false,
    }
    true
}

/// The dialog that is open, so that a second `F_FAVORITE` brings it to the front.
#[derive(Default)]
struct OpenDialog(Option<AnyWindowHandle>);

impl Global for OpenDialog {}

/// How often the dialog looks at the history again (another window may change it) and whether the
/// editor windows are still there.
const REFRESH: Duration = Duration::from_millis(500);

const TEXT: u32 = 0x222222;
const SELECTED_BG: u32 = 0x2f6fd0;

/// What the panel over the dialog does with the text.
#[derive(Clone, Debug, PartialEq, Eq)]
enum EditTarget {
    /// 追加.
    Add,
    /// 編集 of the item with this text.
    Edit(String),
}

/// The panel that asks for the text of 追加 and 編集 (`CDlgInput1`: a message, a field limited to
/// the length of the list's strings, OK and キャンセル).
struct EditPanel {
    target: EditTarget,
    input: Entity<InputState>,
    /// The text of the field after the last change, within the maximum length.
    accepted: String,
    /// The longest text the field takes, in UTF-16 units (`max_size - 1` of `CDlgInput1::DoModal`).
    max_len: usize,
    /// The buttons take the focus too (Tab, Shift+Tab; Enter and Space press the focused one).
    ok_focus: FocusHandle,
    cancel_focus: FocusHandle,
    _subscription: gpui::Subscription,
}

pub(crate) struct FavoriteDialog {
    tab: Tab,
    history: History,
    /// The six tabs of text.
    texts: TextLists,
    selected: BTreeSet<usize>,
    /// The item the keys move from.
    focus_row: usize,
    /// The row a range selection (Shift) starts from.
    anchor: usize,
    focus: FocusHandle,
    scroll: ScrollHandle,
    /// Where the window is, kept for the next dialog when this one is closed
    /// (`CDlgFavorite::OnDestroy`).
    rect: Rect,
    /// The wait for the answer to the box that says the list of excluded paths is full.
    notice_task: Option<gpui::Task<()>>,
    /// The panel of 追加 and 編集, when it is open.
    edit: Option<EditPanel>,
}

/// The rectangle of the window as the dialog opens it: the restored one when it is maximized,
/// the size of its content on macOS (the frame there has the title bar in it, and a window opens
/// from the size of its content).
fn window_rect(window: &Window) -> Rect {
    let bounds = if cfg!(target_os = "macos") {
        crate::saved_state::window::with_content_size(
            window.window_bounds(),
            window.viewport_size(),
        )
    } else {
        window.window_bounds()
    };
    Rect::of_bounds(bounds.get_bounds())
}

impl FavoriteDialog {
    pub(crate) fn new(history: History, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH).await;
                let alive = this
                    .update_in(cx, |this, window, cx| {
                        // The dialog belongs to the editor: it goes when the last window does.
                        if editors(cx).is_empty() {
                            window.remove_window();
                            return;
                        }
                        this.reload(cx);
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        })
        .detach();
        cx.observe_window_bounds(window, |this, window, _| this.rect = window_rect(window))
            .detach();
        // Whichever way the dialog is closed, its place is kept (`OnDestroy`).
        cx.on_release(|this, cx| remember_favorite(cx, this.rect))
            .detach();
        Self {
            tab: Tab::Files,
            history,
            texts: TextLists::current(cx),
            selected: BTreeSet::new(),
            focus_row: 0,
            anchor: 0,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            rect: window_rect(window),
            notice_task: None,
            edit: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn tab(&self) -> Tab {
        self.tab
    }

    pub(crate) fn rows(&self) -> Vec<Row> {
        if self.tab.is_text_list() {
            text_rows(&self.texts, self.tab)
        } else {
            rows(&self.history, self.tab)
        }
    }

    #[cfg(test)]
    pub(crate) fn selected(&self) -> &BTreeSet<usize> {
        &self.selected
    }

    /// Whether the panel of 追加 and 編集 is open.
    #[cfg(test)]
    pub(crate) fn is_editing(&self) -> bool {
        self.edit.is_some()
    }

    /// Reads the history again (the file may have been changed by another window).
    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) {
        let history = super::current(cx).filter(|history| *history != self.history);
        let texts = Some(TextLists::current(cx)).filter(|texts| *texts != self.texts);
        if history.is_none() && texts.is_none() {
            return;
        }
        let before = self.paths();
        if let Some(history) = history {
            self.history = history;
        }
        if let Some(texts) = texts {
            self.texts = texts;
        }
        let same_rows = before == self.paths();
        self.keep_selection(same_rows);
        cx.notify();
    }

    /// Applies a change to the list of the tab that is shown, and shows the result. Returns
    /// whether the change was taken whole ([`apply_to_history`]).
    fn run(&mut self, cx: &mut Context<Self>, op: &Op) -> bool {
        self.run_on(cx, self.tab, op)
    }

    /// [`run`](Self::run) on the list of `tab`, which is not the one shown when the answer to a
    /// question arrives after the user went to another tab.
    fn run_on(&mut self, cx: &mut Context<Self>, tab: Tab, op: &Op) -> bool {
        let before = self.paths();
        let taken = Cell::new(true);
        if tab.is_text_list() {
            // The change runs on our list first and is replayed on the file's list: only the first
            // result is ours (the replay may not find a string that only memory has).
            let first = Cell::new(true);
            let removes = !matches!(op, Op::Add(_));
            texts::change(cx, tab, removes, |list| {
                let took = apply_to_list(list, tab, op);
                if first.replace(false) {
                    taken.set(took);
                }
            });
            self.texts = TextLists::current(cx);
        } else if let Some(history) =
            super::update(cx, |history| taken.set(apply_to_history(history, tab, op)))
        {
            self.history = history;
        }
        let same_rows = before == self.paths();
        self.keep_selection(same_rows);
        cx.notify();
        taken.get()
    }

    fn paths(&self) -> Vec<String> {
        self.rows().into_iter().map(|r| r.text).collect()
    }

    /// The selection is by row number, so it only stays when the rows are the same ones in the same
    /// order; else it would point at other files.
    fn keep_selection(&mut self, same_rows: bool) {
        if !same_rows {
            self.selected.clear();
        }
        let count = self.rows().len();
        self.selected.retain(|i| *i < count);
        self.focus_row = self.focus_row.min(count.saturating_sub(1));
        // The row a range selection starts from goes with the rows: it must not point past the
        // end, or at another item.
        self.anchor = if same_rows {
            self.anchor.min(count.saturating_sub(1))
        } else {
            self.focus_row
        };
    }

    pub(crate) fn select_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        if self.tab != tab {
            self.tab = tab;
            self.selected.clear();
            self.focus_row = 0;
            self.anchor = 0;
            cx.notify();
        }
    }

    /// Clicks on a row: alone it selects the row, with Control it adds or removes it, with Shift
    /// it selects from the row clicked before.
    pub(crate) fn click_row(
        &mut self,
        index: usize,
        control: bool,
        shift: bool,
        cx: &mut Context<Self>,
    ) {
        if shift {
            let (from, to) = (self.anchor.min(index), self.anchor.max(index));
            if !control {
                self.selected.clear();
            }
            self.selected.extend(from..=to);
        } else if control {
            if !self.selected.remove(&index) {
                self.selected.insert(index);
            }
            self.anchor = index;
        } else {
            self.selected = BTreeSet::from([index]);
            self.anchor = index;
        }
        self.focus_row = index;
        cx.notify();
    }

    /// The check box of a row.
    pub(crate) fn toggle_favorite(&mut self, index: usize, cx: &mut Context<Self>) {
        if !self.tab.has_favorites() {
            return;
        }
        let Some(row) = self.rows().into_iter().nth(index) else {
            return;
        };
        self.run(
            cx,
            &Op::SetFavorite {
                text: row.text,
                on: !row.favorite,
            },
        );
    }

    /// Asks, and runs `op` on the list if the answer is yes.
    fn confirm_then(&mut self, message: &str, window: &mut Window, cx: &mut Context<Self>, op: Op) {
        let tab = self.tab;
        let answer = crate::boxes::prompt(
            window,
            PromptLevel::Warning,
            message,
            None,
            &["はい(Y)", "いいえ(N)"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(0) = answer.await {
                this.update(cx, |this, cx| {
                    // The list that was asked about, even if another tab is shown now.
                    this.run_on(cx, tab, &op);
                })
                .ok();
            }
        })
        .detach();
    }

    /// "すべて".
    pub(crate) fn ask_delete_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.rows().is_empty() {
            self.confirm_then(&confirm_all(self.tab), window, cx, Op::DeleteAll);
        }
    }

    /// "お気に入り以外".
    pub(crate) fn ask_delete_non_favorites(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab.has_favorites() && !self.rows().is_empty() {
            let message = confirm_non_favorites(self.tab);
            self.confirm_then(&message, window, cx, Op::DeleteNonFavorites);
        }
    }

    /// "存在しない項目".
    pub(crate) fn ask_delete_missing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab.is_path_list() && !self.rows().is_empty() {
            let message = confirm_missing(self.tab);
            self.confirm_then(&message, window, cx, Op::DeleteMissing);
        }
    }

    /// The texts of the selected rows.
    fn selected_texts(&self) -> Vec<String> {
        self.rows()
            .into_iter()
            .enumerate()
            .filter(|(i, _)| self.selected.contains(i))
            .map(|(_, r)| r.text)
            .collect()
    }

    /// "選択項目": no question (Sakura asks none).
    pub(crate) fn delete_selected_rows(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            return;
        }
        let paths = self.selected_texts();
        let next = self.selected.iter().next_back().copied().unwrap_or(0) + 1 - self.selected.len();
        self.selected.clear();
        self.run(cx, &Op::DeleteRows(paths));
        // The item after the last one deleted is selected, or the last item.
        let count = self.rows().len();
        if count > 0 {
            let at = next.min(count - 1);
            self.selected = BTreeSet::from([at]);
            self.focus_row = at;
            self.anchor = at;
        }
        cx.notify();
    }

    /// 除外リストに追加: the selected files or folders go into the list of excluded paths. A list
    /// that is full says so and takes what fits.
    pub(crate) fn add_selected_to_except(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.tab.can_add_except() || self.selected.is_empty() {
            return;
        }
        let paths = self.selected_texts();
        // The list of excluded paths is another tab's list: it is changed here, not through the
        // tab that is shown.
        let mut all = true;
        let before = self.paths();
        if let Some(history) = super::update(cx, |h| all = add_to_except(h, paths)) {
            self.history = history;
        }
        let same_rows = before == self.paths();
        self.keep_selection(same_rows);
        cx.notify();
        if !all {
            let answer =
                crate::boxes::prompt(window, PromptLevel::Warning, EXCEPT_FULL, None, &["OK"], cx);
            // The dialog keeps the task that waits for the answer: it only has to see the box
            // closed, and it ends with the dialog (and with the box, whose answer then fails).
            self.notice_task = Some(cx.spawn_in(window, async move |_, _| {
                answer.await.ok();
            }));
        }
    }

    /// 追加 (`AddItem`): asks for the text of a new item.
    pub(crate) fn begin_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab.is_editable() {
            self.open_panel(EditTarget::Add, "", window, cx);
        }
    }

    /// 編集 (`EditItem`, a double click): asks for the new text of the first selected item.
    pub(crate) fn begin_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.tab.is_editable() {
            return;
        }
        if let Some(text) = self.selected_texts().into_iter().next() {
            self.open_panel(EditTarget::Edit(text.clone()), &text, window, cx);
        }
    }

    fn open_panel(
        &mut self,
        target: EditTarget,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // `max_size - 1` of `CDlgInput1::DoModal`: a string of the buffer's size does not fit.
        let max_len = self.tab.max_units(&self.texts) - 1;
        let text = matsu_plugin::functions::window::truncate_utf16(text, max_len).to_owned();
        let input = cx.new(|cx| input_field(window, cx));
        input.update(cx, |state, cx| state.set_value(text.clone(), window, cx));
        let subscription = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.limit_field(window, cx),
                InputEvent::PressEnter { .. } => this.ok_edit(window, cx),
                _ => {}
            },
        );
        focus_selecting(&input, window, cx);
        self.edit = Some(EditPanel {
            target,
            input,
            accepted: text,
            max_len,
            ok_focus: cx.focus_handle(),
            cancel_focus: cx.focus_handle(),
            _subscription: subscription,
        });
        cx.notify();
    }

    /// Keeps the field within the maximum length: what is typed or pasted beyond it is cut off
    /// (the edit control of Sakura's dialog refuses it).
    fn limit_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.edit.as_mut() else {
            return;
        };
        let text = panel.input.read(cx).value().to_string();
        match crate::input_dialog::limit_edit(&panel.accepted, &text, panel.max_len) {
            Some((kept, caret)) => {
                panel.accepted.clone_from(&kept);
                panel.input.update(cx, |state, cx| {
                    state.set_value(kept, window, cx);
                    state.set_selected_range(caret..caret, cx);
                });
            }
            None => panel.accepted = text,
        }
    }

    /// OK of the panel.
    fn ok_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.edit.as_ref() else {
            return;
        };
        let text = panel.input.read(cx).value().to_string();
        self.finish_edit(Some(text), window, cx);
    }

    /// Closes the panel. With a text (`None` is キャンセル) the item is added, or replaced
    /// (`AppendItemText`, `EditItemText`); a text that the list refuses (one it has already, or
    /// an empty one) changes nothing, and Sakura says nothing either.
    pub(crate) fn finish_edit(
        &mut self,
        text: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(panel) = self.edit.take() else {
            return;
        };
        window.focus(&self.focus, cx);
        if let Some(text) = text {
            let text =
                matsu_plugin::functions::window::truncate_utf16(&text, panel.max_len).to_owned();
            match panel.target {
                EditTarget::Add => {
                    self.run(cx, &Op::Add(text));
                }
                EditTarget::Edit(from) => {
                    if self.run(
                        cx,
                        &Op::Replace {
                            from,
                            to: text.clone(),
                        },
                    ) && let Some(at) = self.paths().iter().position(|t| *t == text)
                    {
                        // The edited item stays selected (`SetDataOne(m_nCurrentTab, nRecIndex)`).
                        self.selected = BTreeSet::from([at]);
                        self.focus_row = at;
                        self.anchor = at;
                    }
                }
            }
        }
        cx.notify();
    }

    /// Alt with the access key of a button (`IsDialogMessage`): presses it. Returns whether the
    /// letter was one of the dialog's.
    pub(crate) fn use_access_key(
        &mut self,
        letter: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // The panel is modal: the buttons behind it do not take keys.
        if self.edit.is_some() {
            return false;
        }
        let entries = [
            Entry::labelled(labels::ADD, self.tab.is_editable(), Use::Add),
            Entry::labelled(labels::ALL, true, Use::All),
            Entry::labelled(
                labels::NON_FAVORITES,
                self.tab.has_favorites(),
                Use::NonFavorites,
            ),
            Entry::labelled(labels::MISSING, self.tab.is_path_list(), Use::Missing),
            Entry::labelled(labels::SELECTED, true, Use::Selected),
            Entry::labelled(
                labels::ADD_EXCEPT,
                self.tab.can_add_except(),
                Use::AddExcept,
            ),
            Entry::labelled(labels::CLOSE, true, Use::Close),
            Entry::labelled(labels::HELP, true, Use::Help),
        ];
        match pick(letter, &entries) {
            Found::Nothing => return false,
            Found::Several => {}
            Found::One(Use::Add) => self.begin_add(window, cx),
            Found::One(Use::All) => self.ask_delete_all(window, cx),
            Found::One(Use::NonFavorites) => self.ask_delete_non_favorites(window, cx),
            Found::One(Use::Missing) => self.ask_delete_missing(window, cx),
            Found::One(Use::Selected) => self.delete_selected_rows(cx),
            Found::One(Use::AddExcept) => self.add_selected_to_except(window, cx),
            Found::One(Use::Close) => self.close(window),
            Found::One(Use::Help) => open_help(cx),
        }
        true
    }

    fn close(&mut self, window: &mut Window) {
        window.remove_window();
    }

    /// The keys of the panel (as in `InputDialog`): Esc cancels, Tab and Shift+Tab go between the
    /// field and the buttons, Enter and Space press the button that has the focus. Enter in the
    /// field is OK, which the field reports itself.
    fn on_panel_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.edit.as_ref() else {
            return;
        };
        let (ok, cancel) = (panel.ok_focus.clone(), panel.cancel_focus.clone());
        match event.keystroke.key.as_str() {
            "escape" => self.finish_edit(None, window, cx),
            "tab" => {
                let field = panel.input.read(cx).focus_handle(cx);
                crate::input_dialog::cycle_focus(
                    &[field, ok, cancel],
                    !event.keystroke.modifiers.shift,
                    window,
                    cx,
                );
            }
            "enter" | "space" if cancel.is_focused(window) => self.finish_edit(None, window, cx),
            "enter" | "space" if ok.is_focused(window) => self.ok_edit(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        // The panel of 追加 and 編集 has the keys while it is open; Enter in its field is OK.
        if self.edit.is_some() {
            self.on_panel_key(event, window, cx);
            return;
        }
        let count = self.rows().len();
        let modifiers = event.keystroke.modifiers;
        let target = match key {
            "escape" | "enter" => {
                self.close(window);
                return;
            }
            "delete" => {
                self.delete_selected_rows(cx);
                cx.stop_propagation();
                return;
            }
            "f2" => {
                // There is no right-click menu with 編集(E) here.
                self.begin_edit(window, cx);
                cx.stop_propagation();
                return;
            }
            "space" => {
                if count > 0 && self.tab.has_favorites() {
                    self.toggle_favorite(self.focus_row, cx);
                }
                cx.stop_propagation();
                return;
            }
            "a" if modifiers.control || modifiers.platform => {
                self.selected = (0..count).collect();
                cx.notify();
                cx.stop_propagation();
                return;
            }
            "tab" => {
                self.select_tab(self.tab.next(), cx);
                cx.stop_propagation();
                return;
            }
            "up" => self.focus_row.saturating_sub(1),
            "down" => (self.focus_row + 1).min(count.saturating_sub(1)),
            "home" => 0,
            "end" => count.saturating_sub(1),
            _ => return,
        };
        if count > 0 {
            self.click_row(target, false, modifiers.shift, cx);
            self.scroll.scroll_to_item(target);
        }
        cx.stop_propagation();
    }
}

impl Focusable for FavoriteDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for FavoriteDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let panel = self.render_panel(window, cx);
        // The field of the panel of 追加 and 編集 has the focus while the panel is open.
        if self.edit.is_none() && !self.focus.is_focused(window) {
            window.focus(&self.focus, cx);
        }
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let rows = self.rows();
        let items: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let selected = self.selected.contains(&i);
                let mut item = crate::a11y::list_row(
                    div().id(("row", i)),
                    &format!("{} {}", row.key, row.text),
                    selected,
                )
                .px_2()
                .h(px(24.))
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer();
                if selected {
                    item = item.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff));
                }
                let has_favorites = self.tab.has_favorites();
                let has_key = !row.key.is_empty();
                item.when(has_favorites, |item| {
                    item.child(
                        div()
                            .id(("favorite", i))
                            .role(gpui::Role::CheckBox)
                            .aria_label(format!("お気に入り: {}", row.text))
                            .aria_toggled(if row.favorite {
                                gpui::Toggled::True
                            } else {
                                gpui::Toggled::False
                            })
                            .w(px(16.))
                            .h(px(16.))
                            .border_1()
                            .border_color(rgb(0x666666))
                            .bg(rgb(0xffffff))
                            .text_color(rgb(TEXT))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(if row.favorite { "✓" } else { "" })
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.toggle_favorite(i, cx);
                                cx.stop_propagation();
                            })),
                    )
                })
                .when(has_key, |item| {
                    item.child(div().w(px(80.)).child(row.key.clone()))
                })
                .child(div().flex_1().child(row.text.clone()))
                .on_click(cx.listener(
                    move |this, event: &ClickEvent, window, cx| {
                        let modifiers = event.modifiers();
                        this.click_row(
                            i,
                            modifiers.control || modifiers.platform,
                            modifiers.shift,
                            cx,
                        );
                        // A double click edits the item (`NM_DBLCLK`).
                        if event.click_count() == 2 {
                            this.begin_edit(window, cx);
                        }
                    },
                ))
            })
            .collect();

        let button = |id: &'static str, text: &'static str| {
            crate::a11y::button(div().id(id), text)
                .px_3()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0xaaaaaa))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(text)
        };
        let tab_button = |tab: Tab, active: bool| {
            let mut tab_div = div()
                .id(tab.caption())
                .role(gpui::Role::Tab)
                .aria_label(tab.caption())
                .aria_selected(active)
                .px_3()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0xaaaaaa))
                .cursor_pointer()
                .child(tab.caption());
            tab_div = if active {
                tab_div.bg(rgb(0xffffff))
            } else {
                tab_div.bg(rgb(0xe4e4e4))
            };
            tab_div.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.select_tab(tab, cx);
            }))
        };

        crate::a11y::dialog(div().id("favorite-dialog"), "履歴とお気に入りの管理")
            // F1: the page of the dialog
            .on_help_key("F_FAVORITE")
            .key_context("FavoriteDialog")
            .track_focus(&self.focus)
            .on_access_key(move |letter, window, cx| {
                this.update(cx, |dialog, cx| dialog.use_access_key(letter, window, cx))
            })
            .on_key_down(cx.listener(Self::on_key_down))
            .size_full()
            .relative()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .text_color(rgb(TEXT))
            .text_size(px(14.))
            .font_family(ui_font)
            .child(
                div()
                    .id("favorite-tabs")
                    .role(gpui::Role::TabList)
                    .aria_label("対象")
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .children(Tab::ALL.map(|tab| tab_button(tab, self.tab == tab))),
            )
            .child(
                div()
                    .id("favorite-rows")
                    .role(gpui::Role::ListBox)
                    .aria_label("一覧")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xaaaaaa))
                    .children(items),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(self.tab.is_editable(), |row| {
                        row.child(button("add", labels::ADD).on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| this.begin_add(window, cx),
                        )))
                    })
                    .child("削除:")
                    .child(button("delete-all", labels::ALL).on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| this.ask_delete_all(window, cx),
                    )))
                    .when(self.tab.has_favorites(), |row| {
                        row.child(
                            button("delete-non-favorites", labels::NON_FAVORITES).on_click(
                                cx.listener(|this, _: &ClickEvent, window, cx| {
                                    this.ask_delete_non_favorites(window, cx);
                                }),
                            ),
                        )
                    })
                    .when(self.tab.is_path_list(), |row| {
                        row.child(
                            button("delete-missing", labels::MISSING).on_click(cx.listener(
                                |this, _: &ClickEvent, window, cx| {
                                    this.ask_delete_missing(window, cx);
                                },
                            )),
                        )
                    })
                    .child(button("delete-selected", labels::SELECTED).on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.delete_selected_rows(cx)),
                    ))
                    .when(self.tab.can_add_except(), |row| {
                        row.child(
                            button("add-except", labels::ADD_EXCEPT).on_click(cx.listener(
                                |this, _: &ClickEvent, window, cx| {
                                    this.add_selected_to_except(window, cx);
                                },
                            )),
                        )
                    })
                    .child(div().flex_1())
                    .child(button("close", labels::CLOSE).on_click(
                        cx.listener(|this, _: &ClickEvent, window, _| this.close(window)),
                    ))
                    .child(
                        button("help", labels::HELP)
                            .on_click(|_: &ClickEvent, _, cx| open_help(cx)),
                    ),
            )
            .when_some(panel, |root, panel| root.child(panel))
    }
}

impl FavoriteDialog {
    /// The panel of 追加 and 編集 (`IDD_INPUT1`), over the dialog.
    fn render_panel(&self, window: &Window, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let panel = self.edit.as_ref()?;
        let (title, message) = match panel.target {
            EditTarget::Add => ("追加", "追加する文字列を入力してください。"),
            EditTarget::Edit(_) => ("編集", "編集する文字列を入力してください。"),
        };
        let button = |id: &'static str, label: &'static str, handle: &FocusHandle| {
            let ring = if handle.is_focused(window) {
                SELECTED_BG
            } else {
                0xaaaaaa
            };
            crate::a11y::button(div().id(id), label)
                .track_focus(handle)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(ring))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(label)
        };
        let inner = crate::a11y::dialog(div().id("favorite-edit"), title)
            .aria_description(message)
            .w(px(420.))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .child(title)
            .child(div().mt_2().child(message))
            .child(Input::new(&panel.input).aria_label(message))
            .child(
                div()
                    .mt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(button("favorite-edit-ok", "OK", &panel.ok_focus).on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.ok_edit(window, cx)),
                    ))
                    .child(
                        button("favorite-edit-cancel", "キャンセル", &panel.cancel_focus).on_click(
                            cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.finish_edit(None, window, cx)
                            }),
                        ),
                    ),
            );
        Some(
            div()
                .id("favorite-edit-overlay")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .occlude()
                .bg(gpui::rgba(0x00000055))
                .flex()
                .items_center()
                .justify_center()
                .child(inner)
                .into_any_element(),
        )
    }
}

/// Whether the dialog is open.
pub(crate) fn is_open(cx: &App) -> bool {
    cx.try_global::<OpenDialog>()
        .and_then(|d| d.0)
        .is_some_and(|handle| cx.windows().contains(&handle))
}

/// Shows the dialog; one that is open comes to the front instead of a second one.
pub(crate) fn open(cx: &mut App) {
    if let Some(handle) = cx.try_global::<OpenDialog>().and_then(|d| d.0)
        && cx.windows().contains(&handle)
    {
        handle
            .update(cx, |_, window, _| window.activate_window())
            .ok();
        return;
    }
    let history =
        super::current(cx).unwrap_or_else(|| History::new(15, 15, super::CASE_INSENSITIVE));
    // The place the last dialog was closed at, brought inside the screen (`OnInitDialog`).
    let saved = saved_rect(cx);
    let bounds = if saved.is_set() && saved.width() > 0 && saved.height() > 0 {
        saved.fitted(&work_areas(cx)).bounds()
    } else {
        Bounds::centered(None, size(px(760.), px(460.)), cx)
    };
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        icon: crate::app_icon::window_icon(),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("履歴とお気に入りの管理".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let opened = cx.open_window(options, move |window, cx| {
        let dialog: Entity<FavoriteDialog> = cx.new(|cx| FavoriteDialog::new(history, window, cx));
        cx.new(|cx| Root::new(dialog, window, cx))
    });
    if let Ok(handle) = opened {
        cx.set_global(OpenDialog(Some(handle.into())));
    }
}
