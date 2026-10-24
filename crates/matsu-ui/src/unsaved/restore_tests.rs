//! The restore in GPUI's headless test windows, with a real folder: documents, windows and tabs come back as they
//! were, the keeper takes them over, and what cannot be read is kept in `rejected/`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_config::Profile;
use matsu_config::unsaved::{DocInfo, Session, SessionWindow, Store};
use matsu_core::buffer::Pos;

use super::keeper::{flush, tick_window_at};
use super::prefs::KeepPrefs;
use super::restore::restore_all;
use crate::app::Workspace;
use crate::file_ops::windows::{workspace_of, workspaces};
use crate::settings::UserSettings;
use crate::tabs::settings::TabSettings;
use crate::view::EditorView;

struct Fixture {
    tmp: tempfile::TempDir,
    dir: PathBuf,
}

fn fixture(cx: &mut TestAppContext, tabs: bool) -> Fixture {
    cx.update(gpui_component::init);
    let tmp = tempfile::tempdir().unwrap();
    let ini = tmp.path().join("matsu.ini");
    let dir = matsu_config::unsaved::unsaved_dir(&ini);
    cx.update(|app| {
        app.set_global(UserSettings::from_profile(Profile::new(), Some(ini)));
        app.set_global(KeepPrefs::default());
        app.set_global(TabSettings {
            show_bar: tabs,
            grouped: tabs,
            new_window: false,
        });
    });
    Fixture { tmp, dir }
}

fn info(path: Option<&Path>) -> DocInfo {
    DocInfo {
        path: path.map(|p| p.to_string_lossy().into_owned()),
        encoding: "UTF-8".to_owned(),
        line_ending: "LF".to_owned(),
        ..DocInfo::default()
    }
}

/// Writes a kept document and returns its id.
fn keep(store: &Store, info: &DocInfo, text: &str) -> String {
    let id = Store::new_id();
    store
        .write_doc(&id, info, &mut |w| {
            std::io::Write::write_all(w, text.as_bytes())
        })
        .unwrap();
    id
}

fn one_window(docs: &[&str]) -> SessionWindow {
    SessionWindow {
        docs: docs.iter().map(|d| (*d).to_owned()).collect(),
        active: 0,
    }
}

/// The text of the document of a view.
fn text_of(cx: &mut TestAppContext, view: &Entity<EditorView>) -> String {
    view.read_with(cx, |view, app| {
        let doc = view.document().read(app).doc();
        let mut out = String::new();
        for i in 0..doc.line_count() {
            let line = doc.line(i).unwrap().unwrap();
            out.push_str(&line.text);
            if let Some(eol) = line.eol {
                out.push_str(eol.as_str());
            }
        }
        out
    })
}

fn windows(cx: &mut TestAppContext) -> Vec<(AnyWindowHandle, Entity<Workspace>)> {
    cx.update(workspaces)
}

fn tabs_of(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> Vec<Entity<EditorView>> {
    ws.read_with(cx, |ws, _| ws.tab_editors())
}

fn doc_files(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".doc"))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[gpui::test]
fn windows_tabs_and_text_come_back(cx: &mut TestAppContext) {
    let f = fixture(cx, true);
    let store = Store::open(&f.dir).unwrap();
    let mut untitled = info(None);
    untitled.untitled_number = Some(7);
    untitled.bookmarks = vec![0, 2, 99];
    untitled.caret_line = 1;
    untitled.caret_column = 3;
    let a = keep(&store, &untitled, "first\nsecond\nthird");
    let b = keep(&store, &info(None), "日本語\r\nの文書");
    let c = keep(&store, &info(None), "other window");
    store
        .write_session(&Session {
            windows: vec![
                SessionWindow {
                    docs: vec![a.clone(), b.clone()],
                    active: 1,
                },
                one_window(&[&c]),
            ],
            active_window: 0,
        })
        .unwrap();
    drop(store);

    let restored = cx.update(restore_all);
    cx.run_until_parked();
    assert_eq!(restored.documents, 3);
    let list = windows(cx);
    assert_eq!(list.len(), 2, "one window for each window of the session");
    let first = tabs_of(cx, &list[0].1);
    assert_eq!(first.len(), 2, "two tabs");
    assert_eq!(text_of(cx, &first[0]), "first\nsecond\nthird");
    assert_eq!(text_of(cx, &first[1]), "日本語\r\nの文書");
    let second = tabs_of(cx, &list[1].1);
    assert_eq!(text_of(cx, &second[0]), "other window");
    // The tab that was in front.
    assert_eq!(list[0].1.read_with(cx, |ws, _| ws.active_tab()), 1);

    // The state of the first document.
    first[0].read_with(cx, |view, app| {
        let shared = view.document().read(app);
        assert!(shared.model().is_modified(), "it is not saved");
        assert!(!shared.model().history().can_undo(), "Undo starts here");
        assert_eq!(shared.untitled_number(), Some(7));
        assert_eq!(
            shared.model().bookmarks().iter().collect::<Vec<_>>(),
            vec![0, 2],
            "a bookmark past the end is dropped"
        );
        assert_eq!(view.cursor().caret(), Pos::new(1, 3));
    });
}

