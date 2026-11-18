//! Tests of the history management window (`F_FAVORITE`): the operations on the history, and the
//! window in GPUI's headless test windows.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{Entity, TestAppContext, VisualTestContext};
use matsu_config::history::{FileEntry, History};
use matsu_config::settings::EditableList as _;

use super::dialog::{
    FavoriteDialog, Tab, add_to_except, append_except, delete_all, delete_missing,
    delete_non_favorites, delete_selected, edit_except, key_column, path_exists, rows,
    set_favorite,
};
use super::{HistoryOptions, HistoryState, current};
use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;

/// Files `a`, `b`, `c` (`c` is the newest), 2 shown.
fn sample() -> History {
    let mut h = History::new(2, 2, false);
    for name in ["a", "b", "c"] {
        h.add_file(FileEntry::new(format!("/x/{name}")));
    }
    h
}

fn paths(h: &History) -> Vec<&str> {
    h.files.items().iter().map(|f| f.path.as_str()).collect()
}

#[test]
fn the_first_column_has_the_key_and_marks_what_the_menu_does_not_show() {
    assert_eq!(key_column(0, true), "0   ");
    assert_eq!(key_column(9, true), "9   ");
    assert_eq!(key_column(10, true), "A   ");
    assert_eq!(key_column(35, true), "Z   ");
    assert_eq!(key_column(36, true), "0   ");
    assert_eq!(key_column(2, false), "2 (非表示)");
    let r = rows(&sample(), Tab::Files);
    assert_eq!(r.len(), 3);
    assert_eq!(r[0].key, "0   ");
    assert_eq!(r[2].key, "2 (非表示)");
    assert_eq!(r[0].text, "/x/c");
    assert!(r.iter().all(|r| !r.favorite));
}

#[test]
fn the_folders_have_their_own_rows() {
    let mut h = sample();
    h.add_file(FileEntry::new("/y/z"));
    let r = rows(&h, Tab::Folders);
    assert_eq!(
        r.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
        ["/y/", "/x/"]
    );
}

#[test]
fn a_favorite_comes_into_the_shown_part() {
    let mut h = sample();
    // [c, b, a]; `a` is hidden.
    set_favorite(&mut h, Tab::Files, 2, true);
    assert_eq!(paths(&h), ["/x/c", "/x/a", "/x/b"]);
    assert!(rows(&h, Tab::Files)[1].favorite);
    set_favorite(&mut h, Tab::Files, 1, false);
    assert!(!rows(&h, Tab::Files)[1].favorite);
}

#[test]
fn deleting_the_selected_rows_removes_the_right_ones() {
    let mut h = sample();
    delete_selected(&mut h, Tab::Files, &BTreeSet::from([0, 2]));
    assert_eq!(paths(&h), ["/x/b"]);
    // The folders are not touched.
    assert_eq!(h.folders.len(), 1);
}

#[test]
fn deleting_all_or_all_but_the_favorites() {
    let mut h = sample();
    set_favorite(&mut h, Tab::Files, 0, true);
    delete_non_favorites(&mut h, Tab::Files);
    assert_eq!(paths(&h), ["/x/c"]);
    delete_all(&mut h, Tab::Files);
    assert!(h.files.is_empty());
    assert_eq!(h.folders.len(), 1);
    delete_all(&mut h, Tab::Folders);
    assert!(h.folders.is_empty());
}

#[test]
fn deleting_what_is_gone() {
    let mut h = sample();
    delete_missing(&mut h, Tab::Files, |p| p != "/x/b");
    assert_eq!(paths(&h), ["/x/c", "/x/a"]);
    delete_missing(&mut h, Tab::Folders, |_| false);
    assert!(h.folders.is_empty());
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("matsu-ui-favorite-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_path_exists_as_a_file_or_a_folder_with_or_without_the_last_separator() {
    let dir = TempDir::new();
    let file = dir.0.join("f.txt");
    std::fs::write(&file, b"x").unwrap();
    assert!(path_exists(&file.to_string_lossy()));
    assert!(path_exists(&dir.0.to_string_lossy()));
    let with_separator = format!("{}{}", dir.0.display(), std::path::MAIN_SEPARATOR);
    assert!(path_exists(&with_separator));
    assert!(!path_exists(&dir.0.join("none").to_string_lossy()));
    assert!(!path_exists(&format!("{}/none/", dir.0.display())));
    assert!(!path_exists(""));
}

// ---- the window ----

fn setup(
    cx: &mut TestAppContext,
    history: History,
) -> (Entity<FavoriteDialog>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    cx.update(|cx| cx.set_global(HistoryState::in_memory(HistoryOptions::default())));
    // The dialog goes when the last editor window does, so there is one.
    cx.update(|cx| {
        open_editor_window(cx, SharedDocument::empty(), None).unwrap();
    });
    cx.update(|cx| {
        super::update(cx, |h| *h = history);
    });
    let initial = cx.update(|cx| current(cx).unwrap());
    cx.add_window_view(move |window, cx| FavoriteDialog::new(initial, window, cx))
}

fn texts(dialog: &Entity<FavoriteDialog>, cx: &mut VisualTestContext) -> Vec<String> {
    dialog.read_with(cx, |d, _| d.rows().into_iter().map(|r| r.text).collect())
}

fn answer(cx: &mut VisualTestContext, button: &str) {
    assert!(cx.has_pending_prompt(), "no prompt to answer {button:?}");
    cx.simulate_prompt_answer(button);
    cx.run_until_parked();
}

fn history_paths(cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, app| current(app).unwrap())
        .files
        .items()
        .iter()
        .map(|f| f.path.clone())
        .collect()
}

