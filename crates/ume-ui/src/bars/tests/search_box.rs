//! Tests of the search box of the toolbar and the drop-down arrow of the open button
//! (`sakura_core/window/CMainToolBar.cpp`, `CEditWnd::CreateFileDropDownMenu`).

use gpui::{AnyWindowHandle, Entity, Modifiers, TestAppContext, VisualTestContext};
use ume_commands::keymap::KeyMap;
use ume_config::history::{FileEntry, History};
use ume_core::buffer::Pos;

use super::{bounds, center_of, click, run, setup, view, workspace};
use crate::bars::open_menu;
use crate::bars::search_box::SearchBox;
use crate::bars::tool_buttons::FUNCTIONS_OF_BUTTONS;
use crate::history::menu::RecentLists;
use crate::menu::{Entry, MenuAction};
use crate::view::EditorView;

/// The toolbar: the open button (with its arrow) and the search box.
fn toolbar() -> String {
    let number = FUNCTIONS_OF_BUTTONS
        .iter()
        .position(|&c| c == 30908)
        .unwrap()
        + 1;
    format!("[Toolbar]\r\nnToolBarButtonNum=2\r\nnTBB[000]=25\r\nnTBB[001]={number}\r\n")
}

fn search_box(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<SearchBox> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.search_box.clone())
}

fn box_text(cx: &mut TestAppContext, handle: AnyWindowHandle) -> String {
    let search_box = search_box(cx, handle);
    search_box.read_with(cx, |b, app| b.text(app))
}

fn search_keys(cx: &mut TestAppContext) -> Vec<String> {
    cx.update(|app| crate::search::history::search_keys(app))
}

/// Writes `text` in the document and puts the caret at its start.
fn write(cx: &mut TestAppContext, handle: AnyWindowHandle, text: &str) {
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.simulate_input(text);
    cx.simulate_keystrokes("ctrl-home");
}

fn selection(cx: &mut TestAppContext, view: &Entity<EditorView>) -> Option<(Pos, Pos)> {
    view.read_with(cx, |v, _| v.cursor.selection())
}

fn pattern(cx: &mut TestAppContext, view: &Entity<EditorView>) -> String {
    view.read_with(cx, |v, _| v.search.pattern().to_owned())
}

fn editor_has_focus(cx: &mut VisualTestContext, view: &Entity<EditorView>) -> bool {
    let handle = view.read_with(cx, |v, _| v.focus_handle.clone());
    cx.update(|window, _| handle.is_focused(window))
}

fn box_has_focus(cx: &mut VisualTestContext, search_box: &Entity<SearchBox>) -> bool {
    cx.update(|window, app| search_box.read(app).is_focused(window, app))
}

/// Types `text` in the box of the window and presses Enter.
fn search_in_box(cx: &mut TestAppContext, handle: AnyWindowHandle, text: &str) {
    let search_box = search_box(cx, handle);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.update(|window, app| search_box.update(app, |b, cx| b.focus(window, cx)));
    if !text.is_empty() {
        cx.simulate_keystrokes("secondary-a");
        cx.simulate_input(text);
    }
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
}

#[gpui::test]
fn enter_adds_the_text_to_the_history_and_searches_forward(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    let editor = view(cx, handle);
    write(cx, handle, "foo bar foo\nfoo");
    search_in_box(cx, handle, "foo");
    // The first match from the caret; the text has the focus again.
    assert_eq!(
        selection(cx, &editor),
        Some((Pos::new(0, 0), Pos::new(0, 3)))
    );
    assert_eq!(search_keys(cx), ["foo"]);
    assert_eq!(pattern(cx, &editor), "foo");
    {
        let cx = &mut VisualTestContext::from_window(handle, cx);
        assert!(editor_has_focus(cx, &editor));
    }
    // Enter again with the same text: the next match, and the string is in the history once.
    search_in_box(cx, handle, "");
    assert_eq!(
        selection(cx, &editor),
        Some((Pos::new(0, 8), Pos::new(0, 11)))
    );
    assert_eq!(search_keys(cx), ["foo"]);
}

#[gpui::test]
fn the_search_uses_the_options_of_the_find_panel(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    let editor = view(cx, handle);
    write(cx, handle, "Foo foo");
    editor.update(cx, |v, _| {
        let mut options = v.search.options();
        options.match_case = true;
        v.search.set_options(options);
    });
    search_in_box(cx, handle, "foo");
    assert_eq!(
        selection(cx, &editor),
        Some((Pos::new(0, 4), Pos::new(0, 7)))
    );
}

#[gpui::test]
fn japanese_text_is_a_search_key(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    let editor = view(cx, handle);
    write(cx, handle, "あいう\nかきくけこ");
    search_in_box(cx, handle, "きく");
    assert_eq!(search_keys(cx), ["きく"]);
    assert_eq!(
        selection(cx, &editor),
        Some((Pos::new(1, 3), Pos::new(1, 9)))
    );
}

