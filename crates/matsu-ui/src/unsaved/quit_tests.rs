//! The end of the editor with `KeepUnsavedNoAsk` (ADR-0022, section 5), in GPUI's headless test windows with a real
//! folder: the copies are written and the questions about them are left out; everything that is not covered is
//! asked as before.

use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext};
use matsu_config::unsaved::Store;

use super::prefs::KeepPrefs;
use super::window_tests::{Fixture, doc_files, fixture, tick, type_text, workspace};
use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;

fn hot(cx: &mut TestAppContext, on: bool) {
    cx.update(|app| {
        app.set_global(KeepPrefs {
            no_ask: on,
            ..KeepPrefs::default()
        })
    });
}

fn run(cx: &mut TestAppContext, f: &Fixture, name: &str) {
    let code = matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == name)
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
}

fn asks(cx: &mut TestAppContext, f: &Fixture) -> bool {
    VisualTestContext::from_window(f.handle, cx)
        .into_mut()
        .has_pending_prompt()
}

fn window_count(cx: &mut TestAppContext) -> usize {
    cx.update(|app| app.windows().len())
}

#[gpui::test]
fn an_exit_of_all_windows_writes_the_latest_text_and_does_not_ask(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "typed just now");
    // No tick yet: the copy does not exist.
    assert!(doc_files(&f.dir).is_empty());
    run(cx, &f, "F_EXITALL");
    assert!(
        !asks(cx, &f),
        "no question about a document whose copy is kept"
    );
    let files = doc_files(&f.dir);
    assert_eq!(files.len(), 1, "the latest text was written before the end");
    let bytes = std::fs::read(f.dir.join(&files[0])).unwrap();
    assert!(String::from_utf8_lossy(&bytes).ends_with("typed just now"));
}

#[gpui::test]
fn without_the_setting_the_exit_asks_as_before(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, false);
    type_text(cx, &f, "typed");
    run(cx, &f, "F_EXITALL");
    assert!(asks(cx, &f));
}

#[gpui::test]
fn the_last_window_closes_without_a_question_and_keeps_its_copy(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "kept for the next start");
    run(cx, &f, "F_WINCLOSE");
    assert!(!asks(cx, &f));
    assert_eq!(window_count(cx), 0, "the window is closed");
    assert_eq!(doc_files(&f.dir).len(), 1, "the copy stays");
}

#[gpui::test]
fn a_window_that_is_not_the_last_asks(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    cx.update(|app| open_editor_window(app, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    type_text(cx, &f, "one of two windows");
    run(cx, &f, "F_WINCLOSE");
    assert!(asks(cx, &f), "closing one window is not the end");
}

#[gpui::test]
fn a_document_whose_copy_cannot_be_kept_is_asked_about(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    // Another process holds the folder: nothing can be kept.
    let held = Store::open(&f.dir).unwrap();
    type_text(cx, &f, "no copy possible");
    run(cx, &f, "F_EXITALL");
    assert!(asks(cx, &f));
    drop(held);
}

#[gpui::test]
fn a_document_over_the_limit_is_asked_about(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update(|app| {
        app.set_global(KeepPrefs {
            no_ask: true,
            max_mb: 1,
            ..KeepPrefs::default()
        })
    });
    let big = "x".repeat(1024 * 1024 + 10);
    let ws = workspace(cx, f.handle);
    ws.update(cx, |ws, cx| {
        ws.editor().clone().update(cx, |view, cx| {
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
    run(cx, &f, "F_EXITALL");
    assert!(asks(cx, &f), "a document that is not kept is asked about");
}

#[cfg(unix)]
#[gpui::test]
fn a_document_whose_write_fails_is_asked_about(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "first");
    let t0 = Instant::now();
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    type_text(cx, &f, " and more");
    std::fs::set_permissions(&f.dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(f.dir.join("probe"), b"x").is_ok() {
        // Not enforced (running as root): nothing to test.
        std::fs::set_permissions(&f.dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    run(cx, &f, "F_EXITALL");
    let asked = asks(cx, &f);
    std::fs::set_permissions(&f.dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(asked, "the latest text could not be written");
}

#[gpui::test]
fn answering_no_to_an_uncovered_document_discards_its_copy_at_the_end(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    let held = Store::open(&f.dir).unwrap();
    type_text(cx, &f, "will be thrown away");
    run(cx, &f, "F_EXITALL");
    assert!(asks(cx, &f));
    let window = VisualTestContext::from_window(f.handle, cx).into_mut();
    window.simulate_prompt_answer("保存しない(N)");
    window.run_until_parked();
    assert!(doc_files(&f.dir).is_empty());
    drop(held);
}
