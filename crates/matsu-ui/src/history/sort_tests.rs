//! Tests of the history window's column sort, its right-click menu and the line that says another
//! window changed a history (`CDlgFavorite`: `LVN_COLUMNCLICK`, `RightMenu`, `RefreshList`).
//!
//! None of these tests touches a path of the operating system or a line ending: the paths are
//! written in the text (`/x/c`) and only compared as text.

use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};
use matsu_config::history::{FileEntry, History};

use super::dialog::{
    Column, FavoriteDialog, RightAct, RightRow, Row, Sort, Tab, display_order, refreshed_message,
    right_rows,
};
use super::{HistoryOptions, HistoryState, current};
use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;

fn row(text: &str) -> Row {
    Row {
        key: String::new(),
        favorite: false,
        text: text.to_owned(),
    }
}

fn sort(column: Column, ascending: bool) -> Option<Sort> {
    Some(Sort { column, ascending })
}

#[test]
fn without_a_sort_the_order_is_that_of_the_history() {
    let rows = [row("b"), row("a"), row("c")];
    assert_eq!(display_order(&rows, None), [0, 1, 2]);
}

#[test]
fn the_first_column_sorts_by_the_order_of_the_history_and_turns_round() {
    let rows = [row("b"), row("a"), row("c")];
    assert_eq!(display_order(&rows, sort(Column::Key, true)), [0, 1, 2]);
    assert_eq!(display_order(&rows, sort(Column::Key, false)), [2, 1, 0]);
}

#[test]
fn the_second_column_sorts_the_texts_without_regard_to_case() {
    // `CompareListViewFunc` compares with `wmemicmp`: "B" is not before "a".
    let rows = [
        row("b"),
        row("A"),
        row("C"),
        row("a2"),
        row("日本"),
        row("あ"),
    ];
    let order = display_order(&rows, sort(Column::Text, true));
    let sorted: Vec<&str> = order.iter().map(|&i| rows[i].text.as_str()).collect();
    assert_eq!(sorted, ["A", "a2", "b", "C", "あ", "日本"]);
    let order = display_order(&rows, sort(Column::Text, false));
    let sorted: Vec<&str> = order.iter().map(|&i| rows[i].text.as_str()).collect();
    assert_eq!(sorted, ["日本", "あ", "C", "b", "a2", "A"]);
}

#[test]
fn a_click_sorts_ascending_first_then_turns_the_order_and_another_column_starts_again() {
    let first = Sort::clicked(None, Column::Text);
    assert_eq!((first.column, first.ascending), (Column::Text, true));
    let second = Sort::clicked(Some(first), Column::Text);
    assert!(!second.ascending);
    let third = Sort::clicked(Some(second), Column::Text);
    assert!(third.ascending);
    // Another column is ascending whatever the order of the one before was (`ListViewSort`).
    let other = Sort::clicked(Some(second), Column::Key);
    assert_eq!((other.column, other.ascending), (Column::Key, true));
    // The marks that `ListViewSort` puts after the title.
    assert_eq!(first.mark(), "▼");
    assert_eq!(second.mark(), "▲");
}

#[test]
fn the_line_names_the_lists_that_changed() {
    assert_eq!(
        refreshed_message(&[Tab::Files, Tab::Search]),
        "履歴(ファイル、検索)が更新されたため編集中情報を破棄し再表示しました。"
    );
}

fn labels(rows: &[RightRow]) -> Vec<(&'static str, bool)> {
    rows.iter()
        .filter_map(|r| match r {
            RightRow::Item { label, enabled, .. } => Some((*label, *enabled)),
            RightRow::Separator => None,
        })
        .collect()
}

#[test]
fn the_right_click_menu_has_the_items_of_right_menu_grayed_as_sakura_grays_them() {
    // Files: not editable, favorites, paths, can add to the excluded list.
    let rows = right_rows(Tab::Files, true);
    assert_eq!(
        labels(&rows),
        [
            ("編集", false),
            ("新規追加", false),
            ("除外リストに追加", true),
            ("すべて削除", true),
            ("お気に入り以外削除", true),
            ("存在しない項目削除", true),
            ("選択項目削除", true),
        ]
    );
    assert_eq!(rows[1], RightRow::Separator);
    assert_eq!(rows[4], RightRow::Separator);
    // An empty list: nothing to edit or delete, but 新規追加 stays where adding is allowed.
    let empty = labels(&right_rows(Tab::Search, false));
    assert_eq!(
        empty,
        [
            ("編集", false),
            ("新規追加", true),
            ("すべて削除", false),
            ("お気に入り以外削除", false),
            ("存在しない項目削除", false),
            ("選択項目削除", false),
        ]
    );
    // The search list: editable, no favorites, no paths, no 除外リストに追加.
    let search = labels(&right_rows(Tab::Search, true));
    assert_eq!(search[0], ("編集", true));
    assert!(search.iter().all(|(l, _)| *l != "除外リストに追加"));
    assert!(search.contains(&("お気に入り以外削除", false)));
    assert!(search.contains(&("存在しない項目削除", false)));
    // The Grep folders and the current folders are paths and cannot be edited.
    let grep = labels(&right_rows(Tab::GrepFolder, true));
    assert_eq!(grep[0], ("編集", false));
    assert!(grep.contains(&("存在しない項目削除", true)));
}