#[gpui::test]
fn the_window_lists_the_files_and_the_folders(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    assert_eq!(texts(&dialog, cx), ["/x/c", "/x/b", "/x/a"]);
    dialog.update(cx, |d, cx| d.select_tab(Tab::Folders, cx));
    assert_eq!(texts(&dialog, cx), ["/x/"]);
    assert_eq!(dialog.read_with(cx, |d, _| d.tab()), Tab::Folders);
}

#[gpui::test]
fn rows_are_selected_by_click_control_and_shift(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    let selected = |cx: &mut VisualTestContext| -> Vec<usize> {
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect())
    };
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    assert_eq!(selected(cx), [0]);
    dialog.update(cx, |d, cx| d.click_row(2, false, true, cx));
    assert_eq!(selected(cx), [0, 1, 2]);
    dialog.update(cx, |d, cx| d.click_row(1, true, false, cx));
    assert_eq!(selected(cx), [0, 2]);
    dialog.update(cx, |d, cx| d.click_row(1, false, false, cx));
    assert_eq!(selected(cx), [1]);
    // Another tab starts without a selection.
    dialog.update(cx, |d, cx| d.select_tab(Tab::Folders, cx));
    assert!(selected(cx).is_empty());
}

#[gpui::test]
fn the_check_box_makes_a_favorite_and_writes_the_history(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update(cx, |d, cx| d.toggle_favorite(2, cx));
    // The favorite came into the part the menu shows, so the rows moved.
    let rows = dialog.read_with(cx, |d, _| d.rows());
    assert_eq!(rows[1].text, "/x/a");
    assert!(rows[1].favorite);
    assert_eq!(history_paths(cx), ["/x/c", "/x/a", "/x/b"]);
    dialog.update(cx, |d, cx| d.toggle_favorite(1, cx));
    assert!(!dialog.read_with(cx, |d, _| d.rows())[1].favorite);
}

#[gpui::test]
fn deleting_the_selected_rows_selects_the_next_one(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    dialog.update(cx, |d, cx| d.delete_selected_rows(cx));
    assert_eq!(history_paths(cx), ["/x/b", "/x/a"]);
    assert_eq!(
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect::<Vec<_>>()),
        [0]
    );
    // The last row: the one before it is selected.
    dialog.update(cx, |d, cx| d.click_row(1, false, false, cx));
    dialog.update(cx, |d, cx| d.delete_selected_rows(cx));
    assert_eq!(history_paths(cx), ["/x/b"]);
    assert_eq!(
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect::<Vec<_>>()),
        [0]
    );
    // Nothing selected: nothing happens.
    dialog.update(cx, |d, cx| d.click_row(0, true, false, cx));
    dialog.update(cx, |d, cx| d.delete_selected_rows(cx));
    assert_eq!(history_paths(cx), ["/x/b"]);
}

#[gpui::test]
fn deleting_all_asks_first(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update_in(cx, |d, window, cx| d.ask_delete_all(window, cx));
    answer(cx, "いいえ(N)");
    assert_eq!(history_paths(cx).len(), 3);
    dialog.update_in(cx, |d, window, cx| d.ask_delete_all(window, cx));
    answer(cx, "はい(Y)");
    assert!(history_paths(cx).is_empty());
    assert!(texts(&dialog, cx).is_empty());
    // The folders were not asked about.
    assert_eq!(cx.update(|_, app| current(app).unwrap()).folders.len(), 1);
    // An empty list asks nothing.
    dialog.update_in(cx, |d, window, cx| d.ask_delete_all(window, cx));
    assert!(!cx.has_pending_prompt());
}

#[gpui::test]
fn deleting_all_but_the_favorites(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update(cx, |d, cx| d.toggle_favorite(1, cx));
    dialog.update_in(cx, |d, window, cx| d.ask_delete_non_favorites(window, cx));
    answer(cx, "はい(Y)");
    assert_eq!(history_paths(cx), ["/x/b"]);
}

