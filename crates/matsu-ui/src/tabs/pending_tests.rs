//! Tests of what looks at documents across tabs: files that are still being read, the DIFF
//! targets, and switching the tab bar while a file loads.

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_core::buffer::Document;
use matsu_core::model::DocumentModel;

use super::tests::{answer, run, tab_count, tabs, type_text, visual, windows, workspace};
use crate::diff::{DocRef, Target, candidates};
use crate::document::SharedDocument;
use crate::file_ops::windows::{begin_pending, editors, find_open, is_pending};
use crate::view::EditorView;

/// Adds a tab with `text` and the name `path` (the file does not need to exist).
fn add_tab(cx: &mut TestAppContext, handle: AnyWindowHandle, text: &str, path: &str) {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let shared = SharedDocument::new(
        DocumentModel::new(doc),
        Some(std::path::PathBuf::from(path)),
    );
    cx.update(|app| super::flows::open_document(app, None, handle, false, shared, None))
        .unwrap();
    cx.run_until_parked();
}

fn tab(cx: &mut TestAppContext, handle: AnyWindowHandle, index: usize) -> Entity<EditorView> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.tab_editor(index).unwrap())
}

fn doc_ref(cx: &mut TestAppContext, handle: AnyWindowHandle, view: &Entity<EditorView>) -> DocRef {
    DocRef {
        handle,
        document: view.read_with(cx, |v, _| v.document().entity_id()),
    }
}

fn mark_count(cx: &mut TestAppContext, view: &Entity<EditorView>) -> usize {
    view.read_with(cx, |v, cx| {
        v.document().read(cx).model().diff_marks().iter().count()
    })
}

fn diff(cx: &mut TestAppContext, handle: AnyWindowHandle, from: &Entity<EditorView>, to: DocRef) {
    let from = from.clone();
    handle
        .update(cx, |_, window, app| {
            from.update(app, |view, cx| {
                view.run_diff(Target::Window(to), 0, window, cx)
            })
        })
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn diff_targets_name_a_tab_not_a_window(cx: &mut TestAppContext) {
    let handle = super::tests::window(cx, true);
    add_tab(cx, handle, "a\nb", "/work/one.txt");
    add_tab(cx, handle, "a\nc", "/work/two.txt");
    add_tab(cx, handle, "a\nb", "/work/three.txt");
    let (one, two, three) = (tab(cx, handle, 1), tab(cx, handle, 2), tab(cx, handle, 3));
    // the other tabs of the same window can be chosen
    let owner = crate::diff::Owner {
        view: one.entity_id(),
        document: one.read_with(cx, |v, _| v.document().entity_id()),
    };
    let others = cx.update(|app| candidates(app, owner));
    assert_eq!(others.len(), 3);
    assert!(others.iter().all(|c| c.doc.handle == handle));
    let target = doc_ref(cx, handle, &two);
    diff(cx, handle, &one, target);
    assert_eq!(mark_count(cx, &one), 1, "line 2 differs from two.txt");
    let target = doc_ref(cx, handle, &three);
    diff(cx, handle, &one, target);
    assert_eq!(mark_count(cx, &one), 0, "three.txt has the same text");
}

#[gpui::test]
fn a_file_that_a_tab_is_reading_is_found_in_that_tab(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    let reading = tab(cx, handle, 1);
    let path = std::path::PathBuf::from("/work/slow.txt");
    cx.update(|app| begin_pending(app, handle, &reading.downgrade(), path.clone()));
    let (found, view) = cx.update(|app| find_open(app, &path)).unwrap();
    assert_eq!(found, handle);
    assert_eq!(view, reading, "not the first tab of the window");
    assert!(cx.update(|app| is_pending(app, &reading.downgrade())));
    let first = cx.update(|app| tab_view(app, handle, 0).downgrade());
    assert!(!cx.update(|app| is_pending(app, &first)));
}

fn tab_view(app: &mut gpui::App, handle: AnyWindowHandle, index: usize) -> Entity<EditorView> {
    editors(app)
        .into_iter()
        .filter(|(h, _)| *h == handle)
        .nth(index)
        .unwrap()
        .1
}

#[gpui::test]
fn the_bar_is_not_switched_while_a_file_is_being_read(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    let reading = tab(cx, handle, 0);
    cx.update(|app| begin_pending(app, handle, &reading.downgrade(), "/work/slow.txt".into()));
    run(cx, handle, "F_SHOWTAB");
    assert_eq!(windows(cx), 1, "the tabs stay together");
    assert_eq!(tab_count(cx, handle), 2);
    assert!(cx.update(|app| super::settings::current(app).show_bar));
    let notice = super::tests::active_view(cx, handle).read_with(cx, |v, _| v.notice.clone());
    assert!(notice.is_some_and(|n| n.contains("読み込み中")));
}

#[gpui::test]
fn a_file_being_read_is_pending_for_every_pane_of_the_document(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    run(cx, handle, "F_SPLIT_V");
    let panes = workspace(cx, handle).read_with(cx, |ws, _| ws.panes().shown());
    assert_eq!(panes.len(), 2);
    cx.update(|app| begin_pending(app, handle, &panes[0].downgrade(), "/work/slow.txt".into()));
    assert!(cx.update(|app| is_pending(app, &panes[1].downgrade())));
    let other_tab = tab(cx, handle, 0);
    assert!(!cx.update(|app| is_pending(app, &other_tab.downgrade())));
}

const DONT_SAVE: &str = "保存しない(N)";
const CANCEL: &str = "キャンセル";

#[gpui::test]
fn the_close_button_of_the_window_asks_about_every_tab(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    type_text(cx, handle, "x");
    workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(0, window, cx)
    });
    cx.run_until_parked();
    type_text(cx, handle, "y");
    assert!(!visual(cx, handle).simulate_close());
    cx.run_until_parked();
    answer(cx, handle, CANCEL);
    assert_eq!(windows(cx), 1);
    // after a cancel it can be asked again; every tab with changes has its question
    assert!(!visual(cx, handle).simulate_close());
    cx.run_until_parked();
    answer(cx, handle, DONT_SAVE);
    assert_eq!(windows(cx), 1, "the other tab is asked about too");
    answer(cx, handle, DONT_SAVE);
    assert_eq!(windows(cx), 0);
}