// ---- the window ----

/// Files `a`, `B`, `c` (`c` is the newest).
fn sample() -> History {
    let mut h = History::new(5, 5, false);
    for name in ["a", "B", "c"] {
        h.add_file(FileEntry::new(format!("/x/{name}")));
    }
    h
}

fn setup(
    cx: &mut TestAppContext,
    history: History,
) -> (Entity<FavoriteDialog>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    cx.update(|cx| cx.set_global(HistoryState::in_memory(HistoryOptions::default())));
    cx.update(|cx| {
        open_editor_window(cx, SharedDocument::empty(), None).unwrap();
    });
    cx.update(|cx| {
        super::update(cx, |h| *h = history);
    });
    let initial = cx.update(|cx| current(cx).unwrap());
    cx.add_window_view(move |window, cx| FavoriteDialog::new(initial, window, cx))
}

fn shown(dialog: &Entity<FavoriteDialog>, cx: &mut VisualTestContext) -> Vec<String> {
    dialog.read_with(cx, |d, _| {
        let rows = d.rows();
        d.order()
            .into_iter()
            .map(|i| rows[i].text.clone())
            .collect()
    })
}

fn saved(cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, app| current(app).unwrap())
        .files
        .items()
        .iter()
        .map(|f| f.path.clone())
        .collect()
}

fn click_header(cx: &mut VisualTestContext, id: &'static str) {
    let at = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("{id} is painted"))
        .center();
    cx.simulate_click(at, Modifiers::default());
    cx.run_until_parked();
}

#[gpui::test]
fn a_click_on_a_header_sorts_the_list_and_not_the_history(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    assert_eq!(shown(&dialog, cx), ["/x/c", "/x/B", "/x/a"]);
    click_header(cx, "favorite-header-text");
    // Ascending, without regard to case.
    assert_eq!(shown(&dialog, cx), ["/x/a", "/x/B", "/x/c"]);
    // What is saved keeps its order.
    assert_eq!(saved(cx), ["/x/c", "/x/B", "/x/a"]);
    click_header(cx, "favorite-header-text");
    assert_eq!(shown(&dialog, cx), ["/x/c", "/x/B", "/x/a"]);
    click_header(cx, "favorite-header-key");
    assert_eq!(shown(&dialog, cx), ["/x/c", "/x/B", "/x/a"]);
    click_header(cx, "favorite-header-key");
    assert_eq!(shown(&dialog, cx), ["/x/a", "/x/B", "/x/c"]);
    assert_eq!(saved(cx), ["/x/c", "/x/B", "/x/a"]);
}

#[gpui::test]
fn each_tab_keeps_its_own_sort_while_the_dialog_is_open(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    click_header(cx, "favorite-header-text");
    dialog.update(cx, |d, cx| d.select_tab(Tab::Folders, cx));
    assert_eq!(dialog.read_with(cx, |d, _| d.sort()), None);
    dialog.update(cx, |d, cx| d.select_tab(Tab::Files, cx));
    assert_eq!(
        dialog.read_with(cx, |d, _| d.sort()),
        sort(Column::Text, true)
    );
}

#[gpui::test]
fn the_sort_stays_after_a_change_and_so_does_its_direction(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    click_header(cx, "favorite-header-text");
    click_header(cx, "favorite-header-text");
    assert_eq!(shown(&dialog, cx), ["/x/c", "/x/B", "/x/a"]);
    // Another window adds a file. `SetDataOne` sorts the list again; the direction stays
    // descending (Sakura's `ListViewSort(.., false)` would turn it ascending: type A).
    cx.update(|_, app| {
        super::update(app, |h| h.add_file(FileEntry::new("/x/d")));
    });
    dialog.update(cx, |d, cx| d.reload(cx));
    assert_eq!(shown(&dialog, cx), ["/x/d", "/x/c", "/x/B", "/x/a"]);
    assert_eq!(
        dialog.read_with(cx, |d, _| d.sort()),
        sort(Column::Text, false)
    );
}

#[gpui::test]
fn the_keys_and_the_range_follow_the_order_the_list_shows(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    click_header(cx, "favorite-header-text");
    // Shown: a (row 2), B (row 1), c (row 0). Down from nothing goes to the second shown row.
    cx.simulate_keystrokes("home");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect::<Vec<_>>()),
        [2]
    );
    cx.simulate_keystrokes("down");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect::<Vec<_>>()),
        [1]
    );
    // Shift+Down selects the next shown one: the rows 1 and 0.
    cx.simulate_keystrokes("shift-down");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect::<Vec<_>>()),
        [0, 1]
    );
    // What is deleted is what is selected, whatever the order.
    dialog.update(cx, |d, cx| d.delete_selected_rows(cx));
    assert_eq!(saved(cx), ["/x/a"]);
}

