//! Tests of the dialog 「ファイルツリー設定」 in GPUI's headless test windows: it is opened with the
//! 設定 button of the pane, OK writes the settings file (in a private temporary folder) and the
//! pane reads the tree again.

use std::path::{Path, PathBuf};

use gpui::{AnyWindowHandle, Context, Entity, TestAppContext, VisualTestContext, Window};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_config::Profile;
use ume_config::history::HistoryStore;
use ume_config::settings::file_tree::{FileTreeItem, ItemType};
use ume_core::buffer::Document;
use ume_core::model::DocumentModel;

use super::{FileTreeDialog, Kind};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::file_ops::windows::{open_editor_window, workspace_of};
use crate::history::{HistoryOptions, HistoryState};
use crate::outline::file_tree::source::SettingFrom;
use crate::outline::pane::OutlinePane;
use crate::settings::UserSettings;
use crate::tabs::settings::TabSettings;
use crate::view::EditorView;

/// The settings file `ume.ini` of a test and the folder it is in.
struct Ini {
    dir: tempfile::TempDir,
    path: PathBuf,
}

impl Ini {
    fn profile(&self) -> Profile {
        Profile::load(&self.path).unwrap()
    }

    fn get(&self, section: &str, key: &str) -> Option<String> {
        self.profile().get(section, key).map(str::to_owned)
    }

    fn bytes(&self) -> Vec<u8> {
        std::fs::read(&self.path).unwrap()
    }
}

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// A settings file with `text`, read as `app::run` reads it, and one editor window whose document
/// is `document` (in the folder of the settings file when it is a relative name) or has no file.
fn setup(cx: &mut TestAppContext, text: &str, document: Option<&str>) -> (Ini, AnyWindowHandle) {
    cx.update(gpui_component::init);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ume.ini");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    std::fs::write(&path, bytes).unwrap();
    let doc_path = document.map(|name| dir.path().join(name));
    cx.update(|cx| {
        let user = UserSettings::load_from(std::slice::from_ref(&path));
        cx.set_global(HistoryState::on_disk(
            HistoryStore::new(path.clone(), false),
            HistoryOptions::default(),
        ));
        cx.set_global(user);
        cx.set_global(TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        });
    });
    let doc = Document::from_utf8(b"text\n".to_vec()).unwrap();
    let shared = SharedDocument::new(DocumentModel::new(doc), doc_path);
    let handle = cx
        .update(|cx| open_editor_window(cx, shared, None))
        .unwrap();
    cx.run_until_parked();
    (Ini { dir, path }, handle)
}

fn workspace(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<Workspace> {
    cx.update(|app| workspace_of(app, handle)).unwrap()
}

fn active_view(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.editor().clone())
}

/// Shows the file tree (`F_FILETREE`).
fn show_tree(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    let view = active_view(cx, handle);
    let code = code_of("F_FILETREE");
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| {
                view.run_function(code, &[] as &[Arg], window, cx)
            })
        })
        .unwrap();
    cx.run_until_parked();
}

