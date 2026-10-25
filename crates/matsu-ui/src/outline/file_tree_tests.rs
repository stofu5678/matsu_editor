//! Tests of the file tree in the outline pane, in GPUI's headless test windows: `F_FILETREE`, the
//! folders listed when they are opened, opening a file, and the project file.

use std::path::{Path, PathBuf};

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::Profile;
use matsu_core::buffer::Document;
use matsu_core::model::DocumentModel;

use super::pane::{OutlinePane, Row};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::file_ops::windows::{open_editor_window, workspace_of};
use crate::settings::UserSettings;
use crate::tabs::settings::TabSettings;
use crate::view::EditorView;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// A folder with files; removed when it goes out of scope.
fn folder(files: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for file in files {
        let path = dir.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "content of the file\n").unwrap();
    }
    dir
}

/// A window whose document has the file `path` (or none), with the settings `ini`.
fn window(cx: &mut TestAppContext, path: Option<&Path>, ini: &str) -> AnyWindowHandle {
    cx.update(gpui_component::init);
    cx.update(|cx| {
        cx.set_global(UserSettings::from_profile(
            Profile::from_bytes(ini.as_bytes()),
            None,
        ));
        cx.set_global(TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        })
    });
    let doc = Document::from_utf8(b"text\n".to_vec()).unwrap();
    let shared = SharedDocument::new(DocumentModel::new(doc), path.map(Path::to_path_buf));
    let handle = cx
        .update(|cx| open_editor_window(cx, shared, None))
        .unwrap();
    cx.run_until_parked();
    handle
}

fn workspace(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<Workspace> {
    cx.update(|app| workspace_of(app, handle)).unwrap()
}

fn active_view(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.editor().clone())
}

fn run_with(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str, args: &[Arg]) {
    let code = code_of(function);
    let view = active_view(cx, handle);
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, args, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn pane(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<Entity<OutlinePane>> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.outline.clone())
}