#[gpui::test]
fn deleting_in_a_sorted_list_selects_the_item_after_the_last_one_as_shown(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    click_header(cx, "favorite-header-text");
    // Shown: a, B, c. Select the middle one (B, row 1) and delete it: c is selected.
    dialog.update(cx, |d, cx| d.click_row(1, false, false, cx));
    dialog.update(cx, |d, cx| d.delete_selected_rows(cx));
    assert_eq!(saved(cx), ["/x/c", "/x/a"]);
    assert_eq!(
        dialog.read_with(cx, |d, _| {
            let rows = d.rows();
            d.selected()
                .iter()
                .map(|&i| rows[i].text.clone())
                .collect::<Vec<_>>()
        }),
        ["/x/c"]
    );
}

#[gpui::test]
fn the_line_tells_which_lists_another_window_changed_and_the_next_action_clears_it(
    cx: &mut TestAppContext,
) {
    let (dialog, cx) = setup(cx, sample());
    assert_eq!(dialog.read_with(cx, |d, _| d.message().to_owned()), "");
    cx.update(|_, app| {
        super::update(app, |h| h.add_file(FileEntry::new("/new/one")));
    });
    dialog.update(cx, |d, cx| d.reload(cx));
    // The file and its folder both changed.
    assert_eq!(
        dialog.read_with(cx, |d, _| d.message().to_owned()),
        "履歴(ファイル、フォルダー)が更新されたため編集中情報を破棄し再表示しました。"
    );
    // A tab change clears it (`TabSelectChange`).
    dialog.update(cx, |d, cx| d.select_tab(Tab::Search, cx));
    assert_eq!(dialog.read_with(cx, |d, _| d.message().to_owned()), "");
    // So does a delete.
    cx.update(|_, app| {
        super::update(app, |h| h.add_file(FileEntry::new("/new/two")));
    });
    dialog.update(cx, |d, cx| d.reload(cx));
    assert!(!dialog.read_with(cx, |d, _| d.message().is_empty()));
    dialog.update(cx, |d, cx| d.delete_selected_rows(cx));
    assert_eq!(dialog.read_with(cx, |d, _| d.message().to_owned()), "");
}

#[gpui::test]
fn a_change_the_dialog_made_itself_is_not_announced(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    dialog.update(cx, |d, cx| d.delete_selected_rows(cx));
    dialog.update(cx, |d, cx| d.reload(cx));
    assert_eq!(dialog.read_with(cx, |d, _| d.message().to_owned()), "");
}

#[gpui::test]
fn a_right_click_opens_the_menu_and_leaves_the_selection(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update(cx, |d, cx| d.click_row(1, false, false, cx));
    cx.run_until_parked();
    let at = cx.debug_bounds("favorite-rows").unwrap().center();
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    cx.run_until_parked();
    // Sakura's `NM_RCLICK` does not select what is under the pointer.
    assert_eq!(
        dialog.read_with(cx, |d, _| d.selected().iter().copied().collect::<Vec<_>>()),
        [1]
    );
    // The menu has the focus now, so the keys of the dialog do not reach the list.
    let before = dialog.read_with(cx, |d, _| d.selected().clone());
    cx.simulate_keystrokes("delete");
    assert_eq!(saved(cx).len(), 3, "Delete went to the menu, not the list");
    assert_eq!(dialog.read_with(cx, |d, _| d.selected().clone()), before);
}

#[gpui::test]
fn each_item_of_the_menu_does_what_its_button_does(cx: &mut TestAppContext) {
    let (dialog, cx) = setup(cx, sample());
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    dialog.update_in(cx, |d, window, cx| {
        d.right_act(RightAct::DeleteSelected, window, cx);
    });
    assert_eq!(saved(cx), ["/x/B", "/x/a"]);
    // 除外リストに追加 puts the selected path into the excluded list.
    dialog.update(cx, |d, cx| d.click_row(0, false, false, cx));
    dialog.update_in(cx, |d, window, cx| {
        d.right_act(RightAct::AddExcept, window, cx);
    });
    let except = cx.update(|_, app| current(app).unwrap()).except;
    assert_eq!(except, ["/x/B"]);
    // すべて削除 asks first.
    dialog.update_in(cx, |d, window, cx| {
        d.right_act(RightAct::DeleteAll, window, cx);
    });
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    assert!(saved(cx).is_empty());
    // 新規追加 and 編集 open the panel on a tab that can.
    dialog.update(cx, |d, cx| d.select_tab(Tab::Search, cx));
    dialog.update_in(cx, |d, window, cx| d.right_act(RightAct::Add, window, cx));
    assert!(dialog.read_with(cx, |d, _| d.is_editing()));
}
