//! Tests of the tabs with real files: opening files as tabs, a file that is open already, and the
//! places that name a tab.

use gpui::{AnyWindowHandle, TestAppContext};

use super::tests::{active_tab, active_view, captions, run, tab_count, window, windows};
use crate::document::SharedDocument;
use crate::file_ops::windows::{LoadRequest, find_tab_of};

pub(super) fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ume-ui-tabs-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) fn open_file(cx: &mut TestAppContext, handle: AnyWindowHandle, path: &std::path::Path) {
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
    assert_eq!(captions(cx, handle), ["(無題)1", "a.txt", "b.txt"]);
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

// ---- documents from outside (a later launch, the OS): `open_from_outside` ----

fn from_outside(cx: &mut TestAppContext, paths: Vec<std::path::PathBuf>) {
    cx.update(|app| crate::file_ops::windows::open_from_outside(app, paths))
        .unwrap();
    cx.run_until_parked();
}

/// With the tab bar shown, a file from outside is a tab of the active window, and the tab of a
/// file that is open already comes to the front (`CControlTray::OpenNewEditor`,
/// `ActiveAlreadyOpenedWindow`).
#[gpui::test]
fn a_file_from_outside_is_a_tab_of_the_active_window(cx: &mut TestAppContext) {
    let dir = temp_dir("outside-tab");
    let (a, b) = (dir.join("a.txt"), dir.join("b.txt"));
    std::fs::write(&a, "alpha").unwrap();
    std::fs::write(&b, "beta").unwrap();
    let handle = window(cx, true);
    from_outside(cx, vec![a.clone(), b.clone()]);
    assert_eq!(windows(cx), 1, "tabs, not windows");
    assert_eq!(captions(cx, handle), ["(無題)1", "a.txt", "b.txt"]);
    // The first file again: its tab, no copy.
    from_outside(cx, vec![a]);
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, handle), 3);
    assert_eq!(active_tab(cx, handle), 1);
    // No file: a new empty document, a tab.
    from_outside(cx, Vec::new());
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, handle), 4);
    assert_eq!(
        active_tab(cx, handle),
        3,
        "the new document is the active tab"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Without the tab bar every document from outside gets a window of its own.
#[gpui::test]
fn a_file_from_outside_is_a_window_without_tabs(cx: &mut TestAppContext) {
    let dir = temp_dir("outside-window");
    let (a, b) = (dir.join("a.txt"), dir.join("b.txt"));
    std::fs::write(&a, "alpha").unwrap();
    std::fs::write(&b, "beta").unwrap();
    let _handle = window(cx, false);
    from_outside(cx, vec![a.clone(), b]);
    assert_eq!(windows(cx), 3);
    from_outside(cx, vec![a]);
    assert_eq!(windows(cx), 3, "the open file is shown, not opened again");
    from_outside(cx, Vec::new());
    assert_eq!(windows(cx), 4);
    std::fs::remove_dir_all(&dir).ok();
}

/// The command line of the first launch: no window yet, the first file opens the window and the
/// others are its tabs.
#[gpui::test]
fn the_command_line_of_the_first_launch_makes_one_window_of_tabs(cx: &mut TestAppContext) {
    let dir = temp_dir("outside-first");
    let (a, b) = (dir.join("a.txt"), dir.join("b.txt"));
    std::fs::write(&a, "alpha").unwrap();
    std::fs::write(&b, "beta").unwrap();
    cx.update(gpui_component::init);
    super::tests::tab_mode(cx, true, true);
    from_outside(cx, vec![a, b]);
    assert_eq!(windows(cx), 1);
    let handle = cx.windows()[0];
    assert_eq!(captions(cx, handle), ["a.txt", "b.txt"]);
    std::fs::remove_dir_all(&dir).ok();
}

fn tooltips(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<String> {
    super::tests::workspace(cx, handle).read_with(cx, |ws, app| {
        ws.tab_editors()
            .iter()
            .map(|v| crate::app::Workspace::tooltip_of(v.read(app), app))
            .collect()
    })
}

#[gpui::test]
fn the_tooltip_of_a_tab_is_the_path_with_the_replacements_and_the_text_is_the_name(
    cx: &mut TestAppContext,
) {
    use crate::file_names::{FileNames, NoFolders};

    let dir = temp_dir("tooltip");
    let a = dir.join("a.txt");
    std::fs::write(&a, "alpha").unwrap();
    let handle = window(cx, true);
    open_file(cx, handle, &a);
    // Without the setting the tooltip is the whole path, and that of the untitled document is its
    // caption (`CTabWnd::GetTabName`).
    assert_eq!(
        tooltips(cx, handle),
        ["(無題)1".to_owned(), a.to_string_lossy().into_owned()]
    );
    // The list of 「ファイル名表示」: the folder of the file becomes a short name.
    let from = format!("{}{}", dir.display(), std::path::MAIN_SEPARATOR);
    let rows = [vec![from, "作業:".to_owned()]];
    let names = FileNames::new(true, 100, &rows, &NoFolders, false);
    cx.update(|app| app.set_global(names));
    assert_eq!(tooltips(cx, handle), ["(無題)1", "作業:a.txt"]);
    // The text of the tab is the name of the file, as before.
    assert_eq!(captions(cx, handle), ["(無題)1", "a.txt"]);
    // A long path is shortened by the setting, but not cut to the width (`bFitMode` is false).
    let narrow = FileNames::new(true, 3, &[], &NoFolders, false);
    cx.update(|app| app.set_global(narrow));
    let long = tooltips(cx, handle).remove(1);
    assert!(long.ends_with("a.txt"), "{long}");
    assert!(long.contains("..."), "{long}");
    std::fs::remove_dir_all(&dir).ok();
}
