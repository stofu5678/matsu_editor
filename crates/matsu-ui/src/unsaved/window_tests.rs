//! The keeper in GPUI's headless test windows, with a real folder: the copies appear after the text has been
//! still, and go when the document is saved or its tab is closed.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_config::Profile;
use matsu_config::unsaved::Store;

use super::keeper::{flush, housekeeping_at, tick_window_at};
use super::prefs::KeepPrefs;
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::file_ops::windows::{open_editor_window, workspace_of};
use crate::settings::UserSettings;

struct Fixture {
    handle: AnyWindowHandle,
    dir: PathBuf,
    _tmp: tempfile::TempDir,
}

fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(gpui_component::init);
    let tmp = tempfile::tempdir().unwrap();
    let ini = tmp.path().join("matsu.ini");
    let dir = matsu_config::unsaved::unsaved_dir(&ini);
    cx.update(|app| {
        app.set_global(UserSettings::from_profile(Profile::new(), Some(ini)));
        app.set_global(KeepPrefs::default());
    });
    let handle = cx
        .update(|app| open_editor_window(app, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    Fixture {
        handle,
        dir,
        _tmp: tmp,
    }
}

fn workspace(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<Workspace> {
    cx.update(|app| workspace_of(app, handle)).unwrap()
}

fn tick(cx: &mut TestAppContext, f: &Fixture, now: Instant) {
    let ws = workspace(cx, f.handle);
    f.handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| tick_window_at(ws, window, cx, now))
        })
        .unwrap();
    cx.update(|app| flush(app));
}

fn type_text(cx: &mut TestAppContext, f: &Fixture, text: &str) {
    VisualTestContext::from_window(f.handle, cx)
        .into_mut()
        .simulate_input(text);
    cx.run_until_parked();
}

fn doc_files(dir: &std::path::Path) -> Vec<String> {
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

fn read_session(f: &Fixture) -> matsu_config::unsaved::Session {
    // The folder is locked by the writer: read the file directly.
    let text = std::fs::read(f.dir.join("session.ini")).unwrap_or_default();
    let p = Profile::from_bytes(&text);
    let docs = p.get("Session", "Window0.Docs").unwrap_or("").to_owned();
    matsu_config::unsaved::Session {
        windows: if docs.is_empty() {
            vec![]
        } else {
            vec![matsu_config::unsaved::SessionWindow {
                docs: docs.split(',').map(str::to_owned).collect(),
                active: 0,
            }]
        },
        active_window: 0,
    }
}

fn kept_text(f: &Fixture, name: &str) -> String {
    use std::io::Read as _;
    let bytes = std::fs::read(f.dir.join(name)).unwrap();
    // The header is two lines and the information; the text follows. Read it the way the store does.
    let mut lines = bytes.splitn(3, |b| *b == b'\n');
    lines.next();
    let len: usize = std::str::from_utf8(lines.next().unwrap())
        .unwrap()
        .parse()
        .unwrap();
    let rest = lines.next().unwrap();
    let mut text = String::new();
    (&rest[len..]).read_to_string(&mut text).unwrap();
    text
}

#[gpui::test]
fn typed_text_is_kept_after_two_seconds_and_removed_when_saved(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let t0 = Instant::now();
    // Nothing to keep yet.
    tick(cx, &f, t0);
    assert!(doc_files(&f.dir).is_empty());
    type_text(cx, &f, "こんにちは\nworld");
    tick(cx, &f, t0 + Duration::from_secs(1));
    assert!(doc_files(&f.dir).is_empty(), "written before two seconds");
    tick(cx, &f, t0 + Duration::from_secs(4));
    let files = doc_files(&f.dir);
    assert_eq!(files.len(), 1, "{files:?}");
    assert_eq!(kept_text(&f, &files[0]), "こんにちは\nworld");
    let session = read_session(&f);
    assert_eq!(session.windows.len(), 1);
    assert_eq!(
        session.windows[0].docs,
        vec![files[0].trim_end_matches(".doc")]
    );

    // The user saves: the document is unmodified, and the copy goes at the next tick.
    let ws = workspace(cx, f.handle);
    ws.update(cx, |ws, cx| {
        let view = ws.editor().clone();
        view.update(cx, |view, cx| {
            let cursor = view.cursor().clone();
            view.document().update(cx, |shared, _| {
                let (model, _) = shared.parts_mut();
                model.mark_saved(&cursor);
            });
        });
    });
    tick(cx, &f, t0 + Duration::from_secs(5));
    assert!(doc_files(&f.dir).is_empty());
    assert!(read_session(&f).windows.is_empty());
}

#[gpui::test]
fn a_document_that_is_still_being_typed_in_is_written_after_thirty_seconds(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let t0 = Instant::now();
    for s in 0..30 {
        type_text(cx, &f, "x");
        tick(cx, &f, t0 + Duration::from_secs(s));
    }
    assert!(doc_files(&f.dir).is_empty());
    type_text(cx, &f, "y");
    tick(cx, &f, t0 + Duration::from_secs(30));
    assert_eq!(doc_files(&f.dir).len(), 1);
}

#[gpui::test]
fn the_copy_of_a_closed_tab_is_discarded_when_no_window_reports_it(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let t0 = Instant::now();
    type_text(cx, &f, "gone");
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    // The window never ticks again (it was closed with 「保存しない」); the housekeeping finds the document gone.
    cx.update(|app| housekeeping_at(app, t0 + Duration::from_secs(3) + Duration::from_secs(6)));
    cx.update(|app| flush(app));
    assert!(doc_files(&f.dir).is_empty());
    assert!(read_session(&f).windows.is_empty());
}

#[gpui::test]
fn nothing_is_kept_when_the_feature_is_off(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update(|app| {
        app.set_global(KeepPrefs {
            keep: false,
            ..KeepPrefs::default()
        })
    });
    let t0 = Instant::now();
    type_text(cx, &f, "private");
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(10));
    assert!(!f.dir.exists() || doc_files(&f.dir).is_empty());
}