#[gpui::test]
fn an_untitled_number_is_not_given_twice(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let store = Store::open(&f.dir).unwrap();
    let mut i = info(None);
    i.untitled_number = Some(5);
    let id = keep(&store, &i, "x");
    store
        .write_session(&Session {
            windows: vec![one_window(&[&id])],
            active_window: 0,
        })
        .unwrap();
    drop(store);
    cx.update(restore_all);
    cx.run_until_parked();
    let mut fresh = crate::document::SharedDocument::empty();
    assert!(fresh.number_untitled() > 5);
}

#[gpui::test]
fn encoding_bom_line_ending_and_the_file_come_back(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let file = f.tmp.path().join("named.txt");
    std::fs::write(&file, "on disk").unwrap();
    let meta = std::fs::metadata(&file).unwrap();
    let modified = meta.modified().unwrap();
    let since = modified.duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let mut i = info(Some(&file));
    i.encoding = matsu_core::encoding::Encoding::ShiftJis
        .sakura_code()
        .to_string();
    i.bom = false;
    i.line_ending = "CRLF".to_owned();
    i.file_modified_secs = Some(since.as_secs() as i64);
    i.file_modified_nanos = Some(since.subsec_nanos());
    i.file_size = Some(meta.len());
    let store = Store::open(&f.dir).unwrap();
    let id = keep(&store, &i, "edited text");
    store
        .write_session(&Session {
            windows: vec![one_window(&[&id])],
            active_window: 0,
        })
        .unwrap();
    drop(store);

    let restored = cx.update(restore_all);
    cx.run_until_parked();
    assert_eq!(restored.documents, 1);
    assert_eq!(restored.changed_files, 0, "the file is as it was");
    let list = windows(cx);
    let view = tabs_of(cx, &list[0].1).remove(0);
    view.read_with(cx, |view, app| {
        let shared = view.document().read(app);
        assert_eq!(shared.path(), Some(file.as_path()));
        assert_eq!(
            shared.file().encoding,
            matsu_core::encoding::Encoding::ShiftJis
        );
        assert_eq!(shared.new_line(), matsu_core::buffer::Eol::CrLf);
        assert!(
            shared.restored_check().is_some(),
            "the file is not looked at yet"
        );
        assert!(shared.model().is_modified());
    });
}

#[gpui::test]
fn a_file_that_changed_is_said_and_checked_before_the_save(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let file = f.tmp.path().join("named.txt");
    std::fs::write(&file, "original").unwrap();
    let mut i = info(Some(&file));
    // The file was different when the copy was made.
    i.file_modified_secs = Some(1_000_000);
    i.file_modified_nanos = Some(0);
    i.file_size = Some(3);
    let store = Store::open(&f.dir).unwrap();
    let id = keep(&store, &i, "my version");
    store
        .write_session(&Session {
            windows: vec![one_window(&[&id])],
            active_window: 0,
        })
        .unwrap();
    drop(store);
    let restored = cx.update(restore_all);
    cx.run_until_parked();
    assert_eq!(restored.changed_files, 1);
    let list = windows(cx);
    let handle = list[0].0;
    let view = tabs_of(cx, &list[0].1).remove(0);
    let notice = view.read_with(cx, |v, _| v.notice.clone()).unwrap();
    assert!(notice.contains("前回の未保存の文書を1件戻しました"));
    assert!(notice.contains("ディスク上のファイルが変わっています"));

    // The watching of files notices the file at once (the document keeps the time stamp it was opened with) and
    // asks, as it does for any modified document whose file changed. The user chooses to be only told.
    {
        let window = VisualTestContext::from_window(handle, cx).into_mut();
        assert!(window.has_pending_prompt(), "the question of the watching");
        window.simulate_prompt_answer("以後通知メッセージのみ(M)");
        window.run_until_parked();
    }

    // Saving asks first. キャンセル leaves the file alone.
    let code = matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == "F_FILESAVE")
        .unwrap()
        .code;
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    let window = VisualTestContext::from_window(handle, cx).into_mut();
    assert!(window.has_pending_prompt(), "the question before the save");
    window.simulate_prompt_answer("キャンセル");
    window.run_until_parked();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "original");

    // Again, and this time the user overwrites.
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    let window = VisualTestContext::from_window(handle, cx).into_mut();
    assert!(window.has_pending_prompt());
    window.simulate_prompt_answer("上書きする(Y)");
    window.run_until_parked();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "my version");
    // The check is over.
    assert!(view.read_with(cx, |v, app| {
        v.document().read(app).restored_check().is_none()
    }));
}

