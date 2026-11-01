//! Tests of the histories of the Grep dialog: what OK keeps, the file, and the lists of the boxes.
//! The lists themselves are tested in `matsu_config::settings::search_keys`.
//!
//! Expectations cite `CDlgGrep::GetData`, `CDlgGrepReplace::GetData` and
//! `sakura_core/env/CSearchKeywordManager.cpp`.

use std::path::PathBuf;
use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::Document;
use matsu_core::model::DocumentModel;

use super::panel::GrepPanel;
use crate::document::SharedDocument;
use crate::search::history::{self, GrepBoxes, SearchHistory};
use crate::view::EditorView;

struct Fixture {
    view: Entity<EditorView>,
}

fn setup(cx: &mut TestAppContext) -> (Fixture, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(Vec::new()).unwrap();
    let shared = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.executor().allow_parking();
    (Fixture { view }, vcx)
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..50 {
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(60));
    }
}

impl Fixture {
    fn open(&self, cx: &mut VisualTestContext, function: &str) -> Entity<GrepPanel> {
        let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
        self.view.update_in(cx, |view, window, cx| {
            view.run_function(code, &[], window, cx)
        });
        settle(cx);
        self.view
            .read_with(cx, |v, _| v.grep.panel.clone())
            .expect("the dialog is open")
    }
}

fn fill(cx: &mut VisualTestContext, panel: &Entity<GrepPanel>, boxes: [&str; 5]) {
    panel.update_in(cx, |p, window, cx| {
        let [key, file, folder, exclude_file, exclude_folder] = boxes;
        for (state, text) in [
            (&p.key, key),
            (&p.file, file),
            (&p.folder, folder),
            (&p.exclude_file, exclude_file),
            (&p.exclude_folder, exclude_folder),
        ] {
            state.update(cx, |s, cx| s.set_value(text, window, cx));
        }
    });
}

fn temp_ini(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "matsu-ui-grep-history-{}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("matsu.ini")
}

fn lists(cx: &mut VisualTestContext) -> [Vec<String>; 5] {
    cx.update(|_, app| {
        [
            history::grep_files(app),
            history::grep_folders(app),
            history::exclude_files(app),
            history::exclude_folders(app),
            history::search_keys(app),
        ]
    })
}

#[gpui::test]
fn ok_keeps_the_boxes_in_the_histories(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_string_lossy().into_owned();
    let panel = f.open(cx, "F_GREP_DIALOG");
    fill(cx, &panel, ["key", "*.txt;*.md", &root, "*.bak", ".git"]);
    panel.update_in(cx, |p, window, cx| p.ok(window, cx));
    settle(cx);
    let [files, folders, exclude_files, exclude_folders, keys] = lists(cx);
    assert_eq!(files, ["*.txt;*.md"]);
    // the folder is kept as the dialog wrote it back (absolute)
    assert_eq!(folders.len(), 1);
    assert!(folders[0].ends_with(root.rsplit(['/', '\\']).next().unwrap()));
    assert_eq!(exclude_files, ["*.bak"]);
    assert_eq!(exclude_folders, [".git"]);
    assert_eq!(keys, ["key"]);
}

#[gpui::test]
fn a_box_left_empty_leaves_no_blank_line(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_string_lossy().into_owned();
    let panel = f.open(cx, "F_GREP_DIALOG");
    fill(cx, &panel, ["key", "*.*", &root, "", ""]);
    panel.update_in(cx, |p, window, cx| p.ok(window, cx));
    settle(cx);
    let [_, _, exclude_files, exclude_folders, _] = lists(cx);
    assert!(exclude_files.is_empty() && exclude_folders.is_empty());
}

#[gpui::test]
fn a_search_that_the_dialog_refuses_keeps_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let panel = f.open(cx, "F_GREP_DIALOG");
    // the folder is not there
    fill(
        cx,
        &panel,
        ["key", "*.txt", "/no/such/folder/at/all", "", ""],
    );
    panel.update_in(cx, |p, window, cx| p.ok(window, cx));
    settle(cx);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("OK");
    assert!(lists(cx).iter().all(Vec::is_empty));
}