#[gpui::test]
fn deleting_the_paths_that_are_gone(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let here = dir.0.join("here.txt");
    std::fs::write(&here, b"x").unwrap();
    let mut history = History::new(5, 5, false);
    history.add_file(FileEntry::new(here.to_string_lossy()));
    history.add_file(FileEntry::new(dir.0.join("gone.txt").to_string_lossy()));
    history.add_file(FileEntry::new("C:\\no\\such\\日本語.txt"));
    let (dialog, cx) = setup(cx, history);
    dialog.update_in(cx, |d, window, cx| d.ask_delete_missing(window, cx));
    answer(cx, "はい(Y)");
    assert_eq!(history_paths(cx), [here.to_string_lossy()]);
    // The folder of the files that are gone is still there, and so is this tab's other list.
    assert_eq!(texts(&dialog, cx), [here.to_string_lossy()]);
}

#[gpui::test]
fn the_window_follows_changes_made_elsewhere(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update(cx, |d, cx| d.click_row(2, false, false, cx));
    cx.update(|_, app| {
        super::update(app, |h| h.add_file(FileEntry::new("/new/one")));
    });
    dialog.update(cx, |d, cx| d.reload(cx));
    assert_eq!(texts(&dialog, cx)[0], "/new/one");
    assert_eq!(texts(&dialog, cx).len(), 4);
    // Whatever disappears from under the selection leaves it.
    cx.update(|_, app| {
        super::update(app, |h| h.files.clear());
    });
    dialog.update(cx, |d, cx| d.reload(cx));
    assert!(dialog.read_with(cx, |d, _| d.selected().is_empty()));
}

#[gpui::test]
fn the_keys_move_delete_and_close(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    cx.simulate_keystrokes("down down");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect::<Vec<_>>()),
        [2]
    );
    cx.simulate_keystrokes("up delete");
    assert_eq!(history_paths(cx), ["/x/c", "/x/a"]);
    // Space is the check box of the row.
    cx.simulate_keystrokes("space");
    assert!(
        dialog.read_with(cx, |d, _| d.rows())[0].favorite
            || dialog.read_with(cx, |d, _| d.rows())[1].favorite
    );
    // Tab changes the list.
    cx.simulate_keystrokes("tab");
    assert_eq!(dialog.read_with(cx, |d, _| d.tab()), Tab::Folders);
    let before = cx.windows().len();
    cx.simulate_keystrokes("escape");
    assert_eq!(cx.windows().len(), before - 1, "the dialog is closed");
}

#[gpui::test]
fn the_function_shows_one_window(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    cx.update(|cx| cx.set_global(HistoryState::in_memory(HistoryOptions::default())));
    cx.update(|cx| {
        open_editor_window(cx, SharedDocument::empty(), None).unwrap();
    });
    let before = cx.windows().len();
    cx.update(super::dialog::open);
    assert_eq!(cx.windows().len(), before + 1);
    // Asked again, it comes to the front instead of making another.
    cx.update(super::dialog::open);
    assert_eq!(cx.windows().len(), before + 1);
}

#[gpui::test]
fn the_letters_of_the_buttons_press_them(cx: &mut TestAppContext) {
    // `IDD_FAVORITE`: すべて(A), お気に入り以外(F), 存在しない項目(N), 選択項目(D), 閉じる(C)
    let (dialog, cx) = setup(cx, sample());
    cx.simulate_keystrokes("alt-d");
    // no row is selected: nothing to delete
    assert_eq!(history_paths(cx).len(), 3);
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    cx.simulate_keystrokes("alt-d");
    assert_eq!(history_paths(cx), ["/x/b", "/x/a"]);
    dialog.update(cx, |d, cx| d.toggle_favorite(0, cx));
    cx.simulate_keystrokes("alt-f");
    answer(cx, "はい(Y)");
    assert_eq!(history_paths(cx), ["/x/b"]);
    // the files of the sample do not exist
    cx.simulate_keystrokes("alt-n");
    answer(cx, "はい(Y)");
    assert!(history_paths(cx).is_empty());
    // すべて(A) asks nothing of an empty list
    cx.simulate_keystrokes("alt-shift-a");
    assert!(!cx.has_pending_prompt());
    let before = cx.windows().len();
    cx.simulate_keystrokes("alt-c");
    assert_eq!(cx.windows().len(), before - 1, "the dialog is closed");
}

#[gpui::test]
fn alt_a_asks_before_it_deletes_the_whole_history(cx: &mut TestAppContext) {
    let (_dialog, cx) = setup(cx, sample());
    cx.simulate_keystrokes("alt-a");
    answer(cx, "いいえ(N)");
    assert_eq!(history_paths(cx).len(), 3);
    cx.simulate_keystrokes("alt-a");
    answer(cx, "はい(Y)");
    assert!(history_paths(cx).is_empty());
}