fn pane(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<OutlinePane> {
    workspace(cx, handle)
        .read_with(cx, |ws, _| ws.outline.clone())
        .expect("the pane is open")
}

fn pane_labels(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<String> {
    pane(cx, handle).read_with(cx, |p, _| {
        p.rows().iter().map(|r| r.label.clone()).collect()
    })
}

fn dialog_of(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<Entity<FileTreeDialog>> {
    active_view(cx, handle).read_with(cx, |v, _| v.support.file_tree_dialog.clone())
}

fn is_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    dialog_of(cx, handle).is_some()
}

/// Opens the dialog the way the button does.
fn open_dialog(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    let p = pane(cx, handle);
    handle
        .update(cx, |_, window, app| {
            p.update(app, |p, cx| p.open_file_tree_settings(window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    assert!(is_open(cx, handle), "the dialog is open");
}

/// Does something to the dialog and its window, as a click or a key does.
fn with_dialog<R>(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    f: impl FnOnce(&mut FileTreeDialog, &mut Window, &mut Context<FileTreeDialog>) -> R,
) -> R {
    let dialog = dialog_of(cx, handle).expect("the dialog is open");
    let result = handle
        .update(cx, |_, window, app| {
            dialog.update(app, |dialog, dcx| f(dialog, window, dcx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

fn set_boxes(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    kind: Kind,
    path: &str,
    label: &str,
    patterns: &str,
) {
    with_dialog(cx, handle, |d, window, dcx| {
        d.kind = kind;
        let values = [
            (d.path.clone(), path),
            (d.label.clone(), label),
            (d.file.clone(), patterns),
        ];
        for (input, text) in values {
            let text = text.to_owned();
            input.update(dcx, |state, cx| state.set_value(text, window, cx));
        }
    });
}

fn tree_items(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<FileTreeItem> {
    dialog_of(cx, handle)
        .unwrap()
        .read_with(cx, |d, _| d.tree.to_items(0).0)
}

fn labels_with_depth(items: &[FileTreeItem]) -> Vec<String> {
    items
        .iter()
        .map(|i| format!("{}{}", super::model::node_label(i), i.depth))
        .collect()
}

fn ok(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    with_dialog(cx, handle, |d, window, dcx| d.ok(window, dcx));
    assert!(!is_open(cx, handle), "OK closes the dialog");
}

fn cancel(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    with_dialog(cx, handle, |d, window, dcx| d.cancel(window, dcx));
    assert!(!is_open(cx, handle), "キャンセル closes the dialog");
}

/// `[Common]` with a Folder item `Notes` that holds the File item `memo`, and another line of
/// the file that must stay as it is.
fn notes_ini(memo: &Path) -> String {
    format!(
        "[Common]\r\nbDispTabWnd=1\r\nbFileTreeProject=0\r\nnFileTreeItemCount=2\r\n\
         FileTree(0).eItemType=2\r\nFileTree(0).szLabelName=Notes\r\nFileTree(0).nDepth=0\r\n\
         FileTree(1).eItemType=1\r\nFileTree(1).szTargetPath={}\r\nFileTree(1).szLabelName=memo\r\nFileTree(1).nDepth=1\r\n\
         [Other]\r\nkeep=1\r\n",
        memo.display()
    )
}

#[gpui::test]
fn the_setting_button_is_there_for_the_file_tree_only(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    let keys = &mut VisualTestContext::from_window(handle, cx);
    // the bookmark list (the other kind of the pane) has no button
    let view = active_view(keys, handle);
    let bookmark = code_of("F_BOOKMARK_VIEW");
    handle
        .update(keys, |_, window, app| {
            view.update(app, |view, cx| {
                view.run_function(bookmark, &[] as &[Arg], window, cx)
            })
        })
        .unwrap();
    keys.run_until_parked();
    assert!(keys.debug_bounds("outline-settings").is_none());
    // the file tree has it
    show_tree(keys, handle);
    assert!(keys.debug_bounds("outline-settings").is_some());
}

#[gpui::test]
fn the_button_opens_the_dialog_with_the_tree_of_the_settings(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    assert_eq!(pane_labels(cx, handle), ["Notes"]);
    let before = ini.bytes();
    let keys = &mut VisualTestContext::from_window(handle, cx);
    let bounds = keys.debug_bounds("outline-settings").expect("the button");
    keys.simulate_click(bounds.center(), gpui::Modifiers::none());
    keys.run_until_parked();
    assert!(is_open(keys, handle), "the click on 設定 opens the dialog");
    let dialog = dialog_of(keys, handle).unwrap();
    dialog.read_with(keys, |d, cx| {
        assert_eq!(d.setting_text(), "設定: 共通設定");
        assert!(!d.project);
        assert!(d.saves_to_settings());
        assert_eq!(d.default_ini.read(cx).value().to_string(), "");
        // the first item is selected and shown: a Folder item
        assert_eq!(d.kind, Kind::Folder);
        assert_eq!(d.label.read(cx).value().to_string(), "Notes");
        assert!(!d.path_enabled());
        assert!(!d.grep_enabled());
        assert_eq!(d.tree.count(), 2);
        assert_eq!(
            labels_with_depth(&d.tree.to_items(0).0),
            ["Notes0", "memo1"]
        );
    });
    // キャンセル changes nothing, in the file or in the tree
    cancel(keys, handle);
    assert_eq!(ini.bytes(), before);
    assert_eq!(pane_labels(keys, handle), ["Notes"]);
}

#[gpui::test]
fn ok_writes_the_items_in_sakuras_keys_and_the_pane_reads_the_tree_again(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    let docs = tempfile::tempdir().unwrap();
    show_tree(cx, handle);
    open_dialog(cx, handle);
    // `>>` with the Folder item selected: the new item goes last into it
    set_boxes(
        cx,
        handle,
        Kind::Grep,
        &docs.path().to_string_lossy(),
        "Docs",
        "*.txt",
    );
    with_dialog(cx, handle, |d, window, dcx| d.add_item(window, dcx));
    assert_eq!(
        labels_with_depth(&tree_items(cx, handle)),
        ["Notes0", "memo1", "Docs1"]
    );
    ok(cx, handle);

    let docs_path = docs.path().to_string_lossy().into_owned();
    assert_eq!(
        ini.get("Common", "nFileTreeItemCount").as_deref(),
        Some("3")
    );
    assert_eq!(ini.get("Common", "bFileTreeProject").as_deref(), Some("0"));
    assert_eq!(
        ini.get("Common", "szFileTreeProjectIni").as_deref(),
        Some("")
    );
    assert_eq!(
        ini.get("Common", "FileTree(0).eItemType").as_deref(),
        Some("2")
    );
    assert_eq!(
        ini.get("Common", "FileTree(0).szLabelName").as_deref(),
        Some("Notes")
    );
    assert_eq!(
        ini.get("Common", "FileTree(0).nDepth").as_deref(),
        Some("0")
    );
    assert_eq!(
        ini.get("Common", "FileTree(1).eItemType").as_deref(),
        Some("1")
    );
    assert_eq!(
        ini.get("Common", "FileTree(1).nDepth").as_deref(),
        Some("1")
    );
    assert_eq!(
        ini.get("Common", "FileTree(2).eItemType").as_deref(),
        Some("0")
    );
    assert_eq!(
        ini.get("Common", "FileTree(2).szTargetPath").as_deref(),
        Some(docs_path.as_str())
    );
    assert_eq!(
        ini.get("Common", "FileTree(2).szLabelName").as_deref(),
        Some("Docs")
    );
    assert_eq!(
        ini.get("Common", "FileTree(2).nDepth").as_deref(),
        Some("1")
    );
    assert_eq!(
        ini.get("Common", "FileTree(2).szTargetFile").as_deref(),
        Some("*.txt")
    );
    assert_eq!(
        ini.get("Common", "FileTree(2).bIgnoreHidden").as_deref(),
        Some("1")
    );
    assert_eq!(
        ini.get("Common", "FileTree(2).bIgnoreReadOny").as_deref(),
        Some("0")
    );
    assert_eq!(
        ini.get("Common", "FileTree(2).bIgnoreSystem").as_deref(),
        Some("0")
    );
    // the rest of the file is as it was
    assert_eq!(ini.get("Common", "bDispTabWnd").as_deref(), Some("1"));
    assert_eq!(ini.get("Other", "keep").as_deref(), Some("1"));

    // the settings the windows hold are the new ones, and the pane shows the tree of them
    let user_count = cx.update(|app| {
        app.global::<UserSettings>()
            .profile()
            .and_then(|p| p.get("Common", "nFileTreeItemCount").map(str::to_owned))
    });
    assert_eq!(user_count.as_deref(), Some("3"));
    let p = pane(cx, handle);
    p.update(cx, |p, cx| p.toggle_node(0, cx));
    cx.run_until_parked();
    let labels = pane_labels(cx, handle);
    assert_eq!(labels[0], "Notes");
    assert!(labels.contains(&"Docs".to_owned()), "{labels:?}");
}

#[gpui::test]
fn nothing_is_written_when_nothing_changed(cx: &mut TestAppContext) {
    // the keys of Sakura are written in the order of the profile: a file that already has them
    // all, as written by this dialog, stays byte for byte
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    ok(cx, handle);
    let once = ini.bytes();
    open_dialog(cx, handle);
    ok(cx, handle);
    assert_eq!(ini.bytes(), once, "the second OK writes the same file");
}

#[gpui::test]
fn the_settings_of_the_type_are_the_ones_edited_when_the_dock_follows_the_type(
    cx: &mut TestAppContext,
) {
    let text = "[Common]\r\nnOutlineDockSet=1\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=2\r\n\
                FileTree(0).szLabelName=Common\r\nFileTree(0).nDepth=0\r\n\
                [Types(0)]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=2\r\n\
                FileTree(0).szLabelName=OfType\r\nFileTree(0).nDepth=0\r\n";
    let (ini, handle) = setup(cx, text, None);
    show_tree(cx, handle);
    assert_eq!(pane_labels(cx, handle), ["OfType"]);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, _, _| {
        assert_eq!(d.setting_text(), "設定: タイプ別設定");
    });
    set_boxes(cx, handle, Kind::Folder, "", "Second", "");
    with_dialog(cx, handle, |d, window, dcx| d.insert_below(window, dcx));
    ok(cx, handle);
    assert_eq!(
        ini.get("Types(0)", "nFileTreeItemCount").as_deref(),
        Some("2")
    );
    assert_eq!(
        ini.get("Types(0)", "FileTree(0).szLabelName").as_deref(),
        Some("OfType")
    );
    assert_eq!(
        ini.get("Types(0)", "FileTree(1).szLabelName").as_deref(),
        Some("Second")
    );
    // inserted below a Folder item: inside it
    assert_eq!(
        ini.get("Types(0)", "FileTree(1).nDepth").as_deref(),
        Some("1")
    );
    // [Common] is not touched
    assert_eq!(
        ini.get("Common", "nFileTreeItemCount").as_deref(),
        Some("1")
    );
    assert_eq!(
        ini.get("Common", "FileTree(0).szLabelName").as_deref(),
        Some("Common")
    );
    assert_eq!(ini.get("Common", "bFileTreeProject"), None);
    assert_eq!(
        ini.get("Types(0)", "bFileTreeProject").as_deref(),
        Some("1")
    );
}

#[gpui::test]
fn a_project_file_found_above_the_document_gets_the_items(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbFileTreeProject=1\r\nnFileTreeItemCount=1\r\n\
                FileTree(0).eItemType=2\r\nFileTree(0).szLabelName=Settings\r\nFileTree(0).nDepth=0\r\n";
    let (ini, handle) = setup(cx, text, Some("doc.txt"));
    let project = ini.dir.path().join("_sakurafiletree.ini");
    std::fs::write(
        &project,
        "; my comment\r\n[FileTree]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=2\r\n\
         FileTree(0).szLabelName=Project\r\nFileTree(0).nDepth=0\r\n[Mine]\r\nx=1\r\n",
    )
    .unwrap();
    show_tree(cx, handle);
    assert_eq!(pane_labels(cx, handle), ["Project"]);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, _, _| {
        assert_eq!(d.loaded_from(), SettingFrom::File);
        assert!(d.setting_text().starts_with("設定: "));
        assert!(d.setting_text().ends_with("_sakurafiletree.ini"));
        assert!(!d.saves_to_settings());
        assert_eq!(d.tree.count(), 1);
    });
    set_boxes(cx, handle, Kind::Folder, "", "Added", "");
    with_dialog(cx, handle, |d, window, dcx| d.add_item(window, dcx));
    ok(cx, handle);
    // the items went to the project file; its other lines stay
    let written = Profile::load(&project).unwrap();
    assert_eq!(written.get("FileTree", "nFileTreeItemCount"), Some("2"));
    assert_eq!(
        written.get("FileTree", "FileTree(1).szLabelName"),
        Some("Added")
    );
    assert_eq!(written.get("FileTree", "FileTree(1).nDepth"), Some("1"));
    assert_eq!(written.get("Mine", "x"), Some("1"));
    assert!(
        std::fs::read_to_string(&project)
            .unwrap()
            .contains("; my comment")
    );
    // the settings keep their own items; only the flag and the file name are written
    assert_eq!(ini.get("Common", "bFileTreeProject").as_deref(), Some("1"));
    assert_eq!(
        ini.get("Common", "nFileTreeItemCount").as_deref(),
        Some("1")
    );
    assert_eq!(
        ini.get("Common", "FileTree(0).szLabelName").as_deref(),
        Some("Settings")
    );
    assert_eq!(pane_labels(cx, handle), ["Project"]);
}

#[gpui::test]
fn the_project_flag_and_the_default_file_name_are_written(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, window, dcx| {
        d.set_project(true, dcx);
        d.default_ini
            .update(dcx, |state, cx| state.set_value("tree.ini", window, cx));
    });
    ok(cx, handle);
    assert_eq!(ini.get("Common", "bFileTreeProject").as_deref(), Some("1"));
    assert_eq!(
        ini.get("Common", "szFileTreeProjectIni").as_deref(),
        Some("tree.ini")
    );
    assert_eq!(
        ini.get("Common", "nFileTreeItemCount").as_deref(),
        Some("2")
    );
}

#[gpui::test]
fn the_load_button_reads_the_default_file_and_ok_writes_the_items_back_to_it(
    cx: &mut TestAppContext,
) {
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    let project = ini.dir.path().join("shared.ini");
    std::fs::write(
        &project,
        "[FileTree]\r\nnFileTreeItemCount=2\r\nFileTree(0).eItemType=2\r\nFileTree(0).szLabelName=Shared\r\n\
         FileTree(0).nDepth=0\r\nFileTree(1).eItemType=1\r\nFileTree(1).szTargetPath=<iniroot>\\a.txt\r\n\
         FileTree(1).nDepth=1\r\n",
    )
    .unwrap();
    show_tree(cx, handle);
    open_dialog(cx, handle);
    // a file that is not there reads nothing
    with_dialog(cx, handle, |d, window, dcx| {
        d.default_ini
            .update(dcx, |state, cx| state.set_value("missing.ini", window, cx));
        d.load_default_ini(window, dcx);
        assert!(d.saves_to_settings());
        assert_eq!(d.tree.count(), 2);
    });
    // a relative name starts at the folder of the settings file
    with_dialog(cx, handle, |d, window, dcx| {
        d.default_ini
            .update(dcx, |state, cx| state.set_value("shared.ini", window, cx));
        d.load_default_ini(window, dcx);
        assert!(!d.saves_to_settings());
        assert_eq!(d.setting_text(), "設定: 共通設定+ファイル");
    });
    assert_eq!(
        labels_with_depth(&tree_items(cx, handle)),
        ["Shared0", r"<iniroot>\a.txt1"]
    );
    // 隠しファイル: a new item goes into the loaded tree, and OK writes the file
    set_boxes(cx, handle, Kind::File, "b.txt", "B", "");
    with_dialog(cx, handle, |d, window, dcx| d.add_item(window, dcx));
    ok(cx, handle);
    let written = Profile::load(&project).unwrap();
    assert_eq!(written.get("FileTree", "nFileTreeItemCount"), Some("3"));
    assert_eq!(
        written.get("FileTree", "FileTree(2).szTargetPath"),
        Some("b.txt")
    );
    assert_eq!(
        ini.get("Common", "szFileTreeProjectIni").as_deref(),
        Some("shared.ini")
    );
    assert_eq!(
        ini.get("Common", "nFileTreeItemCount").as_deref(),
        Some("2")
    );
    assert_eq!(
        ini.get("Common", "FileTree(1).szLabelName").as_deref(),
        Some("memo")
    );
}

#[gpui::test]
fn the_default_file_is_read_when_the_dialog_opens(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbFileTreeProject=0\r\nszFileTreeProjectIni=shared.ini\r\n";
    let (ini, handle) = setup(cx, text, None);
    std::fs::write(
        ini.dir.path().join("shared.ini"),
        "[FileTree]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=2\r\nFileTree(0).szLabelName=Shared\r\n\
         FileTree(0).nDepth=0\r\n",
    )
    .unwrap();
    show_tree(cx, handle);
    assert_eq!(pane_labels(cx, handle), ["Shared"]);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, _, dcx| {
        assert_eq!(d.setting_text(), "設定: 共通設定+ファイル");
        assert_eq!(d.default_ini.read(dcx).value().to_string(), "shared.ini");
        assert!(!d.saves_to_settings());
    });
}

#[gpui::test]
fn the_tree_holds_twenty_items_when_it_is_saved_in_the_settings(cx: &mut TestAppContext) {
    let mut text = String::from("[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=20\r\n");
    for i in 0..20 {
        text.push_str(&format!(
            "FileTree({i}).eItemType=1\r\nFileTree({i}).szTargetPath=f{i}.txt\r\nFileTree({i}).nDepth=0\r\n"
        ));
    }
    let (ini, handle) = setup(cx, &text, None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, window, dcx| {
        assert_eq!(d.tree.count(), 20);
        assert!(!d.can_add(), "the 21st does not fit");
        d.add_item(window, dcx);
        d.insert_above(window, dcx);
        d.insert_below(window, dcx);
        assert_eq!(d.tree.count(), 20);
        // the files can still be added; OK cuts them off
        d.insert_files(&[PathBuf::from("extra.txt")], window, dcx);
        assert_eq!(d.tree.count(), 21);
    });
    ok(cx, handle);
    assert_eq!(
        ini.get("Common", "nFileTreeItemCount").as_deref(),
        Some("20")
    );
    assert_eq!(
        ini.get("Common", "FileTree(19).szTargetPath").as_deref(),
        Some("f19.txt")
    );
    assert_eq!(ini.get("Common", "FileTree(20).szTargetPath"), None);
}

#[gpui::test]
fn more_than_twenty_items_in_the_settings_say_so(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let items: Vec<FileTreeItem> = (0..22)
        .map(|i| FileTreeItem {
            item_type: ItemType::File,
            target_path: format!("f{i}"),
            ..FileTreeItem::default()
        })
        .collect();
    let keys = &mut VisualTestContext::from_window(handle, cx);
    with_dialog(keys, handle, |d, window, dcx| {
        d.set_data(&items, window, dcx)
    });
    assert!(keys.has_pending_prompt());
    keys.simulate_prompt_answer("OK");
    ok(keys, handle);
}

#[gpui::test]
fn delete_asks_before_it_takes_an_item_with_items_below(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let keys = &mut VisualTestContext::from_window(handle, cx);
    // Notes has memo below it
    with_dialog(keys, handle, |d, window, dcx| d.delete_item(window, dcx));
    assert!(keys.has_pending_prompt());
    keys.simulate_prompt_answer("キャンセル");
    keys.run_until_parked();
    assert_eq!(tree_items(keys, handle).len(), 2);
    with_dialog(keys, handle, |d, window, dcx| d.delete_item(window, dcx));
    keys.simulate_prompt_answer("OK");
    keys.run_until_parked();
    assert!(tree_items(keys, handle).is_empty());
    // the boxes show the blank item then
    with_dialog(keys, handle, |d, _, dcx| {
        assert!(d.tree.selected().is_none());
        assert_eq!(d.kind, Kind::Grep);
        assert_eq!(d.file.read(dcx).value().to_string(), "*.*");
        assert!(d.hidden && !d.read_only && !d.system);
    });
}

#[gpui::test]
fn an_item_without_items_below_is_deleted_without_asking(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, window, dcx| {
        d.select_row(&vec![0, 0], window, dcx);
        assert_eq!(d.kind, Kind::File);
        d.delete_item(window, dcx);
    });
    assert!(!VisualTestContext::from_window(handle, cx).has_pending_prompt());
    assert_eq!(labels_with_depth(&tree_items(cx, handle)), ["Notes0"]);
}

#[gpui::test]
fn the_boxes_follow_the_selected_item_and_the_kind_decides_what_counts(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=2\r\n\
                FileTree(0).eItemType=0\r\nFileTree(0).szTargetPath=C:\\src\r\nFileTree(0).szLabelName=Src\r\n\
                FileTree(0).nDepth=0\r\nFileTree(0).szTargetFile=*.rs;*.toml\r\nFileTree(0).bIgnoreHidden=0\r\n\
                FileTree(0).bIgnoreReadOny=1\r\nFileTree(0).bIgnoreSystem=1\r\n\
                FileTree(1).eItemType=1\r\nFileTree(1).szTargetPath=C:\\a.txt\r\nFileTree(1).nDepth=0\r\n";
    let (ini, handle) = setup(cx, text, None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, _, dcx| {
        assert_eq!(d.kind, Kind::Grep);
        assert_eq!(d.path.read(dcx).value().to_string(), "C:\\src");
        assert_eq!(d.file.read(dcx).value().to_string(), "*.rs;*.toml");
        assert!(!d.hidden && d.read_only && d.system);
        assert!(d.path_enabled() && d.grep_enabled());
    });
    with_dialog(cx, handle, |d, window, dcx| {
        d.select_row(&vec![1], window, dcx);
        assert_eq!(d.kind, Kind::File);
        assert_eq!(d.path.read(dcx).value().to_string(), "C:\\a.txt");
        assert!(d.path_enabled() && !d.grep_enabled());
    });
    // 更新 with the File kind keeps only what belongs to it
    set_boxes(cx, handle, Kind::File, "C:\\b.txt", "Bee", "*.zzz");
    with_dialog(cx, handle, |d, _, dcx| d.update_item(dcx));
    // and a Grep item turned into a Folder item loses its path
    with_dialog(cx, handle, |d, window, dcx| {
        d.select_row(&vec![0], window, dcx);
        d.set_kind(Kind::Folder, dcx);
        d.update_item(dcx);
    });
    ok(cx, handle);
    assert_eq!(
        ini.get("Common", "FileTree(0).eItemType").as_deref(),
        Some("2")
    );
    assert_eq!(
        ini.get("Common", "FileTree(0).szLabelName").as_deref(),
        Some("Src")
    );
    assert_eq!(ini.get("Common", "FileTree(0).szTargetPath"), None);
    assert_eq!(ini.get("Common", "FileTree(0).szTargetFile"), None);
    assert_eq!(
        ini.get("Common", "FileTree(1).szTargetPath").as_deref(),
        Some("C:\\b.txt")
    );
    assert_eq!(
        ini.get("Common", "FileTree(1).szLabelName").as_deref(),
        Some("Bee")
    );
    assert_eq!(ini.get("Common", "FileTree(1).szTargetFile"), None);
}

#[gpui::test]
fn update_does_not_turn_an_item_with_children_into_another_kind(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, window, dcx| {
        assert!(d.can_update());
        d.set_kind(Kind::File, dcx);
        assert!(!d.can_update(), "Notes would lose its place for memo");
        d.update_item(dcx);
        assert_eq!(d.tree.selected_item().unwrap().item_type, ItemType::Folder);
        d.select_row(&vec![0, 0], window, dcx);
        assert!(d.can_update());
    });
}

#[gpui::test]
fn files_are_added_with_their_names_and_percent_signs_doubled(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, window, dcx| {
        d.insert_files(
            &[
                PathBuf::from("/data/100%.txt"),
                PathBuf::from("/data/日本語.md"),
            ],
            window,
            dcx,
        );
    });
    let items = tree_items(cx, handle);
    assert_eq!(
        labels_with_depth(&items),
        ["Notes0", "100%.txt1", "日本語.md1", "memo1"]
    );
    assert_eq!(items[1].target_path, "/data/100%%.txt");
    assert_eq!(items[1].item_type, ItemType::File);
    with_dialog(cx, handle, |d, _, _| {
        assert_eq!(d.tree.selected(), Some(&vec![0, 0]));
    });
}

#[gpui::test]
fn the_buttons_move_the_items(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=4\r\n\
                FileTree(0).eItemType=2\r\nFileTree(0).szLabelName=F\r\nFileTree(0).nDepth=0\r\n\
                FileTree(1).eItemType=1\r\nFileTree(1).szTargetPath=a\r\nFileTree(1).nDepth=0\r\n\
                FileTree(2).eItemType=1\r\nFileTree(2).szTargetPath=b\r\nFileTree(2).nDepth=0\r\n\
                FileTree(3).eItemType=1\r\nFileTree(3).szTargetPath=c\r\nFileTree(3).nDepth=0\r\n";
    let (ini, handle) = setup(cx, text, None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    // → puts `a` into the folder F before it; ← takes it out again, right after F
    with_dialog(cx, handle, |d, window, dcx| {
        d.select_row(&vec![1], window, dcx);
        d.move_right(window, dcx);
    });
    assert_eq!(
        labels_with_depth(&tree_items(cx, handle)),
        ["F0", "a1", "b0", "c0"]
    );
    with_dialog(cx, handle, |d, window, dcx| d.move_left(window, dcx));
    assert_eq!(
        labels_with_depth(&tree_items(cx, handle)),
        ["F0", "a0", "b0", "c0"]
    );
    // ↓ and ↑ change places
    with_dialog(cx, handle, |d, window, dcx| d.move_down(window, dcx));
    assert_eq!(
        labels_with_depth(&tree_items(cx, handle)),
        ["F0", "b0", "a0", "c0"]
    );
    with_dialog(cx, handle, |d, _, dcx| {
        d.move_up(dcx);
        d.move_up(dcx);
    });
    assert_eq!(
        labels_with_depth(&tree_items(cx, handle)),
        ["a0", "F0", "b0", "c0"]
    );
    ok(cx, handle);
    assert_eq!(
        ini.get("Common", "FileTree(0).szTargetPath").as_deref(),
        Some("a")
    );
    assert_eq!(
        ini.get("Common", "FileTree(1).szLabelName").as_deref(),
        Some("F")
    );
    assert_eq!(
        ini.get("Common", "FileTree(3).szTargetPath").as_deref(),
        Some("c")
    );
}

#[gpui::test]
fn the_list_of_the_arrow_puts_a_meta_folder_into_the_path(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=0\r\n\
                FileTree(0).szTargetPath=C:\\work\r\nFileTree(0).nDepth=0\r\n";
    let (_ini, handle) = setup(cx, text, None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    with_dialog(cx, handle, |d, window, dcx| {
        d.path
            .update(dcx, |state, cx| state.set_value("", window, cx));
        d.toggle_path_menu(dcx);
        assert!(d.path_menu);
        let (_, text) = super::actions::PATH_MENU[1];
        d.put_in_path(text, window, dcx);
        assert!(!d.path_menu);
        assert_eq!(d.path.read(dcx).value().to_string(), "%MYDOC%");
        // not for a Folder item
        d.set_kind(Kind::Folder, dcx);
        d.toggle_path_menu(dcx);
        assert!(!d.path_menu);
    });
}

#[gpui::test]
fn the_texts_of_the_list_are_sakuras_meta_folders(_cx: &mut TestAppContext) {
    let texts: Vec<&str> = super::actions::PATH_MENU.iter().map(|(_, t)| *t).collect();
    assert_eq!(
        texts,
        [
            "<iniroot>",
            "%MYDOC%",
            "%MYMUSIC%",
            "%MYVIDEO%",
            "%DESKTOP%",
            "%TEMP%",
            "%SAKURA%",
            "%SAKURADATA%"
        ]
    );
}

#[gpui::test]
fn the_replace_asks_twice_and_changes_the_path_of_every_item(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=2\r\nFileTree(0).eItemType=0\r\n\
                FileTree(0).szTargetPath=C:\\old\\a\r\nFileTree(0).nDepth=0\r\n\
                FileTree(1).eItemType=1\r\nFileTree(1).szTargetPath=C:\\old\\b.txt\r\nFileTree(1).nDepth=0\r\n";
    let (_ini, handle) = setup(cx, text, None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let keys = &mut VisualTestContext::from_window(handle, cx);
    with_dialog(keys, handle, |d, window, dcx| d.replace_paths(window, dcx));
    let view = active_view(keys, handle);
    let asking = |keys: &mut VisualTestContext| {
        view.read_with(keys, |v, cx| {
            v.input_dialog
                .as_ref()
                .map(|d| d.read(cx).message().to_owned())
        })
    };
    assert_eq!(
        asking(keys).as_deref(),
        Some("置換元パスを指定してください")
    );
    keys.simulate_input("C:\\old");
    keys.simulate_keystrokes("enter");
    keys.run_until_parked();
    assert_eq!(
        asking(keys).as_deref(),
        Some("置換先文字列を指定してください")
    );
    keys.simulate_input("D:\\新しい");
    keys.simulate_keystrokes("enter");
    keys.run_until_parked();
    assert_eq!(asking(keys), None);
    let items = tree_items(keys, handle);
    assert_eq!(items[0].target_path, "D:\\新しい\\a");
    assert_eq!(items[1].target_path, "D:\\新しい\\b.txt");
    // the box of the selected item shows the new path
    dialog_of(keys, handle).unwrap().read_with(keys, |d, cx| {
        assert_eq!(d.path.read(cx).value().to_string(), "D:\\新しい\\a");
    });
}

#[gpui::test]
fn cancelling_the_first_question_of_the_replace_changes_nothing(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=1\r\n\
                FileTree(0).szTargetPath=C:\\old\r\nFileTree(0).nDepth=0\r\n";
    let (_ini, handle) = setup(cx, text, None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let keys = &mut VisualTestContext::from_window(handle, cx);
    with_dialog(keys, handle, |d, window, dcx| d.replace_paths(window, dcx));
    keys.simulate_input("C");
    keys.simulate_keystrokes("escape");
    keys.run_until_parked();
    assert_eq!(tree_items(keys, handle)[0].target_path, "C:\\old");
    let view = active_view(keys, handle);
    assert!(view.read_with(keys, |v, _| v.input_dialog.is_none()));
    assert!(is_open(keys, handle), "the dialog stays");
}

#[gpui::test]
fn the_export_and_the_import_round_trip_through_a_file(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let file = ini.dir.path().join("out.ini");
    let keys = &mut VisualTestContext::from_window(handle, cx);
    with_dialog(keys, handle, |d, window, dcx| {
        d.export_to(&file, window, dcx)
    });
    // the message of the export
    assert!(keys.has_pending_prompt());
    keys.simulate_prompt_answer("OK");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("SakuraEditorFileTree_Ver1"), "{text}");
    assert!(text.contains("[FileTree]"));
    let exported = Profile::load(&file).unwrap();
    assert_eq!(exported.get("FileTree", "nFileTreeItemCount"), Some("2"));
    assert_eq!(
        exported.get("FileTree", "FileTree(0).szLabelName"),
        Some("Notes")
    );
    // empty the tree, then import the file
    with_dialog(keys, handle, |d, window, dcx| {
        d.set_data(&[], window, dcx);
        assert_eq!(d.tree.count(), 0);
        d.import_from(&file, window, dcx);
    });
    assert_eq!(
        labels_with_depth(&tree_items(keys, handle)),
        ["Notes0", "memo1"]
    );
    // the import changes the tree; the file is written at OK only
    ok(keys, handle);
}

#[gpui::test]
fn an_import_of_a_file_without_items_leaves_the_tree(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let other = ini.dir.path().join("other.ini");
    std::fs::write(&other, "[Something]\r\nx=1\r\n").unwrap();
    with_dialog(cx, handle, |d, window, dcx| {
        d.import_from(&other, window, dcx)
    });
    assert_eq!(
        labels_with_depth(&tree_items(cx, handle)),
        ["Notes0", "memo1"]
    );
    // a file that is not there says so
    let missing = ini.dir.path().join("missing.ini");
    with_dialog(cx, handle, |d, window, dcx| {
        d.import_from(&missing, window, dcx)
    });
    assert!(VisualTestContext::from_window(handle, cx).has_pending_prompt());
}

#[gpui::test]
fn escape_cancels_and_the_access_keys_use_the_buttons(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let keys = &mut VisualTestContext::from_window(handle, cx);
    // Alt+3 opens the list of the path, Escape closes the list first
    with_dialog(keys, handle, |d, window, dcx| {
        d.set_kind(Kind::Grep, dcx);
        assert!(d.use_access_key('3', window, dcx));
        assert!(d.path_menu);
    });
    keys.simulate_keystrokes("escape");
    assert!(is_open(keys, handle));
    keys.simulate_keystrokes("escape");
    assert!(!is_open(keys, handle), "Esc is キャンセル");
    assert_eq!(
        ini.get("Common", "nFileTreeItemCount").as_deref(),
        Some("2")
    );
    // Alt+B is `>>`, and Alt+O is OK
    open_dialog(keys, handle);
    keys.simulate_keystrokes("alt-b");
    assert_eq!(tree_items(keys, handle).len(), 3);
    keys.simulate_keystrokes("alt-o");
    assert!(!is_open(keys, handle));
    assert_eq!(
        ini.get("Common", "nFileTreeItemCount").as_deref(),
        Some("3")
    );
}

#[gpui::test]
fn every_access_key_of_the_labels_is_used(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    // `IDD_FILETREE`: the keys of the check boxes, radio buttons, labels and buttons
    let keys = [
        'i', 'j', 'g', 'f', '2', 't', 'e', 'k', 'w', 'z', 's', '4', 'q', 'a', 'b', 'p', 'u', 'd',
        'r', 'l', '3',
    ];
    for key in keys {
        // (what the keys that change the tree do is tested above: only the answer matters here)
        let used = with_dialog(cx, handle, |d, window, dcx| {
            d.set_kind(Kind::Grep, dcx);
            d.use_access_key(key, window, dcx)
        });
        assert!(used, "Alt+{key}");
        if !is_open(cx, handle) {
            break;
        }
    }
    assert!(
        !with_dialog(cx, handle, |d, window, dcx| d
            .use_access_key('9', window, dcx)),
        "a key that no label has"
    );
}

#[gpui::test]
fn the_arrow_keys_walk_the_tree_and_open_the_folders(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let keys = &mut VisualTestContext::from_window(handle, cx);
    let selected = |keys: &mut VisualTestContext| {
        dialog_of(keys, handle)
            .unwrap()
            .read_with(keys, |d, _| d.tree.selected().cloned())
    };
    assert_eq!(selected(keys), Some(vec![0]));
    // Notes is closed: Down has nowhere to go; Right opens it, Right again enters it
    keys.simulate_keystrokes("down");
    assert_eq!(selected(keys), Some(vec![0]));
    keys.simulate_keystrokes("right");
    assert_eq!(
        dialog_of(keys, handle)
            .unwrap()
            .read_with(keys, |d, _| d.tree.rows().len()),
        2
    );
    keys.simulate_keystrokes("right");
    assert_eq!(selected(keys), Some(vec![0, 0]));
    dialog_of(keys, handle).unwrap().read_with(keys, |d, cx| {
        assert_eq!(d.kind, Kind::File, "the boxes show the item");
        assert_eq!(d.label.read(cx).value().to_string(), "memo");
    });
    // Left goes to the parent, and closes it
    keys.simulate_keystrokes("left");
    assert_eq!(selected(keys), Some(vec![0]));
    keys.simulate_keystrokes("left");
    assert_eq!(
        dialog_of(keys, handle)
            .unwrap()
            .read_with(keys, |d, _| d.tree.rows().len()),
        1
    );
}

#[gpui::test]
fn the_dialog_covers_the_view_and_is_one_at_a_time(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let first = dialog_of(cx, handle).unwrap();
    let p = pane(cx, handle);
    handle
        .update(cx, |_, window, app| {
            p.update(app, |p, cx| p.open_file_tree_settings(window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        dialog_of(cx, handle).unwrap().entity_id(),
        first.entity_id()
    );
    // it is not for the bookmark list
    cancel(cx, handle);
}

#[gpui::test]
fn a_project_file_that_cannot_be_written_keeps_the_dialog_and_the_edits(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbFileTreeProject=1\r\n";
    let (ini, handle) = setup(cx, text, Some("doc.txt"));
    let project = ini.dir.path().join("_sakurafiletree.ini");
    std::fs::write(
        &project,
        "[FileTree]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=2\r\nFileTree(0).szLabelName=P\r\n",
    )
    .unwrap();
    show_tree(cx, handle);
    open_dialog(cx, handle);
    set_boxes(cx, handle, Kind::Folder, "", "Added", "");
    with_dialog(cx, handle, |d, window, dcx| d.add_item(window, dcx));
    // the file goes away and a folder takes its name
    std::fs::remove_file(&project).unwrap();
    std::fs::create_dir(&project).unwrap();
    let before = ini.bytes();
    let keys = &mut VisualTestContext::from_window(handle, cx);
    with_dialog(keys, handle, |d, window, dcx| d.ok(window, dcx));
    assert!(keys.has_pending_prompt(), "the error is told");
    keys.simulate_prompt_answer("OK");
    assert!(is_open(keys, handle), "the dialog stays");
    assert_eq!(tree_items(keys, handle).len(), 2, "the edits stay");
    assert_eq!(ini.bytes(), before, "the settings are not written either");
}

#[gpui::test]
fn enter_in_the_tree_is_ok(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let keys = &mut VisualTestContext::from_window(handle, cx);
    keys.simulate_keystrokes("enter");
    assert!(!is_open(keys, handle));
    assert_eq!(
        ini.get("Common", "nFileTreeItemCount").as_deref(),
        Some("2")
    );
}

#[gpui::test]
fn the_keys_choose_from_the_open_list_of_the_path(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=0\r\n\
                FileTree(0).szTargetPath=\r\nFileTree(0).nDepth=0\r\n";
    let (_ini, handle) = setup(cx, text, None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    let keys = &mut VisualTestContext::from_window(handle, cx);
    let path = |keys: &mut VisualTestContext| {
        dialog_of(keys, handle)
            .unwrap()
            .read_with(keys, |d, cx| d.path.read(cx).value().to_string())
    };
    // Alt+3 opens the list; Down and Enter choose the second entry (the tree does not move)
    keys.simulate_keystrokes("alt-3");
    keys.simulate_keystrokes("down enter");
    assert_eq!(path(keys), "%MYDOC%");
    assert!(
        is_open(keys, handle),
        "Enter chose an entry; it did not end the dialog"
    );
    // the letter of an entry chooses it: `T` is Temp
    keys.simulate_keystrokes("alt-3");
    keys.simulate_keystrokes("t");
    assert!(path(keys).ends_with("%TEMP%"), "{}", path(keys));
    // a key the list does not know is left alone, Escape closes the list and not the dialog
    keys.simulate_keystrokes("alt-3");
    keys.simulate_keystrokes("escape");
    assert!(is_open(keys, handle));
    dialog_of(keys, handle)
        .unwrap()
        .read_with(keys, |d, _| assert!(!d.path_menu));
}

#[gpui::test]
fn each_entry_of_the_list_has_its_access_key(_cx: &mut TestAppContext) {
    let keys: Vec<Option<char>> = super::actions::PATH_MENU
        .iter()
        .map(|(label, _)| super::actions::menu_key_of(label))
        .collect();
    assert_eq!(
        keys,
        [
            Some('i'),
            Some('y'),
            Some('m'),
            Some('v'),
            Some('d'),
            Some('t'),
            Some('s'),
            Some('a')
        ]
    );
}

#[cfg(unix)]
#[gpui::test]
fn a_settings_file_that_cannot_be_written_keeps_the_dialog(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt as _;
    let (ini, handle) = setup(cx, &notes_ini(Path::new("memo.txt")), None);
    show_tree(cx, handle);
    open_dialog(cx, handle);
    set_boxes(cx, handle, Kind::Folder, "", "Added", "");
    with_dialog(cx, handle, |d, window, dcx| d.add_item(window, dcx));
    let dir = ini.dir.path();
    let writable = std::fs::metadata(dir).unwrap().permissions();
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let before = ini.bytes();
    // (a user who can write anyway, such as root, cannot run this test)
    let can_write = std::fs::File::create(dir.join("probe")).is_ok();
    let keys = &mut VisualTestContext::from_window(handle, cx);
    with_dialog(keys, handle, |d, window, dcx| d.ok(window, dcx));
    std::fs::set_permissions(dir, writable).unwrap();
    if can_write {
        return;
    }
    assert!(keys.has_pending_prompt(), "the failure is told");
    keys.simulate_prompt_answer("OK");
    assert!(is_open(keys, handle), "OK seems to do nothing otherwise");
    assert_eq!(ini.bytes(), before);
    assert_eq!(tree_items(keys, handle).len(), 3, "the edits stay");
}

#[test]
fn the_name_of_the_setting_button_has_no_access_key_mark() {
    let name = crate::a11y::without_key_hint(&crate::a11y::plain_name("設定(S)"));
    assert_eq!(name, "設定");
}