#[gpui::test]
fn the_histories_are_written_to_the_file_with_sakuras_keys(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx);
    let path = temp_ini("file");
    std::fs::write(&path, "\u{feff}[Other]\r\nnExecFlgOpt=1\r\n").unwrap();
    cx.update(|_, app| {
        history::install(app, SearchHistory::load(Some(&path), Some(path.clone())));
        history::remember_grep_boxes(
            app,
            &GrepBoxes {
                file: "*.c",
                folder: "C:\\src",
                exclude_file: "*.obj",
                exclude_folder: "Debug",
            },
        );
        history::remember_grep_boxes(
            app,
            &GrepBoxes {
                file: "*.h",
                folder: "C:\\src",
                exclude_file: "*.obj",
                exclude_folder: "",
            },
        );
        history::remember_tag_keyword(app, "WinMain");
    });
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("nExecFlgOpt=1\r\n"), "{written}");
    assert!(written.contains("[Grep]\r\n"), "{written}");
    assert!(written.contains("GREPFILE[00]=*.h\r\n"), "{written}");
    assert!(written.contains("GREPFILE[01]=*.c\r\n"), "{written}");
    assert!(written.contains("_GREPFOLDER_Counts=1\r\n"), "{written}");
    assert!(
        written.contains("GREPEXCLUDEFILE[00]=*.obj\r\n"),
        "{written}"
    );
    assert!(
        written.contains("GREPEXCLUDEFOLDER[00]=Debug\r\n"),
        "{written}"
    );
    assert!(
        written.contains("TagJumpKeyword[00]=WinMain\r\n"),
        "{written}"
    );
    // a new run reads them back
    let again = SearchHistory::load(Some(&path), None);
    assert_eq!(again.keys().grep_file.items(), ["*.h", "*.c"]);
    assert_eq!(again.keys().tag_keyword.items(), ["WinMain"]);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn two_windows_share_the_histories(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx);
    let path = temp_ini("two");
    // another process saved a folder after this one started
    let mut a = SearchHistory::load(None, Some(path.clone()));
    let mut b = SearchHistory::load(None, Some(path.clone()));
    a.persist(|keys| {
        keys.add_grep_folder("from a");
    });
    b.persist(|keys| {
        keys.add_exclude_file("from b");
    });
    let file = SearchHistory::load(Some(&path), None);
    assert_eq!(file.keys().grep_folder.items(), ["from a"]);
    assert_eq!(file.keys().exclude_file.items(), ["from b"]);
    assert_eq!(b.keys().grep_folder.items(), ["from a"]);
    let _ = cx;
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn the_list_of_a_box_opens_and_enter_picks_from_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    cx.update(|_, app| {
        for file in ["*.a", "*.b", "*.c"] {
            history::remember_grep_boxes(
                app,
                &GrepBoxes {
                    file,
                    folder: "",
                    exclude_file: "",
                    exclude_folder: "",
                },
            );
        }
    });
    let panel = f.open(cx, "F_GREP_DIALOG");
    fill(cx, &panel, ["", "zzz", "", "", ""]);
    panel.update_in(cx, |p, window, cx| {
        p.file.update(cx, |s, cx| s.focus(window, cx));
    });
    cx.simulate_keystrokes("alt-down down enter");
    // c, b, a: "zzz" is not in the list, so it opens on the first; one Down reaches b
    let file = panel.read_with(cx, |p, app| p.file.read(app).value().to_string());
    assert_eq!(file, "*.b");
    // the dialog is still open: Enter picked, it did not search
    assert!(f.view.read_with(cx, |v, _| v.grep_panel_open()));
}

#[gpui::test]
fn escape_closes_the_list_first_and_then_the_dialog(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    cx.update(|_, app| {
        history::remember_grep_boxes(
            app,
            &GrepBoxes {
                file: "*.a",
                folder: "",
                exclude_file: "",
                exclude_folder: "",
            },
        );
    });
    let panel = f.open(cx, "F_GREP_DIALOG");
    panel.update_in(cx, |p, window, cx| {
        p.file.update(cx, |s, cx| s.focus(window, cx));
    });
    cx.simulate_keystrokes("alt-down escape");
    assert!(f.view.read_with(cx, |v, _| v.grep_panel_open()));
    cx.simulate_keystrokes("escape");
    assert!(!f.view.read_with(cx, |v, _| v.grep_panel_open()));
}

#[gpui::test]
fn the_replace_dialog_keeps_the_replacement(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "nothing\n").unwrap();
    let root = dir.path().to_string_lossy().into_owned();
    let panel = f.open(cx, "F_GREP_REPLACE_DLG");
    fill(cx, &panel, ["needle", "*.txt", &root, "", ""]);
    panel.update_in(cx, |p, window, cx| {
        p.replacement
            .update(cx, |s, cx| s.set_value("pin", window, cx));
        p.ok(window, cx);
    });
    settle(cx);
    assert_eq!(cx.update(|_, app| history::replace_keys(app)), ["pin"]);
    assert_eq!(cx.update(|_, app| history::grep_files(app)), ["*.txt"]);
}

#[gpui::test]
fn the_dialog_has_a_list_for_each_of_its_boxes(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    cx.update(|_, app| {
        history::remember_grep_boxes(
            app,
            &GrepBoxes {
                file: "f1",
                folder: "d1",
                exclude_file: "xf1",
                exclude_folder: "xd1",
            },
        );
        history::remember_search(app, "k1");
        history::remember_replace(app, "r1");
    });
    let panel = f.open(cx, "F_GREP_REPLACE_DLG");
    for (pick, expect) in [
        ("file", "f1"),
        ("folder", "d1"),
        ("exclude_file", "xf1"),
        ("exclude_folder", "xd1"),
        ("key", "k1"),
        ("replacement", "r1"),
    ] {
        panel.update_in(cx, |p, window, cx| {
            let state = match pick {
                "file" => &p.file,
                "folder" => &p.folder,
                "exclude_file" => &p.exclude_file,
                "exclude_folder" => &p.exclude_folder,
                "key" => &p.key,
                _ => &p.replacement,
            };
            state.update(cx, |s, cx| {
                s.set_value("", window, cx);
                s.focus(window, cx);
            });
        });
        cx.simulate_keystrokes("alt-down enter");
        let got = panel.read_with(cx, |p, app| {
            let state = match pick {
                "file" => &p.file,
                "folder" => &p.folder,
                "exclude_file" => &p.exclude_file,
                "exclude_folder" => &p.exclude_folder,
                "key" => &p.key,
                _ => &p.replacement,
            };
            state.read(app).value().to_string()
        });
        assert_eq!(got, expect, "the list of the {pick} box");
    }
}