#[test]
fn the_list_of_excluded_paths_has_rows_without_favorites() {
    let mut h = sample();
    h.except = vec!["/tmp/".into(), "secret".into()];
    let r = rows(&h, Tab::Except);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].text, "/tmp/");
    // The key column is there as on the other tabs, and nothing is hidden or checked.
    assert_eq!(r[1].key, "1   ");
    assert!(r.iter().all(|r| !r.favorite));
    // Favorites and the search for what is gone are for paths.
    assert!(!Tab::Except.has_favorites() && !Tab::Except.is_path_list());
    set_favorite(&mut h, Tab::Except, 0, true);
    delete_non_favorites(&mut h, Tab::Except);
    delete_missing(&mut h, Tab::Except, |_| false);
    assert_eq!(h.except.len(), 2);
    delete_selected(&mut h, Tab::Except, &BTreeSet::from([0]));
    assert_eq!(h.except, ["secret"]);
    delete_all(&mut h, Tab::Except);
    assert!(h.except.is_empty());
}

#[test]
fn paths_go_to_the_front_of_the_excluded_list_without_repeats() {
    let mut h = sample();
    h.except = vec!["a".into(), "b".into()];
    assert!(add_to_except(&mut h, vec!["c".into(), "b".into()]));
    // `AppendItemText` puts each at the front, but does nothing for a text that is there.
    assert_eq!(h.except, ["c", "a", "b"]);
    assert!(add_to_except(&mut h, vec!["A".into()]));
    assert_eq!(h.except, ["c", "a", "b"]);
}

#[test]
fn a_full_excluded_list_takes_what_fits_and_says_so() {
    let mut h = sample();
    h.except = (0..matsu_config::history::MAX_EXCEPT_MRU - 1)
        .map(|i| format!("t{i}"))
        .collect();
    assert!(!add_to_except(&mut h, vec!["x".into(), "y".into()]));
    assert_eq!(h.except.len(), matsu_config::history::MAX_EXCEPT_MRU);
    assert_eq!(h.except[0], "x");
    assert!(!h.except.contains(&"y".to_owned()));
}

#[gpui::test]
fn the_selected_files_are_added_to_the_excluded_list(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    dialog.update_in(cx, |d, window, cx| d.add_selected_to_except(window, cx));
    cx.run_until_parked();
    assert_eq!(cx.update(|_, app| current(app).unwrap()).except, ["/x/c"]);
    // Its own tab shows it, and Tab goes round the nine.
    dialog.update(cx, |d, cx| d.select_tab(Tab::Except, cx));
    assert_eq!(texts(&dialog, cx), ["/x/c"]);
    for _ in 0..Tab::ALL.len() - 1 {
        cx.simulate_keystrokes("tab");
    }
    assert_eq!(dialog.read_with(cx, |d, _| d.tab()), Tab::Folders);
    cx.simulate_keystrokes("tab");
    assert_eq!(dialog.read_with(cx, |d, _| d.tab()), Tab::Except);
    // Nothing is added from its own tab.
    dialog.update(cx, |d, cx| d.select_tab(Tab::Except, cx));
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    dialog.update_in(cx, |d, window, cx| d.add_selected_to_except(window, cx));
    assert_eq!(cx.update(|_, app| current(app).unwrap()).except, ["/x/c"]);
    // Delete takes it out.
    cx.simulate_keystrokes("delete");
    assert!(cx.update(|_, app| current(app).unwrap()).except.is_empty());
}

#[gpui::test]
fn the_help_is_alt_h_and_f1_and_the_window_stays(cx: &mut TestAppContext) {
    use crate::support::help::testing::{clear, page_of_function, take};
    let (_dialog, cx) = setup(cx, sample());
    let before = cx.windows().len();
    clear();
    // `CDlgFavorite`: `MyWinHelp( ..., F_FAVORITE )`
    cx.simulate_keystrokes("alt-h");
    assert_eq!(take(), [page_of_function("F_FAVORITE")]);
    cx.simulate_keystrokes("f1");
    assert_eq!(take(), [page_of_function("F_FAVORITE")]);
    assert_eq!(cx.windows().len(), before);
}

// ---- the tabs of text, and the editing of items ----

/// The text lists of the application, as the dialog reads them.
fn lists(cx: &mut VisualTestContext) -> super::texts::TextLists {
    cx.update(|_, app| super::texts::TextLists::current(app))
}

/// Puts strings in the lists of the application (the newest first, as the lists are shown).
fn fill(cx: &mut VisualTestContext) {
    cx.update(|_, app| {
        crate::search::history::edit_keys(app, |k| {
            for s in ["s1", "s2", "s3"] {
                k.search.append_text(s);
            }
            k.replace.append_text("r1");
            k.grep_file.append_text("*.rs");
            k.grep_folder.append_text("/no/such/dir");
        });
        crate::exec::edit_history(app, |h| {
            h.commands.append_text("sort");
            h.cur_dirs.append_text("/no/such/cwd");
        });
    });
}