#[gpui::test]
fn a_string_of_max_path_or_more_is_searched_but_not_remembered(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    let editor = view(cx, handle);
    let long = "a".repeat(260);
    write(cx, handle, &format!("x{long}"));
    search_in_box(cx, handle, &long);
    assert!(search_keys(cx).is_empty());
    assert_eq!(selection(cx, &editor).map(|(a, _)| a), Some(Pos::new(0, 1)));
}

#[gpui::test]
fn enter_with_an_empty_box_does_nothing(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    let editor = view(cx, handle);
    write(cx, handle, "foo");
    search_in_box(cx, handle, "");
    assert_eq!(selection(cx, &editor), None);
    assert!(search_keys(cx).is_empty());
    let search_box = search_box(cx, handle);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    assert!(box_has_focus(cx, &search_box), "the box keeps the focus");
}

#[gpui::test]
fn f_search_box_focuses_the_box_only_where_the_toolbar_has_one(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    let search_box = search_box(cx, handle);
    {
        let cx = &mut VisualTestContext::from_window(handle, cx);
        assert!(!box_has_focus(cx, &search_box));
    }
    run(cx, handle, "F_SEARCH_BOX");
    {
        let cx = &mut VisualTestContext::from_window(handle, cx);
        assert!(box_has_focus(cx, &search_box));
    }
    // A toolbar without the box: nothing happens.
    let (_ini, handle) = setup(cx, "");
    let editor = view(cx, handle);
    run(cx, handle, "F_SEARCH_BOX");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    assert!(editor_has_focus(cx, &editor));
}

#[gpui::test]
fn tab_and_escape_give_the_focus_back_and_the_text_becomes_the_key(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    let editor = view(cx, handle);
    let search_box = search_box(cx, handle);
    for (n, key) in ["tab", "escape"].into_iter().enumerate() {
        let text = format!("word{n}");
        run(cx, handle, "F_SEARCH_BOX");
        let cx2 = &mut VisualTestContext::from_window(handle, cx);
        assert!(box_has_focus(cx2, &search_box));
        cx2.simulate_keystrokes("secondary-a");
        cx2.simulate_input(&text);
        cx2.simulate_keystrokes(key);
        cx2.run_until_parked();
        assert!(editor_has_focus(cx2, &editor), "{key}");
        assert!(!box_has_focus(cx2, &search_box), "{key}");
        // `CBN_KILLFOCUS`: no search, but the text is the key of the view and in the history.
        assert_eq!(selection(cx, &editor), None);
        assert_eq!(pattern(cx, &editor), text);
        assert_eq!(search_keys(cx)[0], text);
    }
    assert_eq!(search_keys(cx), ["word1", "word0"]);
}

#[gpui::test]
fn the_box_follows_the_history_when_another_window_searches(cx: &mut TestAppContext) {
    let (_ini, first) = setup(cx, &toolbar());
    let second = cx
        .update(|app| {
            crate::file_ops::windows::open_editor_window(
                app,
                crate::document::SharedDocument::empty(),
                None,
            )
        })
        .unwrap();
    cx.run_until_parked();
    write(cx, second, "needle hay");
    search_in_box(cx, second, "needle");
    assert_eq!(search_keys(cx), ["needle"]);
    // The first window has not searched: its box shows the newest string.
    assert_eq!(box_text(cx, first), "needle");
    // A later string of the first window is on top, and the second window follows.
    search_in_box(cx, first, "hay");
    assert_eq!(search_keys(cx), ["hay", "needle"]);
    assert_eq!(box_text(cx, second), "hay");
}

