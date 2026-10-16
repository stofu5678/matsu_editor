//! Tests of what looks at documents across tabs: files that are still being read, the DIFF
//! targets, and switching the tab bar while a file loads.

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use ume_core::buffer::Document;
use ume_core::model::DocumentModel;

use super::tests::{run, tab_count, tabs, windows, workspace};
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

fn doc_ref(handle: AnyWindowHandle, view: &Entity<EditorView>) -> DocRef {
    DocRef {
        handle,
        view: view.entity_id(),
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
    let others = cx.update(|app| candidates(app, one.entity_id()));
    assert_eq!(others.len(), 3);
    assert!(others.iter().all(|c| c.doc.handle == handle));
    diff(cx, handle, &one, doc_ref(handle, &two));
    assert_eq!(mark_count(cx, &one), 1, "line 2 differs from two.txt");
    diff(cx, handle, &one, doc_ref(handle, &three));
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
