//! The keeper in GPUI's headless test windows, with a real folder: the copies appear after the text has been
//! still, and go when the document is saved or its tab is closed.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_config::Profile;
use matsu_config::unsaved::Store;

use super::keeper::{app_tick, flush, tick_window_at};
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
fn the_copy_of_a_document_whose_window_is_closed_is_discarded(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let t0 = Instant::now();
    type_text(cx, &f, "closed");
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    // The window goes (closed with 「保存しない」, or by a way that does not ask); no window ticks again.
    f.handle
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    cx.update(|app| app_tick(app, t0 + Duration::from_millis(3500)));
    cx.update(|app| flush(app));
    assert!(doc_files(&f.dir).is_empty());
    assert!(read_session(&f).windows.is_empty());
}

#[gpui::test]
fn a_window_that_stalls_does_not_lose_its_copy(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let t0 = Instant::now();
    type_text(cx, &f, "slow");
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    // The window did not tick for a minute (a long operation); the app's timer runs meanwhile.
    cx.update(|app| app_tick(app, t0 + Duration::from_secs(60)));
    cx.update(|app| flush(app));
    assert_eq!(doc_files(&f.dir).len(), 1);
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
    let ws = workspace(cx, f.handle);
    let notice = ws.read_with(cx, |ws, app| ws.editor().read(app).notice.clone());
    assert!(notice.is_some_and(|n| n.contains("別のマツエディタ")));
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

#[gpui::test]
fn the_copies_stay_while_the_feature_is_off_and_no_window_ticks(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let t0 = Instant::now();
    type_text(cx, &f, "kept on purpose");
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    // The user turned it off and chose 「残す」: the windows stop ticking, the housekeeping goes on.
    cx.update(|app| {
        app.set_global(KeepPrefs {
            keep: false,
            ..KeepPrefs::default()
        })
    });
    for s in [10, 20, 30] {
        cx.update(|app| app_tick(app, t0 + Duration::from_secs(s)));
    }
    cx.update(|app| flush(app));
    assert_eq!(doc_files(&f.dir).len(), 1);
    assert_eq!(read_session(&f).windows.len(), 1);
    // Turned on again: the window is not taken for a new one, and the copy is neither lost nor doubled.
    cx.update(|app| app.set_global(KeepPrefs::default()));
    tick(cx, &f, t0 + Duration::from_secs(31));
    cx.update(|app| app_tick(app, t0 + Duration::from_secs(32)));
    cx.update(|app| flush(app));
    assert_eq!(doc_files(&f.dir).len(), 1);
}

#[gpui::test]
fn a_document_that_is_closed_or_saved_loses_its_copy_at_once(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let t0 = Instant::now();
    type_text(cx, &f, "thrown away");
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    // What `confirm_discard` does when the user answers 「保存しない」: no tick, no wait.
    let ws = workspace(cx, f.handle);
    let document = ws.read_with(cx, |ws, app| ws.editor().read(app).document().entity_id());
    cx.update(|app| super::discard_document(document, app));
    cx.update(|app| flush(app));
    assert!(doc_files(&f.dir).is_empty());
    assert!(read_session(&f).windows.is_empty());
    // The document is still open and unsaved here, so the next change makes a new copy.
    type_text(cx, &f, "!");
    tick(cx, &f, t0 + Duration::from_secs(10));
    tick(cx, &f, t0 + Duration::from_secs(13));
    assert_eq!(doc_files(&f.dir).len(), 1);
}

#[gpui::test]
fn answering_dont_save_to_the_close_question_discards_the_copy_before_any_tick(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let t0 = Instant::now();
    type_text(cx, &f, "will be closed");
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    let code = matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == "F_WINCLOSE")
        .unwrap()
        .code;
    let ws = workspace(cx, f.handle);
    let view = ws.read_with(cx, |ws, _| ws.editor().clone());
    f.handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    let window = VisualTestContext::from_window(f.handle, cx).into_mut();
    assert!(
        window.has_pending_prompt(),
        "the question about the changes"
    );
    window.simulate_prompt_answer("保存しない(N)");
    window.run_until_parked();
    cx.update(|app| flush(app));
    assert!(doc_files(&f.dir).is_empty());
    assert!(read_session(&f).windows.is_empty());
}
