//! The tab list (the ▼ button and `F_WINLIST` with the bar shown), the close button at the right
//! end of the bar and the caption format of the tabs.
//!
//! Sakura: `CTabWnd::TabListMenu`, `OnLButtonDown`, `OnRButtonDown`, `OnLButtonUp`
//! (`sakura_core/window/CTabWnd.cpp`), `CEditWnd::UpdateCaption`.

use gpui::{AnyWindowHandle, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};

use super::bar::close_function;
use super::caption::TabCaption;
use super::file_tests::{open_file, temp_dir};
use super::list::TabList;
use super::settings::{self, TabOptions, TabSettings};
use super::tests::{
    active_tab, active_view, captions, run, tab_count, tab_mode, tabs, type_text, visual, window,
    windows, workspace,
};
use crate::app::Workspace;

fn list_of(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<Entity<TabList>> {
    active_view(cx, handle).read_with(cx, |v, _| v.tab_list.clone())
}

/// The rows of the list that is open in the window.
fn shape(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<String> {
    let list = list_of(cx, handle).expect("the tab list is open");
    list.update(cx, |list, cx| list.shape(cx))
}

fn set_options(cx: &mut TestAppContext, change: impl FnOnce(&mut TabOptions)) {
    cx.update(|app| {
        let mut options = settings::options(app);
        change(&mut options);
        app.set_global(options);
    });
}

fn click_at(cx: &mut VisualTestContext, selector: &'static str, button: MouseButton) {
    let at = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} is painted"))
        .center();
    cx.simulate_mouse_down(at, button, Modifiers::default());
    cx.simulate_mouse_up(at, button, Modifiers::default());
    cx.run_until_parked();
}

/// A window with the untitled document and the files `names` of a new folder as tabs.
fn window_with_files(
    cx: &mut TestAppContext,
    name: &str,
    names: &[&str],
) -> (AnyWindowHandle, std::path::PathBuf) {
    let dir = temp_dir(name);
    let handle = window(cx, true);
    for file in names {
        let path = dir.join(file);
        std::fs::write(&path, "text").unwrap();
        open_file(cx, handle, &path);
    }
    (handle, dir)
}

// ---- the rows ----

#[gpui::test]
fn the_list_has_the_captions_of_the_tabs_sorted_like_the_setting_says(cx: &mut TestAppContext) {
    let (handle, dir) = window_with_files(cx, "list-sort", &["b.txt", "A.txt", "a.txt"]);
    // Sakura sorts by default (`bSortTabList`); "a" and "A" are side by side.
    run(cx, handle, "F_WINLIST");
    assert_eq!(
        shape(cx, handle)[2..],
        ["(無題)", "a.txt", "A.txt", "b.txt"]
            .map(String::from)
            .map(|s| {
                // The tab of this window, the last one opened, has the mark.
                if s == "a.txt" { format!("*{s}") } else { s }
            })
    );
    run(cx, handle, "F_WINLIST");
    assert!(
        list_of(cx, handle).is_none(),
        "the function closes it again"
    );

    set_options(cx, |o| o.sort_list = false);
    run(cx, handle, "F_WINLIST");
    assert_eq!(
        shape(cx, handle)[2..],
        ["(無題)", "b.txt", "A.txt", "*a.txt"]
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn the_list_has_the_paths_when_the_setting_says_so(cx: &mut TestAppContext) {
    let (handle, dir) = window_with_files(cx, "list-paths", &["b.txt", "a.txt"]);
    set_options(cx, |o| o.list_full = true);
    run(cx, handle, "F_WINLIST");
    let path = |name: &str| dir.join(name).to_string_lossy().into_owned();
    // A document with no file shows its caption; the others the path (sorted by it).
    assert_eq!(
        shape(cx, handle),
        [
            "[switch:names]".to_owned(),
            "-".to_owned(),
            "(無題)".to_owned(),
            format!("*{}", path("a.txt")),
            path("b.txt"),
        ]
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn an_ampersand_in_a_name_is_shown_as_it_is(cx: &mut TestAppContext) {
    let (handle, dir) = window_with_files(cx, "list-amp", &["a&b.txt"]);
    run(cx, handle, "F_WINLIST");
    assert_eq!(shape(cx, handle)[2..], ["(無題)", "*a&b.txt"]);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn the_paths_follow_the_file_name_display(cx: &mut TestAppContext) {
    use crate::file_names::{FileNames, NoFolders};

    let (handle, dir) = window_with_files(cx, "list-display", &["a.txt"]);
    let from = format!("{}{}", dir.display(), std::path::MAIN_SEPARATOR);
    let rows = [vec![from, "作業:".to_owned()]];
    cx.update(|app| app.set_global(FileNames::new(true, 100, &rows, &NoFolders, false)));
    set_options(cx, |o| o.list_full = true);
    run(cx, handle, "F_WINLIST");
    assert_eq!(shape(cx, handle)[2..], ["(無題)", "*作業:a.txt"]);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn the_switch_changes_the_display_and_keeps_the_choice_in_the_file(cx: &mut TestAppContext) {
    use crate::history::{HistoryOptions, HistoryState};

    let dir = temp_dir("list-switch");
    let ini = dir.join("ume.ini");
    std::fs::write(&ini, "[Common]\r\nm_bRestoreCurPosition=1\r\n").unwrap();
    let file = dir.join("a.txt");
    std::fs::write(&file, "text").unwrap();
    let handle = window(cx, true);
    cx.update(|app| {
        let store = ume_config::history::HistoryStore::new(&ini, false);
        app.set_global(HistoryState::on_disk(store, HistoryOptions::default()));
    });
    open_file(cx, handle, &file);
    run(cx, handle, "F_WINLIST");
    assert_eq!(shape(cx, handle)[0], "[switch:paths]");
    // The access key of the item is W.
    visual(cx, handle).simulate_keystrokes("w");
    cx.run_until_parked();
    assert!(cx.update(|app| settings::options(app).list_full));
    assert_eq!(shape(cx, handle)[0], "[switch:names]");
    assert!(shape(cx, handle)[3].ends_with("a.txt") && shape(cx, handle)[3].len() > "*a.txt".len());
    let text = std::fs::read_to_string(&ini).unwrap();
    assert!(text.contains("bTab_ListFull=1"), "{text}");
    assert!(
        text.contains("m_bRestoreCurPosition=1"),
        "the rest of the file stays: {text}"
    );
    // And back; the next list starts with the choice that was kept.
    visual(cx, handle).simulate_keystrokes("w");
    cx.run_until_parked();
    assert_eq!(shape(cx, handle)[0], "[switch:paths]");
    assert!(
        std::fs::read_to_string(&ini)
            .unwrap()
            .contains("bTab_ListFull=0")
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn choosing_an_entry_makes_that_tab_the_active_one(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    assert_eq!(active_tab(cx, handle), 2);
    run(cx, handle, "F_WINLIST");
    // The list starts on the tab of this window (the third item after the switch and the line).
    assert_eq!(
        list_of(cx, handle)
            .unwrap()
            .read_with(cx, |l, _| l.selected()),
        4
    );
    visual(cx, handle).simulate_keystrokes("up up enter");
    cx.run_until_parked();
    assert!(list_of(cx, handle).is_none());
    assert_eq!(active_tab(cx, handle), 0);
}

#[gpui::test]
fn escape_closes_the_list_and_chooses_nothing(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    run(cx, handle, "F_WINLIST");
    visual(cx, handle).simulate_keystrokes("escape");
    assert!(list_of(cx, handle).is_none());
    assert_eq!(active_tab(cx, handle), 2);
}

// ---- the ▼ button ----

#[gpui::test]
fn the_button_lists_the_captions_on_a_left_click_and_the_paths_on_a_right_click(
    cx: &mut TestAppContext,
) {
    let (handle, dir) = window_with_files(cx, "list-button", &["a.txt"]);
    click_at(visual(cx, handle), "tab-list-button", MouseButton::Left);
    // Without the item that switches the display (`TabListMenu( pt, FALSE, ... )`).
    assert_eq!(shape(cx, handle), ["(無題)", "*a.txt"]);
    visual(cx, handle).simulate_keystrokes("escape");
    click_at(visual(cx, handle), "tab-list-button", MouseButton::Right);
    let path = dir.join("a.txt").to_string_lossy().into_owned();
    assert_eq!(shape(cx, handle), ["(無題)".to_owned(), format!("*{path}")]);
    // The choice kept for the function does not change the button.
    visual(cx, handle).simulate_keystrokes("escape");
    set_options(cx, |o| o.list_full = true);
    click_at(visual(cx, handle), "tab-list-button", MouseButton::Left);
    assert_eq!(shape(cx, handle), ["(無題)", "*a.txt"]);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn the_button_hides_the_other_windows_behind_an_item(cx: &mut TestAppContext) {
    let first = window(cx, true);
    run(cx, first, "F_FILENEW_NEWWINDOW");
    let second = cx.windows()[1];
    type_text(cx, second, "b");
    click_at(visual(cx, first), "tab-list-button", MouseButton::Left);
    assert_eq!(shape(cx, first), ["*(無題)", "-", "[all]"]);
    // 「すべて表示」 (access key A) lists them.
    visual(cx, first).simulate_keystrokes("a");
    cx.run_until_parked();
    assert_eq!(shape(cx, first), ["*(無題)", "-", "(無題)(更新)"]);
    // Choosing one brings that window to the front.
    visual(cx, first).simulate_keystrokes("down enter");
    cx.run_until_parked();
    assert_eq!(cx.update(|app| app.active_window()), Some(second));
    assert_eq!(windows(cx), 2);
}

#[gpui::test]
fn the_function_lists_the_other_windows_after_a_separator(cx: &mut TestAppContext) {
    let first = window(cx, true);
    run(cx, first, "F_FILENEW_NEWWINDOW");
    run(cx, first, "F_WINLIST");
    assert_eq!(
        shape(cx, first),
        ["[switch:paths]", "-", "*(無題)", "-", "(無題)"]
    );
}

#[gpui::test]
fn without_the_grouping_the_bar_and_the_list_have_all_the_windows(cx: &mut TestAppContext) {
    let first = window(cx, true);
    tab_mode(cx, true, false);
    run(cx, first, "F_FILENEW_NEWWINDOW");
    run(cx, first, "F_WINLIST");
    assert_eq!(
        shape(cx, first),
        ["[switch:paths]", "-", "*(無題)", "(無題)"]
    );
}

// ---- the close button ----

#[test]
fn the_close_button_closes_what_sakura_says() {
    let tab = |show_bar, grouped| TabSettings {
        show_bar,
        grouped,
        new_window: false,
    };
    let options = |close_one_win| TabOptions {
        close_one_win,
        ..TabOptions::default()
    };
    // `CTabWnd::OnLButtonUp`: the tab mode closes the tab, or the group with `bTab_CloseOneWin`.
    assert_eq!(
        close_function(tab(true, true), options(false)),
        "F_WINCLOSE"
    );
    assert_eq!(
        close_function(tab(true, true), options(true)),
        "F_GROUPCLOSE"
    );
    // Without the grouping the button of the window closes one, so this one closes everything.
    assert_eq!(
        close_function(tab(true, false), options(false)),
        "F_EXITALLEDITORS"
    );
    assert_eq!(
        close_function(tab(true, false), options(true)),
        "F_EXITALLEDITORS"
    );
}

#[gpui::test]
fn the_close_button_closes_the_tab_of_the_window(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    click_at(visual(cx, handle), "tab-bar-close", MouseButton::Left);
    assert_eq!(tab_count(cx, handle), 2);
    assert_eq!(windows(cx), 1);
}

#[gpui::test]
fn the_close_button_closes_the_group_with_the_option(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    set_options(cx, |o| {
        o.close_one_win = true;
        o.retain_empty = false;
    });
    click_at(visual(cx, handle), "tab-bar-close", MouseButton::Left);
    assert_eq!(windows(cx), 0);
}

#[gpui::test]
fn the_close_button_asks_about_unsaved_changes(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    type_text(cx, handle, "x");
    click_at(visual(cx, handle), "tab-bar-close", MouseButton::Left);
    super::tests::answer(cx, handle, "キャンセル");
    assert_eq!(tab_count(cx, handle), 2);
}

// ---- the caption ----

fn caption(cx: &mut TestAppContext, handle: AnyWindowHandle) -> String {
    let view = active_view(cx, handle);
    view.read_with(cx, Workspace::caption_of)
}

#[gpui::test]
fn the_default_caption_is_what_the_tab_always_showed(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    assert_eq!(caption(cx, handle), "(無題)");
    type_text(cx, handle, "x");
    assert_eq!(caption(cx, handle), "(無題)(更新)");
    run(cx, handle, "F_VIEWMODE");
    assert_eq!(caption(cx, handle), "(無題)(更新)(ビューモード)");
    run(cx, handle, "F_VIEWMODE");
    run(cx, handle, "F_RECKEYMACRO");
    assert_eq!(caption(cx, handle), "(無題)(更新)【キーマクロの記録中】");
    run(cx, handle, "F_RECKEYMACRO");
    // A Grep window shows 【Grep】 and its key, cut to 32 units.
    active_view(cx, handle).update(cx, |v, cx| v.pretend_grep_running(cx));
    assert_eq!(caption(cx, handle), "【Grep】(更新)");
}

#[gpui::test]
fn a_users_format_makes_the_caption_of_every_tab(cx: &mut TestAppContext) {
    let (handle, dir) = window_with_files(cx, "caption-format", &["a.txt"]);
    cx.update(|app| app.set_global(TabCaption("[$f]${U?*$}".to_owned())));
    cx.run_until_parked();
    assert_eq!(captions(cx, handle), ["[(無題)]", "[a.txt]"]);
    type_text(cx, handle, "x");
    assert_eq!(captions(cx, handle), ["[(無題)]", "[a.txt]*"]);
    // A format with the folder: `$e` is the folder of the file.
    cx.update(|app| app.set_global(TabCaption("$f in $e".to_owned())));
    let folder = format!("{}{}", dir.display(), std::path::MAIN_SEPARATOR);
    assert_eq!(captions(cx, handle)[1], format!("a.txt in {folder}"));
    // The list shows the same captions.
    run(cx, handle, "F_WINLIST");
    assert_eq!(shape(cx, handle)[3], format!("*a.txt in {folder}"));
    // The workspace draws with the caption it is given.
    assert_eq!(
        workspace(cx, handle).read_with(cx, |ws, _| ws.tab_count()),
        2
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Outside Windows a backslash is a character of the file name, not a separator.
#[cfg(not(windows))]
#[gpui::test]
fn a_backslash_in_a_file_name_stays_in_the_caption_and_the_list(cx: &mut TestAppContext) {
    let (handle, dir) = window_with_files(cx, "caption-backslash", &["a\\b.txt"]);
    assert_eq!(captions(cx, handle), ["(無題)", "a\\b.txt"]);
    run(cx, handle, "F_WINLIST");
    assert_eq!(shape(cx, handle)[2..], ["(無題)", "*a\\b.txt"]);
    std::fs::remove_dir_all(&dir).ok();
}