#[test]
fn the_tabs_are_those_of_sakura_in_its_order() {
    let captions: Vec<_> = Tab::ALL.iter().map(|t| t.caption()).collect();
    assert_eq!(
        captions,
        [
            "ファイル",
            "フォルダー",
            "ファイル・フォルダー除外",
            "検索",
            "置換",
            "GREPファイル",
            "GREPフォルダー",
            "コマンド",
            "カレントディレクトリ"
        ]
    );
    // `m_aFavoriteInfo`: (m_bHaveFavorite, m_bFilePath, m_bEditable, m_bAddExcept) of each tab.
    let flags: Vec<_> = Tab::ALL
        .iter()
        .map(|t| {
            (
                t.has_favorites(),
                t.is_path_list(),
                t.is_editable(),
                t.can_add_except(),
            )
        })
        .collect();
    assert_eq!(
        flags,
        [
            (true, true, false, true),
            (true, true, false, true),
            (false, false, true, false),
            (false, false, true, false),
            (false, false, true, false),
            (false, false, true, false),
            (false, true, false, false),
            (false, false, true, false),
            (false, true, false, false),
        ]
    );
    // Tab goes round all nine.
    let mut tab = Tab::Files;
    for _ in 0..Tab::ALL.len() {
        tab = tab.next();
    }
    assert_eq!(tab, Tab::Files);
}

#[test]
fn the_excluded_list_adds_edits_and_compares_without_case() {
    let mut h = sample();
    assert!(append_except(&mut h, "Temp"));
    assert!(append_except(&mut h, "cache"));
    assert_eq!(h.except, ["cache", "Temp"]);
    // `AppendItemText` refuses what is there (in any case) and does not move it.
    assert!(!append_except(&mut h, "TEMP"));
    assert_eq!(h.except, ["cache", "Temp"]);
    assert!(!append_except(&mut h, ""));
    assert!(!append_except(&mut h, &"x".repeat(260)));
    assert!(append_except(&mut h, &"x".repeat(259)));
    // `EditItemText`: in place; the same text in another case is allowed, another item's is not.
    assert!(edit_except(&mut h, "Temp", "TEMP"));
    assert_eq!(h.except[2], "TEMP");
    assert!(!edit_except(&mut h, "TEMP", "CACHE"));
    assert!(!edit_except(&mut h, "TEMP", ""));
    assert!(!edit_except(&mut h, "gone", "x"));
    assert_eq!(h.except[1], "cache");
    // A full list drops its oldest text.
    h.except = (0..matsu_config::history::MAX_EXCEPT_MRU)
        .map(|i| format!("t{i}"))
        .collect();
    assert!(append_except(&mut h, "new"));
    assert_eq!(h.except.len(), matsu_config::history::MAX_EXCEPT_MRU);
    assert_eq!(h.except[0], "new");
    assert!(
        !h.except
            .contains(&format!("t{}", matsu_config::history::MAX_EXCEPT_MRU - 1))
    );
}

#[test]
fn the_text_lists_add_edit_and_delete_like_crecentimp() {
    use super::texts::{add_text, delete_missing, delete_texts, edit_text};
    use matsu_config::settings::SearchKeys;
    let mut keys = SearchKeys::default();
    assert!(add_text(&mut keys.search, Tab::Search, "b"));
    assert!(add_text(&mut keys.search, Tab::Search, "a"));
    // A string that is there is refused and stays where it is; case matters for a search.
    assert!(!add_text(&mut keys.search, Tab::Search, "b"));
    assert!(add_text(&mut keys.search, Tab::Search, "B"));
    assert_eq!(keys.search.texts(), ["B", "a", "b"]);
    // An empty string is refused, except in the replace strings.
    assert!(!add_text(&mut keys.search, Tab::Search, ""));
    assert!(add_text(&mut keys.replace, Tab::Replace, ""));
    assert_eq!(keys.replace.texts(), [""]);
    // Edit: in place, never onto another item.
    assert!(edit_text(&mut keys.search, Tab::Search, "a", "日本語"));
    assert_eq!(keys.search.texts(), ["B", "日本語", "b"]);
    assert!(!edit_text(&mut keys.search, Tab::Search, "日本語", "b"));
    assert!(!edit_text(&mut keys.search, Tab::Search, "日本語", ""));
    assert!(!edit_text(&mut keys.search, Tab::Search, "gone", "x"));
    // The Grep boxes compare without regard to case: only the item itself may change its case.
    assert!(add_text(&mut keys.grep_file, Tab::GrepFile, "*.rs"));
    assert!(!add_text(&mut keys.grep_file, Tab::GrepFile, "*.RS"));
    assert!(edit_text(
        &mut keys.grep_file,
        Tab::GrepFile,
        "*.rs",
        "*.RS"
    ));
    // A string of the whole buffer or more is refused (`ValidateReceiveType`).
    assert!(!add_text(&mut keys.search, Tab::Search, &"x".repeat(260)));
    assert!(add_text(&mut keys.search, Tab::Search, &"x".repeat(259)));
    // Delete by text, the last first.
    delete_texts(&mut keys.search, &["b".to_owned(), "B".to_owned()]);
    assert_eq!(keys.search.texts()[1..], ["日本語"]);
    // 存在しない項目: a string of several folders stays while one of them is there.
    let mut dirs = matsu_config::settings::ExecHistory::default();
    for d in [
        "/gone",
        "/here",
        "/here;/gone",
        "/gone;/gone2",
        " \"/here\" ;/x",
    ] {
        dirs.cur_dirs.append_text(d);
    }
    delete_missing(&mut dirs.cur_dirs, |p| p == "/here");
    assert_eq!(
        dirs.cur_dirs.texts(),
        [" \"/here\" ;/x", "/here;/gone", "/here"]
    );
}

