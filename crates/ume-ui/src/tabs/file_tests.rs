//! Tests of the tabs with real files: opening files as tabs, a file that is open already, and the
//! places that name a tab.

use gpui::{AnyWindowHandle, TestAppContext};

use super::tests::{active_tab, active_view, captions, run, tab_count, window, windows};
use crate::document::SharedDocument;
use crate::file_ops::windows::{LoadRequest, find_tab_of};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ume-ui-tabs-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn open_file(cx: &mut TestAppContext, handle: AnyWindowHandle, path: &std::path::Path) {
    let request = LoadRequest {
        path: path.to_owned(),
        encoding: None,
    };
    cx.update(|app| {
        super::flows::open_document(
            app,
            None,
            handle,
            false,
            SharedDocument::empty(),
            Some(request),
        )
    })
    .unwrap();
    cx.run_until_parked();
}

fn first_line(cx: &mut TestAppContext, handle: AnyWindowHandle) -> String {
    active_view(cx, handle).read_with(cx, |v, cx| {
        v.document().read(cx).doc().line(0).unwrap().unwrap().text
    })
}

#[gpui::test]
fn opened_files_are_tabs_with_their_names(cx: &mut TestAppContext) {
    let dir = temp_dir("files");
    let (a, b) = (dir.join("a.txt"), dir.join("b.txt"));
    std::fs::write(&a, "alpha").unwrap();
    std::fs::write(&b, "beta").unwrap();
    let handle = window(cx, true);
    open_file(cx, handle, &a);
    open_file(cx, handle, &b);
    assert_eq!(windows(cx), 1);
    assert_eq!(captions(cx, handle), ["(無題)", "a.txt", "b.txt"]);
    assert_eq!(active_tab(cx, handle), 2);
    assert_eq!(first_line(cx, handle), "beta");
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn a_file_that_is_open_in_a_tab_is_shown_not_opened_again(cx: &mut TestAppContext) {
    let dir = temp_dir("again");
    let (a, b) = (dir.join("a.txt"), dir.join("b.txt"));
    std::fs::write(&a, "alpha").unwrap();
    std::fs::write(&b, "beta").unwrap();
    let handle = window(cx, true);
    open_file(cx, handle, &a);
    open_file(cx, handle, &b);
    // open a.txt again from the active tab (b.txt)
    let view = active_view(cx, handle);
    let again = a.clone();
    handle
        .update(cx, |_, window, app| {
            view.update(app, |_, cx| {
                cx.spawn_in(window, async move |this, cx| {
                    crate::file_ops::load::open_paths(&this, cx, vec![again], None).await;
                })
                .detach();
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(tab_count(cx, handle), 3, "no second copy");
    assert_eq!(
        active_tab(cx, handle),
        1,
        "the tab of the file came to the front"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn the_tag_jump_stack_names_the_document_not_the_window(cx: &mut TestAppContext) {
    let dir = temp_dir("tag");
    let a = dir.join("a.txt");
    std::fs::write(&a, "x\ny\nz").unwrap();
    let handle = window(cx, true);
    open_file(cx, handle, &a);
    let weak = active_view(cx, handle).downgrade();
    let (found, _) = cx.update(|app| find_tab_of(app, &weak)).unwrap();
    assert_eq!(found, handle);
    // a new tab, separated, leaves the tab of the file where it is
    run(cx, handle, "F_FILENEW");
    run(cx, handle, "F_TAB_SEPARATE");
    let (found, _) = cx.update(|app| find_tab_of(app, &weak)).unwrap();
    assert_eq!(found, handle);
    // and moving the file's own tab to another window is followed
    let other = cx.windows()[1];
    run(cx, handle, "F_TAB_JOINTNEXT");
    let (found, _) = cx.update(|app| find_tab_of(app, &weak)).unwrap();
    assert_eq!(found, other);
    std::fs::remove_dir_all(&dir).ok();
}