#[gpui::test]
fn a_document_over_the_limit_is_not_kept_and_the_user_is_told(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update(|app| {
        // One byte is already more than the limit of a megabyte, so use a limit of 1 MiB and a big text.
        app.set_global(KeepPrefs {
            max_mb: 1,
            ..KeepPrefs::default()
        })
    });
    let big = "a".repeat(1024 * 1024 + 10);
    let ws = workspace(cx, f.handle);
    ws.update(cx, |ws, cx| {
        let view = ws.editor().clone();
        view.update(cx, |view, cx| {
            let cursor = view.cursor().clone();
            let _ = cursor;
            view.document().update(cx, |shared, _| {
                let (model, _) = shared.parts_mut();
                model
                    .replace(
                        matsu_core::buffer::Pos::new(0, 0),
                        matsu_core::buffer::Pos::new(0, 0),
                        &big,
                    )
                    .unwrap();
            });
        });
    });
    let t0 = Instant::now();
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(10));
    assert!(!f.dir.exists() || doc_files(&f.dir).is_empty());
    let notice = ws.read_with(cx, |ws, app| ws.editor().read(app).notice.clone());
    assert!(notice.is_some_and(|n| n.contains("大きい")));
}

#[gpui::test]
fn a_second_process_does_not_use_the_folder(cx: &mut TestAppContext) {
    let f = fixture(cx);
    // Another process holds the folder.
    let held = Store::open(&f.dir).unwrap();
    let t0 = Instant::now();
    type_text(cx, &f, "mine");
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(10));
    assert!(doc_files(&f.dir).is_empty());
    drop(held);
}

#[gpui::test]
fn clearing_removes_the_copies_and_keeps_the_lock(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let t0 = Instant::now();
    type_text(cx, &f, "to be cleared");
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    cx.update(super::clear_copies);
    assert!(doc_files(&f.dir).is_empty());
    assert!(read_session(&f).windows.is_empty());
    // The folder is still held by this process.
    assert!(matches!(
        Store::open(&f.dir),
        Err(matsu_config::unsaved::StoreError::Locked)
    ));
    // The text is still unsaved: the copy comes back after the next change.
    type_text(cx, &f, "!");
    tick(cx, &f, t0 + Duration::from_secs(10));
    tick(cx, &f, t0 + Duration::from_secs(13));
    assert_eq!(doc_files(&f.dir).len(), 1);
}