#[gpui::test]
fn the_arrow_opens_the_list_and_a_click_picks_a_string(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    cx.update(|app| {
        crate::search::history::remember_search(app, "one");
        crate::search::history::remember_search(app, "ふたつ");
    });
    cx.run_until_parked();
    let search_box = search_box(cx, handle);
    // The box starts with the newest string.
    assert_eq!(box_text(cx, handle), "ふたつ");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    click(cx, "search-box-toggle".into(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(search_box.read_with(cx, |b, _| b.open_entry()), Some(0));
    let first = bounds(cx, "search-box-entry-0".into()).expect("the list is drawn");
    let second = bounds(cx, "search-box-entry-1".into()).expect("two entries");
    assert!(second.top() > first.top());
    click(cx, "search-box-entry-1".into(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(search_box.read_with(cx, |b, app| b.text(app)), "one");
    assert_eq!(search_box.read_with(cx, |b, _| b.open_entry()), None);
    assert!(box_has_focus(cx, &search_box));
}

#[gpui::test]
fn up_and_down_go_through_the_history_and_escape_closes_the_list(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    cx.update(|app| {
        for text in ["a", "b", "c"] {
            crate::search::history::remember_search(app, text);
        }
    });
    run(cx, handle, "F_SEARCH_BOX");
    let search_box = search_box(cx, handle);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    // The box shows "c", the newest.
    cx.simulate_keystrokes("down");
    assert_eq!(search_box.read_with(cx, |b, app| b.text(app)), "b");
    cx.simulate_keystrokes("up up");
    assert_eq!(search_box.read_with(cx, |b, app| b.text(app)), "c");
    // Alt+Down opens the list; Down moves in it; Enter takes the entry without searching.
    cx.simulate_keystrokes("alt-down down enter");
    assert_eq!(search_box.read_with(cx, |b, app| b.text(app)), "b");
    assert_eq!(search_box.read_with(cx, |b, _| b.open_entry()), None);
    cx.simulate_keystrokes("f4");
    cx.simulate_keystrokes("escape");
    assert_eq!(search_box.read_with(cx, |b, _| b.open_entry()), None);
    assert!(
        box_has_focus(cx, &search_box),
        "Escape closed the list only"
    );
}

// ---- the drop-down arrow of the open button ----

fn history_with(files: &[&str], folders: &[&str]) -> History {
    let mut history = History::new(20, 20, false);
    for folder in folders {
        history.add_folder(folder);
    }
    for file in files {
        history.add_file(FileEntry::new((*file).to_owned()));
    }
    history
}

/// The rows as short text: the label, `-` for a separator, `>` before a submenu's rows.
fn shape(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .flat_map(|entry| match entry {
            Entry::Separator => vec!["-".to_owned()],
            Entry::Item(item) if !item.enabled => vec![format!("({})", item.label)],
            Entry::Item(item) => vec![item.label.clone()],
            Entry::Submenu { label, entries } => {
                let mut rows = vec![format!("{label} >")];
                rows.extend(shape(entries));
                rows
            }
        })
        .collect()
}

#[test]
fn the_menu_lists_the_files_then_the_folders_then_the_functions() {
    let history = history_with(&["/a/one.txt", "/b/メモ.txt"], &[]);
    let lists = RecentLists::new(Some(history));
    let entries = open_menu::entries(&lists, &KeyMap::default(), &|_: u16| true);
    assert_eq!(
        shape(&entries),
        [
            "1 /b/メモ.txt",
            "2 /a/one.txt",
            "-",
            "最近使ったフォルダー >",
            "1 /b/",
            "2 /a/",
            "-",
            "履歴の管理(M)...",
            "-",
            "新規作成(N)",
            "新規ウインドウを開く(M)",
            "開く(O)...",
        ]
    );
    // Choosing a file opens it; choosing a folder shows the open dialog in it.
    let Entry::Item(file) = &entries[0] else {
        panic!()
    };
    assert_eq!(file.action, MenuAction::RecentFile("/b/メモ.txt".into()));
    let Entry::Submenu {
        entries: folders, ..
    } = &entries[3]
    else {
        panic!()
    };
    let Entry::Item(folder) = &folders[0] else {
        panic!()
    };
    assert_eq!(folder.action, MenuAction::RecentFolder("/b/".into()));
}

#[test]
fn without_a_history_the_folders_are_grayed_and_there_is_no_separator_above() {
    let lists = RecentLists::new(Some(History::new(20, 20, false)));
    let entries = open_menu::entries(&lists, &KeyMap::default(), &|_: u16| true);
    assert_eq!(
        shape(&entries),
        [
            "(最近使ったフォルダー)",
            "-",
            "履歴の管理(M)...",
            "-",
            "新規作成(N)",
            "新規ウインドウを開く(M)",
            "開く(O)...",
        ]
    );
}

#[gpui::test]
fn the_arrow_shows_the_menu_under_the_button(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    cx.update(|app| {
        crate::history::add_file(app, FileEntry::new("/tmp/ume-dropdown/a.txt".to_owned()));
    });
    let editor = view(cx, handle);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let button = bounds(cx, "tool-button-25".into()).expect("the open button");
    let arrow = bounds(cx, "tool-dropdown-25".into()).expect("its arrow");
    assert!(arrow.left() >= button.right() - gpui::px(1.));
    assert!(!editor.read_with(cx, |v, _| v.custom_menu.is_open()));
    click(cx, "tool-dropdown-25".into(), Modifiers::none());
    cx.run_until_parked();
    // The corner of the menu is the left bottom of the whole button (`rc.left`, `rc.bottom`).
    assert!(editor.read_with(cx, |v, _| v.custom_menu.is_open()));
    let at = editor
        .read_with(cx, |v, _| v.custom_menu.open_at())
        .expect("a place");
    assert_eq!(at.x.round(), button.left().round());
    assert_eq!(at.y.round(), button.bottom().round());
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!editor.read_with(cx, |v, _| v.custom_menu.is_open()));
}

#[gpui::test]
fn the_button_itself_still_asks_for_a_file_to_open(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &toolbar());
    let editor = view(cx, handle);
    crate::file_dialog::tests::asked();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let at = center_of(cx, "tool-button-25".into());
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(crate::file_dialog::tests::asked().len(), 1);
    assert!(!editor.read_with(cx, |v, _| v.custom_menu.is_open()));
}