#[gpui::test]
fn a_tab_that_arrives_during_the_question_is_asked_about_too(cx: &mut TestAppContext) {
    let first = super::tests::window(cx, true);
    type_text(cx, first, "a");
    run(cx, first, "F_FILENEW_NEWWINDOW");
    let second = cx.windows()[1];
    type_text(cx, second, "b");
    assert!(!visual(cx, first).simulate_close());
    cx.run_until_parked();
    // while the question about the first window is open, the second window's tab moves into it
    run(cx, second, "F_TAB_JOINTNEXT");
    assert_eq!(tab_count(cx, first), 2);
    answer(cx, first, DONT_SAVE);
    assert_eq!(windows(cx), 1, "the tab that came in is asked about");
    answer(cx, first, DONT_SAVE);
    assert_eq!(windows(cx), 0);
}

#[gpui::test]
fn the_window_list_starts_on_the_active_tab(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(1, window, cx)
    });
    cx.run_until_parked();
    run(cx, handle, "F_DLGWINLIST");
    let list = super::tests::active_view(cx, handle)
        .read_with(cx, |v, _| v.window_list.clone())
        .expect("the list is open");
    assert_eq!(list.read_with(cx, |l, _| l.selected()), 1);
}

#[gpui::test]
fn the_tab_settings_are_written_to_the_settings_file(cx: &mut TestAppContext) {
    use crate::history::{HistoryOptions, HistoryState};
    let dir = std::env::temp_dir().join(format!("matsu-ui-tabs-ini-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("matsu.ini");
    std::fs::write(&path, "[Common]\r\nm_bRestoreCurPosition=1\r\n").unwrap();
    cx.update(|app| {
        let store = matsu_config::history::HistoryStore::new(&path, false);
        app.set_global(HistoryState::on_disk(store, HistoryOptions::default()));
        super::settings::update(app, |s| {
            s.show_bar = true;
            s.grouped = false;
        });
    });
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("bDispTabWnd=1"), "{text}");
    assert!(text.contains("bDispTabWndMultiWin=1"), "{text}");
    assert!(
        text.contains("m_bRestoreCurPosition=1"),
        "the rest of the file stays"
    );
    std::fs::remove_dir_all(&dir).ok();
}