#[gpui::test]
fn a_file_that_is_as_it_was_is_saved_without_a_question(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let file = f.tmp.path().join("same.txt");
    std::fs::write(&file, "disk").unwrap();
    let meta = std::fs::metadata(&file).unwrap();
    let since = meta
        .modified()
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let mut i = info(Some(&file));
    i.file_modified_secs = Some(since.as_secs() as i64);
    i.file_modified_nanos = Some(since.subsec_nanos());
    i.file_size = Some(meta.len());
    let store = Store::open(&f.dir).unwrap();
    let id = keep(&store, &i, "kept version");
    store
        .write_session(&Session {
            windows: vec![one_window(&[&id])],
            active_window: 0,
        })
        .unwrap();
    drop(store);
    cx.update(restore_all);
    cx.run_until_parked();
    let list = windows(cx);
    let handle = list[0].0;
    let view = tabs_of(cx, &list[0].1).remove(0);
    let code = matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == "F_FILESAVE")
        .unwrap()
        .code;
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    let window = VisualTestContext::from_window(handle, cx).into_mut();
    assert!(!window.has_pending_prompt());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "kept version");
}

#[gpui::test]
fn the_keeper_takes_the_copies_over_and_makes_no_second_one(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let store = Store::open(&f.dir).unwrap();
    let id = keep(&store, &info(None), "kept");
    store
        .write_session(&Session {
            windows: vec![one_window(&[&id])],
            active_window: 0,
        })
        .unwrap();
    drop(store);
    cx.update(restore_all);
    cx.run_until_parked();
    let list = windows(cx);
    let t0 = Instant::now();
    for s in [0, 3, 6, 30] {
        let ws = list[0].1.clone();
        list[0]
            .0
            .update(cx, |_, window, app| {
                ws.update(app, |ws, cx| {
                    tick_window_at(ws, window, cx, t0 + Duration::from_secs(s), false);
                })
            })
            .unwrap();
        cx.update(|app| flush(app));
    }
    assert_eq!(doc_files(&f.dir), vec![format!("{id}.doc")]);
    // The session is the one of the live windows, and lists the same copy.
    let session = std::fs::read_to_string(f.dir.join("session.ini")).unwrap();
    assert!(session.contains(&id));
    // When the document changes, the same file is written again.
    let view = tabs_of(cx, &list[0].1).remove(0);
    VisualTestContext::from_window(list[0].0, cx)
        .into_mut()
        .simulate_input("+");
    cx.run_until_parked();
    for s in [40, 43] {
        let ws = list[0].1.clone();
        list[0]
            .0
            .update(cx, |_, window, app| {
                ws.update(app, |ws, cx| {
                    tick_window_at(ws, window, cx, t0 + Duration::from_secs(s), false);
                })
            })
            .unwrap();
        cx.update(|app| flush(app));
    }
    assert_eq!(doc_files(&f.dir), vec![format!("{id}.doc")]);
    let _ = view;
}