#[gpui::test]
fn the_window_has_the_six_text_tabs(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    fill(cx);
    dialog.update(cx, |d, cx| d.reload(cx));
    for (tab, expected) in [
        (Tab::Search, vec!["s3", "s2", "s1"]),
        (Tab::Replace, vec!["r1"]),
        (Tab::GrepFile, vec!["*.rs"]),
        (Tab::GrepFolder, vec!["/no/such/dir"]),
        (Tab::Command, vec!["sort"]),
        (Tab::CurDir, vec!["/no/such/cwd"]),
    ] {
        dialog.update(cx, |d, cx| d.select_tab(tab, cx));
        assert_eq!(texts(&dialog, cx), expected, "{tab:?}");
    }
    // Each row has the access key of the first column, and nothing is hidden.
    dialog.update(cx, |d, cx| d.select_tab(Tab::Search, cx));
    let keys: Vec<_> = dialog.read_with(cx, |d, _| d.rows().into_iter().map(|r| r.key).collect());
    assert_eq!(keys, ["0   ", "1   ", "2   "]);
}

#[gpui::test]
fn each_text_tab_deletes_selected_all_and_missing(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    fill(cx);
    dialog.update(cx, |d, cx| d.reload(cx));
    // 選択項目 on 検索 (the next row is selected, as on the file tab).
    dialog.update(cx, |d, cx| d.select_tab(Tab::Search, cx));
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    dialog.update(cx, |d, cx| d.delete_selected_rows(cx));
    assert_eq!(lists(cx).keys.search.texts(), ["s2", "s1"]);
    // The other lists are not touched.
    assert_eq!(lists(cx).keys.replace.texts(), ["r1"]);
    // すべて asks first.
    dialog.update_in(cx, |d, window, cx| d.ask_delete_all(window, cx));
    answer(cx, "はい(Y)");
    assert!(lists(cx).keys.search.texts().is_empty());
    assert!(texts(&dialog, cx).is_empty());
    // 存在しない項目 is for GREPフォルダー and カレントディレクトリ, not for 検索.
    dialog.update_in(cx, |d, window, cx| d.ask_delete_missing(window, cx));
    assert!(!cx.has_pending_prompt());
    for tab in [Tab::GrepFolder, Tab::CurDir] {
        dialog.update(cx, |d, cx| d.select_tab(tab, cx));
        dialog.update_in(cx, |d, window, cx| d.ask_delete_missing(window, cx));
        answer(cx, "はい(Y)");
        assert!(texts(&dialog, cx).is_empty(), "{tab:?}");
    }
    assert!(lists(cx).keys.grep_folder.texts().is_empty());
    assert!(lists(cx).exec.cur_dirs.texts().is_empty());
    // お気に入り以外 is not there for these tabs.
    dialog.update(cx, |d, cx| d.select_tab(Tab::Command, cx));
    dialog.update_in(cx, |d, window, cx| d.ask_delete_non_favorites(window, cx));
    assert!(!cx.has_pending_prompt());
    assert_eq!(lists(cx).exec.commands.texts(), ["sort"]);
}

