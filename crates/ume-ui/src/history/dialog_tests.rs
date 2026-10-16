//! Tests of the history management window (`F_FAVORITE`): the operations on the history, and the
//! window in GPUI's headless test windows.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{Entity, TestAppContext, VisualTestContext};
use ume_config::history::{FileEntry, History};

use super::dialog::{
    FavoriteDialog, Tab, delete_all, delete_missing, delete_non_favorites, delete_selected,
    key_column, path_exists, rows, set_favorite,
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
        let path = std::env::temp_dir().join(format!("ume-ui-favorite-{}-{n}", std::process::id()));
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