#[gpui::test]
fn an_unreadable_file_goes_to_rejected_and_the_rest_comes_back(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let store = Store::open(&f.dir).unwrap();
    let good = keep(&store, &info(None), "good one");
    let bad = Store::new_id();
    std::fs::write(f.dir.join(format!("{bad}.doc")), b"not a document file").unwrap();
    store
        .write_session(&Session {
            windows: vec![one_window(&[&bad, &good])],
            active_window: 0,
        })
        .unwrap();
    drop(store);
    let restored = cx.update(restore_all);
    cx.run_until_parked();
    assert_eq!(restored.documents, 1);
    assert_eq!(restored.rejected, 1);
    let kept: Vec<String> = std::fs::read_dir(f.dir.join("rejected"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(kept.len(), 1);
    assert!(kept[0].starts_with(&bad));
    let list = windows(cx);
    let view = tabs_of(cx, &list[0].1).remove(0);
    assert_eq!(text_of(cx, &view), "good one");
    let notice = view.read_with(cx, |v, _| v.notice.clone()).unwrap();
    assert!(notice.contains("rejected"));
}

#[gpui::test]
fn files_that_the_session_does_not_list_come_back_and_gone_ones_are_skipped(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, false);
    let store = Store::open(&f.dir).unwrap();
    let listed = keep(&store, &info(None), "listed");
    let orphan = keep(&store, &info(None), "orphan");
    let gone = Store::new_id();
    store
        .write_session(&Session {
            windows: vec![one_window(&[&listed, &gone])],
            active_window: 0,
        })
        .unwrap();
    drop(store);
    let restored = cx.update(restore_all);
    cx.run_until_parked();
    assert_eq!(restored.documents, 2);
    let mut texts = Vec::new();
    for (_, ws) in windows(cx) {
        for view in tabs_of(cx, &ws) {
            texts.push(text_of(cx, &view));
        }
    }
    texts.sort();
    assert_eq!(texts, vec!["listed", "orphan"]);
    let _ = orphan;
}

#[gpui::test]
fn nothing_is_restored_when_the_feature_is_off_or_the_folder_is_in_use(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let store = Store::open(&f.dir).unwrap();
    let id = keep(&store, &info(None), "kept");
    store
        .write_session(&Session {
            windows: vec![one_window(&[&id])],
            active_window: 0,
        })
        .unwrap();
    // Another process holds the folder.
    assert_eq!(cx.update(restore_all).documents, 0);
    drop(store);
    cx.update(|app| {
        app.set_global(KeepPrefs {
            keep: false,
            ..KeepPrefs::default()
        })
    });
    assert_eq!(cx.update(restore_all).documents, 0);
    assert!(windows(cx).is_empty());
    assert_eq!(
        doc_files(&f.dir),
        vec![format!("{id}.doc")],
        "left as it is"
    );
}

#[gpui::test]
fn no_folder_is_made_at_the_first_start(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    assert_eq!(cx.update(restore_all).documents, 0);
    assert!(!f.dir.exists());
}

#[gpui::test]
fn a_file_named_on_the_command_line_that_came_back_is_not_opened_twice(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let file = f.tmp.path().join("named.txt");
    std::fs::write(&file, "disk").unwrap();
    let store = Store::open(&f.dir).unwrap();
    let id = keep(&store, &info(Some(&file)), "kept");
    store
        .write_session(&Session {
            windows: vec![one_window(&[&id])],
            active_window: 0,
        })
        .unwrap();
    drop(store);
    cx.update(restore_all);
    cx.run_until_parked();
    assert_eq!(windows(cx).len(), 1);
    cx.update(|app| crate::file_ops::windows::open_from_outside(app, vec![file.clone()]))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(windows(cx).len(), 1, "no second window for the same file");
    let _ = workspace_of;
}

#[test]
fn a_launch_that_asks_for_nothing_but_a_window_is_told_from_one_that_asks_more() {
    use crate::command_line::CommandLine;
    use crate::file_ops::start_options::LaunchOptions;
    let plain = LaunchOptions::new(&CommandLine::default(), Path::new("/work"));
    assert!(plain.asks_only_for_a_window());
    let line = CommandLine {
        grep_mode: true,
        ..CommandLine::default()
    };
    assert!(!LaunchOptions::new(&line, Path::new("/work")).asks_only_for_a_window());
}

#[gpui::test]
fn a_restored_document_is_saved_in_its_own_encoding(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let file = f.tmp.path().join("sjis.txt");
    std::fs::write(&file, b"x").unwrap();
    let meta = std::fs::metadata(&file).unwrap();
    let since = meta
        .modified()
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let mut i = info(Some(&file));
    i.encoding = matsu_core::encoding::Encoding::ShiftJis
        .sakura_code()
        .to_string();
    i.file_modified_secs = Some(since.as_secs() as i64);
    i.file_modified_nanos = Some(since.subsec_nanos());
    i.file_size = Some(meta.len());
    let store = Store::open(&f.dir).unwrap();
    let id = keep(&store, &i, "日本語");
    store
        .write_session(&Session {
            windows: vec![one_window(&[&id])],
            active_window: 0,
        })
        .unwrap();
    drop(store);
    cx.update(restore_all);
    cx.run_until_parked();
    let list = windows(cx);
    let handle = list[0].0;
    let view = tabs_of(cx, &list[0].1).remove(0);
    let code = matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == "F_FILESAVE")
        .unwrap()
        .code;
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    // Shift_JIS, not the UTF-8 the copy has.
    assert_eq!(
        std::fs::read(&file).unwrap(),
        vec![0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA]
    );
}