fn rows(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<Row> {
    pane(cx, handle)
        .expect("the pane is open")
        .read_with(cx, |p, _| p.rows().to_vec())
}

fn labels(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<String> {
    rows(cx, handle).into_iter().map(|r| r.label).collect()
}

fn toggle(cx: &mut TestAppContext, handle: AnyWindowHandle, row: usize) {
    let pane = pane(cx, handle).unwrap();
    pane.update(cx, |p, cx| p.toggle_node(row, cx));
    cx.run_until_parked();
}

fn open_row(cx: &mut TestAppContext, handle: AnyWindowHandle, row: usize, focus_text: bool) {
    let pane = pane(cx, handle).unwrap();
    handle
        .update(cx, |_, window, app| {
            pane.update(app, |p, cx| p.jump(row, focus_text, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

/// `[Common]` with one Grep item for `dir`, labelled `Project`.
fn one_grep(dir: &Path, patterns: &str) -> String {
    format!(
        "[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=0\r\n\
         FileTree(0).szTargetPath={}\r\nFileTree(0).szLabelName=Project\r\nFileTree(0).nDepth=0\r\n\
         FileTree(0).szTargetFile={patterns}\r\nFileTree(0).bIgnoreHidden=1\r\n",
        dir.display()
    )
}

#[gpui::test]
fn the_command_shows_the_tree_and_a_folder_is_listed_when_it_is_opened(cx: &mut TestAppContext) {
    let dir = folder(&[
        "b.rs",
        "a.txt",
        "sub/c.txt",
        "sub/deep/d.txt",
        "other/e.txt",
    ]);
    let handle = window(cx, None, &one_grep(dir.path(), "*.*"));
    assert!(pane(cx, handle).is_none());
    run_with(cx, handle, "F_FILETREE", &[]);
    let p = pane(cx, handle).expect("F_FILETREE opens the pane");
    p.read_with(cx, |p, _| {
        assert_eq!(p.title(), "ファイルツリー");
        assert!(!p.is_analyzing());
        assert_eq!(p.outline_type(), matsu_outline::OutlineKind::FileTree);
    });
    // the Grep item is there, closed; its folder has been listed at once, so it has a `+`
    let first = rows(cx, handle);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].label, "Project");
    assert!(first[0].has_children && !first[0].expanded);
    // open it: the subfolders first, then the files, each in the order of their names
    toggle(cx, handle, 0);
    assert_eq!(
        labels(cx, handle),
        ["Project", "other", "sub", "a.txt", "b.rs"]
    );
    let all = rows(cx, handle);
    assert_eq!(all[1].depth, 1);
    assert!(
        all[1].has_children,
        "a folder has a `+` before it is opened"
    );
    assert!(all[0].expanded);
    // a subfolder is listed when it is opened
    toggle(cx, handle, 2);
    assert_eq!(
        labels(cx, handle),
        ["Project", "other", "sub", "deep", "c.txt", "a.txt", "b.rs"]
    );
    // and closes again
    toggle(cx, handle, 2);
    assert_eq!(
        labels(cx, handle),
        ["Project", "other", "sub", "a.txt", "b.rs"]
    );
}

#[gpui::test]
fn the_keyboard_opens_and_closes_folders_and_opens_files(cx: &mut TestAppContext) {
    let dir = folder(&["a.txt", "sub/c.txt"]);
    let handle = window(cx, None, &one_grep(dir.path(), "*.*"));
    run_with(cx, handle, "F_FILETREE", &[]);
    // the pane has the focus after the command
    let keys = &mut gpui::VisualTestContext::from_window(handle, cx);
    keys.simulate_keystrokes("down right");
    keys.run_until_parked();
    assert_eq!(labels(cx, handle), ["Project", "sub", "a.txt"]);
    // Right opens a folder (it is listed), Left closes it
    keys.simulate_keystrokes("down right");
    keys.run_until_parked();
    assert_eq!(labels(cx, handle), ["Project", "sub", "c.txt", "a.txt"]);
    keys.simulate_keystrokes("left");
    keys.run_until_parked();
    assert_eq!(labels(cx, handle), ["Project", "sub", "a.txt"]);
    // Enter on a file opens it
    keys.simulate_keystrokes("down enter");
    keys.run_until_parked();
    let view = active_view(cx, handle);
    let opened = view.read_with(cx, |v, cx| {
        v.document().read(cx).path().map(Path::to_path_buf)
    });
    assert_eq!(opened, Some(dir.path().join("a.txt")));
}

#[gpui::test]
fn the_file_tree_has_nothing_to_copy_and_leaves_the_clipboard_alone(cx: &mut TestAppContext) {
    let dir = folder(&["a.txt"]);
    let handle = window(cx, None, &one_grep(dir.path(), "*.*"));
    run_with(cx, handle, "F_FILETREE", &[]);
    let p = pane(cx, handle).unwrap();
    // the button is gray
    assert!(!p.read_with(cx, |p, _| p.copy_enabled()));
    cx.update(|app| app.write_to_clipboard(gpui::ClipboardItem::new_string("keep".to_owned())));
    // the button, and F_COPY / F_CUT in the pane (Ctrl+C, Ctrl+X in the key map)
    p.update(cx, |p, cx| p.copy(cx));
    let keys = &mut gpui::VisualTestContext::from_window(handle, cx);
    keys.simulate_keystrokes("ctrl-c ctrl-x");
    keys.run_until_parked();
    let text = cx.update(|app| app.read_from_clipboard().and_then(|c| c.text()));
    assert_eq!(text.as_deref(), Some("keep"));
    // the outline of a text has its button
    run_with(cx, handle, "F_OUTLINE", &[]);
    assert!(p.read_with(cx, |p, _| p.copy_enabled()));
}

#[gpui::test]
fn opening_a_folder_twice_while_it_is_listed_keeps_it_open(cx: &mut TestAppContext) {
    let dir = folder(&["a.txt", "sub/c.txt"]);
    let handle = window(cx, None, &one_grep(dir.path(), "*.*"));
    run_with(cx, handle, "F_FILETREE", &[]);
    // Right twice before the listing of "sub" is there: it is open, not open and closed again
    toggle(cx, handle, 0);
    let p = pane(cx, handle).unwrap();
    p.update(cx, |p, cx| {
        p.set_node(1, true, cx);
        p.set_node(1, true, cx);
    });
    cx.run_until_parked();
    assert_eq!(labels(cx, handle), ["Project", "sub", "c.txt", "a.txt"]);
    // and Left closes it, also then
    p.update(cx, |p, cx| p.set_node(1, false, cx));
    assert_eq!(labels(cx, handle), ["Project", "sub", "a.txt"]);
}

#[gpui::test]
fn the_old_tree_is_gone_while_a_new_one_is_made(cx: &mut TestAppContext) {
    let dir = folder(&["a.txt", "sub/c.txt"]);
    let handle = window(cx, None, &one_grep(dir.path(), "*.*"));
    run_with(cx, handle, "F_FILETREE", &[]);
    toggle(cx, handle, 0);
    assert_eq!(labels(cx, handle), ["Project", "sub", "a.txt"]);
    // 更新: until the new tree is there, no row of the old one can be opened (a folder opened
    // now would be listed into the new tree)
    let p = pane(cx, handle).unwrap();
    p.update(cx, |p, cx| {
        p.reload(cx);
        assert!(p.rows().is_empty());
        p.toggle_node(0, cx);
    });
    cx.run_until_parked();
    assert_eq!(labels(cx, handle), ["Project"]);
    toggle(cx, handle, 0);
    toggle(cx, handle, 1);
    assert_eq!(labels(cx, handle), ["Project", "sub", "c.txt", "a.txt"]);
}

#[gpui::test]
fn the_patterns_of_the_item_choose_what_the_tree_shows(cx: &mut TestAppContext) {
    let dir = folder(&["a.rs", "b.txt", "sub/c.rs"]);
    let handle = window(cx, None, &one_grep(dir.path(), "*.rs"));
    run_with(cx, handle, "F_FILETREE", &[]);
    toggle(cx, handle, 0);
    assert_eq!(labels(cx, handle), ["Project", "sub", "a.rs"]);
}

#[gpui::test]
fn a_file_of_the_tree_is_opened_in_this_window_when_it_takes_a_file(cx: &mut TestAppContext) {
    let dir = folder(&["a.txt", "sub/b.txt"]);
    // an untitled, unmodified document takes the file (`IsAcceptLoad`)
    let handle = window(cx, None, &one_grep(dir.path(), "*.*"));
    run_with(cx, handle, "F_FILETREE", &[]);
    toggle(cx, handle, 0);
    // the rows: Project, sub, a.txt
    let all = rows(cx, handle);
    assert_eq!(
        all.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        ["Project", "sub", "a.txt"]
    );
    // a folder, a group and the item itself open nothing
    open_row(cx, handle, 1, true);
    open_row(cx, handle, 0, true);
    let view = active_view(cx, handle);
    assert_eq!(
        view.read_with(cx, |v, cx| v
            .document()
            .read(cx)
            .path()
            .map(Path::to_path_buf)),
        None
    );
    // the file opens
    assert_eq!(all[2].path, Some(dir.path().join("a.txt")));
    open_row(cx, handle, 2, true);
    let view = active_view(cx, handle);
    let opened = view.read_with(cx, |v, cx| {
        v.document().read(cx).path().map(Path::to_path_buf)
    });
    assert_eq!(opened, Some(dir.path().join("a.txt")));
    // the pane stays, still showing the tree
    assert_eq!(
        pane(cx, handle)
            .unwrap()
            .read_with(cx, |p, _| p.title().to_owned()),
        "ファイルツリー"
    );
}

#[gpui::test]
fn a_file_item_and_a_folder_item_make_the_tree_of_the_settings(cx: &mut TestAppContext) {
    let dir = folder(&["n.txt"]);
    let ini = format!(
        "[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=3\r\n\
         FileTree(0).eItemType=2\r\nFileTree(0).szLabelName=Notes\r\nFileTree(0).nDepth=0\r\n\
         FileTree(1).eItemType=1\r\nFileTree(1).szTargetPath={}\r\nFileTree(1).szLabelName=memo\r\nFileTree(1).nDepth=1\r\n\
         FileTree(2).eItemType=1\r\nFileTree(2).szTargetPath={}\r\nFileTree(2).nDepth=0\r\n",
        dir.path().join("n.txt").display(),
        dir.path().join("n.txt").display(),
    );
    let handle = window(cx, None, &ini);
    run_with(cx, handle, "F_FILETREE", &[]);
    let n = dir.path().join("n.txt");
    assert_eq!(
        labels(cx, handle),
        ["Notes".to_owned(), n.to_string_lossy().into_owned()]
    );
    toggle(cx, handle, 0);
    assert_eq!(
        labels(cx, handle),
        [
            "Notes".to_owned(),
            "memo".to_owned(),
            n.to_string_lossy().into_owned()
        ]
    );
    // the File items open their file; the Folder item does not
    let all = rows(cx, handle);
    assert_eq!(all[0].path, None);
    assert_eq!(all[1].path.as_deref(), Some(n.as_path()));
    open_row(cx, handle, 1, false);
    let view = active_view(cx, handle);
    assert_eq!(
        view.read_with(cx, |v, cx| v
            .document()
            .read(cx)
            .path()
            .map(Path::to_path_buf)),
        Some(n)
    );
}

#[gpui::test]
fn a_project_file_above_the_document_gives_the_items(cx: &mut TestAppContext) {
    let dir = folder(&["src/main.rs", "src/lib.rs", "README.md"]);
    std::fs::write(
        dir.path().join("_sakurafiletree.ini"),
        "[FileTree]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=0\r\n\
         FileTree(0).szTargetPath=<iniroot>\\src\r\nFileTree(0).szLabelName=Sources\r\n\
         FileTree(0).szTargetFile=*.rs\r\n",
    )
    .unwrap();
    // the default settings have the project mode on; the document is in a folder below
    let document = dir.path().join("src").join("main.rs");
    let handle = window(cx, Some(&document), "");
    run_with(cx, handle, "F_FILETREE", &[]);
    assert_eq!(labels(cx, handle), ["Sources"]);
    toggle(cx, handle, 0);
    assert_eq!(labels(cx, handle), ["Sources", "lib.rs", "main.rs"]);
}

#[gpui::test]
fn the_default_tree_lists_the_folder_of_the_document(cx: &mut TestAppContext) {
    let dir = folder(&["doc.txt", "other.txt", ".hidden"]);
    let document = dir.path().join("doc.txt");
    let handle = window(cx, Some(&document), "");
    run_with(cx, handle, "F_FILETREE", &[]);
    let first = rows(cx, handle);
    // the label is the full path of the folder
    assert_eq!(PathBuf::from(&first[0].label), dir.path().to_path_buf());
    toggle(cx, handle, 0);
    let text = labels(cx, handle);
    // a dot is a hidden file only where the file system has no attribute (not on Windows)
    let expected: Vec<&str> = if cfg!(windows) {
        vec![".hidden", "doc.txt", "other.txt"]
    } else {
        vec!["doc.txt", "other.txt"]
    };
    assert_eq!(&text[1..], expected, "hidden files are left out");
}

#[gpui::test]
fn the_function_shows_toggles_and_gives_way_to_the_outline(cx: &mut TestAppContext) {
    let dir = folder(&["a.txt"]);
    let handle = window(cx, None, &one_grep(dir.path(), "*.*"));
    run_with(cx, handle, "F_FILETREE", &[]);
    // shown again: the same kind only takes the focus; the tree is still there
    run_with(cx, handle, "F_FILETREE", &[]);
    assert_eq!(labels(cx, handle), ["Project"]);
    // the action 1 reads the settings again
    run_with(cx, handle, "F_FILETREE", &[Arg::Int(1)]);
    assert_eq!(labels(cx, handle), ["Project"]);
    // the outline of the type takes the pane over
    run_with(cx, handle, "F_OUTLINE", &[]);
    let p = pane(cx, handle).unwrap();
    assert_ne!(
        p.read_with(cx, |p, _| p.title().to_owned()),
        "ファイルツリー"
    );
    assert!(labels(cx, handle).iter().all(|l| l != "Project"));
    // and back
    run_with(cx, handle, "F_FILETREE", &[]);
    assert_eq!(labels(cx, handle), ["Project"]);
    // the action 2 closes it
    run_with(cx, handle, "F_FILETREE", &[Arg::Int(2)]);
    assert!(pane(cx, handle).is_none());
}

#[gpui::test]
fn the_reload_lists_the_folders_again(cx: &mut TestAppContext) {
    let dir = folder(&["a.txt"]);
    let handle = window(cx, None, &one_grep(dir.path(), "*.*"));
    run_with(cx, handle, "F_FILETREE", &[]);
    toggle(cx, handle, 0);
    assert_eq!(labels(cx, handle), ["Project", "a.txt"]);
    std::fs::write(dir.path().join("b.txt"), "x").unwrap();
    // 更新
    let p = pane(cx, handle).unwrap();
    p.update(cx, |p, cx| p.reload(cx));
    cx.run_until_parked();
    toggle(cx, handle, 0);
    assert_eq!(labels(cx, handle), ["Project", "a.txt", "b.txt"]);
}

#[gpui::test]
fn patterns_that_cannot_be_used_show_a_message(cx: &mut TestAppContext) {
    let dir = folder(&["a.txt"]);
    let handle = window(cx, None, &one_grep(dir.path(), "*\\a.txt"));
    run_with(cx, handle, "F_FILETREE", &[]);
    toggle(cx, handle, 0);
    assert_eq!(labels(cx, handle), ["Project", "<Wild Card Error>"]);
}