#[gpui::test]
fn the_add_panel_adds_to_the_list_and_cancel_changes_nothing(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    fill(cx);
    dialog.update(cx, |d, cx| d.reload(cx));
    dialog.update(cx, |d, cx| d.select_tab(Tab::Search, cx));
    dialog.update_in(cx, |d, window, cx| d.begin_add(window, cx));
    assert!(dialog.read_with(cx, |d, _| d.is_editing()));
    dialog.update_in(cx, |d, window, cx| {
        d.finish_edit(Some("new".into()), window, cx)
    });
    assert!(!dialog.read_with(cx, |d, _| d.is_editing()));
    assert_eq!(lists(cx).keys.search.texts(), ["new", "s3", "s2", "s1"]);
    // Cancel (and Esc) change nothing.
    dialog.update_in(cx, |d, window, cx| d.begin_add(window, cx));
    cx.simulate_keystrokes("escape");
    assert!(!dialog.read_with(cx, |d, _| d.is_editing()));
    assert_eq!(lists(cx).keys.search.texts().len(), 4);
    // A string that is there changes nothing.
    dialog.update_in(cx, |d, window, cx| d.begin_add(window, cx));
    dialog.update_in(cx, |d, window, cx| {
        d.finish_edit(Some("s1".into()), window, cx)
    });
    assert_eq!(lists(cx).keys.search.texts(), ["new", "s3", "s2", "s1"]);
    // The tabs that cannot be added to open no panel.
    dialog.update(cx, |d, cx| d.select_tab(Tab::CurDir, cx));
    dialog.update_in(cx, |d, window, cx| d.begin_add(window, cx));
    assert!(!dialog.read_with(cx, |d, _| d.is_editing()));
    dialog.update(cx, |d, cx| d.select_tab(Tab::Files, cx));
    dialog.update_in(cx, |d, window, cx| d.begin_add(window, cx));
    assert!(!dialog.read_with(cx, |d, _| d.is_editing()));
}

#[gpui::test]
fn the_edit_panel_replaces_the_item_where_it_is(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    fill(cx);
    dialog.update(cx, |d, cx| d.reload(cx));
    dialog.update(cx, |d, cx| d.select_tab(Tab::Command, cx));
    // Nothing selected: nothing to edit.
    dialog.update_in(cx, |d, window, cx| d.begin_edit(window, cx));
    assert!(!dialog.read_with(cx, |d, _| d.is_editing()));
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    dialog.update_in(cx, |d, window, cx| d.begin_edit(window, cx));
    assert!(dialog.read_with(cx, |d, _| d.is_editing()));
    dialog.update_in(cx, |d, window, cx| {
        d.finish_edit(Some("sort /r".into()), window, cx);
    });
    assert_eq!(lists(cx).exec.commands.texts(), ["sort /r"]);
    // The edited item stays selected.
    assert_eq!(
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect::<Vec<_>>()),
        [0]
    );
    // The folders of カレントディレクトリ cannot be edited.
    dialog.update(cx, |d, cx| d.select_tab(Tab::CurDir, cx));
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    dialog.update_in(cx, |d, window, cx| d.begin_edit(window, cx));
    assert!(!dialog.read_with(cx, |d, _| d.is_editing()));
    // F2 opens the panel on an editable tab.
    dialog.update(cx, |d, cx| d.select_tab(Tab::Search, cx));
    dialog.update(cx, |d, cx| d.click_row(1, false, false, cx));
    cx.simulate_keystrokes("f2");
    assert!(dialog.read_with(cx, |d, _| d.is_editing()));
    // A text of another item is refused and the list stays.
    dialog.update_in(cx, |d, window, cx| {
        d.finish_edit(Some("s3".into()), window, cx)
    });
    assert_eq!(lists(cx).keys.search.texts(), ["s3", "s2", "s1"]);
}

#[gpui::test]
fn the_excluded_tab_adds_and_edits_its_strings(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update(cx, |d, cx| d.select_tab(Tab::Except, cx));
    dialog.update_in(cx, |d, window, cx| d.begin_add(window, cx));
    dialog.update_in(cx, |d, window, cx| {
        d.finish_edit(Some("%MYDOC%".into()), window, cx)
    });
    dialog.update_in(cx, |d, window, cx| d.begin_add(window, cx));
    dialog.update_in(cx, |d, window, cx| {
        d.finish_edit(Some("node_modules".into()), window, cx)
    });
    assert_eq!(
        cx.update(|_, app| current(app).unwrap()).except,
        ["node_modules", "%MYDOC%"]
    );
    dialog.update(cx, |d, cx| d.click_row(1, false, false, cx));
    dialog.update_in(cx, |d, window, cx| d.begin_edit(window, cx));
    dialog.update_in(cx, |d, window, cx| {
        d.finish_edit(Some("%DESKTOP%".into()), window, cx)
    });
    assert_eq!(
        cx.update(|_, app| current(app).unwrap()).except,
        ["node_modules", "%DESKTOP%"]
    );
    // Added to by hand, deleted by the key, emptied by すべて.
    cx.simulate_keystrokes("delete");
    assert_eq!(
        cx.update(|_, app| current(app).unwrap()).except,
        ["node_modules"]
    );
    dialog.update_in(cx, |d, window, cx| d.ask_delete_all(window, cx));
    answer(cx, "はい(Y)");
    assert!(cx.update(|_, app| current(app).unwrap()).except.is_empty());
}

#[gpui::test]
fn a_text_list_edit_is_saved_in_the_settings_file_and_does_not_come_back(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.0.join("matsu.ini");
    cx.update(|app| {
        crate::search::history::install(
            app,
            crate::search::history::SearchHistory::load(None, Some(path.clone())),
        );
        crate::exec::install(app, crate::exec::ExecPrefs::load(None, Some(path.clone())));
    });
    let (dialog, cx) = setup(cx, sample());
    fill(cx);
    let saved = |path: &PathBuf| {
        let profile = matsu_config::Profile::load(path).unwrap();
        (
            matsu_config::settings::SearchKeys::from_profile(&profile),
            matsu_config::settings::ExecHistory::from_profile(&profile),
        )
    };
    let (keys, exec) = saved(&path);
    assert_eq!(keys.search.texts(), ["s3", "s2", "s1"]);
    assert_eq!(exec.commands.texts(), ["sort"]);
    // A deleted string is gone from the file, and a search that adds another one does not bring
    // it back.
    dialog.update(cx, |d, cx| d.reload(cx));
    dialog.update(cx, |d, cx| d.select_tab(Tab::Search, cx));
    dialog.update(cx, |d, cx| d.click_row(1, false, false, cx));
    dialog.update(cx, |d, cx| d.delete_selected_rows(cx));
    cx.update(|_, app| {
        crate::search::history::remember_search(app, "later");
    });
    let (keys, _) = saved(&path);
    assert_eq!(keys.search.texts(), ["later", "s3", "s1"]);
    // An edit of the commands is saved too, and the choices of the dialog stay.
    cx.update(|_, app| {
        crate::exec::remember(app, "dir", "/w", matsu_core::exec::ExecFlags::DEFAULT);
    });
    dialog.update(cx, |d, cx| d.reload(cx));
    dialog.update(cx, |d, cx| d.select_tab(Tab::Command, cx));
    dialog.update(cx, |d, cx| d.click_row(1, false, false, cx));
    dialog.update_in(cx, |d, window, cx| d.begin_edit(window, cx));
    dialog.update_in(cx, |d, window, cx| {
        d.finish_edit(Some("sort /r".into()), window, cx)
    });
    let (_, exec) = saved(&path);
    assert_eq!(exec.commands.texts(), ["dir", "sort /r"]);
    assert!(exec.flags.is_some());
}

#[gpui::test]
fn typing_in_the_panel_and_enter_adds_without_closing_the_dialog(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    fill(cx);
    dialog.update(cx, |d, cx| d.reload(cx));
    dialog.update(cx, |d, cx| d.select_tab(Tab::GrepFile, cx));
    let windows = cx.windows().len();
    dialog.update_in(cx, |d, window, cx| d.begin_add(window, cx));
    cx.simulate_input("*.toml");
    cx.simulate_keystrokes("enter");
    assert_eq!(lists(cx).keys.grep_file.texts(), ["*.toml", "*.rs"]);
    assert!(!dialog.read_with(cx, |d, _| d.is_editing()));
    // Enter went to the field, not to the dialog behind it.
    assert_eq!(cx.windows().len(), windows);
    // Esc in the panel closes the panel only; the next Esc closes the dialog.
    dialog.update_in(cx, |d, window, cx| d.begin_add(window, cx));
    cx.simulate_input("x");
    cx.simulate_keystrokes("escape");
    assert_eq!(cx.windows().len(), windows);
    assert_eq!(lists(cx).keys.grep_file.texts().len(), 2);
    cx.simulate_keystrokes("escape");
    assert_eq!(cx.windows().len(), windows - 1);
}

#[gpui::test]
fn an_answer_that_comes_after_a_tab_change_still_deletes_the_list_that_was_asked_about(
    cx: &mut TestAppContext,
) {
    let (dialog, cx) = setup(cx, sample());
    fill(cx);
    dialog.update(cx, |d, cx| d.reload(cx));
    dialog.update(cx, |d, cx| d.select_tab(Tab::Search, cx));
    dialog.update_in(cx, |d, window, cx| d.ask_delete_all(window, cx));
    dialog.update(cx, |d, cx| d.select_tab(Tab::Replace, cx));
    answer(cx, "はい(Y)");
    assert!(lists(cx).keys.search.texts().is_empty());
    assert_eq!(lists(cx).keys.replace.texts(), ["r1"]);
    assert_eq!(texts(&dialog, cx), ["r1"]);
}

#[gpui::test]
fn the_range_anchor_does_not_outlive_the_rows(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    fill(cx);
    dialog.update(cx, |d, cx| d.reload(cx));
    dialog.update(cx, |d, cx| d.select_tab(Tab::Search, cx));
    dialog.update(cx, |d, cx| d.click_row(2, false, false, cx));
    // Another window empties the list; the dialog follows.
    cx.update(|_, app| {
        crate::search::history::edit_keys(app, |k| k.search.clear_all());
        crate::search::history::edit_keys(app, |k| {
            k.search.append_text("only");
        });
    });
    dialog.update(cx, |d, cx| d.reload(cx));
    dialog.update(cx, |d, cx| d.click_row(0, false, true, cx));
    assert_eq!(
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect::<Vec<_>>()),
        [0]
    );
}
